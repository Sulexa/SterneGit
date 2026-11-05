# Connector host services

Proposal from the "host" agent, 2026-11-05. It covers what every engine design shares: HTTPS,
the token store, sign-in, git's credentials, SSH keys, settings and proxies. The wasmi, wasmtime
and Lua designs call the API in section 2.

## Summary

Almost everything connectors need is already on the machine or already linked, so the
recommended design adds no crates. HTTPS runs the system's curl with the request and its token on
curl's stdin. That adds 34 KB and no memory in FastGit, but a request to api.github.com takes about
110 ms, since nothing is reused. In-process clients answer in about 23 ms once connected, but add
0.68 MB (ureq with native-tls, which on Linux ties start-up to libssl.so.3) or 2.1 MB (ureq with
rustls), plus 4–7 MB of working set. Git's own libcurl, loaded at run time, adds 20 KB at that
same speed and is the upgrade path. Tokens go in the OS store, under FastGit's own names, through
bindings GPUI already links. Git gets them from FastGit's exe acting as a credential helper scoped
per URL. A local test with Git 2.35.1 confirmed that design, and showed where `GIT_ASKPASS` and
`http.extraHeader` fall short. GitHub sign-in uses an OAuth App with the device flow. SSH keys are
ed25519 files, set per repository through a quoted `core.sshCommand`.

## Measured numbers

All measured on this machine (Windows 11, rustc 1.99.1 MSVC, Git 2.35.1, curl 8.21.0). Nothing
was measured on macOS or Linux.

**Exe size.** FastGit's release profile (`lto = "thin"`, `strip = true`). The baseline links
serde_json, anyhow and `std::process`: 415,744 bytes. Each HTTPS probe sends GET and POST with
custom headers and a JSON body, reads response headers, parses `Link` and rate limits, and sets
timeouts. Probes under 200 KB were built twice from clean, with identical sizes.

| Probe | Bytes | Added |
|---|---:|---:|
| (a) system curl, request on stdin | 449,536 | +33,792 |
| (d) git's libcurl, loaded at run time | 435,712 | +19,968 |
| native-tls (Schannel) with a hand-written HTTP/1.0 request | 481,792 | +66,048 |
| ureq 3.5.1 without TLS | 980,992 | +565,248 |
| (b) ureq + native-tls | 1,095,168 | +679,424 |
| (b) attohttpc 0.32 + native-tls | 1,128,960 | +713,216 |
| rustls 0.23 + ring + platform-verifier, hand-written request | 1,873,920 | +1,458,176 |
| (c) ureq + rustls + rustls-platform-verifier | 2,539,008 | +2,123,264 |
| (c) ureq + rustls + webpki-roots | 2,586,112 | +2,170,368 |
| CredWriteW / CredReadW / CredDeleteW (windows-sys) | 423,424 | +7,680 |
| keyring 3.6.3, windows-native | 456,192 | +40,448 |
| keyring 4.2.0, which pulls in regex | 2,111,488 | +1,695,744 |
| accounts and approvals JSON, serde derive | 577,024 | +161,280 |
| the same with serde_json `Value` accessors | 498,688 | +82,944 |

These are upper bounds. FastGit already links some of the same code (http 1.x, log,
percent-encoding, flate2, std::fs, and regex through gix).

**Latency and memory.** Requests went to the public `https://api.github.com/zen` (18
unauthenticated requests in all) or to a local test server.

| Measurement | Result |
|---|---|
| Spawn `curl --version` from a GUI-subsystem parent, `CREATE_NO_WINDOW` | median 28 ms (25–37), n = 40 |
| The same with `DETACHED_PROCESS` | median 12 ms (11–18), n = 40 |
| curl GET to the local server, detached / no-window | median 14 / 32 ms, n = 20 each; in-process ureq ~1 ms |
| curl GET api.github.com, detached, n = 5 | 495 ms cold, then 105–122 ms |
| ureq + native-tls, one agent, n = 3 | 161, 34, 22 ms |
| ureq + rustls + platform-verifier | 124, 23, 27 ms |
| ureq + rustls + webpki-roots | 83, 22, 22 ms |
| git's libcurl 7.81 (Schannel), in process | 298, 23, 23 ms; loading took 8–42 ms |
| Working set the first HTTPS request adds to the caller | curl 0 (the child peaks at 9.2 MB, 1.9 MB private); native-tls +5.3 MB (+1.3 private); platform-verifier +7.0 (+2.0); webpki-roots +4.0 (+0.8); libcurl +7.2 (+1.7) |
| GUI-subsystem helper probe: direct / through git's `sh -c` | median 12 / 47 ms |
| FastGit's 37 MB release exe, `--help`: direct / through `sh -c` | median 36 / 77 ms |
| ssh-keygen ed25519, Git's 8.8p1 / Windows' 9.5p2 | 73 / 101 ms; with a passphrase 165 / 276 ms |

Estimated rather than measured: the GPUI `HttpClient` adapter (5–10 KB), libcurl at run time on
macOS and Linux (about the Windows figure), and the macOS and Linux store code (a few KB).

## 1. HTTPS transport

| | (a) curl program | (b) ureq + native-tls | (c) ureq + rustls | (d) libcurl at run time |
|---|---|---|---|---|
| Exe | +34 KB | +679 KB | +2.1 MB | +20 KB |
| FastGit's memory | 0 | +5.3 MB | +4.0 to +7.0 MB | +7.2 MB |
| Warm request | ~110 ms, no reuse | 22–34 ms | 22–27 ms | 23 ms |
| Proxies like git's | libcurl's environment rules, plus git's config passed in; System32's build lacks NTLM | ureq's rules (not per scheme), Basic proxy auth only | as (b) | the library git itself uses |
| Corporate root CAs | Windows yes (Schannel); macOS unverified; Linux the distro bundle | OS store | platform-verifier yes, webpki-roots no | as git |
| On Linux needs | the curl program, not always installed | libssl.so.3 at start-up, or vendored OpenSSL | nothing | libcurl.so.4 or libcurl-gnutls.so.4, which git needs anyway |
| Cancelling | kill the child | abandon the thread until it times out | as (b) | abort from the progress callback |

**Recommendation: (a) for v1.** It adds nothing to FastGit's memory, start-up or idle time, and
proxies the way git's libcurl does. Its latency runs in parallel: independent calls are separate
processes, up to four per account, and after `Link: rel="last"` the remaining pages go together.
(d) is the upgrade if reuse comes to matter, and the Linux fallback. (c) is worth 2.1 MB only to
avoid system programs. Not (b): on Linux, FastGit wouldn't start without libssl.so.3.

**Running curl.**

- Program: `%SystemRoot%\System32\curl.exe` (Windows 10 1803+), `/usr/bin/curl` on macOS,
  `curl` from `PATH` on Linux (the trust FastGit already gives `git`).
- Arguments: exactly `-q --config -`; `-q` first skips `.curlrc`. The process list showed only
  that while the fake token reached the server.
- `DETACHED_PROCESS`, not `CREATE_NO_WINDOW`: curl starts no children, so it needs no hidden
  console, which saves 16 ms.
- stdin carries the whole request:

```text
url = "https://api.github.com/user/repos?per_page=100&page=2"
silent
show-error
include
suppress-connect-headers
ssl-revoke-best-effort
proto = "=https"
request = "POST"
max-time = 30
connect-timeout = 15
header = "Expect:"
header = "Authorization: Bearer gho_..."
header = "Accept: application/vnd.github+json"
data-binary = "{\"title\":\"Fix \\\"quoted\\\" path\"}"
```

- Values escape `\\`, `\"`, `\n`, `\r`, `\t`. A JSON body with quotes, backslashes, `#`, tabs,
  emoji and a control character arrived byte for byte. A non-ASCII header value did not (é
  arrived as Ã©), so the host refuses non-ASCII header values.
- stdout holds the headers, then the body; 1xx blocks are skipped. Without `-L`, curl returned
  a 302 with its `Location`, and the host decides (section 2).
- Exit codes: 28 is `Timeout` (`max-time = 2` stopped at 2.05 s), 6 and 7 `Unreachable`, 35 and
  60 `Tls`, 5 and 97 `Proxy`.
- curl runs through `smol::process` (already linked through smol) with `kill_on_drop(true)`, so
  dropping the `Load` or `Job` kills it.
- Proxies: once per tab, `git config --get-urlmatch http <api origin>` supplies `http.proxy`
  (→ `proxy`), `http.proxyAuthMethod` (→ `proxy-anyauth`, git's default), `http.sslCAInfo`
  (→ `cacert`), `http.schannelCheckRevoke=false` (→ `ssl-no-revoke`) and `http.sslVerify=false`
  (→ `insecure`, that URL only, with a warning). curl reads `https_proxy`, `ALL_PROXY` and
  `NO_PROXY` itself, as git's libcurl does. Git ignores the Windows and macOS system proxies, and
  so does FastGit.
- No curl on Linux: v1 says "Install curl to use accounts", naming the package.

**Avatars.** A token-less, https-only `http_client::HttpClient` on the same runner, installed
with `Application::with_http_client`, makes `img(url)` work at one process per image. Cheaper:
fetch a screen's avatars in one curl call (`--parallel`, one `url`/`output` pair each, into
`<cache>/avatars/<hash>`) and draw them with `img(path)`.

## 2. The request API engines call

```rust
pub struct AccountId(String);
pub struct ConnectorId(String);

#[derive(Clone, Copy)]
pub enum Method { Get, Post, Put, Patch, Delete }

pub struct ApiRequest {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout: Option<Duration>,
}

pub struct ApiResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

pub enum NetError {
    NotSignedIn,
    ServerNotAllowed(String),
    HeaderNotAllowed(String),
    RateLimitedUntil(SystemTime),
    Timeout,
    Cancelled,
    Unreachable(String),
    Tls(String),
    Proxy(String),
    TooLarge,
    NoTransport(String),
}

pub struct AccountView {
    pub id: AccountId,
    pub server: Url,
    pub endpoints: Vec<(String, Url)>,
    pub login: String,
}

pub enum Level { Debug, Info, Warn, Error }

#[derive(Clone)]
pub struct Cancel(smol::channel::Receiver<()>);
pub struct CancelOnDrop(smol::channel::Sender<()>);

pub trait HostServices: Send + Sync + 'static {
    fn account(&self, account: &AccountId) -> Option<AccountView>;
    fn request(
        &self,
        account: &AccountId,
        request: ApiRequest,
        cancel: Cancel,
    ) -> BoxFuture<'static, Result<ApiResponse, NetError>>;
    fn pause(&self, account: &AccountId, until: SystemTime);
    fn log(&self, connector: &ConnectorId, level: Level, message: &str);
    fn now(&self) -> SystemTime;
}
```

Engines with synchronous host calls (wasmi, Lua) run `smol::block_on(host.request(..))` on their
worker thread. The UI's `Job` holds the `CancelOnDrop`; dropping it closes the channel, the
request loses its race against `Cancel`, and curl is killed. The host enforces:

1. **Allow-list**, fixed at account creation from the manifest's server templates:
   `https://api.github.com/` for github.com; `https://ghe.corp/api/v3/` and
   `https://ghe.corp/api/graphql` for an Enterprise Server at `https://ghe.corp/`. URLs match on
   scheme, host (normalized by `url`), port and a path prefix ending at `/`. `http` only for a
   server the user typed as `http://`, marked unencrypted.
2. **Token scheme**, named by the sign-in method: `Bearer`, `Header { name }` or
   `Basic { username }`. GitHub: Bearer. GitLab: `PRIVATE-TOKEN` for personal tokens, Bearer for
   OAuth. Azure DevOps: Basic with an empty user name for personal tokens, Bearer for Entra ID.
   Only origins the manifest marks `token: true` get it.
3. **Headers**: printable ASCII, no CR or LF. Connectors can't set `Authorization`,
   `Proxy-Authorization`, `Cookie`, `Host`, `Content-Length`, `Transfer-Encoding`,
   `Connection`, `Expect` or their scheme's header. `User-Agent` is
   `FastGit/<version> <connector>/<version>`.
4. **Redirects**: up to five; 301–303 for GET and HEAD only, 307 and 308 for any method with the
   body. A target off the allow-list fails with `ServerNotAllowed(location)` uncontacted; an
   allowed target marked token-less gets no token.
5. **Limits**: 32 MB bodies (`TooLarge`), 30 s default timeout, four requests at once per
   account. After `pause`, requests fail at once with `RateLimitedUntil`.
6. **Logs**: 200 lines per connector, in memory only, with the token and its Basic form replaced
   by `•••`.

## 3. Token storage

GPUI's own `write_credentials` and `read_credentials` file entries as `zed:url=<url>` (Windows)
and under the label `zed-github-account` (Linux). That is Zed's namespace, where they could
collide with Zed's entries, and a helper process has no GPUI `App` to call them from. FastGit
should instead write about 150 lines per OS on the same bindings, already linked with the needed
features: `windows` 0.62 with `Win32_Security_Credentials` (via gpui-pre-windows),
objc2-security with `SecItem` (gpui-pre-apple) and oo7 0.6 (gpui-pre-linux). That adds about
8 KB on Windows. The keyring crate adds 40 KB (version 3) or 1.7 MB (version 4), plus, on Linux,
a second Secret Service client beside oo7.

| | FastGit's entry | Git Credential Manager's |
|---|---|---|
| Windows | Generic credential `FastGit/<account>`, user name the login, `CRED_PERSIST_LOCAL_MACHINE` so it doesn't roam | `git:https://github.com` |
| macOS | `kSecClassGenericPassword`, service `FastGit`, account `<account>` (GPUI uses internet passwords) | service `git:https://github.com` |
| Linux | oo7 item labelled `FastGit: <login> on <host>`, attributes `application=fastgit`, `account=<account>` | `service=git:https://github.com` |

- Account ids are `<connector>:<host>:<user id>`, such as `github:github.com:583231`; the
  numeric id survives renames.
- Refresh tokens go in a second entry, `<name>#refresh`. Windows caps blobs at 2,560 bytes, which
  Entra ID access tokens can exceed, so those stay in memory.
- Linux without a Secret Service: by default tokens last the login session, in
  `$XDG_RUNTIME_DIR/fastgit/tokens` (0600 in a 0700 tmpfs, readable by the helper). Opting in to
  "Remember on this computer, unencrypted" writes `$XDG_STATE_HOME/fastgit/tokens` (0600),
  state rather than config so dotfile repos skip it.
- Nothing is read at start-up, where a Keychain prompt or D-Bus activation would delay the first
  frame; the first read comes with the first network use.

## 4. Sign-in

**OAuth App, not GitHub App.** A GitHub App's user tokens see only repositories where the app is
installed, so every organization would have to install it; they expire every 8 hours, and
refreshing needs a client secret a desktop app can't keep. OAuth App tokens don't expire (GitHub
revokes them after a year unused) and see all the user's repositories. Organizations with OAuth
app access restrictions need an owner's approval, which the UI explains when their repositories
are missing. Scopes: `repo read:org write:public_key workflow` (`workflow` lets HTTPS pushes that
touch `.github/workflows` through). The app needs "Enable Device Flow" ticked.

```json
"sign_in": [
  { "method": "device", "client_id": "Ov23li...",
    "device_code_url": "https://github.com/login/device/code",
    "token_url": "https://github.com/login/oauth/access_token",
    "scopes": ["repo", "read:org", "write:public_key", "workflow"], "token": { "kind": "bearer" } },
  { "method": "pasted_token", "token": { "kind": "bearer" },
    "create_url": "{web}/settings/tokens/new?scopes=repo,read:org,write:public_key,workflow&description=FastGit" },
  { "method": "git_credential", "token": { "kind": "bearer" } }
]
```

**The device flow (RFC 8628), run by the host.**

1. POST the form `client_id`, `scope` to `device_code_url` with `Accept: application/json`;
   get `device_code`, `user_code`, `verification_uri`, `expires_in`, `interval`.
2. Show `user_code`; open `verification_uri`.
3. Every `interval`, poll `token_url` with `client_id`, `device_code` and
   `grant_type=urn:ietf:params:oauth:grant-type:device_code`, on a `smol::Timer` inside the
   sign-in dialog's `Job`: the design's only timer, gone with the dialog.
4. `authorization_pending`: keep polling. `slow_down`: the larger of the returned interval and
   the current one plus 5 s. `expired_token`: offer a new code. `access_denied`: stop.
   `device_flow_disabled`, `incorrect_client_credentials`, `unsupported_grant_type`:
   configuration errors.
5. Hold `access_token` (and any `refresh_token`, `expires_in`) in memory, run the connector's
   who-am-I, then store the token and write the account. The approval screen lists these URLs.

**Refresh, for every connector.** Within 2 minutes of an account's `expires_at` (10 before a git
network command), the host POSTs `grant_type=refresh_token`, `refresh_token` and `client_id`
(plus `client_secret` if the manifest has one) to `token_url`. One refresh per account is in
flight at a time, and a rotated refresh token is stored before the new access token is used. A
401 on an expiring token gets one refresh and one retry; `invalid_grant` becomes `NotSignedIn`.
Only the main process refreshes, never the helper, which would race it for the rotated token.
GitHub's OAuth tokens never need this; GitLab's and Entra ID's will.

**A pasted token** is stored only after who-am-I succeeds with it. For Enterprise Server, the
server address the user types also yields the allow-list.

**Reusing git's credential.** This runs only after the user clicks "Use the token Git
Credential Manager has for github.com":

```text
git -c credential.interactive=false -c core.askPass= credential fill
env:   GCM_INTERACTIVE=never  GIT_TERMINAL_PROMPT=0  (GIT_ASKPASS and SSH_ASKPASS removed)
stdin: protocol=https / host=github.com / username=<login, if known> / blank line
```

The empty `core.askPass` stops git falling back to `SSH_ASKPASS`, which KDE sets session-wide.
The token passes who-am-I and is copied into FastGit's own entry; nothing is sent back with
`approve` or `reject`. As instructed, this study never ran it.

**Can the token add SSH keys?** For classic and OAuth tokens, who-am-I's `X-OAuth-Scopes` says,
and the manifest maps `ssh_keys.add` to `write:public_key` or `admin:public_key`. Credential
Manager's GitHub tokens normally carry `repo, gist, workflow`, so the button is disabled with the
device flow offered. Fine-grained tokens send no `X-OAuth-Scopes`; a 403 or 404 with
`X-Accepted-GitHub-Permissions` becomes `MissingPermission(needed)`. The account records the
scopes who-am-I reports.

## 5. Git credential bridge

Tested with Git 2.35.1 against a local server and an isolated config (`GIT_CONFIG_NOSYSTEM=1`,
a `GIT_CONFIG_GLOBAL` file with a decoy helper), so Credential Manager was never touched. The
helper was a GUI-subsystem probe in a path with spaces.

| | FastGit as helper | `GIT_ASKPASS` | `http.extraHeader` via `GIT_CONFIG_COUNT` |
|---|---|---|---|
| Applies to | one URL prefix | the whole process | the whole process |
| User's helpers for other servers | kept: the decoy still answered `localhost` | helper list must be emptied | unaffected |
| Two accounts on one server | one per owner path, verified | only by parsing the prompt | per URL |
| Token in children's environment | no | no | yes: an alias's shell printed `GIT_CONFIG_VALUE_0` |
| On redirect | each host asked separately | not tested | curl 7.81 forwarded the header to another port (dropped it for another host name) |
| Hears of a rejected token | yes, via `erase` | no | no |
| Git needed | any (2.9 for the reset) | any | 2.31; the README allows 2.26 |
| Cost on Windows | `sh` and the exe each call: ~77 ms with FastGit's exe, twice (get, store) | 12–36 ms, twice | none |

The GUI-subsystem exe's stdin and stdout worked whether git started it through `sh` (helper) or
directly (askpass).

**Design.** Each git network call gets one pair of settings per remote owner with a known
account, or one per server where FastGit has a single account:

```text
git -c credential.https://github.com/fastgit-org.helper=
    -c "credential.https://github.com/fastgit-org.helper=!'C:/Program Files/FastGit/fastgit.exe' --git-credential github:github.com:583231"
    fetch --all --prune --quiet
```

- The empty value resets the helper list only under that prefix (verified for a host and for
  owner paths); other servers keep the user's helpers.
- `fastgit --git-credential <account> get` prints `username=` (the manifest's git user, such as
  `x-access-token`) and `password=` from the store, or `quit=1` without a token, which stops git
  at once ("not signed in"). `store` and `erase` do nothing; a rejected token shows as git's
  "Authentication failed" and the API's 401. Timer-driven fetches add `--unattended`, so a locked
  Linux keyring makes the helper quit instead of prompting.
- The helper string holds only the exe path and account id. Git hands it to children in
  `GIT_CONFIG_PARAMETERS`, no secret included, which is also how git-lfs finds it.
- Timer-driven fetches also set `GCM_INTERACTIVE=never`, so Credential Manager can't open a
  window for servers without a FastGit account.

```rust
pub(crate) struct Access {
    pub(crate) helper: PathBuf,
    pub(crate) scopes: Vec<(Url, AccountId)>,
    pub(crate) unattended: bool,
}

impl Access {
    pub(crate) fn config_args(&self) -> Vec<OsString>;
}

pub(crate) fn run_with_access<A: AsRef<OsStr>>(
    location: &Path,
    access: &Access,
    args: &[A],
    exit: Exit,
) -> Result<Vec<u8>> {
    let mut command = git(location, &access.config_args());
    command.args(args);
    if access.unattended {
        command.env("GCM_INTERACTIVE", "never");
    }
    checked(command.output().context(GIT_MISSING)?, exit)
}
```

`Repo::fetch`, `pull`, `push`, `pull_branch`, `push_branch` and `force_push` take `&Access`,
which `repository.rs` builds from the tab's remotes and accounts (`unattended` for the 5-minute
and window-return fetches). `cli.rs` gains `Invocation::GitCredential { account, operation }`
and `Invocation::SshAskpass { prompt }`, recognized only as `--git-credential` or
`--ssh-askpass` in first position, so a repository named `credential` still opens.

## 6. SSH keys

- **Type** comes from the manifest: ed25519 for GitHub, GitLab and Gitea. Azure DevOps is
  believed to take only RSA; check when that connector is written.
- **Generation**, with the ssh-keygen beside git's ssh:
  `ssh-keygen -q -t ed25519 -N "" -C "FastGit <login>@<host>" -f <file>`. Each OpenSSH build
  read the other's keys. Windows' ssh-keygen sets explicit ACLs (owner, SYSTEM,
  Administrators); Git's inherits the folder's (the user, SYSTEM, Administrators), which
  Windows' OpenSSH accepted.
- **Location**: `%LOCALAPPDATA%\FastGit\ssh\` (not roaming),
  `~/Library/Application Support/FastGit/ssh/` or `$XDG_DATA_HOME/fastgit/ssh/` (0700), named
  `<connector>-<host>-<user id>_ed25519` plus `.pub`.
- **Passphrase**: none in v1. The folder's permissions protect the key, and terminal git in the
  repository works too. Later, a random passphrase could live in the OS store, served by
  `fastgit --ssh-askpass` with `SSH_ASKPASS_REQUIRE=force`: with both OpenSSH builds,
  `ssh-keygen -y` opened a locked key with the passphrase the GUI probe supplied. Setting it with
  `ssh-keygen -p` through the same askpass keeps it out of argv.
- **Which ssh**: Git for Windows runs its own `/usr/bin/ssh` (8.8p1 here), by default and for an
  sshCommand starting `ssh`; `PATH` has Windows' 9.5p2. FastGit keeps the program the effective
  `core.sshCommand` names and appends its options. A `GIT_SSH_COMMAND` in the environment
  overrides the repository's setting, so FastGit warns.
- **sshCommand**, written with `git config core.sshCommand <value>` or
  `git clone -c core.sshCommand=<value>`:

```text
ssh -i 'C:/Users/me/AppData/Local/FastGit/ssh/github-github.com-583231_ed25519' -o IdentitiesOnly=yes -o 'UserKnownHostsFile="C:/Users/me/AppData/Local/FastGit/ssh/known_hosts" ~/.ssh/known_hosts' -o StrictHostKeyChecking=yes
```

  Forward slashes, single quotes for sh (an apostrophe becomes `'\''`), double quotes inside the
  `-o` value. With spaces in every path, both OpenSSH builds resolved it under `ssh -G` through
  `sh -c`, and it survived `git clone -c` into `.git/config` and parsed again.
- **Host keys**: FastGit's own `known_hosts` comes first, so new keys land there; the user's is
  still read. The connector supplies keys: GitHub's `GET /meta` returns `ssh_keys` (ed25519,
  ECDSA, RSA), written as `github.com <key>` lines, so `StrictHostKeyChecking=yes` holds. A
  server publishing no keys gets `accept-new` into FastGit's file, and FastGit shows the
  fingerprint.

## 7. Settings and approvals

| | Windows | macOS | Linux |
|---|---|---|---|
| `accounts.json`, `connectors.json` | `%APPDATA%\FastGit\` (beside `tabs`) | `~/Library/Application Support/FastGit/` | `$XDG_CONFIG_HOME/fastgit/` |
| Installed connectors | `%APPDATA%\FastGit\connectors\` | `…/FastGit/connectors/` | `$XDG_DATA_HOME/fastgit/connectors/` |
| SSH keys, `known_hosts` | `%LOCALAPPDATA%\FastGit\ssh\` | `…/FastGit/ssh/` | `$XDG_DATA_HOME/fastgit/ssh/` |

JSON through `serde_json::Value` accessors: 83 KB measured, against 161 KB for serde derive.
Unknown fields survive rewrites; an unparseable file is renamed `.broken` and read as empty.
Writes go to `<name>.unfinished`, then rename into place, as in `saved_tabs.rs`. Each write
re-reads first, so two running FastGits merely race.

```json
{ "version": 1,
  "accounts": [{ "id": "github:github.com:583231", "connector": "github", "server": "https://github.com",
                 "login": "octo", "name": "Octo Cat", "avatar_url": "https://avatars.githubusercontent.com/u/583231",
                 "sign_in": "device", "scheme": { "kind": "bearer" }, "scopes": ["repo", "read:org", "workflow"],
                 "expires_at": null, "ssh_key": "github-github.com-583231_ed25519" }],
  "remote_owners": [{ "connector": "github", "server": "https://github.com", "owner": "fastgit-org",
                      "account": "github:github.com:583231" }] }
```

`connectors.json` holds each connector id's version, SHA-256 and grant (servers, sign-in URLs,
features). A manifest asking no more than its grant is approved, and a new version or hash
inside it is recorded silently. Anything more waits for the user, who sees only the additions.
Bundled connectors, beside the exe, skip the check.

## 8. Layout

```text
src/net/        request.rs   ApiRequest, ApiResponse, Method, NetError, header rules
                curl.rs      config writer, spawn, response parser
                policy.rs    Origin, AllowList, TokenScheme, redirect rule
                proxy.rs     git config --get-urlmatch http <url>
                images.rs    token-less HttpClient for avatars
src/accounts/   account.rs   Account, AccountId, remote-owner choices
                secrets/     windows.rs, macos.rs, linux.rs, file.rs
                device_flow.rs, refresh.rs, git_credential.rs
src/settings/   folders.rs   config, data, state and runtime folders per OS (saved_tabs moves onto it)
                json_file.rs atomic JSON files
                approvals.rs
src/host/       mod.rs       the HostServices the engines receive
src/git/        access.rs    Access and its URL-scoped helper settings
                ssh_key.rs   ssh-keygen, sshCommand, known_hosts
                command.rs   + run_with_access
src/cli.rs      --git-credential, --ssh-askpass
```

None of this is UI code; `app/` only installs the `HttpClient`.

## 9. Decisions worth a second look

- **Pre-approving bundled connectors by location.** The folder beside the exe is user-writable in
  per-user and portable installs, so anything dropped there would be trusted. Pre-approve by
  hash instead: the exe carries the SHA-256 of each connector it ships, and a file that doesn't
  match is treated as user-installed.
- **Reusing Credential Manager's token.** It copies a token issued to GCM's OAuth app. That token
  usually can't add SSH keys, and it dies when the user revokes GCM. Offer it, but make the device
  flow the default.
- **Per-repository `core.sshCommand`** outlives the account. Removing the account would leave
  repositories pointing at a deleted key and break terminal git in them. FastGit should list the
  repositories it changed and offer to undo them when an account is removed.

## 10. Risks and unknowns

- **macOS curl**: does Apple's curl (LibreSSL, SecureTransport also built in) trust roots added
  to the Keychain? Unverified; fallbacks are `CURL_SSL_BACKEND=secure-transport` or (d).
- **Linux without curl**: some Debian and Ubuntu installs may lack it. v1 shows a message; (d)
  fixes it.
- **NTLM**: System32 curl lists none, so NTLM-only proxies fail. Git's bundled
  `mingw64/bin/curl.exe` has it and could take over on a 407 asking for NTLM.
- **Latency**: ~110 ms per curl request, mostly hidden by parallelism; otherwise (d) costs 20 KB
  and 7 MB of working set.
- **Helper cost**: ~150 ms per git network command on Windows, through `sh`. Credential
  Manager's cost wasn't measured.
- **macOS Keychain**: access follows the code signature, so unsigned builds prompt after each
  update; helper mode also lets any local program fetch a token by running FastGit, as with
  Credential Manager or Windows Credential Manager read directly.
- **Credential Manager windows**: FastGit's current background fetch doesn't set
  `GCM_INTERACTIVE=never`, so Credential Manager may pop up. Not tested, since it would touch
  Credential Manager.
- **Old git**: the URL-scoped reset was verified on 2.35.1 only; CI should rerun the local
  401-server test with Git 2.26.
- **Console spawns**: each `CREATE_NO_WINDOW` spawn costs ~16 ms more than a detached one,
  FastGit's existing git calls included.

## 11. Build plan

1. **Settings, store, git.** Settings folders and files, the secret store on each OS,
   `--git-credential`, `Access` in `command.rs`, and pasted-token accounts. Tests: the helper
   protocol, the scoped settings, and a local 401 server like the probe's, with no network.
2. **Network.** The curl transport, allow-list, token schemes, redirects, proxy settings and
   logs, after which engines can call `request`. Tested against a local HTTP server.
3. **Sign-in.** The device flow, refresh (tested against a local fake OAuth server), the
   Credential Manager import, and recording scopes.
4. **SSH.** Key generation, `known_hosts` seeding, the per-repository sshCommand, clone and
   publish.
5. **Later.** Avatars (the `HttpClient` and a batched cache), libcurl at run time as the upgrade
   and Linux fallback, and the NTLM fallback.
