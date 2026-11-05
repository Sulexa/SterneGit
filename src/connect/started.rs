use std::sync::Arc;

use fastgit_protocol::{
    ABI, Hello, PROTOCOL,
    ops::{Greet, Op as _},
    speaks_same_major,
};
use wasmi::{Engine, ExternType, Instance, Linker, Module, Store, ValType};

use super::{
    Budget, CallError, Outcome, Stop, Usage, abi,
    call::{self, Call},
    forge::{decode, envelope},
    sandbox::{self, ENTRY_EXPORT, IMPORTS, MEMORY_EXPORT},
};
use crate::net::services::{AccountView, Cancel, ConnectorId, HostServices, cancel_pair};

pub(crate) struct Started {
    id: ConnectorId,
    engine: Engine,
    module: Module,
    linker: Linker<Call>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Greeting {
    pub(crate) hello: Hello,
    pub(crate) usage: Usage,
}

impl Started {
    pub(crate) fn start(
        id: ConnectorId,
        wasm: &[u8],
        host: Arc<dyn HostServices>,
    ) -> Result<(Self, Greeting), Stop> {
        let started = Self::compile(id, wasm)?;
        let greeting = started.greet(host)?;
        Ok((started, greeting))
    }

    pub(crate) fn compile(id: ConnectorId, wasm: &[u8]) -> Result<Self, Stop> {
        let engine = sandbox::engine();
        let module = Module::new(&engine, wasm)
            .map_err(|error| Stop::Refused(format!("The module can't be loaded: {error}")))?;
        refuse_foreign_imports(&module)?;
        require_exports(&module)?;
        let mut linker = Linker::new(&engine);
        abi::link(&mut linker).map_err(|error| Stop::Refused(error.to_string()))?;
        Ok(Self {
            id,
            engine,
            module,
            linker,
        })
    }

    pub(crate) fn id(&self) -> &ConnectorId {
        &self.id
    }

    pub(crate) fn call(
        self: &Arc<Self>,
        host: Arc<dyn HostServices>,
        account: Option<AccountView>,
        message: Vec<u8>,
        budget: Budget,
        cancel: Cancel,
    ) -> impl Future<Output = Outcome> + Send + use<> {
        let started = Arc::clone(self);
        async move { call::run(&started, host, account, message, budget, cancel).await }
    }

    pub(super) fn engine(&self) -> &Engine {
        &self.engine
    }

    pub(super) fn instantiate(&self, store: &mut Store<Call>) -> Result<Instance, wasmi::Error> {
        self.linker.instantiate_and_start(store, &self.module)
    }

    fn greet(&self, host: Arc<dyn HostServices>) -> Result<Greeting, Stop> {
        let hello = Hello {
            protocol: PROTOCOL.to_owned(),
            sdk: None,
        };
        let message = envelope(Greet::NAME, None, None, &hello);
        let (_keep, cancel) = cancel_pair();
        let outcome = futures::executor::block_on(call::run(self, host, None, message, Budget::CALL, cancel));
        let hello = outcome
            .result
            .and_then(|output| decode::<Hello>(Greet::NAME, &output))
            .map_err(refusal)?;
        if !speaks_same_major(&hello.protocol) {
            return Err(Stop::Refused(format!(
                "The connector speaks protocol {}, and this FastGit speaks {PROTOCOL}",
                hello.protocol
            )));
        }
        Ok(Greeting {
            hello,
            usage: outcome.usage,
        })
    }
}

fn refusal(error: CallError) -> Stop {
    match error {
        CallError::Stopped(stop) => stop,
        CallError::Failed(failure) => {
            Stop::Refused(format!("The connector refused to start: {failure}"))
        }
        CallError::Net(error) => {
            Stop::Refused(format!("The connector reached for the network while starting: {error}"))
        }
        CallError::Unavailable => Stop::Refused("The connector didn't answer hello".to_owned()),
    }
}

fn refuse_foreign_imports(module: &Module) -> Result<(), Stop> {
    for import in module.imports() {
        let offered = (import.module() == ABI)
            .then(|| IMPORTS.iter().find(|offered| offered.name == import.name()))
            .flatten();
        let fits = match (offered, import.ty()) {
            (Some(offered), ExternType::Func(ty)) => {
                ty.params() == offered.params && ty.results() == offered.results
            }
            _ => false,
        };
        if !fits {
            return Err(Stop::Refused(format!(
                "The module imports {}::{}, which FastGit doesn't offer",
                import.module(),
                import.name()
            )));
        }
    }
    Ok(())
}

fn require_exports(module: &Module) -> Result<(), Stop> {
    let exported = |name: &str| {
        module
            .exports()
            .find(|export| export.name() == name)
            .map(|export| export.ty().clone())
    };
    if !matches!(exported(MEMORY_EXPORT), Some(ExternType::Memory(_))) {
        return Err(Stop::Refused(format!(
            "The module exports no {MEMORY_EXPORT}"
        )));
    }
    match exported(ENTRY_EXPORT) {
        Some(ExternType::Func(ty)) if ty.params() == [ValType::I32] && ty.results() == [ValType::I32] => Ok(()),
        _ => Err(Stop::Refused(format!(
            "The module exports no {ENTRY_EXPORT}(i32) -> i32"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use fastgit_protocol::Hello;

    use super::Started;
    use crate::connect::{
        Stop,
        fakes::{FakeHost, answering, connector, echo, entry, fake_connector, greeter, wasm},
    };

    fn refusal(wasm: &[u8]) -> String {
        match Started::compile(fake_connector(), wasm) {
            Err(Stop::Refused(reason)) => reason,
            Err(other) => panic!("expected a refusal, got {other:?}"),
            Ok(_) => panic!("expected a refusal, but the module compiled"),
        }
    }

    fn start(wasm: &[u8]) -> Result<Hello, Stop> {
        Started::start(fake_connector(), wasm, FakeHost::quiet()).map(|(_, greeting)| greeting.hello)
    }

    const EXPORTS: &str = r#"(memory (export "memory") 1) (func (export "fastgit_call") (param i32) (result i32) (i32.const 0))"#;

    #[test]
    fn a_module_importing_wasi_is_refused_by_name() {
        let wasi = wasm(&format!(
            r#"(module (import "wasi_snapshot_preview1" "fd_write" (func (param i32 i32 i32 i32) (result i32))) {EXPORTS})"#
        ));
        assert_eq!(
            refusal(&wasi),
            "The module imports wasi_snapshot_preview1::fd_write, which FastGit doesn't offer"
        );
    }

    #[test]
    fn imports_must_match_abi_1_exactly() {
        for (import, shown) in [
            (r#"(import "fastgit:1" "take" (func (param i64)))"#, "fastgit:1::take"),
            (r#"(import "fastgit:1" "now" (func (result i32)))"#, "fastgit:1::now"),
            (r#"(import "fastgit:1" "spawn" (func))"#, "fastgit:1::spawn"),
            (r#"(import "fastgit:2" "take" (func (param i32)))"#, "fastgit:2::take"),
            (r#"(import "fastgit:1" "take" (global i32))"#, "fastgit:1::take"),
        ] {
            let module = wasm(&format!(r#"(module {import} (memory (export "memory") 1) (func (export "fastgit_call") (param i32) (result i32) (i32.const 0)))"#));
            assert_eq!(
                refusal(&module),
                format!("The module imports {shown}, which FastGit doesn't offer")
            );
        }
    }

    #[test]
    fn a_start_function_is_refused() {
        let started = wasm(&format!(r#"(module {EXPORTS} (func $begin) (start $begin))"#));
        assert!(refusal(&started).contains("start function"), "{}", refusal(&started));
    }

    #[test]
    fn a_module_without_its_memory_or_entry_point_is_refused() {
        let no_memory = wasm(r#"(module (func (export "fastgit_call") (param i32) (result i32) (i32.const 0)))"#);
        assert_eq!(refusal(&no_memory), "The module exports no memory");
        let wrong_entry = wasm(r#"(module (memory (export "memory") 1) (func (export "fastgit_call") (param i64) (result i32) (i32.const 0)))"#);
        assert_eq!(refusal(&wrong_entry), "The module exports no fastgit_call(i32) -> i32");
    }

    #[test]
    fn bytes_that_are_not_webassembly_are_refused() {
        assert!(refusal(b"\0asm but not really").starts_with("The module can't be loaded"));
    }

    #[test]
    fn starting_greets_the_connector_and_keeps_its_answer() {
        let (started, greeting) = Started::start(fake_connector(), &greeter(), FakeHost::quiet()).unwrap();
        assert_eq!(started.id(), &fake_connector());
        assert_eq!(greeting.hello, Hello { protocol: "1.0".into(), sdk: None });
        assert!(greeting.usage.fuel > 0);
        assert_eq!(greeting.usage.requests, 0);
    }

    #[test]
    fn a_connector_speaking_a_newer_major_protocol_is_refused() {
        let newer = answering(r#"{"ok":{"protocol":"2.0","sdk":"9.0.0"}}"#);
        assert_eq!(
            start(&newer),
            Err(Stop::Refused("The connector speaks protocol 2.0, and this FastGit speaks 1.0".into()))
        );
        let minor = answering(r#"{"ok":{"protocol":"1.7","sdk":"1.7.0","later":true}}"#);
        assert_eq!(start(&minor).unwrap().protocol, "1.7");
    }

    #[test]
    fn a_connector_that_cannot_say_hello_does_not_start() {
        match start(&echo()) {
            Err(Stop::BadOutput { op, path, problem }) => {
                assert_eq!((op, path.as_str()), ("hello", "."));
                assert!(problem.starts_with("missing field `protocol`"), "{problem}");
            }
            other => panic!("expected bad output, got {other:?}"),
        }
        assert_eq!(
            start(&answering(r#"{"err":{"kind":"signed_out"}}"#)),
            Err(Stop::Refused("The connector refused to start: Signed out".into()))
        );
        assert!(matches!(
            start(&connector(&entry("unreachable"))),
            Err(Stop::Trapped { .. })
        ));
    }
}
