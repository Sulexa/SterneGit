# Connectors: comparing the four proposals

**Decisions, 2026-11-05:**
- **Engine:** wasmi, as this document recommends.
- **Bundled connectors:** a `connectors/` folder beside the exe, pinned by SHA-256 hashes the exe carries.
- **SDK:** comment-free, documented in a separate authoring guide.

The fixes in section 4 are folded into [design.md](design.md), which is now the reference.

Written 2026-11-05, after the grilling that settled the requirements. Four agents wrote proposals:

| Proposal | What it covers |
| --- | --- |
| [wasmi.md](wasmi.md) | WebAssembly modules run by the wasmi 2.0 interpreter, exchanging JSON messages |
| [wasmtime-components.md](wasmtime-components.md) | WebAssembly components with a typed WIT interface, run by Wasmtime 50 (the way Zed's extensions work) |
| [lua.md](lua.md) | Luau scripts (or Lua 5.4) embedded through mlua 0.12 |
| [host-services.md](host-services.md) | What every engine shares: HTTPS, tokens, sign-in, git credentials, SSH keys, settings |

## Verdict

**Build connectors on wasmi, with the host services as proposed.**
- **Size:** about 1.4 MB in all (3.7% of today's exe). That is the smallest of the options that let connector authors pick their language.
- **Build:** pure Rust, so nothing new to install for anyone building SterneGit.
- **Isolation:** every call gets a fresh instance, so nothing a connector learns while working for one account survives into a call for another.
- **Speed:** wasmi interprets connector code 15–20× slower than native, but that never shows. Reading 100 repositories costs about 15 ms of connector work, next to about 110 ms for each web request.
- **Way out:** if speed or security ever demands a heavier engine, the same connector files run on Wasmtime, and only SterneGit's side changes.

Two runners-up:
- **Lua 5.4** is just as small and uses a seventh of wasmi's memory per connector, but its sandbox is weaker.
- **Luau** has the best-proven sandbox and editor types, but costs 1.1 MB more, a C++ build, and about 1.4 MiB per connector.

**Wasmtime isn't worth it.** Its compiler makes the exe 31% bigger, for speed nobody would notice; its own author reached the same conclusion.

## 1. What all four agree on

These parts of the design no longer depend on the engine:

- **Package.**
  - A connector is a folder holding a JSON manifest and one code file. serde_json is already linked and toml isn't, hence JSON.
  - Bundled connectors sit beside the exe (`Contents/Resources` on macOS); installed ones go in the user data folder.
- **Approval is a grant.**
  - A grant lists the API origins a connector may call, its sign-in URLs, image origins and features.
  - A new connector waits for the user's approval, and so does one asking for more than its grant. The approval screen lists only the additions.
  - A new version that stays within its grant starts without asking.
- **Bundled connectors are pre-approved by a SHA-256 that the exe carries,** not by the folder they sit in. Three agents flagged this independently: a folder beside the exe can be user-writable, so trusting the folder would trust anything dropped into it.
- **"Started at launch" means checked and compiled, not running.**
  - It happens on a background task after the first frame, at 1–12 ms per connector.
  - A connector with no work to do runs no code, starts no thread and uses no CPU.
- **Tokens never enter the sandbox.**
  - SterneGit attaches the token from a closed list of schemes:
    - `Authorization: Bearer`;
    - a named header, such as GitLab's `PRIVATE-TOKEN`;
    - Basic with an empty user, for Azure DevOps personal tokens.
  - The token is attached only for origins in the account's grant.
  - Authorization headers a connector sets itself are dropped, and a redirect to another origin loses the token.
- **GitHub specifics.**
  - Review threads need GraphQL: thread ids, resolved and outdated state, replies and resolving exist nowhere else.
  - Everything else uses REST, with `Link` headers for pages.
  - A pull request's head is fetched from `refs/pull/N/head` and diffed locally with git.
- **Refreshing.** Pull requests refresh when a tab opens and after each fetch, so no timer is added.
- **Platform gaps are features, not interface changes.** Azure DevOps has no public API for SSH keys, so its connector simply won't declare that feature.
- **Testing.**
  - A new `src/cli.rs` mode runs a connector against recorded responses without the window.
  - `cargo test` needs no network.
  - Built connector files are committed, so SterneGit's own `cargo build` needs no extra toolchain, and a test catches stale builds.
- **`ui/load.rs`** needs an async twin of `run_in_background`.

## 2. Measured numbers side by side

All four agents measured probe programs, built outside SterneGit with its release settings on this Windows machine.

| | wasmi 2.0 | Wasmtime 50 | Luau | Lua 5.4 |
| --- | --- | --- | --- | --- |
| Exe added by the engine and its glue | +1.11 MB (engine compiled for size) | +11.58 MB with the compiler at `opt-level = "s"`; +1.98 MB runtime only, plus a separate compiler program | +2.20 MB | +1.06 MB |
| Share of today's 37.17 MB exe | 3.0% | 31.1% (5.3%) | 5.9% | 2.9% |
| Memory per started connector (private) | 0.46–0.65 MB | 0.18–0.40 MB | 1.39 MiB | 0.14 MiB |
| Start one connector | 3.6–4.4 ms | 8–12 ms from cache; first compile 0.45–0.59 s | 2.5–3.0 ms | 0.8–0.9 ms |
| List 100 repositories (about 0.55 MB of JSON) | 14.5–17.9 ms | 1.6–2.2 ms | 11.2 ms | 7.9 ms |
| Endless loop stopped after | 0.3–0.8 s | about 0.1 s | 1.0 s | 1.0 s |
| Memory hog stopped at 64 MB after | 114 ms | about 6 ms | 112 ms | 478 ms |
| Idle CPU / threads added | 0 / 0 | 0 / 0 | 0 / 1 | 0 / 1 |
| GitHub connector file | 0.27 MB for 6 operations; 0.45–0.6 MB in full (estimate) | 0.35 MB in full (1,170 lines) | a 985-line script | the same script |
| New native code in SterneGit's cold build | Rust only | Rust only, but large | 75 s of C++ | 7 s of C |

The host services cost the same whichever engine is chosen:

| Piece | Exe |
| --- | --- |
| curl transport | +34 KB |
| OS credential store | +8 KB |
| JSON settings files | +83 KB |

The serde code for the shared protocol types adds an estimated 0.1–0.2 MB for the JSON-based designs (Wasmtime's generated bindings are already in its figure). Altogether:

| Option | Exe added, before the new screens |
| --- | --- |
| Lua 5.4 | about 1.3 MB (3.6%) |
| **wasmi** | **about 1.4 MB (3.7%)** |
| Luau | about 2.5 MB (6.6%) |
| Wasmtime, runtime only | about 2.1 MB (5.6%), plus a separate compiler program of about 12 MB (estimate) |
| Wasmtime with its compiler | about 11.7 MB (31.5%) |

How far to trust these numbers:
- **Each agent used its own baseline**, from 273 KB to 936 KB, though every baseline links serde_json and anyhow like SterneGit.
- **The deltas are upper bounds,** since SterneGit already links some of the same crates.
- **Timings were taken on a busy machine.** Other builds kept it loaded.
- **Only Windows was measured.**
- **The new screens aren't counted.** Accounts, sign-in, clone, publish, SSH keys, approval, and pull requests with reviews in the diff come on top for every engine.

Measure SterneGit's real delta at the first milestone.

The host services' HTTPS choice:

| Option | Exe | SterneGit's memory | Warm request |
| --- | --- | --- | --- |
| System curl, request and token on stdin (recommended) | +34 KB | +0 MB | about 110 ms (first request 495 ms) |
| Git's own libcurl, loaded at run time | +20 KB | +7.2 MB | 23 ms |
| ureq with rustls | +2.1 MB | +4 to +7 MB | 22–27 ms |
| ureq with native TLS | +0.68 MB | +5.3 MB | rejected: on Linux SterneGit wouldn't start without libssl |

## 3. How they compare

### Footprint

Lua 5.4 and wasmi cost about the same, Luau about twice as much, and Wasmtime with its compiler about eight times as much.

Speed doesn't separate them in practice. A curl request takes about 110 ms; connector work takes 2–18 ms on any engine. Wasmtime's eightfold speed is invisible next to the network.

### Isolation between accounts

**wasmi and Wasmtime start a fresh instance for every call.** Nothing a connector holds while serving one account survives into a call for another.

**The Lua design keeps one script state per connector.**
- Its per-call account handles do stop a script from reaching another account's servers.
- But the script's own variables live on between calls, because the probe loads each script once.
- So a hostile connector could keep private data from a call made for a work account and post it through a personal account in a later call. That goes against the decision that a connector gets no access to other accounts.
- **Fix:** a state per account (Luau +1.4 MiB each, Lua 5.4 +0.14 MiB), or per call (0.8 ms each with Lua 5.4, 2.5 ms with Luau).

### Sandbox strength and engine maturity

- **wasmi:**
  - Its interpreter is written in Rust and checks every module before running it.
  - Fuel and memory limits were measured working.
  - But version 2.0.0 is two months old and unaudited; past audits covered 0.31 and 0.36–0.38.
  - Pausing a call while a request runs uses wasmi's own resumable calls.
- **Wasmtime:**
  - It has the most security engineering behind it (Bytecode Alliance, continuous fuzzing).
  - It brings a JIT compiler and process-wide trap handlers, and nobody has tested those handlers alongside GPUI.
  - Hardened macOS builds probably need a JIT entitlement.
  - A new major version ships every month.
- **Luau:**
  - Roblox built it for hostile scripts and runs it at scale, but its VM is C++.
  - mlua quietly gives every script a `require` that reads files, so SterneGit must remove it.
  - Long built-in calls such as `string.rep` can't be interrupted, only stopped by the memory limit.
- **Lua 5.4:** sandboxing is left to the program embedding it, and it has had memory-safety bugs reachable from scripts.

**All four run inside SterneGit's process,** which can read the credential store. Escaping the engine means escaping completely. None offers process isolation.

### Who can write a connector

- **wasmi:**
  - Rust is first-class. The SDK has typed protocol structs and runs native tests against recordings with a debugger.
  - Any language that can export its memory and call six simple functions also works: C, Zig, AssemblyScript, TinyGo. Their modules are estimated at 15–250 KB.
  - Authors need `rustup target add wasm32-unknown-unknown`.
- **Wasmtime:**
  - Rust works through wit-bindgen and the `wasm32-wasip2` target, with types on both sides.
  - Other languages make huge components: about 8 MB for JavaScript, tens of MB for Python.
  - Every author must also avoid WASI, the WebAssembly system interface. SterneGit's sandbox leaves it out, so a connector that touches it stops with an error.
- **Luau:**
  - One script file and no build step.
  - The editor checks types through luau-lsp, and errors carry line numbers.
  - But authors can only write Luau.

### Interface and versioning

- **Wasmtime's WIT is the best specified.** Types are checked at compile time on both sides, and version matching was tested from 1.0 to 1.1 in both directions. But its records are frozen: a forgotten field means a whole new interface.
- **wasmi's JSON absorbs additions.** Unknown fields are ignored, and unknown enum values fall back to a catch-all. It's versioned by the name of the import module plus a protocol major.minor. Rust authors get types from the shared protocol crate; others work from documentation.
- **Luau checks what comes back.** A bad value is reported with its path, such as `items[0].clone: missing field https`, the clearest error any design gives an author.

### Effect on SterneGit's build

wasmi and Wasmtime are pure Rust, though Wasmtime takes long to compile. Luau adds 75 s of C++ to every cold build and needs a C++ compiler on Linux. Lua 5.4 adds 7 s of C.

### How much was proven

- **Luau and Wasmtime** each ran a complete GitHub connector (985 and 1,170 lines) against recorded GitHub responses.
- **wasmi** ran a six-operation stand-in.
- **Wasmtime** also proved its version matching in both directions.
- **The host services** proved three things with Git 2.35.1:
  - the curl transport keeps the token off the command line;
  - the credential helper can be scoped per repository owner;
  - the SSH command survives its quoting.

## 4. Review findings to fix, whichever engine wins

1. **One state shared across accounts in the Lua design** (section 3).
2. **Line anchors.** wasmi's `Anchor { side, line }` holds one line number, but GitLab needs both the old and new numbers for comments on unchanged lines. Use the `{ old, new }` pair from the Lua and Wasmtime designs; SterneGit knows both from its own diff.
3. **One host API.** Each engine proposal invented its own call into the host. Adopt host-services.md's `HostServices::request(account, ApiRequest, Cancel)` and its `NetError`. wasmi's `send` and `wait` map onto it directly.
4. **One domain model, taking the best of each:**
   - **From Lua:**
     - `pull_requests_for_branch`, for pull request badges on branches;
     - the list of verdicts a server supports;
     - `queued` merges, since Azure DevOps completes merges later.
   - **From Wasmtime:**
     - `unfinished { posted }`, for reviews a server can't post in one go (Azure DevOps, GitLab draft notes);
     - a `checking` mergeability state, for GitHub's `mergeable: null`;
     - the `semi-linear` and `fast-forward` merge methods;
     - a head owner, for pull requests from forks.
   - **From wasmi:** the fullest failure list (`Conflict`, `Unsupported`, `Protocol`, `Refused { status }`).
5. **Sign-in details.**
   - **One scope list:** `repo read:org write:public_key workflow`. The Lua draft asked for the broader `admin:public_key`. The wasmi draft left out `workflow`, which HTTPS pushes that touch `.github/workflows` need.
   - **The reused Credential Manager token** is copied into SterneGit's own store and sent as Bearer, as host-services.md says. wasmi's draft sent Credential Manager's user and password pair instead.
6. **Silent updates.** An update that stays within its grant starts without asking, so the connector list should mark it "updated".
7. **Cleaning up accounts.** A removed account leaves its per-repository SSH setting behind, so SterneGit should list the repositories it changed and offer to undo them.

## 5. Recommendation

**wasmi, with the host services as proposed:**

1. **Size.** The smallest exe of the options that let authors pick their language: about 1.4 MB in all.
2. **Build.** Pure Rust: no C++ and nothing new to install for anyone building SterneGit.
3. **Isolation.** A fresh instance per call means:
   - nothing carries from one account to another;
   - nothing stays alive while idle;
   - calls can run in parallel without limit;
   - dropping a `Job` cancels its call.
4. **Distribution.** One portable `.wasm` file per connector, a typed Rust SDK with native tests, and room for other languages.
5. **A way out.** The same `.wasm` files and their six imports would run unchanged on Wasmtime; only SterneGit's side would change.
6. **No visible slowness.** About 15 ms of connector work per 100 repositories, next to about 110 ms per request.

To keep its risks small:
- pin wasmi 2.0.x;
- keep module validation on;
- fuzz the six host functions;
- follow its releases.

**Runners-up:**
- **Lua 5.4,** if the least memory and script-only authoring matter most. It's about as big as wasmi in the exe, with a seventh of its memory, but needs a state per account and a hand-built sandbox.
- **Luau,** if the Roblox-tested sandbox and editor types are worth 1.1 MB more, a C++ build and about 1.4 MiB per connector.

**Not recommended: Wasmtime.** It costs a 31% bigger exe, or a separate 12 MB compiler program, for speed nobody will see. Its WIT is the best-drawn interface, though, so its shapes should feed the domain model.

**What would change this:**
- **Luau,** if connectors will mostly be written by people who won't install a toolchain.
- **Lua 5.4,** if dozens of connectors are expected, so memory per connector adds up.
- **Wasmtime, reusing the wasmi files,** if connectors ever do heavy work themselves (large diffs, search indexes).

## 6. Decisions for you

1. **Engine:** wasmi (recommended), Luau, Lua 5.4 or Wasmtime.
2. **Bundled connectors:**
   - **Embedded in the exe** with `include_bytes!` and run through the same sandbox. SterneGit stays one file, with nothing beside it to tamper with.
   - **Or a `connectors/` folder beside the exe,** checked against hashes the exe carries. This is what you chose earlier, and it makes SterneGit a folder rather than one file.
3. **SDK documentation:** allow `///` comments in `connectors/sdk/` only, or keep the SDK comment-free and document it in `docs/connectors/authoring.md`. Either way, `tests/no_comments.rs` should cover `connectors/` too.
4. **Your step, needed by milestone 4:** register a SterneGit OAuth App on GitHub with "Enable Device Flow" ticked. Its public client ID goes in the GitHub connector's manifest.

## 7. Build plan

1. **Host foundation, no UI:**
   - settings folders and JSON files;
   - the OS credential store;
   - `--git-credential`, and passing the account through `src/git/command.rs`;
   - the curl transport, with the allow-list and token schemes.

   Tested against local servers.
2. **Engine:** the wasmi host, the six imports, the protocol crate, the SDK, misbehaving test connectors and `--connector-test`. Then measure SterneGit's real exe delta.
3. **Discovery and approval:** manifests, grants, hash pinning or embedding, starting at launch, and the build script.
4. **Accounts and repositories,** mockup first:
   - accounts and device-flow sign-in;
   - a clone browser;
   - creating and publishing repositories;
   - SSH keys, with GitHub's host keys written up front;
   - the approval screen.

   Then GitHub's who-am-I, repositories and SSH keys.
5. **Pull requests,** mockup first: list, details, checks, fetching the head and diffing it locally, create, merge.
6. **Reviews,** mockup first:
   - threads in the comparison view, which means changing the vendored diff component;
   - drafts, submit, reply, resolve.
7. **Author kit and a second platform:** a template and an authoring guide, then a Gitea or GitLab connector that must not need any change to SterneGit.

## Found along the way

The host-services study noticed that SterneGit's 5-minute background fetch doesn't set `GCM_INTERACTIVE=never`. Git Credential Manager may therefore open a sign-in window in the middle of an unattended fetch. This wasn't tested, because testing it would have touched Credential Manager. The fix belongs to today's fetch code, not to connectors.
