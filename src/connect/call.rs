use std::{
    sync::Arc,
    time::{Duration, Instant, UNIX_EPOCH},
};

use sternegit_protocol::{RequestHead, ResponseHead, resolve};
use futures::FutureExt as _;
use wasmi::{
    Memory, Store, StoreLimits, TrapCode, TypedResumableCall, Val,
    errors::{ErrorKind, InstantiationError, MemoryError, TableError},
};

use super::{
    CallError, Stop,
    abi::Interrupt,
    requests::Requests,
    sandbox::{ENTRY_EXPORT, FUEL_PER_SLICE, MEMORY_EXPORT, limits},
    started::Started,
};
use crate::net::{
    request::{ApiRequest, ApiResponse, Method, NetError},
    services::{AccountView, Cancel, ConnectorId, HostServices, Level},
};

const LOG_BYTES_PER_CALL: usize = 64 * 1024;
const API_ENDPOINT: &str = "api";
const LOG_FULL: &str = "The connector logged more than 64 KB in one call; later lines were dropped";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Budget {
    slices: u32,
}

impl Budget {
    pub(crate) const CALL: Self = Self { slices: 50 };
    pub(crate) const LISTING: Self = Self { slices: 200 };

    #[cfg(test)]
    pub(crate) const fn of(slices: u32) -> Self {
        Self { slices }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Usage {
    pub(crate) fuel: u64,
    pub(crate) slices: u32,
    pub(crate) memory: usize,
    pub(crate) requests: usize,
    pub(crate) elapsed: Duration,
}

#[derive(Debug)]
pub(crate) struct Outcome {
    pub(crate) result: Result<Vec<u8>, CallError>,
    pub(crate) usage: Usage,
}

#[derive(Default)]
struct Log {
    lines: Vec<String>,
    bytes: usize,
    full: bool,
}

pub(super) struct Call {
    connector: ConnectorId,
    host: Arc<dyn HostServices>,
    account: Option<AccountView>,
    cancel: Cancel,
    limits: StoreLimits,
    pub(super) memory: Option<Memory>,
    pub(super) waiting: Vec<u8>,
    pub(super) requests: Requests,
    output: Option<Vec<u8>>,
    log: Log,
}

impl Call {
    pub(super) fn send(&mut self, head: &[u8], body: &[u8]) -> Result<u32, String> {
        let head: RequestHead = serde_json::from_slice(head)
            .map_err(|error| format!("the request head isn't valid: {error}"))?;
        let method = Method::named(&head.method)
            .ok_or_else(|| format!("SterneGit doesn't send {} requests", head.method))?;
        self.requests.make_room(body.len())?;
        let journey = match &self.account {
            Some(account) => {
                let request = ApiRequest {
                    method,
                    url: resolve(api_endpoint(account), &head.url),
                    headers: head.headers,
                    body: (!body.is_empty() || !method.only_reads()).then(|| body.to_vec()),
                    timeout: None,
                };
                self.host.request(&account.id, request, self.cancel.clone())
            }
            None => futures::future::ready(Err(NetError::NotSignedIn)).boxed(),
        };
        Ok(self.requests.launch(journey, body.len()))
    }

    pub(super) fn deliver(&mut self, response: ApiResponse) -> i32 {
        let head = ResponseHead {
            status: response.status,
            headers: response.headers,
        };
        self.waiting = head.frame(&response.body);
        i32::try_from(self.waiting.len()).unwrap_or(i32::MAX)
    }

    pub(super) fn finish(&mut self, output: &[u8]) -> Result<(), String> {
        if self.output.is_some() {
            return Err("the call already has its answer".to_owned());
        }
        self.output = Some(output.to_vec());
        Ok(())
    }

    pub(super) fn log(&mut self, level: i32, text: &[u8]) {
        if self.log.full {
            return;
        }
        if self.log.bytes + text.len() > LOG_BYTES_PER_CALL {
            self.log.full = true;
            self.record(Level::Warn, LOG_FULL);
            return;
        }
        self.log.bytes += text.len();
        self.record(level_named(level), &String::from_utf8_lossy(text));
    }

    pub(super) fn now(&self) -> i64 {
        self.host
            .now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |since| i64::try_from(since.as_millis()).unwrap_or(i64::MAX))
    }

    fn record(&mut self, level: Level, text: &str) {
        self.host.log(&self.connector, level, text);
        self.log.lines.push(format!("{level:?} {text}"));
    }
}

fn api_endpoint(account: &AccountView) -> &str {
    account
        .endpoints
        .iter()
        .find(|(name, _)| name == API_ENDPOINT)
        .map_or("", |(_, url)| url.as_str())
}

fn level_named(level: i32) -> Level {
    match level {
        0 => Level::Debug,
        2 => Level::Warn,
        3 => Level::Error,
        _ => Level::Info,
    }
}

pub(super) async fn run(
    started: &Started,
    host: Arc<dyn HostServices>,
    account: Option<AccountView>,
    message: Vec<u8>,
    budget: Budget,
    cancel: Cancel,
) -> Outcome {
    let clock = Instant::now();
    let input_len = i32::try_from(message.len()).unwrap_or(i32::MAX);
    let call = Call {
        connector: started.id().clone(),
        host,
        account,
        cancel,
        limits: limits(),
        memory: None,
        waiting: message,
        requests: Requests::default(),
        output: None,
        log: Log::default(),
    };
    let mut execution = Execution::new(started, call);
    let result = execution.run(started, input_len, budget).await;
    Outcome {
        result,
        usage: execution.usage(clock.elapsed()),
    }
}

struct Execution {
    store: Store<Call>,
    fuel_given: u64,
    slices: u32,
}

impl Execution {
    fn new(started: &Started, call: Call) -> Self {
        let mut store = Store::new(started.engine(), call);
        store.limiter(|call| &mut call.limits);
        Self {
            store,
            fuel_given: 0,
            slices: 0,
        }
    }

    async fn run(&mut self, started: &Started, input_len: i32, budget: Budget) -> Result<Vec<u8>, CallError> {
        self.fill(FUEL_PER_SLICE)?;
        let instance = started
            .instantiate(&mut self.store)
            .map_err(|error| self.stopped(&error))?;
        self.store.data_mut().memory = instance.get_memory(&self.store, MEMORY_EXPORT);
        let entry = instance
            .get_typed_func::<i32, i32>(&self.store, ENTRY_EXPORT)
            .map_err(|error| self.stopped(&error))?;
        let mut step = entry
            .call_resumable(&mut self.store, input_len)
            .map_err(|error| self.stopped(&error))?;
        loop {
            step = match step {
                TypedResumableCall::Finished(status) => return self.finished(status),
                TypedResumableCall::HostTrap(paused) => {
                    let id = match paused.host_error().downcast_ref::<Interrupt>() {
                        Some(Interrupt::Wait(id)) => *id,
                        _ => return Err(self.trapped(paused.host_error().to_string())),
                    };
                    let length = self.arrival(id).await?;
                    paused.resume(&mut self.store, &[Val::I32(length)])
                }
                TypedResumableCall::OutOfFuel(paused) => {
                    self.next_slice(budget, paused.required_fuel()).await?;
                    paused.resume(&mut self.store)
                }
            }
            .map_err(|error| self.stopped(&error))?;
        }
    }

    fn fill(&mut self, fuel: u64) -> Result<(), CallError> {
        self.store
            .set_fuel(fuel)
            .map_err(|error| self.stopped(&error))?;
        self.fuel_given += fuel;
        self.slices += 1;
        Ok(())
    }

    async fn next_slice(&mut self, budget: Budget, required: u64) -> Result<(), CallError> {
        if self.slices >= budget.slices {
            return Err(CallError::Stopped(Stop::OutOfFuel));
        }
        smol::future::yield_now().await;
        if self.store.data().cancel.is_cancelled() {
            return Err(CallError::Stopped(Stop::Cancelled));
        }
        self.fill(FUEL_PER_SLICE.max(required))
    }

    async fn arrival(&mut self, id: u32) -> Result<i32, CallError> {
        let call = self.store.data_mut();
        let response = call.requests.arrival(id, &call.cancel).await?;
        Ok(call.deliver(response))
    }

    fn finished(&mut self, status: i32) -> Result<Vec<u8>, CallError> {
        if status != 0 {
            return Err(self.trapped(format!("{ENTRY_EXPORT} ended with status {status}")));
        }
        match self.store.data_mut().output.take() {
            Some(output) => Ok(output),
            None => Err(self.trapped(format!("{ENTRY_EXPORT} ended without calling finish"))),
        }
    }

    fn trapped(&self, message: String) -> CallError {
        CallError::Stopped(Stop::Trapped {
            message,
            log: self.store.data().log.lines.clone(),
        })
    }

    fn stopped(&self, error: &wasmi::Error) -> CallError {
        if ran_out_of_memory(error) {
            CallError::Stopped(Stop::OutOfMemory)
        } else if error.as_trap_code() == Some(TrapCode::OutOfFuel) {
            CallError::Stopped(Stop::OutOfFuel)
        } else {
            self.trapped(error.to_string())
        }
    }

    fn usage(&self, elapsed: Duration) -> Usage {
        Usage {
            fuel: self
                .fuel_given
                .saturating_sub(self.store.get_fuel().unwrap_or(0)),
            slices: self.slices,
            memory: self
                .store
                .data()
                .memory
                .map_or(0, |memory| memory.data_size(&self.store)),
            requests: self.store.data().requests.sent(),
            elapsed,
        }
    }
}

fn ran_out_of_memory(error: &wasmi::Error) -> bool {
    let memory_denied = |error: &MemoryError| {
        matches!(
            error,
            MemoryError::ResourceLimiterDeniedAllocation | MemoryError::OutOfSystemMemory
        )
    };
    let table_denied = |error: &TableError| matches!(error, TableError::ResourceLimiterDeniedAllocation);
    error.as_trap_code() == Some(TrapCode::GrowthOperationLimited)
        || match error.kind() {
            ErrorKind::Memory(error)
            | ErrorKind::Instantiation(InstantiationError::FailedToInstantiateMemory(error)) => {
                memory_denied(error)
            }
            ErrorKind::Table(error)
            | ErrorKind::Instantiation(InstantiationError::FailedToInstantiateTable(error)) => {
                table_denied(error)
            }
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use sternegit_protocol::ops::{Op as _, WhoAmI};
    use serde_json::{Value, json};

    use super::{Budget, Outcome, run};
    use crate::{
        connect::{
            CallError, Started, Stop,
            fakes::{
                FakeHost, NOW_MILLIS, answering, connector, connector_with_pages, echo, entry,
                escaped, fake_account, fake_connector, fetching, get, json_response,
            },
            forge::{context, envelope},
        },
        net::{
            request::{Method, NetError},
            services::{AccountView, cancel_pair},
        },
    };

    fn started(wasm: &[u8]) -> Started {
        Started::compile(fake_connector(), wasm).unwrap()
    }

    fn message() -> Vec<u8> {
        envelope(WhoAmI::NAME, Some(context(&fake_account())), None, &())
    }

    fn call(wasm: &[u8], host: Arc<FakeHost>, account: Option<AccountView>, budget: Budget) -> Outcome {
        let (_keep, cancel) = cancel_pair();
        smol::block_on(run(&started(wasm), host, account, message(), budget, cancel))
    }

    fn answered(wasm: &[u8], host: Arc<FakeHost>) -> Value {
        let output = call(wasm, host, Some(fake_account()), Budget::CALL).result.unwrap();
        serde_json::from_slice(&output).unwrap()
    }

    fn stopped(wasm: &[u8]) -> Stop {
        match call(wasm, FakeHost::quiet(), Some(fake_account()), Budget::CALL).result {
            Err(CallError::Stopped(stop)) => stop,
            other => panic!("expected a stop, got {other:?}"),
        }
    }

    fn trap_message(wasm: &[u8]) -> String {
        match stopped(wasm) {
            Stop::Trapped { message, .. } => message,
            other => panic!("expected a trap, got {other:?}"),
        }
    }

    fn within_seconds<T>(seconds: u64, work: impl Future<Output = T>) -> T {
        smol::block_on(smol::future::or(work, async {
            smol::Timer::after(Duration::from_secs(seconds)).await;
            panic!("still running after {seconds} s");
        }))
    }

    #[test]
    fn an_echo_receives_its_call_with_the_account_but_no_token() {
        let echoed = answered(&echo(), FakeHost::quiet());
        assert_eq!(
            echoed,
            json!({ "ok": {
                "op": "account.who_am_i",
                "account": {
                    "id": "fake:example.com:1",
                    "login": "octo",
                    "endpoints": { "api": "https://api.example.com/v1/", "web": "https://example.com/" }
                },
                "repo": null,
                "input": null
            }})
        );
    }

    #[test]
    fn requests_sent_before_waiting_travel_together_and_resolve_under_the_api_endpoint() {
        let host = FakeHost::answering_together(2);
        let wasm = fetching(&[(&get("first"), ""), (&get("/second"), "")]);
        let (_keep, cancel) = cancel_pair();
        let outcome = within_seconds(
            10,
            run(&started(&wasm), host.clone(), Some(fake_account()), message(), Budget::CALL, cancel),
        );
        let head = json!({ "status": 200, "headers": [["x-fake", "yes"]] });
        assert_eq!(
            serde_json::from_slice::<Value>(&outcome.result.unwrap()).unwrap(),
            json!({ "ok": [
                head, "https://api.example.com/v1/second",
                head, "https://api.example.com/v1/first"
            ]})
        );
        let urls: Vec<String> = host.requests().into_iter().map(|request| request.url).collect();
        assert_eq!(urls, ["https://api.example.com/v1/first", "https://api.example.com/v1/second"]);
        assert_eq!(outcome.usage.requests, 2);
    }

    #[test]
    fn a_request_carries_its_method_headers_and_body_and_absolute_urls_stay_put() {
        let host = FakeHost::replying(|request| Ok(json_response(&request.url)));
        let head = r#"{"method":"post","url":"https://example.com/graphql","headers":[["Accept","application/json"]]}"#;
        answered(&fetching(&[(head, r#"{"query":"{ viewer }"}"#), (&get("user"), "")]), host.clone());
        let requests = host.requests();
        assert_eq!(requests[0].method, Method::Post);
        assert_eq!(requests[0].url, "https://example.com/graphql");
        assert_eq!(requests[0].headers, [("Accept".to_owned(), "application/json".to_owned())]);
        assert_eq!(requests[0].body.as_deref(), Some(&br#"{"query":"{ viewer }"}"#[..]));
        assert_eq!(requests[1].method, Method::Get);
        assert_eq!(requests[1].body, None);
    }

    #[test]
    fn an_endless_loop_runs_out_of_fuel_after_its_slices() {
        let outcome = call(
            &connector(&entry("(loop $forever (br $forever)) (i32.const 0)")),
            FakeHost::quiet(),
            Some(fake_account()),
            Budget::of(3),
        );
        assert_eq!(outcome.result, Err(CallError::Stopped(Stop::OutOfFuel)));
        assert_eq!(outcome.usage.slices, 3);
        assert!(outcome.usage.fuel > 50_000_000, "{:?}", outcome.usage);
    }

    #[test]
    fn growing_memory_past_64_mb_stops_the_call() {
        let hog = connector(&entry(
            "(loop $more (drop (memory.grow (i32.const 16))) (br $more)) (i32.const 0)",
        ));
        let outcome = call(&hog, FakeHost::quiet(), Some(fake_account()), Budget::CALL);
        assert_eq!(outcome.result, Err(CallError::Stopped(Stop::OutOfMemory)));
        assert!(outcome.usage.memory <= 64 * 1024 * 1024);
        assert!(outcome.usage.memory > 60 * 1024 * 1024);
    }

    #[test]
    fn a_module_asking_for_more_than_64_mb_at_once_never_runs() {
        let greedy = connector_with_pages(1100, &entry("(i32.const 0)"));
        assert_eq!(stopped(&greedy), Stop::OutOfMemory);
    }

    #[test]
    fn a_trap_stops_the_call_with_the_lines_logged_before_it() {
        let host = FakeHost::quiet();
        let crash = connector(&format!(
            r#"(data (i32.const 0) "about to fail") {}"#,
            entry("(call $log (i32.const 3) (i32.const 0) (i32.const 13)) unreachable")
        ));
        let outcome = call(&crash, host.clone(), Some(fake_account()), Budget::CALL);
        match outcome.result {
            Err(CallError::Stopped(Stop::Trapped { message, log })) => {
                assert!(message.contains("unreachable"), "{message}");
                assert_eq!(log, ["Error about to fail"]);
            }
            other => panic!("expected a trap, got {other:?}"),
        }
        assert_eq!(host.logs(), ["Error about to fail"]);
    }

    #[test]
    fn wild_pointers_are_refused_at_every_import() {
        for (import, instructions) in [
            ("take", "(call $take (i32.const 65530))"),
            ("take", "(call $take (i32.const -1))"),
            ("send", "(drop (call $send (i32.const 0) (i32.const -1) (i32.const 0) (i32.const 0)))"),
            ("send", "(drop (call $send (i32.const 0) (i32.const 2) (i32.const 65535) (i32.const 2)))"),
            ("finish", "(call $finish (i32.const 65530) (i32.const 100))"),
            ("log", "(call $log (i32.const 1) (i32.const 65536) (i32.const 1))"),
        ] {
            let message = trap_message(&connector(&entry(&format!("{instructions} (i32.const 0)"))));
            assert!(
                message.starts_with(&format!("{import}: "))
                    && message.contains("outside the connector's 65536 bytes"),
                "{instructions}: {message}"
            );
        }
    }

    #[test]
    fn a_call_must_end_with_status_zero_and_exactly_one_answer() {
        assert_eq!(
            trap_message(&connector(&entry("(i32.const 0)"))),
            "sternegit_call ended without calling finish"
        );
        assert_eq!(
            trap_message(&connector(&entry("(call $finish (i32.const 0) (i32.const 2)) (i32.const 7)"))),
            "sternegit_call ended with status 7"
        );
        assert_eq!(
            trap_message(&connector(&entry(
                "(call $finish (i32.const 0) (i32.const 2)) (call $finish (i32.const 0) (i32.const 2)) (i32.const 0)"
            ))),
            "finish: the call already has its answer"
        );
    }

    #[test]
    fn waiting_for_a_request_never_sent_stops_the_call() {
        assert_eq!(
            trap_message(&connector(&entry("(drop (call $wait (i32.const 9))) (i32.const 0)"))),
            "wait: request 9 isn't waiting to be read"
        );
    }

    #[test]
    fn a_broken_request_head_or_an_unknown_method_stops_the_call() {
        let broken = fetching(&[("{\"url\":", "")]);
        assert!(trap_message(&broken).starts_with("send: the request head isn't valid"));
        let brew = fetching(&[(r#"{"method":"BREW","url":"pot"}"#, "")]);
        assert_eq!(trap_message(&brew), "send: SterneGit doesn't send BREW requests");
    }

    #[test]
    fn a_transport_failure_ends_the_call_without_the_connector_seeing_it() {
        let host = FakeHost::replying(|_| Err(NetError::Timeout));
        let outcome = call(&fetching(&[(&get("user"), "")]), host, Some(fake_account()), Budget::CALL);
        assert_eq!(outcome.result, Err(CallError::Net(NetError::Timeout)));
    }

    #[test]
    fn a_call_without_an_account_cannot_reach_the_network() {
        let host = FakeHost::quiet();
        let outcome = call(&fetching(&[(&get("user"), "")]), host.clone(), None, Budget::CALL);
        assert_eq!(outcome.result, Err(CallError::Net(NetError::NotSignedIn)));
        assert!(host.requests().is_empty());
    }

    #[test]
    fn too_many_requests_waiting_at_once_stop_the_call_and_cancel_the_rest() {
        let heads = vec![get("user"); 17];
        let requests: Vec<(&str, &str)> = heads.iter().map(|head| (head.as_str(), "")).collect();
        let host = FakeHost::waiting_until_cancelled();
        let outcome = call(&fetching(&requests), host.clone(), Some(fake_account()), Budget::CALL);
        match outcome.result {
            Err(CallError::Stopped(Stop::Trapped { message, .. })) => {
                assert_eq!(message, "send: 16 requests are already waiting to be read")
            }
            other => panic!("expected a trap, got {other:?}"),
        }
        assert_eq!(host.requests().len(), 16);
        assert!(host.cancels().iter().all(|cancel| cancel.is_cancelled()));
    }

    #[test]
    fn cancelling_a_waiting_call_stops_it_and_its_request() {
        let host = FakeHost::waiting_until_cancelled();
        let started = started(&fetching(&[(&get("slow"), "")]));
        let (keep, cancel) = cancel_pair();
        let outcome = within_seconds(10, async {
            let canceller = async {
                while host.requests().is_empty() {
                    smol::future::yield_now().await;
                }
                drop(keep);
            };
            let (outcome, ()) = futures::join!(
                run(&started, host.clone(), Some(fake_account()), message(), Budget::CALL, cancel),
                canceller
            );
            outcome
        });
        assert_eq!(outcome.result, Err(CallError::Stopped(Stop::Cancelled)));
        assert!(host.cancels()[0].is_cancelled());
    }

    #[test]
    fn a_running_call_notices_cancellation_between_slices() {
        let started = started(&connector(&entry("(loop $forever (br $forever)) (i32.const 0)")));
        let (keep, cancel) = cancel_pair();
        let outcome = within_seconds(10, async {
            let canceller = async {
                smol::Timer::after(Duration::from_millis(30)).await;
                drop(keep);
            };
            let (outcome, ()) = futures::join!(
                run(&started, FakeHost::quiet(), Some(fake_account()), message(), Budget::of(u32::MAX), cancel),
                canceller
            );
            outcome
        });
        assert_eq!(outcome.result, Err(CallError::Stopped(Stop::Cancelled)));
    }

    #[test]
    fn dropping_a_waiting_call_cancels_its_requests() {
        let host = FakeHost::waiting_until_cancelled();
        let started = Arc::new(started(&fetching(&[(&get("slow"), "")])));
        let (keep, cancel) = cancel_pair();
        let call = started.call(host.clone(), Some(fake_account()), message(), Budget::CALL, cancel);
        let mut call = Box::pin(async move {
            let _keep = keep;
            call.await
        });
        within_seconds(10, async {
            while host.requests().is_empty() {
                assert!(futures::poll!(call.as_mut()).is_pending());
                smol::future::yield_now().await;
            }
        });
        assert!(!host.cancels()[0].is_cancelled());
        drop(call);
        assert!(host.cancels()[0].is_cancelled());
    }

    #[test]
    fn logs_stop_at_64_kb_per_call() {
        let host = FakeHost::quiet();
        let chatty = connector(&format!(
            r#"(data (i32.const 0) "{}") {}"#,
            escaped(r#"{"ok":{}}"#),
            entry(
                "(local $line i32)
                 (loop $more
                     (call $log (i32.const 0) (i32.const 1024) (i32.const 1000))
                     (local.set $line (i32.add (local.get $line) (i32.const 1)))
                     (br_if $more (i32.lt_u (local.get $line) (i32.const 100))))
                 (call $finish (i32.const 0) (i32.const 9))
                 (i32.const 0)"
            )
        ));
        assert_eq!(answered(&chatty, host.clone()), json!({ "ok": {} }));
        let logs = host.logs();
        assert_eq!(logs.len(), 66);
        assert_eq!(
            logs.last().unwrap(),
            "Warn The connector logged more than 64 KB in one call; later lines were dropped"
        );
    }

    #[test]
    fn the_clock_is_the_hosts() {
        let clock = connector(&format!(
            r#"(data (i32.const 0) "{}") {}"#,
            escaped(r#"{"ok":{}}"#),
            entry(&format!(
                "(if (i64.ne (call $now) (i64.const {NOW_MILLIS})) (then unreachable))
                 (call $finish (i32.const 0) (i32.const 9))
                 (i32.const 0)"
            ))
        ));
        assert_eq!(answered(&clock, FakeHost::quiet()), json!({ "ok": {} }));
    }

    #[test]
    fn usage_counts_fuel_memory_and_requests() {
        let outcome = call(&answering(r#"{"ok":{}}"#), FakeHost::quiet(), Some(fake_account()), Budget::CALL);
        assert!(outcome.result.is_ok());
        assert_eq!(outcome.usage.slices, 1);
        assert!(outcome.usage.fuel > 0 && outcome.usage.fuel < 1_000, "{:?}", outcome.usage);
        assert_eq!(outcome.usage.memory, 65_536);
        assert_eq!(outcome.usage.requests, 0);
    }
}
