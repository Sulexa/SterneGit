# Connectors on wasmi

A connector is a folder holding `connector.json` and a WebAssembly core module built for
`wasm32-unknown-unknown`, run by the wasmi 2.0 interpreter. The module imports six functions from
FastGit and exports one entry point, and every message is JSON. An HTTP request suspends the
connector with a wasmi resumable call, so no thread blocks while it waits. Each operation gets a
fresh instance, dropped when it ends, so an idle connector holds none. Fuel slices and a memory
limit stop runaway connectors, and dropping a `Job` cancels the call and its requests. The cost
is the engine: **+1.11 MB of exe with wasmi at `opt-level = "s"`** (+1.57 MB at opt-level 3).
Running connectors is cheap: 2–4 ms to start one, about 0.5 MB each once started, no threads and
no idle CPU. The component model on wasmi was measured too, and isn't worth it.

## Measured numbers

Probe crates built with FastGit's release profile and rustc 1.99.1. The baseline links
serde_json and anyhow and does the engine probe's JSON work. Other builds were running, so
timings are ranges.

| What | Result |
| --- | --- |
| Exe delta, wasmi + host glue, opt-level 3 | **+1,568,256 B** (382,464 → 1,950,720) |
| Exe delta, wasmi and wasmparser at opt-level "s" (recommended) | **+1,114,624 B**; two other builds +1,111,040 and +1,115,648 |
| Other knobs: opt "z"; `portable-dispatch`; `indirect-dispatch`; wasmi 1.1.0 | +1,103,360; +1,573,376; +1,661,952; +1,571,328 B |
| Component model on wasmi (wasm_component_layer 0.1.18) | +5,296,640 B, and it only builds against wasmi 0.40 |
| Where the opt-3 engine's code goes | wasmi 688 KB and wasmparser 294 KB of the probe's 1.5 MB of code |
| `.wasm`: six-operation GitHub stand-in (serde_json), opt "s" / "z" | 271,732 / 256,434 B |
| `.wasm`: list-only connector, serde_json / miniserde | 68,552 / 48,967 B |
| Start one connector, cold, fresh process (load + handshake) | 1.9–2.1 ms (Lazy, 128 KiB stack); 3.6–4.4 ms (LazyTranslation) |
| Start one connector, warm: load + first call | Lazy 0.27 + 1.5 ms; LazyTranslation 1.9 + 1.3 ms; Eager 6.9 + 0.7 ms |
| Start 10 connectors on one thread | 19 ms Lazy, 36 ms LazyTranslation, 87 ms Eager |
| Private bytes per started connector, no instance kept | 463 KB Lazy, 479 KB LazyTranslation, 1,178 KB Eager; about 650 KB after a listing |
| Memory per live instance | 1,130 KiB with Rust's default 1 MiB stack; about 160 KiB with a 128 KiB stack |
| Per-call overhead: new instance, small call, drop | 0.16 ms (128 KiB stack), 0.54 ms (1 MiB) |
| `repositories.list`, 100 repos, 620 KB JSON | 12.6–13.6 ms in wasmi (14.5–17.9 ms with wasmi at "s") vs 0.64–0.92 ms native; 53 M fuel; linear memory peaks at 1.5–2.4 MB |
| Idle, 11 started connectors, 10 s | 0.000 ms CPU; threads 4 → 4 |
| Misbehaving connectors | endless loop stopped by fuel in 0.3–0.8 s; memory hog stopped at 64 MB in 114 ms; panic trapped and logged in 1.6 ms; wild pointer refused in 6.7 ms |

*Estimates:* the full GitHub connector (about 17 operations) at 0.45–0.6 MB of `.wasm` and
0.8–1.1 MB while started; FastGit's serde code for the protocol types at 100–200 KB, which every
JSON design pays.

## 1. Package, manifest and discovery

A connector is a folder named after its id: `github/connector.json` and `github/github.wasm`.
FastGit reads the manifests in two places:

- **bundled:** `connectors/` next to the exe, or `Contents/Resources/connectors` on macOS;
- **installed:** `%APPDATA%\FastGit\connectors`, `~/Library/Application Support/FastGit/connectors`
  or `$XDG_DATA_HOME/fastgit/connectors`.

The manifest is JSON, because serde_json is linked and toml isn't:

```json
{
  "id": "github", "name": "GitHub", "version": "1.0.0", "module": "github.wasm",
  "interface": { "abi": 1, "protocol": "1.0" },
  "features": ["repositories", "repositories.create", "ssh_keys", "pulls", "pulls.review", "pulls.resolve", "pulls.merge"],
  "services": [
    { "id": "github.com", "label": "GitHub", "auth": "bearer", "git_user": "{login}",
      "endpoints": { "api": "https://api.github.com", "graphql": "https://api.github.com/graphql", "web": "https://github.com" },
      "images": ["https://avatars.githubusercontent.com"],
      "remotes": [{ "hosts": ["github.com", "ssh.github.com"], "path": "{owner}/{name}" }],
      "sign_in": [
        { "method": "device", "client_id": "Ov23li…", "device_url": "https://github.com/login/device/code",
          "token_url": "https://github.com/login/oauth/access_token", "scopes": ["repo", "read:org", "write:public_key"] },
        { "method": "token", "create_url": "https://github.com/settings/tokens/new?scopes=repo,read:org,write:public_key" },
        { "method": "git_credential", "host": "github.com" } ] },
    { "id": "enterprise", "label": "GitHub Enterprise Server", "auth": "bearer", "git_user": "{login}",
      "fields": [{ "id": "server", "label": "Server address", "kind": "url" }],
      "endpoints": { "api": "{server}/api/v3", "graphql": "{server}/api/graphql", "web": "{server}" },
      "remotes": [{ "hosts": ["{server.host}"], "path": "{owner}/{name}" }],
      "sign_in": [{ "method": "token", "create_url": "{server}/settings/tokens/new" },
                  { "method": "git_credential", "host": "{server.host}" }] }
  ]
}
```

**Permissions**, shown at approval, are the endpoint and image origins, the claimed remote hosts,
the features and the sign-in URLs. FastGit stores approved permissions per connector id. A new
connector waits for approval. An update within the approved permissions starts without asking,
even with a new module; one that adds anything waits, showing what it adds. Bundled connectors
never wait.

**The same id in both folders:** the installed copy replaces the bundled one after an approval
that says so, which is how an author tests a new GitHub connector. Accounts are keyed by
connector id and keep working.

**Two connectors claiming one server** is fine, because claims belong to accounts: FastGit asks
once per remote owner (decision 8), and the approval warns that github.com is also claimed.

**Source and shipping.**

- `connectors/github/` is the connector crate, with its own `[workspace]`,
  `rust-toolchain.toml`, cassettes and manifest. `connectors/sdk/` is the author SDK.
  `connectors/protocol/` holds the serde types the SDK and FastGit share, and is FastGit's only
  new path dependency.
- The module is committed as `connectors/github/dist/github.wasm`, so a plain `cargo build` needs
  no wasm target. CI rebuilds it and fails on any difference.
- A tiny `build.rs` copies each manifest and module into `target/<profile>/connectors/`, so
  `cargo run` finds them.
- `scripts/build-and-relaunch.ps1` rebuilds stale modules when the wasm target is installed,
  mirrors `target/release/connectors` into `build/connectors`, and relaunches on either change.

## 2. Engine and sandbox

**wasmi 2.0.0** (1 September 2026), with `default-features = false, features = ["std",
"validate"]`: no `wat`, `memory64` or `simd`, and validation kept for untrusted modules.
FastGit's `Cargo.toml` sets `opt-level = "s"` for the wasmi crates and `wasmparser`, saving
450 KB for 15–30% slower interpretation.

```rust
let mut config = Config::default();
config
    .consume_fuel(true)
    .compilation_mode(CompilationMode::LazyTranslation)
    .allow_start_fn(false)
    .wasm_multi_memory(false)
    .set_max_recursion_depth(4096)
    .enforced_limits(EnforcedLimits::strict());
```

- **Compilation.** `LazyTranslation` validates everything at start, so a broken module is
  reported at once, and translates each function on first use. `Lazy` loads 7× faster but defers
  validation. `Eager` takes 2.5× the memory.
- **Fuel.** Calls run in 20 M-fuel slices of about 6–15 ms. At `OutOfFuel` the host yields, which
  is where cancellation happens, then refuels. The budget is 50 slices (0.3–0.8 s); listings get
  more. A 100-repository page used 3.
- **Memory.** `StoreLimitsBuilder` allows 64 MB, one memory, one instance, four tables and 20,000
  table elements, with `trap_on_grow_failure`. The SDK links with `-zstack-size=131072`, so an
  instance takes 160 KiB, not 1.1 MiB. Rust's stack-first layout makes a stack overflow trap.
- **Traps and hangs** end one call with `Stop`, carrying its log lines; the SDK's panic hook logs
  the message first. Waiting on a request is bounded by the host's HTTP timeouts and the user.
- **One instance per call.** Connectors are stateless: cursors come back as input, and ETag
  caching sits in the host's HTTP layer. A fresh instance costs 0.16 ms, isolates calls, lets
  them run in parallel, and leaves nothing idle.
- **One `Engine` per connector**, because wasmi frees compiled code only with its engine. A
  connector without accounts drops its engine after the handshake and reloads it on sign-in.
- **Target:** `wasm32-unknown-unknown`. FastGit refuses, by name, any other import, such as the
  WASI of a `wasip1` build.
- **Component model on wasmi: no.** `wasm_component_layer` was last released in August 2024 and
  doesn't compile against wasmi 2.0's `wasm_runtime_layer` 0.7. Pinned to wasmi 0.40 it added
  5.3 MB (wasmtime-environ 18, wit-component, two older wasmparsers).

## 3. The interface

### ABI 1

| Import from `fastgit:1` | Meaning |
| --- | --- |
| `take(into: i32)` | Copy the waiting message, the call input or the last response, into guest memory |
| `send(head, head_len, body, body_len: i32) -> i32` | Start a request; returns its id at once |
| `wait(id: i32) -> i32` | Suspend until that response arrives; returns its length for `take` |
| `finish(out, len: i32)` | The call's result |
| `log(level, text, len: i32)` | One log line, 64 KB per call at most |
| `now() -> i64` | Unix time in ms |

The module exports `memory` and `fastgit_call(input_len: i32) -> i32`. A request head is
`{"method","url","headers"}`, with a relative `url` resolved against the account's `api`. A
response is a `u32` head length, the head `{"status","headers","error"}`, then the raw body. The
guest does all the allocating, so the host never calls into it, as in Typst's plugin protocol.

### Calls

A call's input is `{"op", "account": {"api","graphql","web","login"}, "repo": <locator>,
"input"}`. Its output is `{"ok": …}` or `{"err": {"kind": …}}`.

| Operation | Input | Output |
| --- | --- | --- |
| `hello` | versions | `{protocol, sdk}` |
| `account.who_am_i` | — | `Identity {login, name, avatar_url, scopes}` |
| `repositories.list` | `{cursor?, search?}` | `Page<Repository>` |
| `repositories.owners` | — | `[Owner {login, kind, can_create}]` |
| `repositories.create` | `{owner, name, description, visibility, initialize?}` | `Repository` |
| `ssh_keys.list` / `.add` | — / `{title, key}` | `[SshKey]` / `SshKey` |
| `pulls.list` | `{state, cursor?}` | `Page<PullSummary>` |
| `pulls.get` | `{number}` | `PullDetails` |
| `pulls.checks` | `{number}` | `[Check {name, state, url, summary}]` |
| `pulls.create` | `{base, head, title, body, draft}` | `PullSummary` |
| `pulls.threads` | `{number}` | `[Thread]` |
| `pulls.comments` / `.comment` | `{number, cursor?}` / `{number, body}` | `Page<Comment>` / `Comment` |
| `pulls.review` | `{number, commit, verdict, body, comments: [{anchor, body}]}` | `{}` |
| `pulls.reply` / `.resolve` | `{number, thread, body}` / `{number, thread, resolved}` | `Comment` / `Thread` |
| `pulls.merge` | `{number, method, title, message, expected_head}` | `Merged {sha}` |

Publishing is `repositories.create`, after which FastGit adds the remote and pushes. A review
draft stays in FastGit until one `pulls.review` submits it. The main types:

```rust
pub struct PullDetails { pub summary: PullSummary, pub body: String, pub base: Tip, pub head: Tip,
    pub mergeable: Mergeable, pub merge_methods: Vec<MergeMethod>, pub diff: DiffRefs, pub fetch: FetchSpec }
pub struct FetchSpec { pub url: Option<String>, pub refspec: String, pub sha: String }
pub struct Anchor { pub path: String, pub old_path: Option<String>, pub side: Side, pub line: Option<u32>,
    pub start_line: Option<u32>, pub start_side: Option<Side>, pub commit: Option<String>, pub extra: Option<Value> }
pub struct Thread { pub id: String, pub resolved: bool, pub outdated: bool, pub anchor: Anchor, pub comments: Vec<Comment> }
pub enum Failure { SignedOut, Forbidden { needs: Option<String>, message: String },
    RateLimited { retry_after_seconds: u64 }, NotFound, Invalid { fields: Vec<FieldError>, message: String },
    Conflict { message: String }, Network { message: String }, Refused { status: u16, message: String },
    Unsupported, Protocol { message: String } }
```

The host adds `Stopped { reason }` for traps and budgets. FastGit's screens need only these
values, the catalog's `Standing`, and each service's `fields` and `sign_in`.

### Sign-in and the token

Sign-in is a host service driven by the manifest (a device-flow token response is the token
itself), followed by `account.who_am_i`. The host checks and completes every `send`:

1. The URL must be https on one of the account's origins (http only for a typed http server).
2. Connector-set `authorization`, `proxy-authorization`, `cookie` and `private-token` are dropped.
3. The token is attached per `auth`, which a sign-in method may override:
   - `bearer`: `Authorization: Bearer` (GitHub, GitLab OAuth);
   - `private_token`: `PRIVATE-TOKEN` (GitLab PAT);
   - `basic_empty_user`: `Authorization: Basic base64(":" + pat)` (Azure DevOps PAT);
   - `basic_credential`: Git Credential Manager's user and password, as git sends them.
4. `set-cookie` is removed from responses, and cross-origin redirects come back as 3xx.

The token never enters linear memory. `git_user` tells the credential-helper bridge which user
name to give git.

### Four operations in the GitHub connector

The probe's code, compiled to wasm and run in wasmi against recorded responses. The SDK provides
`Request`, `Response` (`next_page()` reads `Link`), the `Forge` trait and `export!`.

```rust
fn checked(response: Response) -> Result<Response, Failure> {
    let limited = response.header("x-ratelimit-remaining") == Some("0") || response.header("retry-after").is_some();
    match response.status {
        200..=299 => Ok(response),
        401 => Err(Failure::SignedOut),
        403 | 429 if limited => Err(Failure::RateLimited { retry_after_seconds: retry_after(&response) }),
        403 => Err(Failure::Forbidden { needs: response.header("x-accepted-oauth-scopes").map(str::to_owned), message: message(&response) }),
        404 => Err(Failure::NotFound),
        409 => Err(Failure::Conflict { message: message(&response) }),
        422 => Err(invalid(&response)),
        status => Err(Failure::Refused { status, message: message(&response) }),
    }
}

impl Forge for GitHub {
    fn repositories(&self, account: &Account, query: RepositoryQuery) -> Result<Page<Repository>, Failure> {
        let searching = query.search.is_some();
        let url = match (query.cursor, query.search) {
            (Some(next), _) => next,
            (None, Some(text)) => search_url(account, &text),
            (None, None) => LIST.to_owned(),
        };
        let response = rest(Request::get(url))?;
        let found: Vec<RestRepository> = match searching {
            true => response.json::<SearchResults>()?.items,
            false => response.json()?,
        };
        Ok(Page { items: found.into_iter().map(Repository::from).collect(), next: response.next_page() })
    }

    fn create_pull(&self, _: &Account, repo: &Locator, new: NewPull) -> Result<PullSummary, Failure> {
        let repo: Repo = repo.parse()?;
        let body = RestNewPull { title: &new.title, body: &new.body, head: &new.head, base: &new.base, draft: new.draft };
        let url = format!("/repos/{}/{}/pulls", repo.owner, repo.name);
        Ok(rest(Request::post(url).json(&body))?.json::<RestPull>()?.into())
    }

    fn threads(&self, account: &Account, repo: &Locator, pull: PullNumber) -> Result<Vec<Thread>, Failure> {
        let repo: Repo = repo.parse()?;
        let (mut threads, mut after) = (Vec::new(), None);
        loop {
            let variables = ThreadsAfter { owner: &repo.owner, name: &repo.name, number: pull.number, after };
            let data: ThreadsData = graphql(account, THREADS, variables)?;
            let page = data.repository.and_then(|found| found.pull_request).ok_or(Failure::NotFound)?.review_threads;
            threads.extend(page.nodes.into_iter().map(Thread::from));
            match (page.page_info.has_next_page, page.page_info.end_cursor) {
                (true, Some(cursor)) => after = Some(cursor),
                _ => return Ok(threads),
            }
        }
    }

    fn submit_review(&self, _: &Account, repo: &Locator, review: Review) -> Result<(), Failure> {
        let repo: Repo = repo.parse()?;
        let event = match review.verdict {
            Verdict::Comment => "COMMENT",
            Verdict::Approve => "APPROVE",
            Verdict::RequestChanges => "REQUEST_CHANGES",
        };
        let comments = review.comments.iter().map(|comment| RestLineComment {
            path: &comment.anchor.path,
            body: &comment.body,
            line: comment.anchor.line,
            side: side(comment.anchor.side),
            start_line: comment.anchor.start_line,
            start_side: comment.anchor.start_line.map(|_| side(comment.anchor.start_side.unwrap_or(comment.anchor.side))),
        }).collect();
        let body = RestReview { commit_id: &review.commit, body: &review.body, event, comments };
        let url = format!("/repos/{}/{}/pulls/{}/reviews", repo.owner, repo.name, review.number);
        rest(Request::post(url).json(&body)).map(drop)
    }

    fn merge(&self, _: &Account, repo: &Locator, merge: Merge) -> Result<Merged, Failure> {
        let repo: Repo = repo.parse()?;
        let merge_method = match merge.method {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        };
        let body = RestMerge { merge_method, commit_title: &merge.title, commit_message: &merge.message, sha: &merge.expected_head };
        let url = format!("/repos/{}/{}/pulls/{}/merge", repo.owner, repo.name, merge.number);
        Ok(Merged { sha: rest(Request::put(url).json(&body))?.json::<RestMerged>()?.sha })
    }
}

export!(GitHub);
```

`THREADS` queries `reviewThreads(first:50, after:$after)` with `isResolved`, `path`, `line`,
`diffSide` and the comments.

## 4. Versioning

- **The ABI version** is the import module name. A breaking change would be `fastgit:2`, linked
  alongside `fastgit:1` for a while. A module importing a version FastGit lacks is refused before
  it is instantiated.
- **The protocol** is `major.minor`, declared in the manifest and confirmed by `hello`, which
  catches a manifest that disagrees with its module. Minor versions only add operations, optional
  fields and enum variants (the host has `#[serde(other)]` fallbacks). Neither side denies
  unknown fields.
- **Features** gate the UI. A stray `Unsupported` reads as "not available".
- **An older connector** runs, with what it lacks hidden. **A newer one** with the same major
  runs too; FastGit ignores what it doesn't know. One needing a newer major or ABI shows "needs a
  newer FastGit" and doesn't start.

## 5. Async, concurrency, cancellation, start-up

A call is a `Send` future on `cx.background_spawn`; the probe checked at compile time that the
store and both suspended-call types are `Send`. `wait` returns a host error, which wasmi turns
into a suspended call:

```rust
step = match step {
    TypedResumableCall::Finished(_) => break,
    TypedResumableCall::HostTrap(paused) => {
        let Some(&Wait(id)) = paused.host_error().downcast_ref::<Wait>() else {
            return Err(Stop::Trapped(paused.host_error().to_string()));
        };
        let framed = store.data_mut().in_flight.arrival(id).await;
        let len = framed.len() as i32;
        store.data_mut().waiting = framed;
        paused.resume(&mut store, &[Val::I32(len)]).map_err(trapped)?
    }
    TypedResumableCall::OutOfFuel(paused) => {
        slices.spend()?;
        smol::future::yield_now().await;
        store.set_fuel(FUEL_PER_SLICE).map_err(trapped)?;
        paused.resume(&mut store).map_err(trapped)?
    }
};
```

- **Concurrency.** `send` starts a request at once, and `arrival(id)` polls every in-flight
  request until `id` is done, so requests sent before a `wait` run concurrently. The guest is
  single-threaded. Separate calls have separate stores and run in parallel, four at a time per
  account.
- **Cancellation.** Dropping the `Job` drops the foreground task, then the background task, then
  the store, the suspended call and its requests. That is immediate while waiting, and within one
  slice (15 ms at most) while running.
- **Start-up.** After the window opens, one background task scans both folders and starts each
  approved connector (validate, `hello`, keep or drop the module), queued behind the tabs'
  history reads. One connector takes 2–4 ms and ten take 19–36 ms, by compilation mode. No timers
  are added.

## 6. FastGit side

`src/connect/` is UI-free. `Found` pairs a manifest with its folder, origin and `Standing`
(`Ready`, `Waiting(Permissions)` or `Unusable(String)`).

```rust
impl Manifest {
    pub(crate) fn parse(text: &str) -> anyhow::Result<Self>;
    pub(crate) fn permissions(&self) -> Permissions;
}
impl Permissions {
    pub(crate) fn beyond(&self, approved: &Permissions) -> Permissions;
}
pub(crate) fn scan(folders: &[(Origin, PathBuf)], approvals: &Approvals) -> Vec<Found>;
pub(crate) fn claims(remote_url: &str, found: &[Found], accounts: &[Account]) -> Vec<Claim>;

impl Started {
    pub(crate) fn start(found: &Found) -> Result<Self, Stop>;
}
pub(crate) trait Host: Send + Sync + 'static {
    fn exchange(&self, account: &AccountContext, head: RequestHead, body: Vec<u8>) -> BoxFuture<'static, Arrival>;
    fn log(&self, connector: &str, level: Level, line: &str);
}
#[derive(Clone)]
pub(crate) struct Forge { started: Arc<Started>, host: Arc<dyn Host>, account: Arc<AccountContext> }
impl Forge {
    pub(crate) fn ask<I: Serialize, O: DeserializeOwned + Send + 'static>(&self, op: &'static str,
        repo: Option<&Locator>, input: &I) -> impl Future<Output = Result<O, Failure>> + Send + 'static;
}
```

`claims` normalizes the remote URL (scp form, `ssh://` with a port, user info, `.git`) and
matches it against each service's `remotes`. The captures become the locator: `{owner, name}`
for GitHub, a multi-segment `{owner...}` for GitLab, `{org, project, name}` for Azure DevOps.
The remembered account choice is keyed by host and `owner`.

`ui/load.rs` gains an async twin of `run_in_background`:

```rust
pub(crate) fn run_async_in_background<V: 'static, R: Send + 'static>(
    cx: &mut Context<V>,
    work: impl Future<Output = R> + Send + 'static,
    then: impl FnOnce(&mut V, R, &mut Context<V>) + 'static,
) -> Task<()>;
```

`src/app/` holds a `Connectors` global (catalog, started connectors, accounts). `Repository`
gets `Option<Hosted { forge, repo }>` from its remotes. It fills `pulls`, `pull` and `threads` as
`Load`s and runs `review` and `merge` as `Job`s, the way it runs `fetch`. Pulls refresh when the
tab opens and when a fetch finishes, so they need no timer of their own.

## 7. The GitHub connector

- **REST** covers lists (`per_page=100`, the `Link` next URL as the cursor), repositories,
  `/user/keys`, creating pull requests, the review (one `POST …/reviews` with `event` and line
  comments) and the merge (`PUT …/merge` with `sha`; 405 means not mergeable, 409 that the head
  moved).
- **Search** uses `/search/repositories` with `user:`/`org:` (30 requests a minute), so FastGit
  also filters loaded pages as the user types.
- **GraphQL** is required for review threads: REST has no thread ids or resolved state, and
  `resolveReviewThread` and `addPullRequestReviewThreadReply` are GraphQL only. `pulls.get` is one
  query: `mergeable`, `mergeStateStatus`, ref oids, `*Allowed` merge settings, `statusCheckRollup`.
- **The head** is `refs/pull/N/head` on the base repository, which works for forks too. FastGit
  fetches it into `refs/fastgit/pull/N` and diffs locally.
- **Missing scopes** come back as a 403, or sometimes a 404, with `X-Accepted-OAuth-Scopes`, which
  becomes `Forbidden { needs }`. A token reused from Git Credential Manager usually lacks
  `write:public_key`, so FastGit offers the device flow.

## 8. Platform check

**GitLab merge requests** fit unchanged:

- The `iid` is the `number`; `refs/merge-requests/N/head` is the `FetchSpec`.
- Discussions are `Thread`s with `resolved`. The `position` (`base_sha`, `start_sha`,
  `head_sha`, `old_line`/`new_line`, `line_range`) is built from `Anchor` plus
  `PullDetails.diff`; `line_code` rides in `Anchor.extra`.
- A review is draft notes, `bulk_publish`, then `POST …/approve`; request changes is a feature
  flag where the server supports it.
- Merge methods are a project setting plus a squash flag. PATs use `private_token`.

**Azure DevOps pull requests** fit with small absorptions:

- `threadContext` maps to `Anchor`; iteration ids and `changeTrackingId` ride in `extra`. Four
  of the six thread statuses count as `resolved`, and the raw status rides in `extra`.
- A review is one thread per comment plus a vote (10 approve, −5 waiting for author), sent
  concurrently through `send`/`wait`.
- `refs/pull/N/merge` is only the merge preview. The head is the source branch, maybe in a fork
  that `FetchSpec.url` names.
- PATs use `basic_empty_user`, Git Credential Manager `basic_credential`. With no public SSH-key
  API, the connector leaves out `ssh_keys`.

**What doesn't fit** goes into `extra` (anchor data, rich statuses), feature flags (missing
verdicts), opaque cursors (continuation tokens) and locator captures (deep paths).

**Gitea** is GitHub-shaped REST (`/api/v1`, `Link` paging, `refs/pull/N/head`, `POST …/merge`
with `Do`) without GraphQL. *Estimate:* 700–900 lines against about 1,300 for GitHub, a
350–450 KB module, and no change in FastGit.

## 9. Author experience

- **Start** from `connectors/template/`: a cdylib `Cargo.toml` (opt "s", LTO,
  `panic = "abort"`), the 128 KiB stack in `.cargo/config.toml`, a manifest and a cassette.
- **Test natively.** On native targets the SDK swaps its imports for a replay backend, so
  `cargo test` runs the connector against cassettes with a debugger and backtraces.
- **Test the real module** with `fastgit --connector-test <folder> [cassette]`, a new
  `src/cli.rs` mode that prints outputs, logs, fuel and memory without a window.
- **Cassettes** are lists of `{method, url, status, headers, body}`. A connector never sees a
  token, so a cassette can't hold one.
- **Logs** go to FastGit's log under `connector:<id>`.
- **Other languages** need only i32 imports and an exported memory. *Estimates, no compilers
  installed:* TinyGo (`wasm-unknown`) 100–250 KB with a JSON library, AssemblyScript 20–60 KB,
  C with cJSON 15–40 KB, Zig with `std.json` 30–80 KB.
- **miniserde** measured 29% smaller than serde_json on the list-only connector.

## 10. Testing in FastGit

- Unit tests cover manifests, permission diffs, remote matching and cursors.
- Engine tests use fake connectors in WAT (the `wat` crate as a dev-dependency) and a fake `Host`.
  The fakes echo, send two requests before waiting, spin, hog memory, trap, pass wild pointers
  and import WASI.
- A golden test replays `connectors/github/tests/cassettes` through the committed module.
- None needs a network or the wasm target. `tests/no_comments.rs` should scan `connectors/` too.

## 11. Risks and unknowns

- **Exe size:** 1.1 MB, 3% of today's exe. Validation alone is about 300 KB and can't go.
- **wasmi 2.0 is two months old and unaudited** (audits covered 0.31 and 0.36–0.38). Resumable
  calls are wasmi-only; the fallback is a blocking thread per running call.
- **Interpretation is 15–20× native**: fine next to the network, but a 3,000-repository account
  spends about 0.4 s of CPU listing.
- **Module size drives memory**, and serde_json generates code per type.
- **The six host functions are the attack surface**, to be fuzzed. The transport, not the
  connector, enforces origins.
- **Committed `.wasm` files** grow git history by 0.3–0.6 MB per change.

**This is the wrong choice** if 1.1 MB is too much (Lua is probably smaller), if authors should
write JavaScript, or if typed WIT bindings outweigh size.

## 12. Decisions I would flag

1. **A connectors folder next to the exe** ends the single-file build. Embedding bundled modules
   with `include_bytes!` would keep the same loader and sandbox.
2. **No comments vs a public SDK.** Rustdoc needs `///`. Either allow it in `connectors/sdk`
   only, or document the SDK in `docs/connectors/authoring.md` and examples.
3. **Approval follows permissions, not code**, so the connector list should say "updated".
4. **"Every connector starts"**: those with no accounts should keep only their manifest.

## 13. Build plan

1. Engine, ABI, SDK, protocol crate, fake-connector tests, `--connector-test`; measure FastGit's
   real size delta.
2. Manifests, discovery, approvals, accounts, remote matching, the remembered account choice.
3. GitHub: who am I, repositories (list, search, create, publish), SSH keys.
4. Reading pull requests: list, details, checks, threads, comments, fetching the head.
5. Writing pull requests: create, review, reply, resolve, merge.
6. The author kit, then a Gitea connector to prove the cost of a fourth platform.
