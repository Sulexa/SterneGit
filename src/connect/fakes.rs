use std::{
    future::poll_fn,
    sync::{Arc, Mutex},
    task::{Poll, Waker},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures::{FutureExt as _, future::BoxFuture};
use url::Url;

use crate::{
    accounts::AccountId,
    net::{
        request::{ApiRequest, ApiResponse, NetError},
        services::{AccountView, Cancel, ConnectorId, HostServices, Level},
    },
};

pub(super) const NOW_MILLIS: u64 = 1_762_300_800_000;
pub(super) const GREETING: &str = r#"{"ok":{"protocol":"1.0","sdk":null}}"#;

const IMPORTS: &str = r#"
    (import "fastgit:1" "take" (func $take (param i32)))
    (import "fastgit:1" "send" (func $send (param i32 i32 i32 i32) (result i32)))
    (import "fastgit:1" "wait" (func $wait (param i32) (result i32)))
    (import "fastgit:1" "finish" (func $finish (param i32 i32)))
    (import "fastgit:1" "log" (func $log (param i32 i32 i32)))
    (import "fastgit:1" "now" (func $now (result i64)))
"#;

const RESPONSES_AT: usize = 32_768;
const OUTPUT_AT: usize = 4096;

type Answering = dyn Fn(&ApiRequest, Cancel) -> BoxFuture<'static, Result<ApiResponse, NetError>>
    + Send
    + Sync;

pub(super) fn wasm(text: &str) -> Vec<u8> {
    wat::parse_str(text).unwrap()
}

pub(super) fn connector(body: &str) -> Vec<u8> {
    connector_with_pages(1, body)
}

pub(super) fn connector_with_pages(pages: u32, body: &str) -> Vec<u8> {
    wasm(&format!(
        r#"(module {IMPORTS} (memory (export "memory") {pages}) {body})"#
    ))
}

pub(super) fn escaped(text: &str) -> String {
    text.bytes().map(|byte| format!("\\{byte:02x}")).collect()
}

pub(super) fn entry(instructions: &str) -> String {
    format!(r#"(func (export "fastgit_call") (param $length i32) (result i32) {instructions})"#)
}

pub(super) fn answering(output: &str) -> Vec<u8> {
    connector(&format!(
        r#"(data (i32.const 0) "{}") {}"#,
        escaped(output),
        entry(&format!(
            "(call $finish (i32.const 0) (i32.const {})) (i32.const 0)",
            output.len()
        ))
    ))
}

pub(super) fn greeter() -> Vec<u8> {
    answering(GREETING)
}

pub(super) fn echo() -> Vec<u8> {
    connector(&format!(
        r#"(data (i32.const 0) "{}") {}"#,
        escaped("{\"ok\":"),
        entry(
            "(call $take (i32.const 6))
             (i32.store8 (i32.add (local.get $length) (i32.const 6)) (i32.const 125))
             (call $finish (i32.const 0) (i32.add (local.get $length) (i32.const 7)))
             (i32.const 0)"
        )
    ))
}

pub(super) fn recorder(output: &str) -> Vec<u8> {
    connector(&format!(
        r#"(data (i32.const 0) "{}") {}"#,
        escaped(output),
        entry(&format!(
            "(call $take (i32.const 1024))
             (call $log (i32.const 1) (i32.const 1024) (local.get $length))
             (call $finish (i32.const 0) (i32.const {}))
             (i32.const 0)",
            output.len()
        ))
    ))
}

pub(super) fn two_faced(answer: &str) -> Vec<u8> {
    let head = get("user");
    let note = "asked for the user";
    assert!(answer.len() < 768);
    connector(&format!(
        r#"(data (i32.const 0) "{}") (data (i32.const 256) "{}")
           (data (i32.const 1024) "{}") (data (i32.const 1536) "{note}")
           {}"#,
        escaped(GREETING),
        escaped(answer),
        escaped(&head),
        entry(&format!(
            "(call $take (i32.const 8192))
             (if (i32.eq (i32.load8_u (i32.const 8199)) (i32.const 104))
                 (then (call $finish (i32.const 0) (i32.const {})) (return (i32.const 0))))
             (drop (call $wait (call $send (i32.const 1024) (i32.const {}) (i32.const 0) (i32.const 0))))
             (call $log (i32.const 1) (i32.const 1536) (i32.const {}))
             (call $finish (i32.const 256) (i32.const {}))
             (i32.const 0)",
            GREETING.len(),
            head.len(),
            note.len(),
            answer.len()
        ))
    ))
}

pub(super) fn fetching(requests: &[(&str, &str)]) -> Vec<u8> {
    let mut data = format!(
        r#"(data (i32.const 0) "{}") (data (i32.const 8) ",") (data (i32.const 9) "{}")"#,
        escaped("{\"ok\":["),
        escaped("]}")
    );
    let mut locals = String::new();
    let mut sends = String::new();
    let mut at = 16;
    for (index, (head, body)) in requests.iter().enumerate() {
        data += &format!(
            r#"(data (i32.const {at}) "{}") (data (i32.const {}) "{}")"#,
            escaped(head),
            at + head.len(),
            escaped(body)
        );
        locals += &format!("(local $request{index} i32)");
        sends += &format!(
            "(local.set $request{index} (call $send (i32.const {at}) (i32.const {}) (i32.const {}) (i32.const {})))",
            head.len(),
            at + head.len(),
            body.len()
        );
        at += head.len() + body.len();
    }
    assert!(at < OUTPUT_AT);
    let waits = (0..requests.len())
        .rev()
        .map(|index| format!("(call $append_response (local.get $request{index}))"))
        .collect::<Vec<_>>()
        .join("(call $append (i32.const 8) (i32.const 1))");
    connector(&format!(
        r#"{data}
        (global $end (mut i32) (i32.const {OUTPUT_AT}))
        (func $append (param $at i32) (param $length i32)
            (memory.copy (global.get $end) (local.get $at) (local.get $length))
            (global.set $end (i32.add (global.get $end) (local.get $length))))
        (func $append_response (param $id i32)
            (local $length i32) (local $head i32)
            (local.set $length (call $wait (local.get $id)))
            (call $take (i32.const {RESPONSES_AT}))
            (local.set $head (i32.load (i32.const {RESPONSES_AT})))
            (call $append (i32.const {head_at}) (local.get $head))
            (call $append (i32.const 8) (i32.const 1))
            (call $append
                (i32.add (i32.const {head_at}) (local.get $head))
                (i32.sub (i32.sub (local.get $length) (i32.const 4)) (local.get $head))))
        (func (export "fastgit_call") (param $length i32) (result i32) {locals}
            {sends}
            (call $append (i32.const 0) (i32.const 7))
            {waits}
            (call $append (i32.const 9) (i32.const 2))
            (call $finish (i32.const {OUTPUT_AT}) (i32.sub (global.get $end) (i32.const {OUTPUT_AT})))
            (i32.const 0))"#,
        head_at = RESPONSES_AT + 4
    ))
}

pub(super) fn get(url: &str) -> String {
    format!(r#"{{"method":"GET","url":"{url}","headers":[]}}"#)
}

pub(super) fn fake_account() -> AccountView {
    AccountView {
        id: AccountId::parse("fake:example.com:1").unwrap(),
        server: Url::parse("https://example.com/").unwrap(),
        endpoints: vec![
            ("api".to_owned(), Url::parse("https://api.example.com/v1/").unwrap()),
            ("web".to_owned(), Url::parse("https://example.com/").unwrap()),
        ],
        login: "octo".to_owned(),
    }
}

pub(super) fn fake_connector() -> ConnectorId {
    ConnectorId("fake".to_owned())
}

pub(super) fn json_response(text: &str) -> ApiResponse {
    ApiResponse {
        status: 200,
        headers: vec![("x-fake".to_owned(), "yes".to_owned())],
        body: serde_json::to_vec(text).unwrap(),
    }
}

pub(super) struct FakeHost {
    answering: Box<Answering>,
    requests: Mutex<Vec<ApiRequest>>,
    cancels: Mutex<Vec<Cancel>>,
    logs: Mutex<Vec<String>>,
    pauses: Mutex<Vec<SystemTime>>,
}

impl FakeHost {
    pub(super) fn answering(
        answering: impl Fn(&ApiRequest, Cancel) -> BoxFuture<'static, Result<ApiResponse, NetError>>
        + Send
        + Sync
        + 'static,
    ) -> Arc<Self> {
        Arc::new(Self {
            answering: Box::new(answering),
            requests: Mutex::default(),
            cancels: Mutex::default(),
            logs: Mutex::default(),
            pauses: Mutex::default(),
        })
    }

    pub(super) fn replying(
        reply: impl Fn(&ApiRequest) -> Result<ApiResponse, NetError> + Send + Sync + 'static,
    ) -> Arc<Self> {
        Self::answering(move |request, _| futures::future::ready(reply(request)).boxed())
    }

    pub(super) fn quiet() -> Arc<Self> {
        Self::replying(|_| Err(NetError::Unreachable("no server here".to_owned())))
    }

    pub(super) fn waiting_until_cancelled() -> Arc<Self> {
        Self::answering(|_, cancel| {
            async move {
                cancel.cancelled().await;
                Err(NetError::Cancelled)
            }
            .boxed()
        })
    }

    pub(super) fn answering_together(expected: usize) -> Arc<Self> {
        #[derive(Default)]
        struct Gathering {
            arrived: usize,
            waiting: Vec<Waker>,
        }

        let gathering = Arc::new(Mutex::new(Gathering::default()));
        Self::answering(move |request, _| {
            let gathering = Arc::clone(&gathering);
            let url = request.url.clone();
            let mut counted = false;
            poll_fn(move |cx| {
                let mut gathering = gathering.lock().unwrap();
                if !counted {
                    counted = true;
                    gathering.arrived += 1;
                }
                if gathering.arrived < expected {
                    gathering.waiting.push(cx.waker().clone());
                    return Poll::Pending;
                }
                gathering.waiting.drain(..).for_each(Waker::wake);
                Poll::Ready(Ok(json_response(&url)))
            })
            .boxed()
        })
    }

    pub(super) fn requests(&self) -> Vec<ApiRequest> {
        self.requests.lock().unwrap().clone()
    }

    pub(super) fn cancels(&self) -> Vec<Cancel> {
        self.cancels.lock().unwrap().clone()
    }

    pub(super) fn logs(&self) -> Vec<String> {
        self.logs.lock().unwrap().clone()
    }

    pub(super) fn pauses(&self) -> Vec<SystemTime> {
        self.pauses.lock().unwrap().clone()
    }
}

impl HostServices for FakeHost {
    fn account(&self, account: &AccountId) -> Option<AccountView> {
        Some(fake_account()).filter(|known| known.id == *account)
    }

    fn request(
        &self,
        _: &AccountId,
        request: ApiRequest,
        cancel: Cancel,
    ) -> BoxFuture<'static, Result<ApiResponse, NetError>> {
        self.requests.lock().unwrap().push(request.clone());
        self.cancels.lock().unwrap().push(cancel.clone());
        (self.answering)(&request, cancel)
    }

    fn pause(&self, _: &AccountId, until: SystemTime) {
        self.pauses.lock().unwrap().push(until);
    }

    fn log(&self, _: &ConnectorId, level: Level, message: &str) {
        self.logs.lock().unwrap().push(format!("{level:?} {message}"));
    }

    fn now(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_millis(NOW_MILLIS)
    }
}
