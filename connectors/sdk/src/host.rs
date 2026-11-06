#[cfg(target_arch = "wasm32")]
pub(crate) use guest::{finish, log, now, send, take, wait};
#[cfg(not(target_arch = "wasm32"))]
pub(crate) use tape::{Tape, log, now, playing, send, wait};

#[cfg(target_arch = "wasm32")]
mod guest {
    use crate::Level;

    mod imports {
        #[link(wasm_import_module = "sternegit:1")]
        unsafe extern "C" {
            pub(super) fn take(into: *mut u8);
            pub(super) fn send(head: *const u8, head_len: usize, body: *const u8, body_len: usize) -> i32;
            pub(super) fn wait(id: i32) -> usize;
            pub(super) fn finish(out: *const u8, len: usize);
            pub(super) fn log(level: i32, text: *const u8, len: usize);
            pub(super) fn now() -> i64;
        }
    }

    pub(crate) fn take(len: usize) -> Vec<u8> {
        let mut bytes = vec![0; len];
        unsafe { imports::take(bytes.as_mut_ptr()) };
        bytes
    }

    pub(crate) fn send(head: &[u8], body: &[u8]) -> i32 {
        unsafe { imports::send(head.as_ptr(), head.len(), body.as_ptr(), body.len()) }
    }

    pub(crate) fn wait(id: i32) -> Vec<u8> {
        let len = unsafe { imports::wait(id) };
        take(len)
    }

    pub(crate) fn finish(output: &[u8]) {
        unsafe { imports::finish(output.as_ptr(), output.len()) }
    }

    pub(crate) fn log(level: Level, text: &str) {
        unsafe { imports::log(level.code(), text.as_ptr(), text.len()) }
    }

    pub(crate) fn now() -> i64 {
        unsafe { imports::now() }
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod tape {
    use std::{
        cell::RefCell,
        time::{SystemTime, UNIX_EPOCH},
    };

    use sternegit_protocol::{Reel, RequestHead, resolve};

    use crate::Level;

    pub(crate) struct Tape {
        api: String,
        pub(crate) reel: Reel,
        answers: Vec<Vec<u8>>,
        pub(crate) log: Vec<String>,
    }

    impl Tape {
        pub(crate) fn new(api: &str, reel: Reel) -> Self {
            Self {
                api: api.to_owned(),
                reel,
                answers: Vec::new(),
                log: Vec::new(),
            }
        }
    }

    thread_local! {
        static PLAYING: RefCell<Option<Tape>> = const { RefCell::new(None) };
    }

    pub(crate) fn playing<R>(tape: Tape, run: impl FnOnce() -> R) -> (R, Tape) {
        PLAYING.with(|playing| *playing.borrow_mut() = Some(tape));
        let result = run();
        let tape = PLAYING
            .with(|playing| playing.borrow_mut().take())
            .expect("the tape stays in place while the connector runs");
        (result, tape)
    }

    fn with_tape<R>(work: impl FnOnce(&mut Tape) -> R) -> R {
        PLAYING.with(|playing| {
            let mut playing = playing.borrow_mut();
            let tape = playing
                .as_mut()
                .expect("requests reach a server only inside sternegit_sdk::replay or check");
            work(tape)
        })
    }

    pub(crate) fn send(head: &[u8], _body: &[u8]) -> i32 {
        with_tape(|tape| {
            let head: RequestHead =
                serde_json::from_slice(head).expect("the SDK writes heads SterneGit can read");
            let url = resolve(&tape.api, &head.url);
            let exchange = tape.reel.play(&head.method, &url).unwrap_or_else(|| {
                panic!("The cassette holds no answer to {} {url}", head.method)
            });
            tape.answers
                .push(exchange.head().frame(&exchange.body_bytes()));
            i32::try_from(tape.answers.len()).expect("fewer than two billion requests")
        })
    }

    pub(crate) fn wait(id: i32) -> Vec<u8> {
        with_tape(|tape| {
            usize::try_from(id)
                .ok()
                .and_then(|id| id.checked_sub(1))
                .and_then(|index| tape.answers.get(index).cloned())
                .expect("a request is waited for after it was sent")
        })
    }

    pub(crate) fn log(level: Level, text: &str) {
        let line = format!("{level:?} {text}");
        let kept = PLAYING.with(|playing| match playing.borrow_mut().as_mut() {
            Some(tape) => {
                tape.log.push(line.clone());
                true
            }
            None => false,
        });
        if !kept {
            eprintln!("{line}");
        }
    }

    pub(crate) fn now() -> i64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
    }
}
