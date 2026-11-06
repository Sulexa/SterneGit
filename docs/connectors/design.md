# Connectors: the design

Chosen on 2026-11-05. This is the reference for building connectors and drawing their screens. It merges [wasmi.md](wasmi.md) and [host-services.md](host-services.md) with the fixes from [comparison.md](comparison.md), section 4. Those files keep the measurements and the reasoning; where they disagree with this one, this one wins.

## Decisions

| Topic | Decision |
| --- | --- |
| Plugins | Connectors are plugins in folders, and anyone may write one. GitHub's connector is a plugin like any other. |
| Engine | WebAssembly core modules (`wasm32-unknown-unknown`) on wasmi 2.0, exchanging JSON messages. |
| Sandbox | A connector can only make web requests to its account's servers, through FastGit, which adds the token. It sees no token, file, process, SSH key or other account. |
| Start | Every approved connector starts at launch: compiled and greeted on a background task, with no live instance. |
| Approval | A new connector, or an update asking for more, waits for the user. An update within its grant starts and is marked "updated". |
| Bundled | A `connectors/` folder beside the exe. The exe carries the SHA-256 of every bundled file, and a file that doesn't match is treated as installed. |
| Installing | Copy a connector into the user's connectors folder. No store, no auto-update. |
| Screens | Connectors only supply data. FastGit draws every screen, in the mockup first. |
| Network | Only when a tab opens, after each fetch (every 5 minutes, on window return, Ctrl+Shift+R) and when the user acts. No new timers. |
| v1 platform | GitHub: github.com, and Enterprise Server through a server address and a token. |
| v1 features | Sign-in; who am I; browse, clone, create and publish repositories; SSH keys; pull requests (list, view, checks, create, check out, review with line comments, reply, resolve, merge). |
| Accounts | Several, even two on one server. A repository finds its account from its remote URL; when two fit, FastGit asks once per remote owner and remembers. |
| Sign-in | Device flow by default; a pasted token; or the token Git Credential Manager holds, after the user agrees. |
| Tokens | In the OS credential store, under FastGit's own names. FastGit is git's credential helper for the remotes its accounts cover. |
| SSH | One ed25519 key per account, set per repository with `core.sshCommand`. `~/.ssh/config` is never edited. |
| Size | No cap; reported. Measured after milestone 2: +1.89 MB of exe before the new screens (engine 1.36 MB, decoding every answer 0.27 MB, `serde_path_to_error` 0.26 MB). Milestone 3: +0.29 MB more (discovery 0.18 MB, the network layer 0.11 MB). |
| SDK docs | The SDK is comment-free like the rest of FastGit, documented in `docs/connectors/authoring.md`. |

## Words

- **Connector:** a plugin: a folder holding a manifest and a module.
- **Service:** one kind of server a connector handles, such as github.com or GitHub Enterprise Server.
- **Account:** one identity on one server of one service.
- **Forge:** a started connector bound to one account. Screens call it.
- **Grant:** what the user approved for a connector: origins, remote hosts, sign-in URLs, features.
- **Locator:** a repository's `owner` and `name` on a service. `owner` may hold slashes: `group/subgroup` on GitLab, `org/project` on Azure DevOps.
- **Cassette:** recorded HTTP exchanges a connector can be replayed against.

## 1. Pieces and where they live

```text
connectors/protocol/      serde types shared by FastGit and the SDK (FastGit's only new path dependency)
connectors/sdk/           the author crate: Request, Response, the Connector trait, export!, replay on native targets
connectors/github/        the GitHub connector, its own [workspace], rust-toolchain.toml with the wasm target
connectors/github/tests/  cassettes
connectors/github/dist/   connector.json, github.wasm, SHA256SUMS, all committed
connectors/template/      the starting point for authors
scripts/build-connectors.ps1   rebuilds dist/ (needs the wasm target) and rewrites SHA256SUMS

src/settings/   folders per OS, atomic JSON files
src/accounts/   accounts, secrets per OS, device flow, refresh, Git Credential Manager import
src/net/        requests, policy (allow-list, token schemes, redirects), curl transport, proxies, the HostServices implementation
src/connect/    manifests, grants, discovery and pinning, the wasmi engine, ABI, calls, claims, Forge
src/git/        access.rs (scoped credential helper), ssh_key.rs, clone.rs; command.rs gains run_with_access
src/hosting/    the screens: Settings tab (Accounts, Connectors, approval), sign-in, SSH key, remove, Clone, New repository, Publish, the account bar
src/repo/pulls/ the repository tab's pull requests: the left-pane section, branch marks, the overview and files of one, Create and Merge dialogs
src/git/pull_head.rs  fetching a pull request's head into refs/fastgit/pull/N, comparing it with where it branched off, checking it out
src/cli.rs      --git-credential, --connector-test
src/ui/load.rs  run_async_in_background
```

`src/hosting/` draws the screens approved in the mockup on 2026-11-06; `app::connectors::Connectors` is the Global they read and change. `src/repo/pulls/` draws the pull request screens approved the same day. `connect/`, `net/`, `accounts/` and `settings/` stay UI-free, like `git/`.

The `HostServices` trait and its types (section 7) live in `src/net/`, and `net::Gateway` implements it. `connect/` depends on `net/`, never the other way round.

## 2. The connector package

A connector is a folder named after its id, holding `connector.json` and the module it names. The manifest is JSON, since serde_json is already linked:

```json
{
  "interface": { "abi": 1, "protocol": "1.0" },
  "id": "github", "name": "GitHub", "version": "1.0.0", "module": "github.wasm",
  "features": ["repositories", "repositories.create", "ssh_keys", "pulls", "pulls.create", "pulls.review", "pulls.resolve", "pulls.merge"],
  "ssh_key_type": "ed25519",
  "services": [
    { "id": "github.com", "label": "GitHub", "git_user": "x-access-token",
      "endpoints": { "api": "https://api.github.com/", "graphql": "https://api.github.com/graphql", "web": "https://github.com/" },
      "token_endpoints": ["api", "graphql"],
      "images": ["https://avatars.githubusercontent.com/"],
      "remotes": [{ "hosts": ["github.com", "ssh.github.com"], "path": "{owner}/{name}" }],
      "sign_in": [
        { "method": "device", "client_id": "Ov23li…", "token": { "kind": "bearer" },
          "device_code_url": "https://github.com/login/device/code",
          "token_url": "https://github.com/login/oauth/access_token",
          "scopes": ["repo", "read:org", "write:public_key", "workflow"] },
        { "method": "pasted_token", "token": { "kind": "bearer" },
          "create_url": "{web}settings/tokens/new?scopes=repo,read:org,write:public_key,workflow&description=FastGit" },
        { "method": "git_credential", "token": { "kind": "bearer" } } ] },
    { "id": "enterprise", "label": "GitHub Enterprise Server", "git_user": "x-access-token",
      "fields": [{ "id": "server", "label": "Server address", "kind": "url", "example": "https://github.example.com" }],
      "endpoints": { "api": "{server}/api/v3/", "graphql": "{server}/api/graphql", "web": "{server}/" },
      "token_endpoints": ["api", "graphql"],
      "remotes": [{ "hosts": ["{server.host}"], "path": "{owner}/{name}" }],
      "sign_in": [
        { "method": "pasted_token", "token": { "kind": "bearer" }, "create_url": "{web}settings/tokens/new" },
        { "method": "git_credential", "token": { "kind": "bearer" } } ] }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `interface.abi` | The ABI the module imports (`fastgit:1`). |
| `interface.protocol` | The protocol version of its messages. `hello` must confirm it. |
| `features` | What the connector offers. Each feature unlocks screens and actions. |
| `ssh_key_type` | The key type FastGit generates for its accounts. Azure DevOps is believed to need `rsa`. |
| `services[].fields` | What the user types when adding an account. Only `url` and `text` kinds in v1. |
| `services[].endpoints` | Named origins. `{server}` and `{server.host}` come from the fields. |
| `services[].token_endpoints` | The only endpoints that receive the token. |
| `services[].images` | Token-less origins for avatars (later). |
| `services[].remotes` | The remote hosts the service claims and the path pattern that yields the locator. `{owner...}` captures several segments. |
| `services[].git_user` | The user name git sends with the token. |
| `sign_in[].token` | The closed list of token schemes: `{"kind":"bearer"}`, `{"kind":"header","name":"PRIVATE-TOKEN"}`, `{"kind":"basic","username":""}`. Nothing else, so a manifest can't send a token into a URL or a body. |

**Remote patterns.**
- `claims` normalizes a remote URL first: scp form, `ssh://` with a port, user info, a trailing `.git`.
- Then it matches every service's `remotes`.
- GitLab will use `{owner...}/{name}`. Azure DevOps will use `{org}/{project}/_git/{name}` and `v3/{org}/{project}/{name}`, joining `org/project` into `owner`.
- Matching runs no connector code.

## 3. Finding, approving and starting

**Folders.**
- **Bundled:**
  - `connectors/` beside the exe, or `Contents/Resources/connectors` in a macOS bundle;
  - when that folder is missing, `env!("CARGO_MANIFEST_DIR")/connectors/<id>/dist`, so `cargo run` works.
- **Installed:**
  - Windows: `%APPDATA%\FastGit\connectors\`
  - macOS: `~/Library/Application Support/FastGit/connectors/`
  - Linux: `$XDG_DATA_HOME/fastgit/connectors/`
- A folder whose name differs from its manifest's `id` is refused.
- A folder in the bundled folder that the exe doesn't pin is treated as installed.

**Pinning.**
- `src/connect/bundled.rs` embeds each bundled connector's `dist/SHA256SUMS` with `include_str!`.
- At launch FastGit hashes `connector.json` and the module it names, about 2 ms for 0.6 MB, and later compiles the very bytes it hashed. Matching files are pre-approved; anything else is treated as installed, with the standing `Modified`.
- A `cargo test` checks that every `SHA256SUMS` matches its `dist/` files, and that every `connectors/*/dist` has a pin.
- `.gitattributes` marks `connectors/*/dist/**` as `-text`: with `core.autocrlf`, a checkout would otherwise rewrite `connector.json` with CRLF and break its hash.

**Grants.**
- `connectors.json` records each connector id's version, file hashes and grant: endpoint origins, image origins, remote hosts, sign-in URLs, features.
- A connector asking for nothing beyond its grant starts. A new version or new hashes within the grant are recorded, and the connector shows as `Updated` until the user opens its entry.
- Anything beyond the grant waits, and the approval screen lists only the additions.
- **Same id as a bundled connector:** an installed connector replaces the bundled one only after an approval saying so, which is how an author tests a new GitHub connector. Until then it waits, marked `replaces_bundled`, and the bundled one keeps running. The approval stores `replaces_bundled: true`, and from then on the bundled one isn't listed. Accounts are keyed by connector id and keep working.
- **The file:** `connectors.json` holds `{"version": 1, "connectors": [{"id", "version", "files": {name: sha256}, "grant", "updated", "replaces_bundled"}]}`. Bundled connectors matching their pins need no entry. Entries and fields FastGit can't read survive rewrites.
- **The grant** lists strings exactly as the manifest writes them, templates included (`{server}/api/v3/`, `{server.host}`): the approval covers any server the user types for that service.
- **Two connectors claiming one server** is fine: the approval mentions the existing claim, and FastGit asks once per remote owner which account to use.

**Standing.**

```rust
pub(crate) enum Standing {
    Ready,
    Updated,
    Waiting(Grant),
    Modified(Grant),
    Unusable(String),
    Failed(Stop),
}
```

**Start-up.** After the window opens, one background task does the following for each connector, queued behind the tabs' history reads:
1. scans both folders and checks hashes and grants;
2. compiles each approved module (`LazyTranslation`: validated at once, translated on first use);
3. calls `hello`, then drops the instance and keeps the `Engine` and `Module`.

Measured: 3.6–4.4 ms per connector, 0.46–0.65 MB each.

## 4. The sandbox

wasmi 2.0.x, pinned to its minor version, with `default-features = false` and validation kept on. The wasmi crates and `wasmparser` get `opt-level = "s"` in `[profile.release.package.*]`; that's the 1.11 MB build. The configuration and the call loop are in wasmi.md, sections 2 and 5.

- **One `Engine` per connector,** because wasmi frees compiled code only with its engine.
- **One instance per call.** A call starts from a fresh instance (0.16 ms) and drops it when done. Nothing survives from one call to the next, so nothing crosses between accounts and nothing stays alive while idle.
- **Imports:** only `fastgit:1`. Any other import, such as WASI from a `wasip1` build, is refused by name before instantiation. Start functions are refused.
- **Memory:**
  - 64 MB per instance, one memory, one instance, four tables, 20,000 table elements;
  - growth past a limit traps;
  - the SDK sets a 128 KiB stack, so an instance takes about 160 KiB. A library can't pass link flags to the crate using it, so a connector's `build.rs` calls `fastgit_sdk::build::link_small_stack()`. Measured on a 0.2 MB test connector: 192 KiB at start, 256 KiB after a call.
- **CPU:**
  - fuel in slices of 20 M (6–15 ms); at each slice the call yields, which is where cancellation lands, then refuels;
  - 50 slices per call, and 200 for operations that page through lists;
  - an endless loop stops in 0.3–0.8 s.
- **Stops:**

```rust
pub(crate) enum Stop {
    Refused(String),
    Trapped { message: String, log: Vec<String> },
    OutOfFuel,
    OutOfMemory,
    BadOutput { op: &'static str, path: String, problem: String },
    Cancelled,
}
```

- **`Refused`** is a module that never runs: not valid WebAssembly, a foreign import, a start function, no `memory` or `fastgit_call(i32) -> i32` export, or a `hello` answering a newer major protocol.
- **`BadOutput`** comes from deserializing the result through `serde_path_to_error`, so an author reads `items[0].clone: missing field https`.
- **The SDK's panic hook** logs the message before trapping.
- **wasmi 2.0.0 and the dev profile.** Built at opt-level 2 or 3 in the dev profile, wasmi's tail-call dispatch overflows the stack on long loops, and `Caller::get_export` inside a host function reads a bad instance pointer and crashes. At `"s"`, as in release, both work. So dev builds and tests compile wasmi exactly like release, and the host functions take the memory from the call's data, never from `get_export`.

## 5. ABI 1

| Import from `fastgit:1` | Meaning |
| --- | --- |
| `take(into: i32)` | Copy the waiting message (the call input, or the last response) into guest memory. |
| `send(head, head_len, body, body_len: i32) -> i32` | Start a request; returns its id at once. |
| `wait(id: i32) -> i32` | Suspend until that response arrives; returns its length for `take`. |
| `finish(out, len: i32)` | The call's result. |
| `log(level, text, len: i32)` | One log line; 64 KB per call at most. |
| `now() -> i64` | Unix time in milliseconds. |

- **Exports:** the module exports `memory` and `fastgit_call(input_len: i32) -> i32`. It returns 0; any other status stops the call, and so does returning without one `finish`.
- **Who allocates:** the guest does all the allocating, so the host never calls into it.
- **Request head:** `{"method","url","headers"}`, headers as `[name, value]` pairs. A relative `url`, with or without its leading `/`, goes under the account's `api` endpoint, so `/user/repos` reaches `{server}/api/v3/user/repos` on Enterprise Server.
- **Response:** a little-endian `u32` head length, the head `{"status","headers"}`, then the raw body.
- **Per call:** at most 16 requests sent and not yet read, and 32 MB of request bodies on their way. A pointer outside the guest's memory, a `wait` for a request never sent or already read, or a broken head stops the call.
- **Log levels:** 0 debug, 1 info, 2 warn, 3 error.
- **Transport failures end the call.** If the host can't complete a request (`NetError`: not signed in, origin not allowed, timeout, TLS, proxy, too large, cancelled), the call stops with `CallError::Net`. The guest never sees them; a connector couldn't recover from any of them, and this way every screen handles them alike. This replaces the `error` field in wasmi.md's response head.

## 6. Protocol 1.0

A call's input is `{"op", "account", "repo", "input"}`. Its output is `{"ok": …}` or `{"err": Failure}`. `account` is the `AccountContext` below, and it carries no token.

| Operation | Feature | Input | Output |
| --- | --- | --- | --- |
| `hello` | | `{protocol, sdk}` | `{protocol, sdk}` |
| `account.who_am_i` | | | `Identity` |
| `repositories.owners` | repositories | | `[Owner]` |
| `repositories.list` | repositories | `RepositoryQuery` | `Page<Repository>` |
| `repositories.get` | repositories | (repo) | `Repository` |
| `repositories.create` | repositories.create | `NewRepository` | `Repository` |
| `ssh_keys.list` | ssh_keys | | `[SshKey]` |
| `ssh_keys.add` | ssh_keys | `NewSshKey` | `SshKey` |
| `ssh_keys.host_keys` | ssh_keys | | `[String]`, as `type base64` lines |
| `pulls.list` | pulls | `{state, cursor?}` | `Page<PullSummary>` |
| `pulls.for_branch` | pulls | `{branch}` | `[PullSummary]` |
| `pulls.get` | pulls | `{number}` | `PullDetails` |
| `pulls.checks` | pulls | `{sha}` | `[Check]` |
| `pulls.conversation` | pulls | `{number, cursor?}` | `Page<Comment>` |
| `pulls.comment` | pulls | `{number, body}` | `Comment` |
| `pulls.create` | pulls.create | `NewPull` | `PullSummary` |
| `pulls.threads` | pulls.review | `{number}` | `[Thread]` |
| `pulls.review` | pulls.review | `Review` | `{}`, or `Unfinished` |
| `pulls.reply` | pulls.review | `{number, thread, body}` | `Comment` |
| `pulls.resolve` | pulls.resolve | `{number, thread, resolved}` | `Thread` |
| `pulls.merge` | pulls.merge | `Merge` | `Merged` |

Publishing is `repositories.create` with `initialize: false`; FastGit then adds the remote and pushes. A review stays a draft in FastGit until one `pulls.review` submits it.

- **`hello`** carries FastGit's protocol with `sdk: null`; the connector answers its own protocol and SDK version.
- **No input** is sent as `null`.
- **Inputs in braces** have names in `connectors/protocol`: `PullsQuery`, `BranchQuery`, `PullNumber`, `CommitQuery`, `ConversationQuery`, `NewComment`, `NewReply`, `Resolution`. `pulls.list` filters by `StateFilter`: `open`, `closed`, `all`.
- **`ops`** lists every operation once, with its name, input, output and whether it needs `repo`; FastGit's `Forge` and the SDK's dispatch both use it.
- **`Failure`** also has an `Unknown` fallback, and `Unsupported` reaches screens as `CallError::Unavailable`.
- **A cassette** is one call and its exchanges: `{"op", "account", "repo", "input", "exchanges": [{"method", "url", "status", "headers", "body"}], "output"}`. A `body` that is a string is sent as is, anything else as JSON. Each exchange answers one request with the same method and URL, in order. `output`, when present, is the answer expected.

The types, in `connectors/protocol`:

```rust
pub type Cursor = String;
pub type Timestamp = String;

pub struct AccountContext { pub id: String, pub login: String, pub endpoints: BTreeMap<String, String> }
pub struct RepoRef { pub owner: String, pub name: String }
pub struct Page<T> { pub items: Vec<T>, pub next: Option<Cursor> }

pub struct Identity { pub login: String, pub name: Option<String>, pub avatar_url: Option<String>, pub scopes: Vec<String> }
pub struct Owner { pub login: String, pub kind: OwnerKind, pub avatar_url: Option<String>, pub can_create: bool,
    pub visibilities: Vec<Visibility>, pub can_initialize: bool }
pub enum OwnerKind { User, Organization, Group, Project, Unknown }
pub enum Visibility { Public, Private, Internal, Unknown }

pub struct Repository { pub repo: RepoRef, pub description: Option<String>, pub visibility: Visibility,
    pub default_branch: Option<String>, pub clone: CloneUrls, pub web_url: String,
    pub updated_at: Option<Timestamp>, pub fork: bool, pub archived: bool }
pub struct CloneUrls { pub https: String, pub ssh: Option<String> }
pub struct RepositoryQuery { pub search: Option<String>, pub owner: Option<String>, pub cursor: Option<Cursor> }
pub struct NewRepository { pub owner: String, pub name: String, pub description: Option<String>,
    pub visibility: Visibility, pub initialize: bool }

pub struct SshKey { pub id: String, pub title: String, pub key: String, pub created_at: Option<Timestamp>, pub read_only: bool }
pub struct NewSshKey { pub title: String, pub key: String }

pub struct Person { pub login: String, pub name: Option<String>, pub avatar_url: Option<String> }
pub struct Tip { pub branch: String, pub sha: String, pub repo: Option<RepoRef> }
pub enum PullState { Open, Closed, Merged, Unknown }
pub enum ReviewStanding { Approved, ChangesRequested, ReviewRequired, Unknown }
pub struct PullSummary { pub number: u64, pub title: String, pub state: PullState, pub draft: bool, pub author: Person,
    pub base: Tip, pub head: Tip, pub review: Option<ReviewStanding>, pub checks: Option<CheckState>,
    pub created_at: Timestamp, pub updated_at: Timestamp, pub web_url: String }
pub enum Mergeability { Checking, Ready, Conflicts, Behind, FailingChecks, Draft, Blocked { reason: String }, Unknown }
pub enum MergeMethod { Merge, Squash, Rebase, SemiLinear, FastForward, Unknown }
pub enum Verdict { Comment, Approve, RequestChanges, Unknown }
pub struct FetchSpec { pub url: Option<String>, pub refspec: String, pub sha: String }
pub struct PullDetails { pub summary: PullSummary, pub body: String, pub base_sha: String, pub mergeability: Mergeability,
    pub merge_methods: Vec<MergeMethod>, pub verdicts: Vec<Verdict>, pub fetch: FetchSpec }
pub enum CheckState { Pending, Running, Success, Failure, Neutral, Cancelled, Skipped, ActionRequired, Unknown }
pub struct Check { pub name: String, pub state: CheckState, pub description: Option<String>, pub web_url: Option<String> }
pub struct NewPull { pub base: String, pub head: String, pub head_owner: Option<String>, pub title: String,
    pub body: String, pub draft: bool }

pub struct Line { pub old: Option<u32>, pub new: Option<u32> }
pub struct Spot { pub path: String, pub old_path: Option<String>, pub first: Option<Line>, pub last: Option<Line> }
pub struct Anchor { pub spot: Spot, pub commit: String, pub extra: Option<Value> }
pub struct Comment { pub id: String, pub author: Person, pub body: String, pub created_at: Timestamp,
    pub reply_to: Option<String>, pub web_url: Option<String> }
pub struct Thread { pub id: String, pub anchor: Anchor, pub outdated: bool, pub resolved: bool,
    pub can_resolve: bool, pub can_reply: bool, pub comments: Vec<Comment> }
pub struct Draft { pub spot: Spot, pub body: String }
pub struct Review { pub number: u64, pub commit: String, pub verdict: Verdict, pub body: String, pub drafts: Vec<Draft> }
pub struct Merge { pub number: u64, pub method: MergeMethod, pub title: Option<String>, pub message: Option<String>,
    pub expected_head: String }
pub struct Merged { pub sha: Option<String>, pub queued: bool }

pub struct FieldError { pub field: String, pub message: String }
pub enum Failure {
    SignedOut,
    Forbidden { needs: Option<String>, message: String },
    RateLimited { retry_after_seconds: u64 },
    NotFound,
    Invalid { fields: Vec<FieldError>, message: String },
    Conflict { message: String },
    Refused { status: u16, message: String },
    Unsupported,
    Unfinished { posted: u32, failure: Box<Failure> },
    Protocol { message: String },
}
```

The fields worth explaining:
- **`PullSummary.checks`** is the overall state of the head commit's checks, worst first, when the server gives it with the list (GitHub's GraphQL `statusCheckRollup`); `None` otherwise. Added in milestone 5 as an optional field, so protocol 1.0 still reads older answers.
- **`Line`** holds both line numbers. GitHub reads one side from it, while GitLab needs both on unchanged lines, and FastGit knows both from its own diff.
- **`Spot`** with no `last` is a comment on the whole file, or an outdated thread whose position is gone.
- **`Anchor.extra`** carries platform data FastGit passes back untouched: GitLab's `line_code`, Azure DevOps' iteration ids.
- **`verdicts`** lists what this server accepts for this viewer. GitHub forbids approving your own pull request; GitLab has no "request changes" over REST.
- **`Merged.queued`** is for Azure DevOps, which completes merges later.
- **`Unfinished.posted`** is for reviews a server can't post at once (Azure DevOps threads, GitLab draft notes). FastGit keeps the drafts that weren't sent.

**Versioning.**
- **ABI:** the version is the import module's name. A breaking ABI would be `fastgit:2`, linked alongside `fastgit:1` for a while.
- **Protocol:** `major.minor`. Minor versions only add operations, optional fields and enum values.
- **Fallbacks:** every enum has an `Unknown` fallback through `#[serde(other)]`; those with data are tagged by `kind`. Neither side refuses unknown fields.
- **Older connectors** run with what they lack hidden. **Newer ones** with the same major also run. One needing a newer major or ABI shows "needs a newer FastGit" and doesn't start.
- **Features** gate the UI, and a stray `Unsupported` reads as "not available".

## 7. Host services

### The API the engine calls

```rust
pub struct AccountId(String);
pub struct ConnectorId(String);
pub enum Method { Get, Post, Put, Patch, Delete }
pub struct ApiRequest { pub method: Method, pub url: String, pub headers: Vec<(String, String)>, pub body: Option<Vec<u8>>, pub timeout: Option<Duration> }
pub struct ApiResponse { pub status: u16, pub headers: Vec<(String, String)>, pub body: Vec<u8> }
pub enum NetError { NotSignedIn, ServerNotAllowed(String), HeaderNotAllowed(String), RateLimitedUntil(SystemTime),
    Timeout, Cancelled, Unreachable(String), Tls(String), Proxy(String), TooLarge, NoTransport(String) }
pub struct AccountView { pub id: AccountId, pub server: Url, pub endpoints: Vec<(String, Url)>, pub login: String }
pub enum Level { Debug, Info, Warn, Error }
pub struct Cancel(smol::channel::Receiver<()>);
pub struct CancelOnDrop(smol::channel::Sender<()>);

pub trait HostServices: Send + Sync + 'static {
    fn account(&self, account: &AccountId) -> Option<AccountView>;
    fn request(&self, account: &AccountId, request: ApiRequest, cancel: Cancel) -> BoxFuture<'static, Result<ApiResponse, NetError>>;
    fn pause(&self, account: &AccountId, until: SystemTime);
    fn log(&self, connector: &ConnectorId, level: Level, message: &str);
    fn now(&self) -> SystemTime;
}
```

The engine maps the ABI onto it:
- `send` starts `request` and keeps the future;
- `wait(id)` drives every request in flight until `id` is done, so requests sent before a `wait` run together;
- a call's `CancelOnDrop` lives in the call's future.

### Every request is checked

1. **Allow-list.** The URL must be https on one of the account's endpoint origins, with a path prefix ending at `/`. http is allowed only for a server the user typed as `http://`, and is marked unencrypted.
2. **Token.** It's attached per the sign-in method's scheme, only for `token_endpoints`.
3. **Headers.**
   - Printable ASCII only, no CR or LF.
   - Connectors can't set `Authorization`, `Proxy-Authorization`, `Cookie`, `Host`, `Content-Length`, `Transfer-Encoding`, `Connection`, `Expect`, or their scheme's header.
   - `User-Agent` is `FastGit/<version> <connector>/<version>`.
4. **Redirects:**
   - up to five;
   - 301–303 only for GET and HEAD, while 307 and 308 keep the method and body;
   - a target off the allow-list fails uncontacted, and a token-less target gets no token.
5. **Limits:** 32 MB bodies, a 30 s default timeout, four requests at once per account. `pause` (from `RateLimited`) makes requests fail at once until the reset.
6. **Logs:** 200 lines per connector, in memory, with the token and its Basic form replaced by `•••`. `set-cookie` is removed from responses.

### Transport: the system's curl

- **Programs:** `%SystemRoot%\System32\curl.exe`, `/usr/bin/curl` on macOS, `curl` from `PATH` on Linux.
- **How it runs:** with exactly `-q --config -`. The request, including the token, goes to stdin, never into arguments.
- **Process:** `DETACHED_PROCESS` on Windows, run by `std::process` on a worker thread that also reads the proxy settings. The request's future kills curl when it's cancelled or dropped. `smol::process` and `smol::unblock` did the same for 204 KB more, measured at milestone 3, since nothing else in FastGit links async-process or the blocking pool.
- **Proxies and certificates:**
  - `git config --get-urlmatch http <origin>` supplies `proxy`, `proxy-anyauth`, `cacert`, `ssl-no-revoke` and `insecure`.
  - curl reads `https_proxy`, `ALL_PROXY` and `NO_PROXY` itself, as git does.
  - System proxies are ignored, as git ignores them.
- **Exit codes:** 28 is `Timeout`; 6 and 7 `Unreachable`; 35 and 60 `Tls`; 5 and 97 `Proxy`.
- **No curl on Linux:** "Install curl to use accounts".
- **Cost:** about 110 ms per request (495 ms for the first), and no memory in FastGit. Independent calls run in parallel. The probe measured +34 KB of exe; in FastGit the gateway, policy and curl transport together cost about 114 KB once milestone 3 made them reachable.
- **Upgrade path, also the Linux fallback:** git's own libcurl loaded at run time, +20 KB and 23 ms per request.
- The config syntax and escaping are in host-services.md, section 1.

### Tokens

- **Where:**
  - Windows: a generic credential `FastGit/<account>`, `CRED_PERSIST_LOCAL_MACHINE`;
  - macOS: a generic password, service `FastGit`;
  - Linux: an oo7 item with `application=fastgit`.
- **Code:** written on the bindings GPUI already links. Not GPUI's own credential API, which files entries under Zed's names.
- **Names:** account ids are `<connector>:<host>:<user id>`, since the numeric id survives renames. Refresh tokens go in `<name>#refresh`.
- **Linux without a Secret Service:** tokens last the login session, in `$XDG_RUNTIME_DIR/fastgit/tokens`. "Remember on this computer, unencrypted" writes `$XDG_STATE_HOME/fastgit/tokens` instead.
- **When:** nothing is read at start-up. The first read comes with the first network use.

### Sign-in

- **Device flow (RFC 8628), run by FastGit from the manifest, the default:**
  - The dialog shows the user code and opens the verification page.
  - It polls on a timer inside the dialog's `Job`. That is the design's only timer, and it ends with the dialog.
  - `slow_down` adds 5 s to the polling interval; `expired_token` offers a new code.
  - The token is stored only after `account.who_am_i` succeeds with it.
- **OAuth App, not GitHub App:**
  - App tokens see only the repositories where the app is installed, expire every 8 hours, and refreshing them needs a secret a desktop app can't keep.
  - Scopes: `repo read:org write:public_key workflow`.
  - "Enable Device Flow" must be ticked.
- **Refresh, for any connector:**
  - within 2 minutes of expiry, or 10 minutes before a git network command;
  - one refresh per account at a time, with a rotated refresh token stored first;
  - a 401 gets one refresh and one retry.

  GitHub's OAuth tokens never expire; GitLab's and Entra ID's will.
- **Pasted token:** validated by `account.who_am_i`. For Enterprise Server, the typed address also sets the allow-list.
- **Git Credential Manager's token:**
  - Only after the user clicks "Use the token Git Credential Manager has for github.com".
  - Command: `git -c credential.interactive=false -c core.askPass= credential fill`, with `GCM_INTERACTIVE=never` and `GIT_TERMINAL_PROMPT=0`.
  - The token is copied into FastGit's own entry and sent as Bearer.
  - It usually lacks `write:public_key`, so the SSH key button offers the device flow instead.
- **Scopes:** classic and OAuth tokens report theirs in `X-OAuth-Scopes`, and the account records them. Fine-grained tokens fail with `Forbidden { needs }`, read from `X-Accepted-GitHub-Permissions`.

### Git's credentials

Each git network command FastGit runs gets one pair of settings per remote owner that has an account, or per server where FastGit holds a single account:

```text
git -c credential.https://github.com/fastgit-org.helper=
    -c "credential.https://github.com/fastgit-org.helper=!'C:/Program Files/FastGit/fastgit.exe' --git-credential github:github.com:583231"
    fetch --all --prune --quiet
```

- **The empty value** resets the helper list under that prefix only, so other servers keep the user's helpers. This was verified with Git 2.35.1.
- **`fastgit --git-credential <account> get`** prints `username=` (the service's `git_user`) and `password=`, or `quit=1` when there's no token. `store` and `erase` do nothing.
- **Unattended fetches** (the timer and window return) add `--unattended` and `GCM_INTERACTIVE=never`.
- **Cost:** about 150 ms per git network command on Windows.

```rust
pub(crate) struct Access { pub(crate) helper: PathBuf, pub(crate) scopes: Vec<(Url, AccountId)>, pub(crate) unattended: bool }
impl Access {
    pub(crate) fn config_args(&self) -> Vec<OsString>;
}
pub(crate) fn run_with_access<A: AsRef<OsStr>>(location: &Path, access: &Access, args: &[A], exit: Exit) -> Result<Vec<u8>>;
```

`Repo::fetch`, `pull`, `push`, `pull_branch`, `push_branch` and `force_push` take `&Access`, which `repository.rs` builds from the tab's remotes and accounts.

### SSH keys

- **Generation:** `ssh-keygen -q -t <ssh_key_type> -N "" -C "FastGit <login>@<host>" -f <file>`, using the ssh-keygen beside git's ssh. No passphrase in v1: the folder's permissions protect the key.
- **Location:**
  - Windows: `%LOCALAPPDATA%\FastGit\ssh\`
  - macOS: `~/Library/Application Support/FastGit/ssh/`
  - Linux: `$XDG_DATA_HOME/fastgit/ssh/` (0700)
  - File name: `<connector>-<host>-<user id>_<type>`.
- **Host keys:** `ssh_keys.host_keys` (GitHub's `GET /meta`) fills FastGit's own `known_hosts` for every host the service claims, so `StrictHostKeyChecking=yes` holds from the first clone. A server that publishes no keys gets `accept-new` into FastGit's file, and FastGit shows the fingerprint.
- **`core.sshCommand`,** set by `git clone -c core.sshCommand=…` or `git config` when publishing:

```text
ssh -i '<key>' -o IdentitiesOnly=yes -o 'UserKnownHostsFile="<fastgit known_hosts>" ~/.ssh/known_hosts' -o StrictHostKeyChecking=yes
```

  - Paths use forward slashes and single quotes for sh, with an apostrophe written as `'\''`.
  - FastGit keeps the program the effective `core.sshCommand` names and warns about a `GIT_SSH_COMMAND` in the environment.
- **Removing an account** lists the repositories whose `core.sshCommand` names its key, and offers to undo them.

### Settings files

| | Windows | macOS | Linux |
| --- | --- | --- | --- |
| `accounts.json`, `connectors.json` | `%APPDATA%\FastGit\` | `~/Library/Application Support/FastGit/` | `$XDG_CONFIG_HOME/fastgit/` |

- **Reading:** through `serde_json::Value` accessors, measured at 83 KB against 161 KB for derive. Unknown fields survive rewrites.
- **Damaged files:** one that can't be parsed is renamed `.broken` and read as empty.
- **Writing:** to `<name>.unfinished`, then renamed, after re-reading the file first.
- **`accounts.json`** holds the accounts (no secrets) and the remote-owner choices.
- **`connectors.json`** holds the grants.

## 8. Inside FastGit

```rust
pub(crate) struct ConnectorFolders { pub(crate) bundled: Option<PathBuf>, pub(crate) source: Option<PathBuf>, pub(crate) installed: Option<PathBuf> }
pub(crate) struct Catalog { connectors: Vec<Connector>, approvals: Approvals }
pub(crate) struct Connector { pub(crate) id: ConnectorId, pub(crate) folder: PathBuf, pub(crate) origin: Origin,
    pub(crate) manifest: Option<Arc<Manifest>>, pub(crate) standing: Standing, pub(crate) replaces_bundled: bool,
    pub(crate) started: Option<Arc<Started>>, files: Hashes, module: Option<Vec<u8>> }
pub(crate) enum Origin { Bundled, Installed }
pub(crate) struct Claim { pub(crate) connector: ConnectorId, pub(crate) service: String, pub(crate) repo: RepoRef, pub(crate) accounts: Vec<AccountId> }

impl Catalog {
    pub(crate) fn discover(folders: &ConnectorFolders, approvals: Approvals) -> Self;
    pub(crate) fn start(&mut self, host: &Arc<dyn HostServices>);
    pub(crate) fn approve(&mut self, id: &ConnectorId) -> io::Result<Option<Grant>>;
    pub(crate) fn seen(&mut self, id: &ConnectorId) -> io::Result<()>;
    pub(crate) fn claims(&self, remote_url: &str, book: &AccountBook) -> Vec<Claim>;
}

impl Started {
    pub(crate) fn start(id: ConnectorId, wasm: &[u8], host: Arc<dyn HostServices>) -> Result<(Self, Greeting), Stop>;
}

#[derive(Clone)]
pub(crate) struct Forge { started: Arc<Started>, host: Arc<dyn HostServices>, account: AccountId, repo: Option<RepoRef> }

pub(crate) enum CallError { Failed(Failure), Net(NetError), Stopped(Stop), Unavailable }

impl Forge {
    pub(crate) fn repositories(&self, query: RepositoryQuery) -> impl Future<Output = Result<Page<Repository>, CallError>> + Send + 'static;
    pub(crate) fn pulls(&self, state: StateFilter, cursor: Option<Cursor>) -> impl Future<Output = Result<Page<PullSummary>, CallError>> + Send + 'static;
    pub(crate) fn pull(&self, number: u64) -> impl Future<Output = Result<PullDetails, CallError>> + Send + 'static;
    pub(crate) fn review(&self, review: Review) -> impl Future<Output = Result<(), CallError>> + Send + 'static;
    pub(crate) fn merge(&self, merge: Merge) -> impl Future<Output = Result<Merged, CallError>> + Send + 'static;
}
```

One method per operation, each built on one private `ask(op, input)`.

- **`ui/load.rs`** gains `run_async_in_background`, the async twin of `run_in_background`. It runs a `Send` future on the background executor and hands the result back on the foreground.
- **`Catalog`, the started connectors and the accounts** live in a GPUI `Global`, `app::connectors::Connectors`, set in `app::run` after the window opens. The first frame doesn't wait for them: one background task reads `accounts.json` and `connectors.json`, discovers, then starts.
- **`Connector.module`** keeps the hashed bytes only until they're compiled, so a waiting connector starts from the bytes the user approved.
- **Claims:**
  - Only started connectors claim, and matching runs no connector code.
  - A remote is normalized first (scp form, `ssh://` with a port, user info, `.git`), then matched against each service's `remotes`; the first pattern that fits wins.
  - A literal host lists every account of that connector and service; `{field.host}` lists the accounts whose server has that host.
  - When several accounts fit, the owner choice remembered in `accounts.json` narrows them to one.
  - A claim may list no account: the tab can then offer to sign in.
- **Pull requests (built in milestone 5):**
  - `repo::pulls::Pulls`, one per tab, finds its `Hosting` (forge, account, remote, locator, offered features) from the tab's remotes, `origin` first, and lists pull requests with `pulls.list`; `Reach` says why there is none (no claim, no account, two accounts to choose from, signed out);
  - `repo::pulls::PullView` is the selected pull request: `pulls.get`, `pulls.checks`, `pulls.conversation` and the local comparison, each read on its own and kept while it is read again;
  - the head is fetched with `FetchSpec.refspec` into `refs/fastgit/pull/N` (the ref digest ignores those refs, so no history reread follows), together with the base branch; the files are `git diff --find-renames <merge-base> <head>`, and a file opens in the comparison with `Baseline::Commit(merge_base)`;
  - `FetchSpec.url` is fetched only when it is `https://`, and the refspec must be a plain `refs/…` name;
  - when a remote matches accounts in two connectors, or two accounts on one server, the tab asks once and `accounts.json` remembers; the section waits for the answer;
  - pull requests are read when the tab opens and after each fetch finishes (`RepositoryEvent::Fetched`), so no timer is added.
- **`src/cli.rs`** gains, both recognized only in first position:
  - `--git-credential <account> <get|store|erase>`;
  - `--connector-test <folder> [cassette]`, which runs a connector in the real sandbox without a window and prints outputs, logs, fuel and memory.

## 9. The GitHub connector

- **Built in milestone 3:** `connectors/github/` with `account.who_am_i` (`GET /user`, scopes from `X-OAuth-Scopes`) and the error mapping below in `refusal.rs`, replayed from three cassettes. Its manifest lists the pasted token and Git Credential Manager's token; the device flow joins it once the OAuth App's client ID exists. Version 0.1.0, 257,272 bytes of wasm, most of it serde.
- **Built in milestone 4:** version 0.2.0 declares `repositories`, `repositories.create` and `ssh_keys`.
  - `who_am_i` returns the user's numeric `id`.
  - `repositories.owners` reads `/user` and `/user/memberships/orgs`; an organization where the user is only a member is read once more for its `members_can_create_*` rules.
  - `repositories.list` pages `/user/repos`, `/orgs/{owner}/repos` or `/search/repositories` (with `in:name fork:true` and one `user:` qualifier per owner); the cursor is the `Link` URL and must stay on the API endpoint.
  - `repositories.get` and `repositories.create` (`/user/repos` or `/orgs/{owner}/repos`); `ssh_keys.list`, `ssh_keys.add` and `ssh_keys.host_keys` (`GET /meta`).
  - 19 cassettes, also replayed through the committed `dist/github.wasm` inside the sandbox by FastGit's own tests.
- **Built in milestone 5:** version 0.3.0 declares `pulls`, `pulls.create` and `pulls.merge`, with `pulls.list` and `pulls.get` in GraphQL, `pulls.for_branch`, `pulls.checks` (check runs and commit statuses, sent together), `pulls.conversation`, `pulls.comment`, `pulls.create` and `pulls.merge` in REST; 39 cassettes; 324,657 B of wasm (+38,362 B).
- **REST covers:**
  - lists, using `per_page=100` and the `Link` `rel="next"` URL as the cursor, checked to stay on the API endpoint;
  - repositories (`/user/repos` with `affiliation`, and search with `user:` and `org:` qualifiers);
  - `/user/keys` and `GET /meta`;
  - creating pull requests, and the pull request for a branch (`?head=owner:branch`);
  - checks (`check-runs` plus the combined `status`);
  - the conversation;
  - the review, one `POST …/reviews` with `event` and line comments;
  - the merge, `PUT …/merge` with `sha` as a guard.
- **GraphQL covers what REST can't:**
  - review threads: ids, `isResolved`, `isOutdated`, `path`, `line`, `startLine`, `diffSide` and each comment's commit;
  - `resolveReviewThread`;
  - `addPullRequestReviewThreadReply`;
  - `pulls.get` in one query: `mergeable`, `mergeStateStatus`, ref oids, the `*Allowed` merge settings and `viewerDidAuthor`.
- **The head** is `refs/pull/N/head` on the base repository, which works for forks too. FastGit fetches it into `refs/fastgit/pull/N`, checks the SHA and diffs locally.
- **Errors:**
  - 401 becomes `SignedOut`;
  - 403 or 429 with `x-ratelimit-remaining: 0` or `retry-after` becomes `RateLimited`;
  - 403, or 404 when the scope is missing from `X-Accepted-OAuth-Scopes`, becomes `Forbidden { needs }`;
  - any other 404 becomes `NotFound`;
  - 405 and 409 become `Conflict`;
  - 422 becomes `Invalid`, per field.
- **Mergeability:** `mergeable: null` is `Checking`, read again after the next fetch.
- **Size:** about 1,300 lines, and 0.45–0.6 MB of wasm (estimated).

## 10. Other platforms, checked on paper

| Protocol 1.0 | GitLab | Azure DevOps | Gitea |
| --- | --- | --- | --- |
| `RepoRef.owner` | group path, `a/b` | `org/project` | user or organization |
| Pull request head | `refs/merge-requests/N/head` | the source branch at `lastMergeSourceCommit`, maybe in a fork (`FetchSpec.url`) | `refs/pull/N/head` |
| Threads | discussions with `position`, built from `Line` and `PullDetails`; `line_code` in `extra` | `threadContext` sides map to `Line`; iteration ids go in `extra`, and four statuses count as resolved | resolvable review comments |
| Review | draft notes, `bulk_publish`, `/approve`; no request changes in `verdicts` | a thread per draft, then a vote of 10 or −5; `Unfinished.posted` | GitHub-like |
| Merge | methods from `merge_method` plus the squash option | `queued`; semi-linear and fast-forward methods | `POST …/merge` with `Do` |
| Tokens | `PRIVATE-TOKEN` header, or Bearer for OAuth | Basic with an empty user, or Bearer (Entra ID) | Bearer |
| SSH keys | `/user/keys` | no public API: the feature isn't declared | `/user/keys` |

Gitea needs no FastGit change and should be about 700–900 lines.

## 11. Screens to draw, mockup first

| Screen | Data |
| --- | --- |
| Connectors list | Name, version, bundled or installed, `Standing`, grant summary |
| Approval | Endpoint and image origins, remote hosts (and other connectors claiming them), sign-in URLs, features; for an update or a replacement, only what's new |
| Accounts | Login, name, service, server, sign-in method, scopes, SSH key; sign in again; remove, with the `core.sshCommand` undo list |
| Add an account | Connector and service, its `fields`, the sign-in methods; device-flow states: code, waiting, expired, denied; the pasted token and its `create_url`; consent before reusing Credential Manager's token |
| Which account? | The remote owner and host, and the accounts that fit |
| Clone | Account, `Owner`s, search, `Repository` rows, pages, folder, SSH or HTTPS |
| Create or publish | `Owner` with `can_create`, name, description, the owner's `visibilities`, `can_initialize` |
| SSH keys | `SshKey` rows; add (generated key, title); `Forbidden { needs }` leading to the device flow |
| Pull requests | `PullSummary` rows: number, title, author, state, draft, review standing, checks summary; state filter; pages; badges on branches |
| Pull request | Body, base and head, `Mergeability`, `Check`s, conversation, threads count; check out, comment, review, merge (from `merge_methods`, with title and message) |
| Review in the comparison view | `Thread`s on their old and new lines, outdated, resolved, replies; drafts; submit with a `verdicts` choice and a body; `Unfinished` keeps unsent drafts |
| Errors | Signed out → sign in again; `Forbidden` → the needed scope; `RateLimited` → when; `Invalid` → fields; `NetError` → message; `Stop` → the connector's last log lines |

## 12. Testing

`cargo test` needs no network and no wasm target.

- **Units:**
  - manifests, templates and grants (permission diffs);
  - pinning;
  - remote patterns into locators;
  - the curl config writer and response parser;
  - the request policy: allow-list, headers, redirects;
  - `Access` arguments and the `--git-credential` protocol;
  - settings round-trips;
  - the device-flow states against a local fake OAuth server.
- **Engine:**
  - fake connectors in WAT, with the `wat` crate as a dev-dependency, run against a fake `HostServices`;
  - they echo, send two requests before waiting, spin, hog memory, trap, pass wild pointers, import WASI, and get cancelled while waiting.
- **Golden:** `connectors/github/tests/cassettes` replayed through the committed `dist/github.wasm`.
- **Pins:** every `SHA256SUMS` matches its `dist/`.
- **Credential helper:** a local 401 server, as in the host probe, checks the scoped settings.
- **Comments:** `tests/no_comments.rs` also scans the Rust files under `connectors/`.

## 13. Build plan

1. **Host foundation, no UI:**
   - `settings/`;
   - the secret store on each OS;
   - `--git-credential` and `Access`;
   - the curl transport with the policy and token schemes;
   - pasted-token accounts.

   Tests use local servers.
2. **Engine:**
   - wasmi with ABI 1;
   - `connectors/protocol` and `connectors/sdk`, plus the fakes and `--connector-test`.

   Then measure FastGit's real exe delta.
3. **Discovery and approval:**
   - manifests, grants and pinning;
   - starting at launch;
   - `scripts/build-connectors.ps1`;
   - `build-and-relaunch.ps1` mirroring `connectors/*/dist` into `build/connectors/`.

   Built, with remote claims and a first GitHub connector (who-am-I) to pin. Approving, `seen` and claims are tested but have no caller until the screens of milestone 4.
4. **Accounts and repositories,** mockup first (built 2026-11-06):
   - the screens in section 11 up to SSH keys;
   - the device flow, which needs the OAuth App's client ID;
   - GitHub's who-am-I, repositories, creating and publishing, SSH keys and host keys.
5. **Pull requests,** mockup first (built 2026-11-06): list, branch badges, details, checks, conversation and comments, fetching the head and diffing it locally, check out, create, merge, deleting the merged branch.
6. **Reviews,** mockup first:
   - threads in the comparison view, which needs changes to the vendored diff component;
   - drafts, submit, reply, resolve.
7. **Author kit and a second platform:** `docs/connectors/authoring.md`, `connectors/template/`, then a Gitea or GitLab connector that must need no FastGit change.

## 14. Open items

- **Your step:** register the FastGit OAuth App on GitHub with "Enable Device Flow" ticked. Its client ID goes in `connector.json`, as a `device` sign-in with `device_code_url`, `token_url` and `scopes`. FastGit's device flow is built and tested against a local fake OAuth server; the Add account dialog offers it as soon as the manifest does.
- **Done in milestone 4:** `Identity` carries the user's `id`, so account ids are `<connector>:<host>:<user id>`.
- **Unverified:**
  - whether macOS curl trusts roots added to the Keychain;
  - whether curl is present on minimal Linux installs;
  - NTLM-only proxies, since System32's curl lacks NTLM;
  - the scoped helper reset on Git 2.26, the README's minimum;
  - anything on macOS or Linux.
- **Measured at milestone 2,** against a release build of d58113f (37,505,024 B):
  - +1,886,208 B (+5.03%) in all; thin LTO moves a build by about 80 KB;
  - without checking answers against their types, +1,355,264 B: wasmi, the engine, `--connector-test` and the envelope;
  - typed answers decoded by plain serde, +271,360 B; `serde_path_to_error` on top of them, +259,584 B;
  - a test connector built with the SDK: 202,446 B of wasm, started in 4.7–5.7 ms, 256 KiB of memory after a call; an endless loop stopped after 50 slices in 0.41 s, a memory hog at 64 MiB in 30 ms.
- **Measured at milestone 3,** against milestone 2's release build (39,391,232 B):
  - +293,376 B (+0.74%) in all: about 179 KB for manifests, grants, pins, the catalog, `sha2` and starting at launch, and about 114 KB for the gateway and curl transport, which nothing reached before;
  - with the transport still on `smol::process`, it was +497,664 B;
  - memory over three launches each, opening FastGit's own repository: 149.2 MB of private bytes before, 150.5 MB after, ±1.5 MB from run to run;
  - the GitHub connector: 257,272 B of wasm, started in 6.5 ms, `hello` in 3.0 ms and `who_am_i` in 1.8 ms on a replayed cassette.
- **Measured at milestone 4,** against milestone 3's release build (39,682,048 B):
  - +2,091,008 B (+5.27%) in all, for the Settings tab, its pages and dialogs, Clone, New repository, Publish and the account bar (`src/hosting/`, about 4,900 lines with tests), plus sign-in, the device flow, SSH keys and cloning behind them;
  - memory over three launches each, opening FastGit's own repository: 144.7 MB of private bytes before, 143.7 MB after, within the ±1.5 MB runs vary by;
  - the GitHub connector grew to 286,295 B of wasm with its repository and SSH key operations.
- **Measured at milestone 5,** against milestone 4's release build (41,769,984 B):
  - +1,081,856 B (+2.59%) in all, for the Pull requests section, branch marks, the overview, files, Create and Merge dialogs (`src/repo/pulls/`, about 2,800 lines with tests) and fetching, comparing and checking out a head (`src/git/pull_head.rs`);
  - memory over three launches each, opening FastGit's own repository (no remote, so no section): 149.8 MB of private bytes before, 149.4 MB after; a small repository with three pull requests listed: 138.8 MB;
  - the GitHub connector grew to 324,657 B of wasm with its eight pull request operations;
  - live-checked against a fake GitHub Enterprise Server on 127.0.0.1 whose remote URL `insteadOf` pointed git at a local bare repository: list, filter, badges, overview, checks, conversation and comment, file comparison, check out (same repository and fork), squash merge, deleting the merged branch, creating a pull request from an unpublished branch.
- **Later:**
  - avatars, fetched in batches through curl into a cache and drawn with `img(path)`;
  - libcurl at run time;
  - SSH key passphrases through `--ssh-askpass`;
  - ETag caching per account;
  - the NTLM fallback to git's bundled curl.
- **wasmi 2.0** is young and unaudited. Pin its minor version, fuzz the six host functions and follow its releases. If it ever has to go, the same modules run on Wasmtime with async host functions; only `src/connect/` changes.
