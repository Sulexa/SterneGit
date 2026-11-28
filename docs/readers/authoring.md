# Writing a reader

A reader teaches SterneGit a file format. It is a WebAssembly module plus a `reader.json` manifest, in a folder named after the reader's id. SterneGit hands it a file, it answers what the file holds, and SterneGit compares and draws everything itself. The design behind it is in [design.md](design.md).

Start from `readers/template/`: a working reader for `.ini` files, about a hundred lines with its test.

## 1. The crate

```toml
[package]
name = "my-reader"
edition = "2024"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
sternegit-reader-sdk = { path = "../sdk" }

[build-dependencies]
sternegit-reader-sdk = { path = "../sdk" }

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"
strip = true
```

- `build.rs` calls `sternegit_reader_sdk::build::link_stack()`, which gives the module a 1 MiB stack.
- `rust-toolchain.toml` asks for the `wasm32-unknown-unknown` target.
- `cdylib` is the module SterneGit runs; `rlib` lets `cargo test` run the same code natively.

## 2. The reader

```rust
use sternegit_reader_sdk::{Failure, Model, ReadInput, Reader, export, read_text};

pub struct MyReader;

impl Reader for MyReader {
    fn read(&self, input: ReadInput) -> Result<Model, Failure> {
        let text = read_text(&input.path).ok_or_else(|| Failure::Missing { path: input.path.clone() })?;
        Ok(Model::default())
    }
}

export!(MyReader);
```

- `read` is called once per side of a comparison. `input.side` says which; `input.kind` is the id of the kind the manifest matched.
- Answer `Failure::NotThisFormat` for a file that has the extension but isn't yours: SterneGit then shows it as text.
- A panic stops the call; its message reaches SterneGit's log and the "couldn't read this file" bar.

## 3. The model

A `Model` is what one version of a file holds.

| Piece | What SterneGit does with it |
| --- | --- |
| `objects` | Each `Object` has a stable `id`, an optional `parent` id, a `name`, a `role` (`Node`, `Instance` or `Asset`) and `parts`. Objects with a parent, or with the `Node` or `Instance` role and no parent, form the hierarchy. Objects are paired by `id` between the two sides, so keep ids stable across saves. |
| `parts` | Like components: an `id`, a `title`, an optional `script` name, and `fields`. |
| `fields` | A tree. A field's path is its parent's path, a dot and its `key`, with no dot before a key starting with `[` or `:`. List entries pair by key, so give entries an id-based key when the format has one. |
| `Value` | `Text`, `Number` (as written), `Flag`, `Vector` (axis, value), `Color`, `Reference` (a name, a kind, an icon and a target), `Nothing`, `Count`, `Long` and `Data` for long text and binary blobs (with a digest), `Empty` for a group. |
| `tags` | On objects and fields: option ids from the manifest. `noise` marks fields hidden while Hide noise is on. |
| `scopes` | Line ranges with a label, shown in the text diff above each change. |
| `notes` | Line numbers with a short text, shown after that line in the text diff, such as the name a reference points to. |
| `curves`, `graph`, `layout`, `pictures` | Optional views: animation curves, a node graph (script or state machine) and a 2D layout with pictures cut from image files. |
| `text` | A readable text to diff instead of the raw file, such as pretty-printed embedded JSON. |

## 4. Reading other files

- `read_file(path)` and `read_text(path)` read any file of the repository being shown: the commit being compared first, then the working tree for what it lacks. With `"read": "file"` in the grant, only the file itself and its side files.
- `search(regex, glob)` runs `git grep` over the same trees and answers each match with its path and text. SterneGit keeps the answers per tree, so asking the same search for both sides costs one `git grep`.
- There is no network, no clock beyond `now()`, and no way to write a file.

## 5. Other jobs

Each is a method of `Reader` with a default that does nothing, and a job in the manifest's grant.

- **`side_files`**: given both versions of a file, answer `Warning`s with a title, an explanation, the places involved and `FixOffer`s.
- **`fix`**: given an offer's id, answer a `Fix`: whole-file `Edit`s, each with `digest()` of the bytes it was made from. SterneGit shows the edits and writes them only when the user applies them, and only if each file still has that digest.
- **`merge`**: given the base, current and incoming text and the user's `Choice`s, merge everything you can and answer the `Conflict`s left; once every conflict has a choice, answer the merged `text`.

## 6. The manifest

```json
{
  "id": "ini",
  "name": "INI files",
  "version": "0.1.0",
  "protocol": "1.0",
  "module": "my_reader.wasm",
  "kinds": [
    { "id": "ini", "label": "Settings", "extensions": [".ini"], "sniff": [], "views": ["inspector", "text"], "text": "INI" }
  ],
  "side_files": [{ "pattern": "{path}.meta" }],
  "options": [{ "id": "noise", "label": "Hide noise", "default": true, "hides": true }],
  "grant": { "read": "file", "jobs": ["compare"] }
}
```

- **`id`**: up to 40 lowercase letters, digits, `-` or `_`; the folder's name.
- **`kinds`**: `sniff` strings must all appear in the file's first 4 KB; `views` are `inspector`, `layout`, `curves`, `graph` and `text`; `text` names the text view (default Text).
- **`side_files`** (manifest): patterns where `{path}` stands for the file's path. While the reader runs, a side file that changed along with its file has no row of its own in the changes list or a commit's files: the file shows a tag named after what the pattern adds (`meta` for `{path}.meta`), and staging, unstaging or discarding the file does the same to its side files.
- **`grant.read`**: `file` or `repository`. **`grant.jobs`**: `compare`, `side_files`, `fixes`, `merge`. Asking for more in an update waits for the user's approval again.

## 7. Testing and installing

- Test natively: `with_files(Folder::new(root), || MyReader.read(input))` serves files from a folder, and `search` uses `git grep` when the folder is a repository. `cargo test` runs it like any crate.
- Build: `cargo build --release --target wasm32-unknown-unknown`, then copy the `.wasm` next to `reader.json`.
- Install: copy the folder into `%APPDATA%\SterneGit\readers` (Settings → Readers → Open folder) and restart SterneGit. The reader waits for approval on the Readers page.
- Limits per call: 512 MB of memory and 20 billion fuel, about tens of seconds of work. Reading the largest Unity prefab measured takes 0.2 s.
- Bundled readers live in `readers/<id>/dist`, built by `scripts/build-connectors.ps1` and pinned in `src/read/catalog.rs`.
