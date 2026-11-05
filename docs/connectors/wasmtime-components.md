# Connectors as Wasmtime components

A connector is a folder with `manifest.json` and a WebAssembly component built against a typed WIT
world, `fastgit:connector@1.0.0`, run by Wasmtime 50 as Zed runs extensions, minus WASI and tokio.
- **Network:** only through a `session` resource lent for one call. FastGit attaches the token,
  which never enters the sandbox.
- **Idle cost:** each call gets a fresh instance, so a started connector is ~0.2 MB of compiled
  code, with no threads and no CPU.
- **Speed:** a full GitHub connector is a 348 KB component that parses 100 repositories in ~2 ms.
- **Price:** Cranelift adds **14.6 MB** to the exe, or 11.6 MB at `opt-level = "s"`. The runtime
  alone adds **2.0 MB** but can't compile.
- **Verdict:** the typed interface is worth ~10 MB only if an interpreter is too slow, and the
  probes suggest it isn't (§13).

## Measured numbers

These come from probes, not FastGit: a baseline exe (serde_json, anyhow, futures, windows-sys),
and engine exes that call a real connector through every export. All were built with FastGit's
release profile and rustc 1.99.1 on Windows 11 x86-64. Timings are ranges, because other builds
ran alongside.

| Exe (Wasmtime 50.0.1, component model, async) | Bytes | Delta | Of 37,174,784 |
|---|---|---|---|
| Baseline, built twice | 272,896 / 272,896 | — | — |
| Runtime only, loads precompiled native code | 2,248,704 | +1,975,808 | +5.3% |
| Runtime only + Pulley interpreter, built twice | 2,356,736 / 2,356,736 | +2,083,840 | +5.6% |
| Cranelift JIT, built twice | 14,861,824 / 14,860,800 | +14,588,928 | +39.2% |
| Cranelift, compiler crates `opt-level = "s"` | 11,849,728 | +11,576,832 | +31.1% |
| Cranelift, compiler crates `opt-level = "z"` | 11,210,752 | +10,937,856 | +29.4% |
| Winch (links Cranelift too, for trampolines) | 14,289,920 | +14,017,024 | +37.7% |
| Cranelift emitting Pulley bytecode | 14,987,264 | +14,714,368 | +39.6% |
| Cranelift + `component-model-async` | 16,051,200 | +15,778,304 | +42.4% |
| Bindings: all 21 exports called vs one, two builds each | 2,356,736 vs 2,031,616 | 325,120 | per WIT version, upper bound |

| Per connector (GitHub probe) | Cranelift | Winch | Pulley (both Pulley builds) | Runtime only + native |
|---|---|---|---|---|
| Cold compile, first / median of 10 | 452 / 458 ms (later 541 / 590) | 115 / 100 ms | 649 / 741 ms | separate compiler |
| `.cwasm` cache file | 1,127,768 B | 1,721,736 B | 914,440 B | 1,127,832 B |
| Start from cache (load, link, find exports) | 7.8–9.5 ms | 11.6 ms | 9.0–9.6 ms | 9.2 ms |
| Private bytes, 1 / 10 started | +292–404 / +1,720–1,808 KiB | +332 / +1,688 KiB | +180–392 / +1,724–1,848 KiB | +180 / +1,580 KiB |
| Working set, 1 / 10 started | +528–616 / +2,276–2,396 KiB | +364 / +2,004 KiB | +628–652 / +2,244–2,412 KiB | +660 / +2,512 KiB |
| New instance per call | 82–131 µs | 123 µs | 176–187 µs | 178 µs |
| List 100 repos (545 KB JSON), fresh instance | 1.6–2.2 ms | 3.9 ms | 31.6–43.4 ms | 1.7 ms |
| Threads before → after starting 10 | 4 → 4 | 4 → 4 | 4 → 4 | 4 → 4 |
| CPU in 10 s idle (10 started, 10 live) | 0 ns | 0 ns | 0 ns | 0 ns |

Also measured on Cranelift:
- **Component:** 348,326 B at `opt-level = "s"` (117,504 gzipped); 311,963 B at `"z"`.
- **The listing:** 1.28 ms with no interruption, 1.73 ms with epochs, 1.6–2.1 ms with fuel.
- **Fuel:** 11.4 M for the listing; a loop burns 10⁹ in 47–58 ms.
- **A live instance after a listing:** +1.67 MB, or +0.78 MB with a 128 KiB guest stack.
- **Faults:** a panic traps in 66–86 µs, a hog stops at 64 MiB in 5.7–7.2 ms, and a stalled call
  drops without blocking.
- **Dependencies:** no tokio, hyper, rustls or mio.

Not measured: FastGit's own delta (it already links object, gimli and hashbrown), macOS, Linux.

## 1. What to copy from Zed, and what to drop

Zed's `extension_host` (Wasmtime 48.0.7: `runtime, cranelift, component-model, async,
incremental-cache, parallel-compilation`) has:
- one global `Engine`, and a `Linker` per API version;
- bindgen modules `since_v0_0_1` … `since_v0_8_0`, picked by a `zed:api-version` custom section;
- `wasmtime-wasi` on tokio, and a long-lived `Store` per extension;
- a task calling `increment_epoch` every 100 ms, forever;
- an in-memory cache only, so every launch recompiles.

**Copy:** WIT and `bindgen!` on both sides, plain `cargo build --target wasm32-wasip2`, one
`Engine` and `Linker`, capability checks in host functions, a module per *major* version.

**Drop:** WASI and tokio; the epoch ticker (fuel needs no timer); rayon compiling; the memory cache,
for an on-disk `.cwasm`; the custom section, which package semver replaces; long-lived stores.

## 2. Package, manifest and folders

The connector folder is `<id>/manifest.json` plus `<id>/connector.wasm`:

```json
{
  "schema": 1, "id": "github", "name": "GitHub", "version": "1.0.0", "component": "connector.wasm",
  "servers": [{ "name": "GitHub", "web": "https://github.com", "ssh_hosts": ["github.com", "ssh.github.com"],
                "endpoints": { "api": "https://api.github.com/", "graphql": "https://api.github.com/graphql" } }],
  "self_hosted": { "name": "GitHub Enterprise Server", "endpoints": { "api": "{web}/api/v3/", "graphql": "{web}/api/graphql" } },
  "sign_in": [
    { "method": "device", "servers": "public", "client_id": "Ov23li…", "token": { "kind": "bearer" },
      "device_code_url": "https://github.com/login/device/code", "token_url": "https://github.com/login/oauth/access_token",
      "scopes": ["repo", "read:org", "write:public_key", "workflow"] },
    { "method": "pasted_token", "token": { "kind": "bearer" },
      "create_url": "{web}/settings/tokens/new?scopes=repo,read:org,write:public_key,workflow&description=FastGit" },
    { "method": "git_credential", "token": { "kind": "bearer" } }
  ],
  "features": ["repositories", "create-repository", "ssh-keys", "pull-requests", "reviews", "merge"]
}
```

The sign-in fields follow `host-services.md`. Token schemes are a closed list, so a manifest can't
send a token into a URL or a body:

| Platform | Pasted token | OAuth or device token |
|---|---|---|
| GitHub | `bearer` | `bearer` |
| GitLab | `{ "kind": "header", "name": "PRIVATE-TOKEN" }` | `bearer` |
| Azure DevOps | `{ "kind": "basic", "username": "" }` | `bearer` (Entra ID) |

The interface version is read from the component's import names (`…/http@1.0.0`).

**Folders.**
- **Bundled:** `connectors/` beside the exe (`Contents/Resources/` on macOS), or
  `CARGO_MANIFEST_DIR/connectors` in debug builds.
- **Installed:** `%APPDATA%\FastGit\connectors\`, `~/Library/Application Support/FastGit/connectors/`
  or `$XDG_DATA_HOME/fastgit/connectors/`.

**Approvals.** A grant (endpoints, self-hosted or not, sign-in URLs, features) lives in host
services' `connectors.json`.
- **Updates** within the grant start silently; more waits, showing only the additions.
- **A self-hosted address**, typed while adding an account, is granted to that account.
- **Bundled** connectors are pre-approved only if their SHA-256 matches one compiled into the exe,
  since portable installs leave the folder writable.

**Clashes.** An installed connector reusing a bundled id needs its own approval. Two connectors on
one server are fine; FastGit asks only when a remote matches accounts in both, and remembers per
remote owner.

**In this repository,** each crate has its own `[workspace]`, so FastGit's `cargo build` needs no
wasm target:

```text
connectors/wit/connector.wit   read by FastGit's bindgen! and the SDK
connectors/sdk/                fastgit-connector: generate!, request helpers, Link parsing, panic hook
connectors/github/             cdylib, manifest.json, tests/fixtures/, committed connector.wasm + .sha256
connectors/fake/               misbehaving test connector (committed .wasm)
connectors/template/           the copy-me start
scripts/build-connectors.ps1   cargo build --release --target wasm32-wasip2, copy, hash
```

`build-and-relaunch.ps1` copies changed `manifest.json` and `connector.wasm` files into
`build/connectors/<id>/`, but never compiles wasm. A test catches stale committed builds.

## 3. Engine and sandbox

**Configuration.**
- **Runtime only** can't compile a third party's `.wasm`.
- **Pulley bytecode** is portable, but tied to one Wasmtime version.
- **Winch** compiles 4× faster, runs 2.4× slower, and isn't smaller.
- **Recommended:** Cranelift with its crates at `opt-level = "s"` (+11.6 MB), costing 0.65 s of
  compiling per new connector.
- **Alternative:** a runtime-only FastGit (+2.0 MB) plus a `fastgit-compile` helper exe. The
  probe ran helper-built code at full speed (1.7 ms), with an unchanged start-up working set
  (4.76 vs 4.78 MB). The helper must be a non-default workspace member, or feature unification
  pulls Cranelift back in.

```toml
wasmtime = { version = "50.0.1", default-features = false, features = ["runtime", "component-model", "async", "anyhow", "std", "cranelift"] }

[profile.release.package.cranelift-codegen]
opt-level = "s"
```

The same override applies to cranelift-frontend, cranelift-assembler-x64, regalloc2,
wasmtime-internal-cranelift and wasmparser.

```rust
pub(crate) fn engine() -> wasmtime::Result<Engine> {
    let mut config = Config::new();
    config.wasm_component_model(true);
    config.consume_fuel(true);
    Engine::new(&config)
}
```

**Cache.**
- **Where:** `%LOCALAPPDATA%\FastGit\cache\connectors\` (or the OS cache folder).
- **Name:** `<id>-<sha256(component)[..16]>-<hash of Engine::precompile_compatibility_hash()>.cwasm`,
  written to `.unfinished`, then renamed.
- **Invalidation:** an upgraded Wasmtime changes the name; a file `deserialize_file` rejects is
  rebuilt; unused files are swept.
- **Safety:** deserializing loads native code. Only FastGit writes there.

**No WASI.** Rust std on `wasm32-wasip2` imports 14 WASI interfaces the probe never used: `io/*`,
`clocks/monotonic-clock`, `cli/{stdin,stdout,stderr,environment,exit}` and `cli/terminal-*`. Only
panics and out-of-memory reach them.
- **Stubs:** FastGit links each `wasi:*` import as a trap and refuses any import outside
  `fastgit:connector/`. A panic trapped in 66 µs.
- **Pitfall (measured):** `Linker::define_unknown_imports_as_traps` defines the exact
  `fastgit:connector/http@1.0.0` and so shadows the semver match to `@1.1.0`. Old connectors then
  fail with "resource type mismatch". Stub `wasi:*` by name instead (~40 lines).
- **Rejected:** `wasmtime-wasi` brings tokio, files and sockets to deny. `wasi:http` exposes raw
  headers, streams and pollables, and needs hyper, tokio and rustls.
- **`session.send`** takes whole requests. FastGit adds the token after the guest has let go; the
  probe's transport asserted the added header on every request.

**Limits.**
- **Memory:** 64 MiB per call (`StoreLimits`).
- **CPU:** 2×10⁹ fuel per call, about 100 ms native, 175× the largest measured call.
- **Traps:** they become `ConnectorError::Crashed`, with the last log lines.
- **Network:** the host's 30 s timeout bounds waits.

**An instance per call** costs 0.08–0.19 ms. A per-account instance would keep its grown memory
(+1.67 MB). Per call gives unlimited concurrency, trivial cancellation and nothing shared between
accounts. "Started" means an `InstancePre` with resolved exports.

## 4. The interface

```wit
package fastgit:connector@1.0.0;

interface types {
  type cursor = string;
  type timestamp = string;
  record server { web: string, self-hosted: bool, endpoints: list<tuple<string, string>> }
  record user { login: string, name: option<string>, avatar-url: option<string> }
  record repo-ref { owner: string, name: string }
  record field-problem { field: string, message: string }
  variant error {
    not-signed-in, token-expired, missing-permission(string), rate-limited(u32), not-found,
    invalid(list<field-problem>), refused(string), network(string), unsupported, server(tuple<u16, string>),
  }
}

interface http {
  use types.{server};
  enum method { get, head, post, put, patch, delete }
  record request { method: method, url: string, headers: list<tuple<string, string>>, body: option<list<u8>> }
  record response { status: u16, headers: list<tuple<string, string>>, body: list<u8> }
  variant failure {
    not-signed-in, origin-not-allowed(string), header-not-allowed(string), rate-limited(u32), timed-out, too-large, network(string),
  }
  resource session {
    server: func() -> server;
    send: func(request: request) -> result<response, failure>;
    send-all: func(requests: list<request>) -> list<result<response, failure>>;
  }
}

interface log { enum level { debug, info, warn, error } write: func(level: level, message: string); }
interface clock { now: func() -> s64; }

interface account {
  use types.{server, user, repo-ref, error};
  use http.{session};
  record recognized { server: u32, repo: repo-ref }
  recognize: func(remote-url: string, servers: list<server>) -> option<recognized>;
  whoami: func(session: borrow<session>) -> result<user, error>;
}

interface repositories {
  use types.{cursor, timestamp, repo-ref, error};
  use http.{session};
  enum visibility { public, private, internal }
  enum owner-kind { user, organization }
  record owner { login: string, kind: owner-kind, avatar-url: option<string> }
  record clone-urls { https: string, ssh: option<string> }
  record repository {
    repo: repo-ref, description: option<string>, visibility: visibility, default-branch: option<string>,
    clone: clone-urls, web-url: string, updated-at: option<timestamp>, fork: bool, archived: bool,
  }
  record page { items: list<repository>, next: option<cursor> }
  record query { text: option<string>, owner: option<string> }
  record new-repository { owner: string, name: string, description: option<string>, visibility: visibility, initialize: bool }
  list-owners: func(session: borrow<session>) -> result<list<owner>, error>;
  list-repositories: func(session: borrow<session>, query: query, page: option<cursor>) -> result<page, error>;
  get-repository: func(session: borrow<session>, repo: repo-ref) -> result<repository, error>;
  create-repository: func(session: borrow<session>, repository: new-repository) -> result<repository, error>;
}

interface ssh-keys {
  use types.{timestamp, error};
  use http.{session};
  record ssh-key { id: string, title: string, public-key: string, created-at: option<timestamp> }
  list-keys: func(session: borrow<session>) -> result<list<ssh-key>, error>;
  add-key: func(session: borrow<session>, title: string, public-key: string) -> result<ssh-key, error>;
}

interface pull-requests {
  use types.{cursor, timestamp, user, repo-ref, error};
  use http.{session};
  enum state { open, closed, merged }
  enum state-filter { open, closed, merged, all }
  record tip { branch: string, sha: string, repo: option<repo-ref> }
  record summary {
    number: u64, title: string, author: user, state: state, draft: bool, base: tip, head: tip,
    created-at: timestamp, updated-at: timestamp, web-url: string,
  }
  record page { items: list<summary>, next: option<cursor> }
  variant mergeability { checking, ready, conflicts, behind, blocked(string) }
  enum merge-method { merge, squash, rebase, semi-linear, fast-forward }
  record head-fetch { refspec: string, sha: string, url: option<string> }
  record details { summary: summary, body: string, mergeability: mergeability, merge-methods: list<merge-method>, fetch: head-fetch }
  enum check-state { pending, running, success, failure, neutral, cancelled, skipped, action-required }
  record check { name: string, state: check-state, description: option<string>, url: option<string> }
  record new-pull-request { base: string, head: string, head-owner: option<string>, title: string, body: string, draft: bool }
  record merge-options { method: merge-method, title: option<string>, message: option<string>, expected-head: string }
  list-pull-requests: func(session: borrow<session>, repo: repo-ref, state: state-filter, page: option<cursor>) -> result<page, error>;
  get-pull-request: func(session: borrow<session>, repo: repo-ref, number: u64) -> result<details, error>;
  list-checks: func(session: borrow<session>, repo: repo-ref, sha: string) -> result<list<check>, error>;
  create-pull-request: func(session: borrow<session>, repo: repo-ref, pull-request: new-pull-request) -> result<summary, error>;
  merge: func(session: borrow<session>, repo: repo-ref, number: u64, options: merge-options) -> result<string, error>;
}

interface reviews {
  use types.{cursor, timestamp, user, repo-ref, error};
  use http.{session};
  record line { old: option<u32>, new: option<u32> }
  record spot { path: string, old-path: option<string>, first: option<line>, last: line }
  record anchor { spot: spot, commit: string }
  record comment { id: string, author: user, body: string, created-at: timestamp, reply-to: option<string>, web-url: option<string> }
  record thread { id: string, anchor: option<anchor>, outdated: bool, resolved: bool, comments: list<comment> }
  record comment-page { items: list<comment>, next: option<cursor> }
  enum verdict { comment, approve, request-changes }
  record draft { spot: spot, body: string }
  record review { commit: string, verdict: verdict, body: string, drafts: list<draft> }
  record unfinished { posted: u32, error: error }
  list-threads: func(session: borrow<session>, repo: repo-ref, number: u64) -> result<list<thread>, error>;
  list-conversation: func(session: borrow<session>, repo: repo-ref, number: u64, page: option<cursor>) -> result<comment-page, error>;
  add-comment: func(session: borrow<session>, repo: repo-ref, number: u64, body: string) -> result<comment, error>;
  reply-to-thread: func(session: borrow<session>, repo: repo-ref, number: u64, thread: string, body: string) -> result<comment, error>;
  submit-review: func(session: borrow<session>, repo: repo-ref, number: u64, review: review) -> result<_, unfinished>;
}

world connector {
  import http; import log; import clock;
  export account; export repositories; export ssh-keys; export pull-requests; export reviews;
}
```

This file compiles with wit-bindgen 0.63. Four choices in it matter:
- **Sessions** are borrowed. A `borrow<session>` can't outlive its call, so a connector can't keep
  one or reach another account.
- **Diff lines** carry both numbers, enough for GitHub's sides, GitLab's `old_line`/`new_line` and
  Azure's left and right positions.
- **Publishing** is `create-repository` with `initialize: false`; FastGit then adds the remote and
  pushes.
- **Sign-in** stays in the host, which checks each token with `whoami`.

## 5. Four operations, guest and host

The guest is from the probe. Its text-search branch, `/search/repositories` with a `user:`
qualifier per owner, is cut here.

```rust
mod bindings {
    wit_bindgen::generate!({ path: "../wit", world: "connector" });
}

struct GitHub;

bindings::export!(GitHub with_types_in bindings);

impl repositories::Guest for GitHub {
    fn list_repositories(session: &Session, query: repositories::Query, page: Option<String>) -> Result<repositories::Page, Error> {
        let api = Api::new(session);
        let url = match (page, &query.owner) {
            (Some(next), _) => api.checked_cursor(next)?,
            (None, None) => api.rest("/user/repos?per_page=100&sort=updated&affiliation=owner,collaborator,organization_member"),
            (None, Some(owner)) => api.rest(&format!("/users/{}/repos?per_page=100&sort=updated", query_value(owner))),
        };
        let (body, next) = api.get_page(&url)?;
        let repos: Vec<GitHubRepo> = shapes::parse(&body)?;
        Ok(repositories::Page { items: repos.into_iter().map(Into::into).collect(), next })
    }
}

impl pull_requests::Guest for GitHub {
    fn create_pull_request(session: &Session, repo: RepoRef, pull_request: pull_requests::NewPullRequest) -> Result<pull_requests::Summary, Error> {
        let api = Api::new(session);
        let head = match &pull_request.head_owner {
            Some(owner) => format!("{owner}:{}", pull_request.head),
            None => pull_request.head.clone(),
        };
        let body = json!({ "title": pull_request.title, "head": head, "base": pull_request.base,
                           "body": pull_request.body, "draft": pull_request.draft });
        let created: GitHubPull = api.send_json(Method::Post, &api.repo(&repo, "/pulls"), &body)?;
        Ok(created.summary())
    }

    fn merge(session: &Session, repo: RepoRef, number: u64, options: pull_requests::MergeOptions) -> Result<String, Error> {
        let api = Api::new(session);
        let method = match options.method {
            pull_requests::MergeMethod::Merge => "merge",
            pull_requests::MergeMethod::Squash => "squash",
            pull_requests::MergeMethod::Rebase => "rebase",
            pull_requests::MergeMethod::SemiLinear | pull_requests::MergeMethod::FastForward => return Err(Error::Unsupported),
        };
        let body = json!({ "merge_method": method, "commit_title": options.title,
                           "commit_message": options.message, "sha": options.expected_head });
        let merged: Merged = api.send_json(Method::Put, &api.repo(&repo, &format!("/pulls/{number}/merge")), &body)?;
        Ok(merged.sha)
    }
}

impl reviews::Guest for GitHub {
    fn list_threads(session: &Session, repo: RepoRef, number: u64) -> Result<Vec<reviews::Thread>, Error> {
        let api = Api::new(session);
        let mut threads = Vec::new();
        let mut after: Option<String> = None;
        loop {
            let variables = json!({ "owner": repo.owner, "name": repo.name, "number": number, "after": after });
            let page = api.graphql::<ThreadsReply>(shapes::THREADS_QUERY, variables)?.into_page()?;
            threads.extend(page.nodes.into_iter().map(Into::into));
            if !page.page_info.has_next_page {
                return Ok(threads);
            }
            after = page.page_info.end_cursor;
        }
    }

    fn submit_review(session: &Session, repo: RepoRef, number: u64, review: reviews::Review) -> Result<(), reviews::Unfinished> {
        let api = Api::new(session);
        let event = match review.verdict {
            reviews::Verdict::Comment => "COMMENT",
            reviews::Verdict::Approve => "APPROVE",
            reviews::Verdict::RequestChanges => "REQUEST_CHANGES",
        };
        let comments: Vec<serde_json::Value> = review.drafts.iter().map(shapes::draft_comment).collect();
        let body = json!({ "commit_id": review.commit, "body": review.body, "event": event, "comments": comments });
        api.send_json::<serde_json::Value>(Method::Post, &api.repo(&repo, &format!("/pulls/{number}/reviews")), &body)
            .map(|_| ())
            .map_err(|error| reviews::Unfinished { posted: 0, error })
    }
}
```

The helpers it calls:
- **`THREADS_QUERY`** reads `id isResolved isOutdated path line startLine originalLine diffSide`
  and each comment's `originalCommit{oid}` and `replyTo{id}`.
- **`draft_comment`** maps a `spot` to `path`, `line`, `side`, `start_line` and `start_side`.
- **`get_page`** returns the `Link` header's `rel="next"` URL. Such URLs hold commas, which broke
  a naive split; a native test covers that now.
- **`problem()`** maps statuses:
  - 401 → `token-expired`;
  - 403 or 429 with `x-ratelimit-remaining: 0` → `rate-limited`;
  - 403 or 404 when `X-Accepted-OAuth-Scopes` aren't granted → `missing-permission`;
  - 405 or 409 → `refused`;
  - 422 → `invalid`, from `errors[].field`.

The host lives in the UI-free `src/connect/`:
- **`send_all`** is `send` under `join_all`.
- **`Exports`** holds an `Option<…::GuestIndices>` per interface, and a five-line `Indices` trait
  wraps their `load`.

```rust
mod bindings {
    wasmtime::component::bindgen!({
        path: "connectors/wit",
        world: "connector",
        imports: { default: async | trappable },
        exports: { default: async },
        with: { "fastgit:connector/http.session": super::Session },
    });
}

impl HostSession for CallState {
    async fn send(&mut self, session: Resource<Session>, request: Request) -> wasmtime::Result<Result<Response, Failure>> {
        let account = self.table.get(&session)?.account.clone();
        let reply = self.host.request(&account, ApiRequest::from(request), self.cancel.clone()).await;
        Ok(reply.map(Response::from).map_err(Failure::from))
    }
}

impl Connector {
    pub(crate) async fn list_repositories(self, account: AccountId, query: Query, page: Option<String>) -> Result<Page, ConnectorError> {
        let (mut call, guest) = self.begin(&account, &self.0.exports.repositories).await?;
        let answer = guest.call_list_repositories(&mut call.store, lend(&call.session), &query, page.as_ref()).await;
        call.finish(answer)
    }

    pub(crate) async fn create_pull_request(self, account: AccountId, repo: RepoRef, new: NewPullRequest) -> Result<Summary, ConnectorError> {
        let (mut call, guest) = self.begin(&account, &self.0.exports.pull_requests).await?;
        let answer = guest.call_create_pull_request(&mut call.store, lend(&call.session), &repo, &new).await;
        call.finish(answer)
    }

    pub(crate) async fn list_threads(self, account: AccountId, repo: RepoRef, number: u64) -> Result<Vec<Thread>, ConnectorError> {
        let (mut call, guest) = self.begin(&account, &self.0.exports.reviews).await?;
        let answer = guest.call_list_threads(&mut call.store, lend(&call.session), &repo, number).await;
        call.finish(answer)
    }

    pub(crate) async fn submit_review(self, account: AccountId, repo: RepoRef, number: u64, review: Review) -> Result<(), ReviewError> {
        let (mut call, guest) = self.begin(&account, &self.0.exports.reviews).await?;
        let answer = guest.call_submit_review(&mut call.store, lend(&call.session), &repo, number, &review).await;
        call.finish_review(answer)
    }

    pub(crate) async fn merge(self, account: AccountId, repo: RepoRef, number: u64, options: MergeOptions) -> Result<String, ConnectorError> {
        let (mut call, guest) = self.begin(&account, &self.0.exports.pull_requests).await?;
        let answer = guest.call_merge(&mut call.store, lend(&call.session), &repo, number, &options).await;
        call.finish(answer)
    }

    async fn begin<I: Indices>(&self, account: &AccountId, indices: &Option<I>) -> Result<(Call, I::Guest), ConnectorError> {
        let indices = indices.as_ref().ok_or(ConnectorError::Unsupported)?;
        let mut store = Store::new(&self.0.engine, CallState::new(&self.0, account));
        store.limiter(|state| &mut state.limits);
        store.set_fuel(FUEL_PER_CALL).map_err(ConnectorError::crashed)?;
        let instance = self.0.pre.instantiate_async(&mut store).await.map_err(ConnectorError::crashed)?;
        let guest = indices.load(&mut store, &instance).map_err(ConnectorError::crashed)?;
        let session = store.data_mut().table.push(Session::new(account)).map_err(ConnectorError::crashed)?;
        Ok((Call { store, session }, guest))
    }
}

fn lend(session: &Resource<Session>) -> Resource<Session> {
    Resource::new_borrow(session.rep())
}

impl Call {
    fn finish<T>(self, answer: wasmtime::Result<Result<T, types::Error>>) -> Result<T, ConnectorError> {
        match answer {
            Ok(Ok(value)) => Ok(value),
            Ok(Err(error)) => Err(ConnectorError::Remote(error)),
            Err(trap) => Err(ConnectorError::Crashed { reason: format!("{trap:#}"), log: self.store.into_data().log }),
        }
    }
}
```

The probe ran this host through all 21 exports against recorded GitHub responses, with a fixture
transport standing in for `HostServices`.

## 6. Versioning

- **Start at 1.0.0.** Wasmtime matches semver-compatible names both ways, by major from 1 up but
  only by minor for 0.x (`wasmtime-environ/src/component/names.rs`).
- **Freeze what's released.** Minor versions add interfaces, or functions in host interfaces. A
  changed record means a new interface.
- **One bindgen.** FastGit binds the newest 1.x and resolves each export with
  `GuestIndices::new`; an absent export means unsupported.

Measured with a 1.1.0 WIT that adds an import, `clock.monotonic-millis`, and an export,
`thread-resolution`:

| Connector on FastGit | Result |
|---|---|
| 1.0 on 1.1 | `ConnectorPre::new` refuses; per interface, `repositories` is found and `thread-resolution` absent; listing works |
| 1.1 without the new import, on 1.0 | works; the extra export is ignored |
| 1.1 using `monotonic-millis`, on 1.0 | refused at link ("function implementation is missing"), shown as "needs a newer FastGit" |

A feature is offered when the manifest declares it and its export exists. `unsupported` covers
servers that lack an operation. A 2.0 would keep a `since_v1` module for a while, at up to
0.33 MB.

## 7. Async and lifecycle

- **No tokio.** `send` is an `async fn`. Wasmtime suspends the guest's fiber, and the outer future
  returns `Pending`; the probe used only `futures::executor::block_on`. FastGit gives the future to
  `cx.background_spawn`, so wasm runs inside `poll` on a background thread, bounded by fuel.
- **Concurrency.** A store per call means any number of calls at once. Within a call, `send-all`
  fans out. `component-model-async` would cost +1.19 MB and isn't needed.
- **Cancellation.** Dropping a `Job` drops the future. Wasmtime resumes the fiber with "future
  dropped" (`runtime/fiber.rs`), the guest unwinds as a trap, and the `Store`, the request and its
  `CancelOnDrop` are dropped too.
- **Start-up.** `FastGit::new` sets a `Connectors` global to `Starting`, and one background task
  loads, links and resolves each approved connector, then sets `Ready`.
  - **Speed:** ten cached connectors take ~0.1 s, beside the history read.
  - **Early calls** await a shared future.
  - **Compiles** after an install or upgrade run one at a time.

## 8. FastGit side

```text
src/connect/  mod.rs Connectors (Global), Connector, ConnectorError    manifest.rs   found.rs discover()
              grant.rs Grant, verdict()    engine.rs engine(), linker(), stub_wasi_as_traps(), refuse_foreign_imports()
              cache.rs CompiledCache    sandbox.rs CallState, Session, host impls    calls.rs    bindings.rs
```

```rust
pub(crate) enum ConnectorError { NotStarted, Unsupported, Remote(types::Error), Crashed { reason: String, log: Vec<String> } }
pub(crate) enum ReviewError { Unfinished { posted: u32, error: types::Error }, Connector(ConnectorError) }
pub(crate) enum Verdict { Start, AskFirst { wanted: Grant, granted: Option<Grant> }, Shadowed { by: ConnectorId } }

pub(crate) fn discover(bundled: &Path, installed: &Path) -> Vec<Found>;
pub(crate) fn verdict(found: &Found, approvals: &Approvals) -> Verdict;
pub(crate) fn start(engine: &Engine, linker: &Linker<CallState>, found: &Found, cache: &CompiledCache) -> Result<Connector, StartError>;
pub(crate) fn candidates(connectors: &[Connector], remote_url: &str, accounts: &[AccountView]) -> Vec<(ConnectorId, AccountId, RepoRef)>;
```

Two small changes in `ui/`:
- **`ui/load.rs`** gains `run_future_in_background`, the async twin of `run_in_background`:
  `self.pulls.reload(run_future_in_background(cx, connector.list_pull_requests(..), |this, result, cx| ..))`.
- **`ui::Failure`** gains an optional `Remedy`: sign in again, ask for a scope, retry later, or
  fix fields.

Calls ride only existing triggers:
- **Opening a tab** runs `recognize` (pure) and a pull request read.
- **The end of a `Repository::fetch`** refreshes open lists. That covers the 5-minute poll, the
  window return and Ctrl+Shift+R.
- **Each user action** makes one call.

## 9. The GitHub connector

- **REST and GraphQL.** REST throughout, except review threads: their resolved and outdated state,
  positions and replies by thread id exist only in GraphQL (`{web}/api/graphql` on Enterprise
  Server).
- **Pagination.** The `Link` `rel="next"` URL is the cursor, checked to stay on the API endpoint.
- **Pull request heads.** `refs/pull/N/head` sits in the base repository even for forks, so
  `url: none` means "the matched remote". FastGit fetches it, checks the SHA and diffs locally.
- **Mergeability.** `mergeable: null` reads as `checking`, re-read on the next fetch.
- **Merging.** The methods come from `allow_*`, and the `sha` guard turns a moved head into
  `refused`.
- **Size.** The probe connector is 1,170 lines.

## 10. Platform check

| Interface | GitLab | Azure DevOps |
|---|---|---|
| `repo-ref` | group path `a/b` + project | `org/project` + repo |
| repositories | `/projects?membership=true`; create with `namespace_id` | `{org}/_apis/git/repositories`; README via Pushes API |
| SSH keys | `/user/keys` | no public API → `unsupported` |
| merge methods | `merge_method` (merge, `rebase_merge` → semi-linear, `ff` → fast-forward) + squash | noFastForward, squash, rebase, rebaseMerge → semi-linear |
| head fetch | `refs/merge-requests/N/head` | `refs/pull/N/merge` is a trial merge, so `sourceRefName` at `lastMergeSourceCommit` (fork URL from `forkSource`) |
| threads | discussions; `position` has base/start/head SHAs, old/new path and line, `line_range`; `resolved` | `threadContext` left/right positions; resolved = fixed, wontFix, closed or byDesign |
| anchor commit | `head_sha` | iteration → `sourceRefCommit` |
| submit review | draft notes, `bulk_publish`, `/approve`; request-changes has no documented REST call (gitlab-org/gitlab#451995) → `unsupported` | a thread per draft, then vote 10 or −5 |
| merge | `PUT /merge` with `sha`, `squash` | `PATCH` completed with `lastMergeSourceCommit` |
| tokens | `PRIVATE-TOKEN` or Bearer | Basic `:PAT` or Bearer; who-am-I on `app.vssps.visualstudio.com` |

Misfits, absorbed without WIT changes:
- **Half-posted reviews:** `unfinished.posted` records them.
- **Votes:** Azure's five map onto three verdicts.
- **Merge settings per project:** `merge-methods` is a list.
- **Missing APIs:** `unsupported`.
- **Azure's org-scoped URLs:** `recognize` handles them.

**Gitea's** REST is GitHub-shaped: `Link` headers, `refs/pull/N/head`, `/user/keys`, merge `Do`
values and resolvable review comments, but no GraphQL. Estimate: 800–1,000 lines and ~300 KB, in a
new folder only.

## 11. Author experience

- **Build:** `rustup target add wasm32-wasip2` and `cargo build --release --target wasm32-wasip2`.
  rustc's bundled `wasm-component-ld` emits the component, with no cargo-component (verified).
- **Template:** `opt-level = "s"`, `lto`, `panic = "abort"`, `-C link-arg=-zstack-size=131072`.
- **Unit tests** run natively, because wit-bindgen's imports compile to stubs (verified).
- **No GUI needed:**
  - `fastgit --connector-check <folder>` validates, compiles and prints imports, exports and
    version.
  - `fastgit --connector-replay <folder> <fixtures.json> <operation> <args.json>` runs one
    operation in the real sandbox against recorded responses.
- **Logs** go to the host's 200-line ring, and to stderr in CLI modes. The SDK's panic hook logs
  the message before trapping.
- **Debugging:** `strip = "debuginfo"` keeps names (+70 KB), so traps say where.
- **Other languages,** from their docs, not measured:
  - ComponentizeJS embeds ~8 MB of StarlingMonkey;
  - componentize-py bundles CPython, tens of MB;
  - TinyGo `wasip2` produces hundreds of KB to a few MB.

  All three must be built without WASI, or they trap on the stubs.

## 12. Testing in FastGit

`cargo test` stays offline, with a fixture transport behind `HostServices`.
- **Units:** manifests, grants, cache keys, import refusal, remote candidates.
- **Integration:** compile `connectors/github/connector.wasm` once (~0.5 s), then replay recorded
  responses through every operation.
- **`connectors/fake/`:** loop, memory hog, panic, stalled-then-dropped request, 1.0 against 1.1.
- **A source-hash test** keeps the committed `.wasm` current.

## 13. Risks and unknowns

- **The compiler** is 9.6 MB at `opt-level = "s"` (12.6 MB at the default) of code that runs once
  per connector per release.
- **The interpreter rival.** `wasmi.md` reports +1.11 MB and 13–18 ms for the same page, and
  Pulley's 43 ms shows interpreted speed is enough. Unless typed WIT, borrowed capabilities and
  many languages are worth ~10 MB (or ~1 MB more with the helper), this is the wrong choice.
- **Frozen records.** A forgotten field costs a new interface; JSON absorbs it.
- **macOS:** hardened builds likely need a JIT entitlement; Pulley avoids it at ~20× slower calls.
  Unverified.
- **Trap handlers:** Wasmtime installs process-wide handlers. How they coexist with GPUI and crash
  reporters is untested; `signals_based_traps(false)` is the fallback.
- **Churn:** a Wasmtime major release ships monthly (50 dropped `async_support`), and each
  upgrade recompiles every connector once.
- **Std:** if it ever touches WASI at start-up, every connector traps, until FastGit adds a few
  null functions.

Decisions worth a second look:
- **"Anyone may write one"** (1) is what forces a compiler into FastGit, because portable code
  must be compiled locally. With only bundled connectors, the runtime alone (+2.0 MB) would do.
- **"Starts when FastGit starts"** (3) should mean compiled and linked, not a live instance. A
  live instance per connector costs 0.8–1.7 MB and buys nothing.
- **"Bundled are pre-approved"** should go by hash, not by folder.

## 14. Build plan

1. WIT, `src/connect`, and GitHub `whoami` and `list-repositories` with fixtures. Then measure
   FastGit's real delta and choose in-exe Cranelift or the helper.
2. Manifest, discovery, grants, start-up, CLI modes, SDK and template.
3. With host services: sign-in, accounts, remote → account, repositories, publishing, SSH keys.
4. Pull requests: list, details, checks, head fetch, create, merge.
5. Reviews: threads, conversation, submit, reply.
6. A GitLab spike against the frozen WIT, with no FastGit change allowed, before declaring 1.0.
