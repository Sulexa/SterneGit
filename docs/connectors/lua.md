# Connectors on Luau

## Summary

Each connector is a Luau script plus a JSON manifest. SterneGit runs it in its own sandboxed state through mlua 0.12 (`luau` and `async`, without `serialize`). All states share one connector thread that starts at launch. Calls run as coroutines that are suspended during HTTP requests, so idle connectors run no code. Scripts never see tokens: a per-call account handle reaches only that account's servers. Returned values are checked into Rust types, naming the bad field. A 985-line GitHub connector (REST plus GraphQL) passed every check against recorded responses: the four sample operations, pagination, every error kind, concurrency, cancellation, an endless loop and a memory hog. Luau adds 2.2 MB of exe and about 1 MiB of RAM per connector, where Lua 5.4 would add 1.1 MB and 0.15 MiB. I recommend Luau for its sandbox, type syntax and error lines.

## Measured numbers

The probes are separate binaries built with SterneGit's release profile. The baseline links serde_json (`preserve_order`, `raw_value`), anyhow, futures and url, and (de)serializes the interface types through serde_json. Times are medians of 5 runs × 30 iterations, on a Ryzen 9 5950X with Windows 11 that other sessions kept about 50% busy.

| | Luau (chosen) | Lua 5.4.9 |
|---|---|---|
| Exe: proposed host layer, all 19 calls (baseline 936,448 B) | **+2,199,040 B** (+5.9% of 37,174,784) | +1,060,352 B (+2.9%) |
| Exe: same layer through mlua `serialize` | +3,543,040 B | +2,351,104 B |
| Exe: engine floor with `serialize`, one call (minimal baseline 544,256 B) | +1,743,360 B | +602,112 B |
| Exe: mlua `send` on top (two builds, identical) | +34,816 B | — |
| Start one 985-line connector (compile plus top level) | 3.0 ms first, 2.5 ms next | 0.93 ms, 0.76 ms |
| Lua heap per started connector | 947 KiB | 108 KiB |
| Private bytes, 1 / 10 connectors | +1.39 / +10.2 MiB | +0.14 / +1.52 MiB |
| Working set, 1 / 10 connectors | +1.77 / +7.8 MiB | +0.54 / +1.9 MiB |
| 100 repositories, 530,779 B of JSON: serde_json parse only | 4.5 ms | 4.7 ms |
| … decoded into Lua tables | 10.2 ms | 6.6 ms |
| … whole `list_repositories` call (decode, script, shape check) | 11.2 ms | 7.9 ms |
| Endless loop stopped (1 s budget) | 1.0001 s | 1.0001 s |
| Memory hog hits the 64 MiB limit | 112 ms, state reusable | 478 ms, state reusable |
| Idle CPU, 30 s with 10 connectors started | 0 ns | 0 ns |
| Threads | +1 (44 KiB private) | +1 |
| Clean build of the vendored C or C++ | 74.5 s (parallel, on a loaded machine) | 6.9 s |

Estimated, not measured: inside SterneGit the delta should be a little lower, since parking_lot, bstr, futures-util and rustc-hash are already linked. macOS and Linux weren't built. A bare Luau state holds 322 KiB, because its allocator takes a 16 or 32 KB page per size class, a `const` in `lmem.cpp`.

## 1. Engine and sandbox

**Options.** `vendored` builds Lua 5.4.9 or 5.5.1. Luau is always vendored (luau0-src 0.740). `async` turns host functions into futures, and `send` makes `Lua: Send`. `serialize` cost about 1.35 MB, because each shape gets its own deserializer through mlua. A 100-line bridge to `serde_json::Value` replaces it. No pure-Rust Lua is mature: piccolo 0.3.3 calls itself "very experimental" and lacks most of `string` and `table`.

**Why Luau over Lua 5.4:**

1. **It was built for hostile scripts, at Roblox.** It has no `io` or `package`, `os` holds only clock and date functions, and `debug` only `info` and `traceback`. `luaL_sandbox` makes globals and libraries read-only and gives each script its own environment. Interrupts fire at calls and loop back-edges. PUC Lua leaves sandboxing to the embedder, and has had memory-safety bugs reachable from source (CVE-2022-28805, CVE-2022-33099).
2. **Its types are syntax.** LuaLS types are `---@` comments, which SterneGit's no-comments rule forbids.
3. **Its error lines survive host calls.** Lua 5.4 lost the script line behind a tail call into `account:request`, where Luau kept `tests.luau:10: in function 'elsewhere'`.

The price is +1.14 MB of exe, +0.85 MiB per connector and +1.6 ms per start. If footprint wins, Lua 5.4 fits behind the same interface.

**The sandbox:**

1. `Lua::new_with(COROUTINE | TABLE | STRING | UTF8 | MATH | BIT | BUFFER)`.
2. Remove `require`, `loadstring`, `collectgarbage`, `getfenv`, `setfenv`, `newproxy`, `gcinfo` and `string.dump`. mlua gives every Luau state a filesystem `require` (`FsRequirer`) by default.
3. Route `print` to the connector's log.
4. Install `sternegit` and call `sandbox(true)`.
5. Load the script with `ChunkMode::Text` only, because Luau doesn't verify bytecode.
6. Call `set_memory_limit(64 MiB)`.

**Stopping a connector.** The interrupt checks the clock every 64 ticks. A call that runs 1 s without awaiting fails with `Stopped`, and one that allocates past the limit fails with `MemoryError`. The state stays usable. After three failures in a row SterneGit closes the connector, and **Restart** builds a new state.

**One state per connector.** A state per call would cost 2.5 ms and 1 MiB each time, and a state per account would multiply the memory. Per-call handles already isolate the accounts.

**Build.** SterneGit already compiles C++ through cc on Windows (vswhom-sys) and C on Linux (freetype-sys). Linux would also need g++ or clang++.

## 2. Package, manifest and approval

```
connectors/github/connector.json   connectors/github/main.luau   connectors/github/.luaurc   connectors/github/tests/*.json
connectors/sdk/sternegit.d.luau      connectors/sdk/template/
```

`.luaurc` sets `{"languageMode": "strict"}`, so the source needs no `--!strict` line.

```json
{ "interface": 1, "id": "github", "name": "GitHub", "version": "1.0.0", "main": "main.luau",
  "features": ["repositories", "create_repository", "ssh_keys", "pull_requests", "reviews", "merge"],
  "servers": [
    { "kind": "cloud", "address": "https://github.com",
      "api": ["https://api.github.com/"], "remote_hosts": ["github.com", "ssh.github.com"] },
    { "kind": "self_hosted", "address_field": { "label": "Server address", "example": "https://github.example.com" },
      "api": ["{address}/api/v3/", "{address}/api/graphql"], "remote_hosts": ["{host}"] } ],
  "sign_in": [
    { "method": "device", "servers": ["cloud"], "client_id": "Ov23li…", "scopes": ["repo", "read:org", "admin:public_key"],
      "device_code_url": "https://github.com/login/device/code",
      "token_url": "https://github.com/login/oauth/access_token", "scheme": { "kind": "bearer" } },
    { "method": "token", "help_url": "{address}/settings/tokens/new?scopes=repo,read:org,admin:public_key",
      "scheme": { "kind": "bearer" } },
    { "method": "git_credential_manager", "scheme": { "kind": "bearer" } } ],
  "images": ["https://avatars.githubusercontent.com/"] }
```

GitLab's token method declares `{"kind": "header", "name": "PRIVATE-TOKEN"}`, and `bearer` for OAuth. Azure DevOps declares `{"kind": "basic", "username": ""}`. The host services' `TokenScheme` turns this into `Authorization: Bearer <token>`, `PRIVATE-TOKEN: <token>` or `Authorization: Basic base64(":<token>")`. It marks the value sensitive and adds it only for origins in the account's grant, so the token never enters Lua.

**Discovery.** At launch, the connector thread reads `<exe dir>/connectors/*/connector.json`, or `Contents/Resources/connectors` in a macOS bundle. Debug builds also read `CARGO_MANIFEST_DIR/connectors`, for `cargo run`. Then it reads the user folders from host-services.md. It refuses a folder whose name isn't its `id`, and a user connector that reuses a bundled id.

**Approval.** A grant is the set of API origins, sign-in URLs, image origins and features. `connectors.json` stores the approved grant, version and SHA-256. A manifest within its grant starts, even with changed code. Anything more waits, and the approval screen lists only the additions.

**Flag on decision 1:** pre-approving the folder beside the exe would also pre-approve files dropped into a writable portable install. The exe should pin the bundled files' SHA-256 with a `const fn` over `include_bytes!`, so that only the digest is stored.

**Same server twice.** Servers belong to accounts, so two connectors may both claim github.com. Approval mentions the existing claim, and decision 8 covers a remote that matches both.

**Shipping.** `scripts/build-and-relaunch.ps1` should also mirror each `connectors\<id>\` that has a manifest into `build\connectors\`, comparing hashes, and relaunch on any change. Release archives carry `connectors/` beside the exe.

## 3. The host–connector interface

`connectors/sdk/sternegit.d.luau` holds these declarations. At start, SterneGit checks that every declared feature has its functions.

```luau
type Request = { method: string?, url: string, query: { [string]: any }?, headers: { [string]: string }?, json: any? }
type Response = { status: number, headers: { [string]: string }, json: any?, body: string? }
declare class Account
	server: string
	login: string?
	function request(self, request: Request): Response
end
declare sternegit: { interface: number, log: (string, string) -> (), now: () -> number, fail: (Failure) -> never,
	json: { array: <T>({ T }?) -> { T }, decode: (string) -> any, encode: (any) -> string }, url: { encode: (string) -> string } }

type Page<T> = { items: { T }, next: string? }
type Person = { login: string, avatar_url: string? }
type RepoRef = { owner: string, name: string }
type Repository = { id: string, owner: string, name: string, description: string?, private: boolean, fork: boolean,
	archived: boolean, default_branch: string?, clone: { https: string, ssh: string? }, web_url: string, updated_at: string? }
type RepositoryQuery = { search: string?, cursor: string? }
type NewRepository = { owner: string?, name: string, description: string?, visibility: "public" | "private" | "internal", initialize: boolean }
type SshKey = { id: string, title: string, key: string, created_at: string?, read_only: boolean }
type PullRequestSummary = { number: number, title: string, state: "open" | "closed" | "merged", draft: boolean, author: Person,
	base_ref: string, head_ref: string, head_sha: string, head_repository: string?,
	review: ("approved" | "changes_requested" | "review_required")?, web_url: string, created_at: string, updated_at: string }
type PullRequest = { summary: PullRequestSummary, body: string, base_sha: string, mergeable: "mergeable" | "conflicting" | "unknown",
	merge_state: "clean" | "blocked" | "behind" | "conflicting" | "failing_checks" | "draft" | "unknown",
	merge_methods: { "merge" | "squash" | "rebase" }, verdicts: { Verdict }, can_update: boolean,
	fetch: { url: string?, ref: string, sha: string } }
type Check = { name: string, state: "pending" | "success" | "failure" | "neutral" | "skipped" | "cancelled", description: string?, web_url: string? }
type NewPullRequest = { base: string, head: string, title: string, body: string, draft: boolean }
type LineRef = { old: number?, new: number? }
type Comment = { id: string, author: Person, body: string, created_at: string }
type ReviewThread = { id: string, path: string, line: LineRef?, start: LineRef?, on_file: boolean, commit: string?,
	outdated: boolean, resolved: boolean, can_resolve: boolean, can_reply: boolean, comments: { Comment } }
type Verdict = "comment" | "approve" | "request_changes"
type ReviewComment = { path: string, line: LineRef, start: LineRef?, body: string }
type NewReview = { commit: string, body: string, verdict: Verdict, comments: { ReviewComment } }
type Review = { id: string, state: string }
type Merge = { method: "merge" | "squash" | "rebase", title: string?, message: string?, head_sha: string }
type Merged = { merged: boolean, sha: string?, message: string? }
type Failure = { kind: "signed_out" | "not_found" | "conflict" | "failed", message: string }
	| { kind: "missing_permission", message: string, needed: string? }
	| { kind: "rate_limited", message: string, retry_after: number? }
	| { kind: "invalid", message: string, fields: { { field: string, message: string } } }

type Connector = {
	remote: (url: string) -> { host: string, owner: string, name: string }?,
	who_am_i: (Account) -> { login: string, name: string?, avatar_url: string?, scopes: { string } },
	owners: (Account) -> { { login: string, avatar_url: string?, kind: "user" | "organization" } },
	list_repositories: (Account, RepositoryQuery) -> Page<Repository>,
	create_repository: (Account, NewRepository) -> Repository,
	list_ssh_keys: (Account) -> { SshKey },
	add_ssh_key: (Account, { title: string, public_key: string }) -> SshKey,
	list_pull_requests: (Account, RepoRef, { state: "open" | "closed" | "merged" | "all", cursor: string? }) -> Page<PullRequestSummary>,
	pull_requests_for_branch: (Account, RepoRef, branch: string) -> { PullRequestSummary },
	pull_request: (Account, RepoRef, number: number) -> PullRequest,
	checks: (Account, RepoRef, sha: string) -> { Check },
	create_pull_request: (Account, RepoRef, NewPullRequest) -> PullRequestSummary,
	review_threads: (Account, RepoRef, number: number) -> { ReviewThread },
	conversation: (Account, RepoRef, number: number, cursor: string?) -> Page<Comment>,
	comment: (Account, RepoRef, number: number, body: string) -> Comment,
	submit_review: (Account, RepoRef, number: number, NewReview) -> Review,
	reply: (Account, RepoRef, number: number, thread: string, body: string) -> Comment,
	resolve_thread: (Account, RepoRef, number: number, thread: string, resolved: boolean) -> boolean,
	merge: (Account, RepoRef, number: number, Merge) -> Merged,
}
```

- **Publishing** is `create_repository`, then SterneGit adds the remote and pushes.
- **Cursors** are opaque: a Link URL, an `endCursor` or a `$skip`.
- **Network failures** are the host's `NetError` and bypass the script.
- **Shapes.** A returned value becomes a `serde_json::Value`: keys must be strings or `1..n`, depth is at most 64, and functions and userdata are refused. `serde_path_to_error` then deserializes it, ignoring unknown fields. Measured: `path: "items[0].clone", problem: "missing field `https`"`. `sternegit.json.array` marks an empty array.

## 4. Four operations

This Luau is verbatim from the probe connector. `send` is shared by every call.

```luau
local function send(account: Account, request: Request): Response
	request.headers = { accept = ACCEPT, ["x-github-api-version"] = REST_VERSION }
	local response = account:request(request)
	if response.status >= 200 and response.status < 300 then
		return response
	end
	return sternegit.fail(failure_of(response))
end

local function next_link(link: string?): string?
	if link == nil then
		return nil
	end
	for part in string.gmatch(link, "[^,]+") do
		local url, rel = string.match(part, "<([^>]+)>%s*;%s*rel=\"([^\"]+)\"")
		if rel == "next" then
			return url
		end
	end
	return nil
end

function connector.list_repositories(account: Account, query: RepositoryQuery): Page<Repository>
	if not is_blank(query.search) then
		return search_repositories(account, query)
	end
	local response
	if query.cursor ~= nil then
		response = follow(account, query.cursor)
	else
		response = get(account, "/user/repos", {
			per_page = REPOSITORY_PAGE,
			sort = "pushed",
			affiliation = "owner,collaborator,organization_member",
		})
	end
	return repositories(response.json, next_link(header(response, "link")))
end

function connector.create_pull_request(account: Account, repo: RepoRef, new: NewPullRequest): PullRequestSummary
	local response = post(account, repo_path(repo) .. "/pulls", {
		title = new.title,
		head = new.head,
		base = new.base,
		body = new.body,
		draft = new.draft == true,
	})
	return rest_pull_summary(response.json)
end

local function github_line(at: LineRef): (number, string)
	if at.new ~= nil then
		return at.new, "RIGHT"
	end
	return at.old, "LEFT"
end

local function line_comment(draft: ReviewComment): {[string]: any}
	local line, side = github_line(draft.line)
	local comment = { path = draft.path, body = draft.body, line = line, side = side }
	if draft.start ~= nil then
		comment.start_line, comment.start_side = github_line(draft.start)
	end
	return comment
end

function connector.submit_review(account: Account, repo: RepoRef, number: number, review: NewReview): Review
	local comments = {}
	for _, draft in ipairs(review.comments) do
		table.insert(comments, line_comment(draft))
	end
	local response = post(account, pull_path(repo, number) .. "/reviews", {
		commit_id = review.commit,
		body = review.body,
		event = VERDICTS[review.verdict] or "COMMENT",
		comments = json.array(comments),
	})
	local raw = response.json
	return { id = tostring(raw.id), state = string.lower(raw.state or "commented") }
end

function connector.merge(account: Account, repo: RepoRef, number: number, merge: Merge): Merged
	local method = MERGE_METHODS[merge.method]
	if method == nil then
		return sternegit.fail({
			kind = "invalid",
			message = "GitHub cannot merge that way",
			fields = { { field = "method", message = "must be merge, squash or rebase" } },
		})
	end
	local body = { merge_method = method, sha = merge.head_sha }
	if method ~= "rebase" then
		body.commit_title = merge.title
		body.commit_message = merge.message
	end
	local raw = put(account, pull_path(repo, number) .. "/merge", body).json
	return { merged = raw.merged == true, sha = raw.sha, message = raw.message }
end
```

`review_threads` pages through GraphQL `reviewThreads`, with resolved and outdated state, `path`, `line`, `diffSide` and the comments' commits. It follows `node(id:)` past 100 comments. Against the recordings it returned 12 threads (3 resolved, 4 outdated, 30 comments), the review sent `{"side":"RIGHT","start_line":36,"line":40,…}`, and a moved head made the merge return `Conflict`.

The host side: `request` and `shaped` are probe code adapted to host-services.md's `HostServices`, and the typed methods are the proposed API.

```rust
impl UserData for AccountHandle {
    fn add_methods<M: UserDataMethods<Self>>(methods: &mut M) {
        methods.add_async_method("request", |lua, this, request: Table| async move {
            if !this.live.load(Ordering::SeqCst) {
                return Err(runtime("this account was given to an earlier call and can't be used any more".into()));
            }
            let outgoing = Outgoing::read(&lua, &request, &this.view)?;
            let answer = this.host.request(&this.view.id, outgoing, this.cancel.clone()).await.map_err(mlua::Error::external)?;
            answer.into_lua(&lua)
        });
    }
}

impl Connector {
    pub(crate) fn list_repositories(&self, account: &AccountId, query: &RepositoryQuery) -> Reply<Page<Repository>> {
        self.invoke("list_repositories", account, vec![json(query)])
    }
    pub(crate) fn create_pull_request(&self, account: &AccountId, repo: &RepoRef, new: &NewPullRequest) -> Reply<PullRequestSummary> {
        self.invoke("create_pull_request", account, vec![json(repo), json(new)])
    }
    pub(crate) fn review_threads(&self, account: &AccountId, repo: &RepoRef, number: u64) -> Reply<Vec<ReviewThread>> {
        self.invoke("review_threads", account, vec![json(repo), json(&number)])
    }
    pub(crate) fn submit_review(&self, account: &AccountId, repo: &RepoRef, number: u64, review: &NewReview) -> Reply<Review> {
        self.invoke("submit_review", account, vec![json(repo), json(&number), json(review)])
    }
    pub(crate) fn merge(&self, account: &AccountId, repo: &RepoRef, number: u64, merge: &Merge) -> Reply<Merged> {
        self.invoke("merge", account, vec![json(repo), json(&number), json(merge)])
    }
}

fn shaped<T: DeserializeOwned>(lua: &Lua, value: Value) -> Result<T, (String, String)> {
    let json = bridge::to_json(lua, value, 0).map_err(|problem| (String::new(), problem))?;
    serde_path_to_error::deserialize(json).map_err(|error| (error.path().to_string(), error.inner().to_string()))
}
```

- **`Outgoing::read`** builds host-services.md's `ApiRequest`. It refuses `Authorization`, `Cookie`, `Host`, `Proxy-Authorization`, `PRIVATE-TOKEN` and URLs off the account's allow-list, and the host re-checks both. Measured: "https://example.com/collect is not on this account's servers".
- **Account handles** are per call: `live` turns false when the call ends, so a kept handle fails (measured).

## 5. Versioning

`interface` in the manifest is the major version. `sternegit.interface` gives the host's major.minor, and `sternegit.has(name)` tests for an optional host function.

- **Minor versions only add** optional fields, optional exports, host functions and enum values. Returned enums deserialize through `#[serde(other)] Unknown`, so a newer connector can't break an older SterneGit.
- **Features gate the UI.** Without `ssh_keys`, a connector gets no SSH key button.
- **A newer major** shows "needs a newer SterneGit". A connector only a minor ahead starts.
- **An older major** runs through a Rust adapter, one major back. Anything older shows "written for an older SterneGit".

## 6. Async and lifecycle

The connector thread runs `smol::block_on(executor.run(serve(jobs)))` on an `async_executor::LocalExecutor` (both already linked), with `send` off. Lua never runs on GPUI's threads, and a runaway script delays other connectors by one 1 s slice at most. Waiting on its channel, the thread is parked: 0 ns of CPU over 30 s.

- **HTTP and concurrency.** `account:request` awaits `HostServices::request`'s `BoxFuture` directly, without `block_on`. Each call is a coroutine, and two calls joined on one state both finished.
- **Cancellation.** `Reply<T>` holds the host's `CancelOnDrop`. Dropping a `Load` or `Job` drops its `Task` and the `Reply`, `Cancel` fires, and a `select` on the connector thread drops the call. mlua terminates and recycles the coroutine. In the probe the in-flight request was dropped, and the state answered the next call.
- **Launch.** `app::run` spawns the thread after the window opens. Discovery and the 3 ms per connector happen there, so the first frame and the history read don't wait.

## 7. SterneGit side

```
src/connect/  mod.rs Connectors · found.rs · manifest.rs · grant.rs · thread.rs · sandbox.rs · bridge.rs · handle.rs · shapes.rs · failure.rs
```

```rust
pub(crate) struct Connectors { thread: ConnectorThread, found: Vec<Installed> }
impl Connectors {
    pub(crate) fn start(bundled: PathBuf, installed: PathBuf, host: Arc<dyn HostServices>) -> Self;
    pub(crate) fn installed(&self) -> &[Installed];
    pub(crate) fn connector(&self, id: &ConnectorId) -> Option<Connector>;
    pub(crate) fn approve(&mut self, id: &ConnectorId);
}
pub(crate) enum Standing { Running, NeedsApproval(Grant), Refused(String), Failed(String) }
#[derive(Clone)]
pub(crate) struct Connector { id: ConnectorId, jobs: async_channel::Sender<Job> }
pub(crate) struct Reply<T> { answer: oneshot::Receiver<Result<T, ConnectorError>>, _cancel: CancelOnDrop }
impl<T> Future for Reply<T> { type Output = Result<T, ConnectorError>; fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>; }
pub(crate) enum ConnectorError { Failed(Failure), Network(NetError), Stopped(Stop), Script(String), Shape { function: &'static str, path: String, problem: String }, Unavailable(String) }
```

`Connectors` is a GPUI `Global` set in `app::run`. `Repository` gains `hosting: Option<Hosting>`, resolved from the remote URL through `remote()` and the account match, plus `pulls: Load<Page<PullRequestSummary>>`. `read_pull_requests` uses the existing pattern, `self.pulls.reload(cx.spawn(async move |this, cx| { let result = reply.await; … }))`. It runs when the tab opens and after each successful `fetch()` (the 5-minute fetch, window return and Ctrl+Shift+R), so no timer is added.

## 8. The GitHub connector

- **REST** handles repositories (`/user/repos` with `affiliation`, and search with `user:` and `org:`), creating repositories, SSH keys, creating PRs, the PR for a branch (`?head=owner:branch`), checks (`check-runs` plus `status`) and the conversation. A review is one atomic `POST …/reviews`. A merge is `PUT …/merge` with `sha` as a guard, so a 409 becomes `conflict`.
- **GraphQL** handles what REST can't: listing PRs (it has a `MERGED` state and cursors), getting mergeability, merge methods and `viewerDidAuthor` (no approving your own PR) in one query, and review threads with resolved state, replies and resolving.
- **Pagination and checkout.** REST follows `Link rel="next"` and GraphQL follows `endCursor`. `fetch` is `{ ref = "refs/pull/N/head", sha = headRefOid }`: SterneGit fetches the ref from the base remote, checks the commit, then checks out or diffs `base_sha...sha` itself. Enterprise Server uses `{address}/api/v3` and `{address}/api/graphql`.
- **Errors.** 401 becomes `signed_out`. 403 or 429 with `x-ratelimit-remaining: 0` or `retry-after` becomes `rate_limited`, and the probe got 117 s from the reset header. 403 or 404 without the scope in `x-accepted-oauth-scopes` becomes `missing_permission { needed }`, and any other 404 becomes `not_found`. 422 becomes `invalid`, per field. Each kind was checked against a recorded response.

## 9. Platform check

- **GitLab.** `owner` is a namespace path, slashes included, and merge requests filter by `opened|closed|merged|all`. Discussion positions carry `old_line` and `new_line`, which is why `LineRef` holds both sides: GitLab needs both for unchanged lines, and SterneGit knows them from its own diff. A review is draft notes plus `bulk_publish` using `diff_refs`. Approving is `POST …/approve`, and `verdicts` hides request changes where the server lacks it. The head is `refs/merge-requests/:iid/head`, and the merge methods come from `merge_method` and `squash_option`.
- **Azure DevOps.** `owner` is `org/project`. Its `*.visualstudio.com` hosts need `*.` patterns in manifests. `threadContext` left and right map to `LineRef` old and new, the thread status maps to resolved, and the connector maps iterations to commits. Votes are 10 for approve and −5 for request changes. `refs/pull/N/merge` is the merge commit, so `fetch.url` points at the source repository's `sourceRefName`. With no documented SSH key API, the connector doesn't declare `ssh_keys`.
- **What doesn't fit, absorbed by additions.** Azure's review isn't atomic, so `failed` gains a `posted` count and SterneGit keeps the unsent drafts. Azure can merge asynchronously, so `Merged` gains `queued`. Visibility is per project, so owners gain `visibilities` and `can_initialize`.
- **Gitea** has a GitHub-like REST API and no GraphQL. I estimate 500–600 lines, since about 250 of GitHub's 985 lines are GraphQL.

## 10. Author experience

The SDK gives authors `sternegit.d.luau` for luau-lsp, where a misspelled field is an editor error in strict mode. It also ships a template: a manifest, stubs and one recorded case. `src/cli.rs` gains two modes that use the app's own sandbox:

- `sternegit --connector-check <folder>` validates the manifest, compiles the script and compares its exports with its features.
- `sternegit --connector-run <folder> <case.json>` replays recorded exchanges. It prints the shaped result or the error and the requests made, and flags unrecorded ones.

Errors carry lines, as measured: `broken/main.luau:4: Expected ')' (to close '(' at line 3), got 'end'`, and `tests.luau:8: attempt to index nil with 'name'` with a traceback. `print` and `sternegit.log` go to the host's token-scrubbed log.

Non-Lua authors must write Luau: roblox-ts targets Roblox APIs, and TypeScriptToLua is untested against Luau. That is the main weakness against the Wasm designs.

## 11. Testing in SterneGit

`cargo test` needs no network:

- **Host tests.** `src/connect/` tests use a fake `HostServices` that answers from recordings, and a fake connector under `tests/connectors/fake/`. Examples: `an_endless_loop_stops_after_one_second`, `a_dropped_reply_cancels_its_request`, `dangerous_globals_are_absent`.
- **GitHub cases.** `tests/connectors.rs` replays every `connectors/github/tests/*.json` case through the real connector.
- **Comments.** `tests/no_comments.rs` learns Luau's `--`.

## 12. Risks and plan

- **Memory.** Luau's allocator sets a floor of about 1 MiB per connector.
- **VM bugs** escape into a process that can reach the credential store, with no process isolation. Never load bytecode, keep mlua current, and keep the globals test, because mlua's defaults change between releases.
- **Build.** Cold builds spend about 75 s on C++. macOS and Linux are unbuilt.
- **mlua churn.** 0.10 reworked the whole API, and 0.12 moved `ChunkMode` and the serde options.
- **C calls can't be interrupted.** `string.rep(x, 1e8)` is bounded only by the memory limit.
- **Wrong choice if** authors need other languages, process isolation is required, or 100 KiB per connector or +2.2 MB matters (then Lua 5.4).

**Milestones:**

1. Sandbox, bridge, thread, fake host and the CLI modes.
2. Discovery, approval, start at launch, build script.
3. GitHub sign-in, repositories, publishing, SSH keys (mockup first).
4. Pull requests: list, details, checks, checkout, create.
5. Reviews in the diff, conversation, merge.
6. SDK, template and a GitLab spike.
