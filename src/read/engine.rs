use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

use serde::de::DeserializeOwned;
use sternegit_reader_protocol::{
    ABI, Answer, Call, Failure, Hello, Model, PROTOCOL, ReadInput, Search, speaks_same_major,
};
use wasmtime::{Caller, Linker, Memory, Module, Store};

use super::Tree;
use crate::plugin::{
    ENTRY_EXPORT, FINISH, FUEL_PER_SLICE, Guest, Import, LOG, Limits, MEMORY_EXPORT, NOW, Stop, TAKE, Ty, compile,
    engine, fault, guest_memory, length, link_common, span, trap_message, unless_cancelled,
};

pub(crate) const MEMORY_LIMIT_BYTES: usize = 512 * 1024 * 1024;
const SLICES_PER_READ: u32 = 1_000;
const LOG_BYTES_PER_CALL: usize = 64 * 1024;
const READER: &str = "reader";
const ABSENT: i32 = -1;

const READ_FILE: Import = Import {
    name: "read_file",
    params: &[Ty::I32, Ty::I32],
    results: &[Ty::I32],
};

const SEARCH: Import = Import {
    name: "search",
    params: &[Ty::I32, Ty::I32],
    results: &[Ty::I32],
};

const IMPORTS: [Import; 6] = [TAKE, FINISH, LOG, NOW, READ_FILE, SEARCH];

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ReadError {
    Stopped(Stop),
    Failed(Failure),
}

impl fmt::Display for ReadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadError::Stopped(stop) => f.write_str(&stop.told_of(READER)),
            ReadError::Failed(failure) => failure.fmt(f),
        }
    }
}

pub(crate) struct Reader {
    module: Module,
    linker: Linker<ReadCall>,
}

struct ReadCall {
    tree: Arc<dyn Tree>,
    limits: Limits,
    memory: Option<Memory>,
    waiting: Vec<u8>,
    output: Option<Vec<u8>>,
    log: Vec<String>,
    logged_bytes: usize,
}

impl Guest for ReadCall {
    fn memory(&self) -> Option<Memory> {
        self.memory
    }

    fn waiting(&self) -> &[u8] {
        &self.waiting
    }

    fn finish(&mut self, output: &[u8]) -> Result<(), String> {
        if self.output.is_some() {
            return Err("the call already has its answer".to_owned());
        }
        self.output = Some(output.to_vec());
        Ok(())
    }

    fn log(&mut self, _: i32, text: &[u8]) {
        if self.logged_bytes + text.len() <= LOG_BYTES_PER_CALL {
            self.logged_bytes += text.len();
            self.log.push(String::from_utf8_lossy(text).into_owned());
        }
    }

    fn now(&self) -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
    }
}

impl ReadCall {
    fn hand_over(&mut self, bytes: Option<Vec<u8>>) -> i32 {
        match bytes {
            Some(bytes) => {
                self.waiting = bytes;
                i32::try_from(self.waiting.len()).unwrap_or(i32::MAX)
            }
            None => ABSENT,
        }
    }
}

impl Reader {
    pub(crate) fn compile(wasm: &[u8]) -> Result<Self, Stop> {
        let module = compile(wasm, &IMPORTS)?;
        let mut linker = Linker::new(engine());
        link(&mut linker).map_err(|error| Stop::Refused(error.to_string()))?;
        Ok(Self { module, linker })
    }

    pub(crate) fn hello(&self, tree: Arc<dyn Tree>, cancel: &AtomicBool) -> Result<Hello, ReadError> {
        let hello: Hello = self.call(
            tree,
            &Call::Hello(Hello {
                protocol: PROTOCOL.to_owned(),
                sdk: None,
            }),
            cancel,
        )?;
        if !speaks_same_major(&hello.protocol) {
            return Err(ReadError::Stopped(Stop::Refused(format!(
                "The reader speaks protocol {}, and this SterneGit speaks {PROTOCOL}",
                hello.protocol
            ))));
        }
        Ok(hello)
    }

    pub(crate) fn read(&self, tree: Arc<dyn Tree>, input: ReadInput, cancel: &AtomicBool) -> Result<Model, ReadError> {
        self.call(tree, &Call::Read(input), cancel)
    }

    fn call<T: DeserializeOwned>(&self, tree: Arc<dyn Tree>, call: &Call, cancel: &AtomicBool) -> Result<T, ReadError> {
        let message = serde_json::to_vec(call).unwrap_or_default();
        let output = self.run(tree, message, cancel)?;
        serde_json::from_slice::<Answer<T>>(&output)
            .map_err(|error| {
                ReadError::Stopped(Stop::BadOutput {
                    op: op_name(call),
                    path: ".".to_owned(),
                    problem: error.to_string(),
                })
            })?
            .into_result()
            .map_err(ReadError::Failed)
    }

    fn run(&self, tree: Arc<dyn Tree>, message: Vec<u8>, cancel: &AtomicBool) -> Result<Vec<u8>, ReadError> {
        let input_len = i32::try_from(message.len()).unwrap_or(i32::MAX);
        let call = ReadCall {
            tree,
            limits: Limits::new(MEMORY_LIMIT_BYTES),
            memory: None,
            waiting: message,
            output: None,
            log: Vec::new(),
            logged_bytes: 0,
        };
        let mut store = Store::new(engine(), call);
        store.limiter(|call| &mut call.limits);
        let stopped = |store: &Store<ReadCall>, error: wasmtime::Error| {
            ReadError::Stopped(store.data().limits.stop_for(&error).unwrap_or_else(|| Stop::Trapped {
                message: trap_message(&error),
                log: store.data().log.clone(),
            }))
        };
        store
            .set_fuel(FUEL_PER_SLICE * u64::from(SLICES_PER_READ))
            .map_err(|error| stopped(&store, error))?;
        store
            .fuel_async_yield_interval(Some(FUEL_PER_SLICE))
            .map_err(|error| stopped(&store, error))?;
        let status = futures::executor::block_on(async {
            let instance = self.linker.instantiate_async(&mut store, &self.module).await?;
            let memory = instance.get_memory(&mut store, MEMORY_EXPORT);
            store.data_mut().memory = memory;
            let entry = instance.get_typed_func::<i32, i32>(&mut store, ENTRY_EXPORT)?;
            Ok(unless_cancelled(entry.call_async(&mut store, input_len), || cancel.load(Ordering::Relaxed)).await)
        });
        match status {
            Ok(Some(Ok(status))) => finished(&mut store, status),
            Ok(Some(Err(error))) | Err(error) => Err(stopped(&store, error)),
            Ok(None) => Err(ReadError::Stopped(Stop::Cancelled)),
        }
    }
}

fn finished(store: &mut Store<ReadCall>, status: i32) -> Result<Vec<u8>, ReadError> {
    let trapped = |store: &Store<ReadCall>, message: String| {
        ReadError::Stopped(Stop::Trapped {
            message,
            log: store.data().log.clone(),
        })
    };
    if status != 0 {
        return Err(trapped(store, format!("{ENTRY_EXPORT} ended with status {status}")));
    }
    match store.data_mut().output.take() {
        Some(output) => Ok(output),
        None => Err(trapped(store, format!("{ENTRY_EXPORT} ended without calling finish"))),
    }
}

fn op_name(call: &Call) -> &'static str {
    match call {
        Call::Hello(_) => "hello",
        Call::Read(_) => "read",
    }
}

fn link(linker: &mut Linker<ReadCall>) -> wasmtime::Result<()> {
    link_common(linker)?;
    linker.func_wrap(ABI, READ_FILE.name, read_file)?;
    linker.func_wrap(ABI, SEARCH.name, search)?;
    Ok(())
}

fn guest_text(caller: &mut Caller<'_, ReadCall>, import: &str, at: i32, len: i32) -> wasmtime::Result<String> {
    let memory = guest_memory(caller)?;
    let bytes = memory.data(&*caller);
    let span = span(import, bytes.len(), at, length(len))?;
    String::from_utf8(bytes[span].to_vec()).map_err(|_| fault(import, "the text isn't UTF-8".to_owned()))
}

fn read_file(mut caller: Caller<'_, ReadCall>, path: i32, len: i32) -> wasmtime::Result<i32> {
    let path = guest_text(&mut caller, READ_FILE.name, path, len)?;
    let call = caller.data_mut();
    let bytes = call.tree.read(&path);
    Ok(call.hand_over(bytes))
}

fn search(mut caller: Caller<'_, ReadCall>, query: i32, len: i32) -> wasmtime::Result<i32> {
    let query = guest_text(&mut caller, SEARCH.name, query, len)?;
    let query: Search =
        serde_json::from_str(&query).map_err(|error| fault(SEARCH.name, format!("the search isn't valid: {error}")))?;
    let call = caller.data_mut();
    let found = call.tree.search(&query);
    let bytes = serde_json::to_vec(&found).unwrap_or_default();
    Ok(call.hand_over(Some(bytes)))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, atomic::AtomicBool},
        time::Instant,
    };

    use sternegit_reader_protocol::{Found, ReadInput, Search, Side};

    use crate::{
        plugin::Stop,
        read::{ReadError, Reader, Tree, WorkingTree},
    };

    struct Files(HashMap<&'static str, &'static str>);

    impl Tree for Files {
        fn read(&self, path: &str) -> Option<Vec<u8>> {
            self.0.get(path).map(|text| text.as_bytes().to_vec())
        }

        fn search(&self, _: &Search) -> Vec<Found> {
            Vec::new()
        }
    }

    fn files(entries: &[(&'static str, &'static str)]) -> Arc<dyn Tree> {
        Arc::new(Files(entries.iter().copied().collect()))
    }

    fn answering_with_file(name: &str) -> Vec<u8> {
        wat::parse_str(format!(
            r#"(module
                (import "sternegit:1" "take" (func $take (param i32)))
                (import "sternegit:1" "finish" (func $finish (param i32 i32)))
                (import "sternegit:1" "read_file" (func $read_file (param i32 i32) (result i32)))
                (memory (export "memory") 1)
                (data (i32.const 0) "{name}")
                (func (export "sternegit_call") (param i32) (result i32) (local $len i32)
                    (local.set $len (call $read_file (i32.const 0) (i32.const {len})))
                    (call $take (i32.const 1024))
                    (call $finish (i32.const 1024) (local.get $len))
                    (i32.const 0)))"#,
            len = name.len()
        ))
        .unwrap()
    }

    #[test]
    fn a_reader_reads_the_files_its_tree_holds() {
        let reader = Reader::compile(&answering_with_file("hello.json")).unwrap();
        let tree = files(&[("hello.json", r#"{"ok":{"protocol":"1.3","sdk":"0.1.0"}}"#)]);
        let hello = reader.hello(tree, &AtomicBool::new(false)).unwrap();
        assert_eq!(hello.protocol, "1.3");
    }

    #[test]
    fn a_reader_speaking_a_newer_major_protocol_is_refused() {
        let reader = Reader::compile(&answering_with_file("hello.json")).unwrap();
        let tree = files(&[("hello.json", r#"{"ok":{"protocol":"2.0","sdk":null}}"#)]);
        assert_eq!(
            reader.hello(tree, &AtomicBool::new(false)),
            Err(ReadError::Stopped(Stop::Refused(
                "The reader speaks protocol 2.0, and this SterneGit speaks 1.0".into()
            )))
        );
    }

    #[test]
    fn a_reader_cannot_reach_the_network() {
        let sending = wat::parse_str(
            r#"(module
                (import "sternegit:1" "send" (func (param i32 i32 i32 i32) (result i32)))
                (memory (export "memory") 1)
                (func (export "sternegit_call") (param i32) (result i32) (i32.const 0)))"#,
        )
        .unwrap();
        assert!(matches!(
            Reader::compile(&sending),
            Err(Stop::Refused(reason)) if reason == "The module imports sternegit:1::send, which SterneGit doesn't offer"
        ));
    }

    #[test]
    fn a_missing_file_reads_as_absent() {
        let reader = Reader::compile(&answering_with_file("gone.json")).unwrap();
        let error = reader.hello(files(&[]), &AtomicBool::new(false)).unwrap_err();
        assert!(matches!(error, ReadError::Stopped(Stop::Trapped { .. })), "{error:?}");
    }

    #[test]
    #[ignore = "times a real reader on a real project; set STERNEGIT_READER_WASM, STERNEGIT_TIME_ROOT and STERNEGIT_TIME_PATH"]
    fn times_a_reader_on_a_real_file() {
        let wasm = std::fs::read(std::env::var("STERNEGIT_READER_WASM").unwrap()).unwrap();
        let root = std::env::var("STERNEGIT_TIME_ROOT").unwrap();
        let path = std::env::var("STERNEGIT_TIME_PATH").unwrap();
        let started = Instant::now();
        let reader = Reader::compile(&wasm).unwrap();
        println!("compiled in {:?}", started.elapsed());
        let tree: Arc<dyn Tree> = Arc::new(WorkingTree::new(root));
        for run in 1..=3 {
            let started = Instant::now();
            let input = ReadInput {
                kind: "prefab".into(),
                path: path.clone(),
                side: Side::After,
                options: Default::default(),
            };
            let model = reader.read(Arc::clone(&tree), input, &AtomicBool::new(false)).unwrap();
            println!("run {run}: {} objects in {:?}", model.objects.len(), started.elapsed());
        }
    }
}
