# Readers: the design

Chosen on 2026-11-27. A draft for review: nothing here is built yet. It follows [the connectors design](../connectors/design.md) wherever the two can share, and says so where readers need something else.

## Decisions

| Topic | Decision |
| --- | --- |
| What | A **reader** is a plugin that teaches SterneGit a file format. Connectors talk to servers; readers read files. |
| Plugins | Readers are plugins in folders, and anyone may write one. Unity's reader is a plugin like any other. |
| Engine | The connectors' engine: WebAssembly core modules on wasmi 2.0, exchanging JSON messages. No new engine in the exe. |
| Screens | Readers only supply data. SterneGit compares and draws every view, in the mockup first. |
| Views | Every view Unity uses today, made generic: object tree and inspector, graph, curves, layout sketch, text with scopes. |
| Jobs | Compare two versions; pair side files and warn about them; propose fixes; merge conflicts by object. |
| Reading | A reader may read and search any file of the repository, read-only: the commit being shown first, then the working tree for what that commit lacks. No disk paths, no network, no process. |
| Search | A `search(regex, glob)` import backed by `git grep`, so Unity finds GUID owners as it does today without SterneGit knowing Unity. |
| Options | Each reader declares its options; the model tags what each one hides, so toggling filters without reading again. Show unchanged stays SterneGit's, for every reader. |
| Cache | Models in memory per window, keyed by blob id, reader version and lookup inputs, under a size cap. An entry is dropped when a working-tree file or search it used changes. No disk cache. |
| Elsewhere | The reader's scopes label the plain text diff everywhere; file lists show the kind as a muted tag, by extension only. |
| Writing | A reader never writes. It proposes file edits; SterneGit previews them as a change and writes only when the user applies it. |
| Matching | The manifest lists extensions and an optional first-bytes check. A file that fails the check falls back to text. Two readers claiming one file: the user picks once. |
| Unity | `src/unity/` leaves the exe and becomes `readers/unity/`, bundled beside the exe like the GitHub connector. |
| Speed | Same engine, measured first. Milestone 2 ports prefabs only and times the largest prefab available. Text shows at once; the reader's view replaces it when ready. If it is still too slow, the engine question comes back with numbers. |
| Bundled, approval, installing | As connectors: a `readers/` folder beside the exe with SHA-256 pins; a new reader or an update asking for more waits for the user; installing is copying a folder. |
| Updates | Bundled readers update with SterneGit releases. Installed readers update by copying a new folder. No store. |
| In-repo readers | Not in v1: a repository can't bring a reader, even sandboxed. |
| Merge | Changes that don't conflict merge on their own; the user picks Ours or Theirs per conflicted field, or for a whole object at once. |
| Second reader | Godot `.tscn` and `.tres`. |
| Work | On `develop` in a separate git worktree, one commit per milestone; the Readers screens in the mockup before milestone 4. |

## Words

- **Reader:** a plugin: a folder holding a manifest and a module.
- **Kind:** one sort of file a reader handles, such as a Unity prefab or an animation clip. A kind decides which views are offered.
- **Model:** what a reader makes of one version of one file. SterneGit compares two models.
- **Object:** one thing in a model with a stable id, such as a GameObject or a component. Objects nest.
- **Field:** a named, typed value on an object. Fields nest.
- **Side file:** a file that belongs with another, such as `Player.prefab.meta` with `Player.prefab`.
- **Fix:** a set of file edits a reader proposes, with a title, for the user to apply.

## 1. Pieces and where they live

```text
readers/protocol/      serde types shared by SterneGit and the reader SDK
readers/sdk/           the author crate: the Reader trait, export!, file access, native test runner
readers/unity/         the Unity reader (today's src/unity/ moved, minus drawing and comparing)
readers/unity/dist/    reader.json, unity.wasm, SHA256SUMS, all committed
readers/template/      the starting point for authors

src/plugin/            what connectors and readers share: the wasmi engine, ABI, pins, grants, discovery, Standing, Stop
src/connect/           connector-only parts: claims, Forge, the gateway
src/read/              reader-only parts: matching, the file host, the model, comparing models
src/repo/compare/read/ the generic views, made from today's src/repo/compare/unity/
src/settings/          readers folder per OS
src/hosting/           Settings gains a Readers page beside Connectors
```

The engine moves from `src/connect/` to `src/plugin/` once, before readers exist, with no change of behaviour.

## 2. The manifest

```json
{
  "id": "unity",
  "name": "Unity",
  "version": "1.0.0",
  "protocol": 1,
  "module": "unity.wasm",
  "kinds": [
    { "id": "prefab", "label": "Prefab", "extensions": [".prefab"], "sniff": ["%YAML", "!u!"], "views": ["inspector", "layout", "text"] },
    { "id": "scene", "label": "Scene", "extensions": [".unity"], "sniff": ["%YAML", "!u!"], "views": ["inspector", "layout", "text"] },
    { "id": "clip", "label": "Animation clip", "extensions": [".anim"], "sniff": ["%YAML", "!u!"], "views": ["curves", "text"] },
    { "id": "controller", "label": "Animator controller", "extensions": [".controller"], "sniff": ["%YAML", "!u!"], "views": ["graph", "text"] }
  ],
  "side_files": [{ "pattern": "{path}.meta" }],
  "options": [
    { "id": "noise", "label": "Hide noise", "detail": "Hide fields Unity rewrites on every save", "default": true, "hides": true },
    { "id": "ids", "label": "File ids", "detail": "Show each object's fileID", "default": false, "hides": false }
  ],
  "grant": { "read": "repository", "jobs": ["compare", "side_files", "fixes", "merge"] }
}
```

- **`sniff`:** every string must appear in the first 4 KB. Without it, the extension decides.
- **`views`:** the views offered for that kind, first one shown by default; `text` is always available even when left out.
- **`grant.read`:** `"file"` (only the file shown) or `"repository"` (any file at the same commit). Unity needs `"repository"` for GUID lookups and sprites.
- **`grant.jobs`:** the calls SterneGit may make. An update adding a job or widening `read` waits for approval.
- **`options`:** shown in the comparison's options menu under the reader's name, after SterneGit's own Show unchanged, and saved per reader in settings. With `"hides": true`, objects and fields tagged with the option's id are hidden while it is on; with `"hides": false`, they are shown only while it is on. An option with `"rereads": true` is passed to `read` instead, and switching it reads both sides again.

## 3. Calls

Each call starts a fresh instance, as for connectors.

| Call | Input | Output |
| --- | --- | --- |
| `hello` | protocol version | name, version, kinds |
| `read` | kind, path, which side, the values of `rereads` options | `Model` |
| `side_files` | path, both sides' text of the file and its side files | warnings with optional fixes |
| `fixes` | path, the change the user picked | `Fix` |
| `merge` | kind, path, base, ours, theirs | merged text and the objects left in conflict |

`read` is called once per side, both sides in parallel. Comparing happens in SterneGit, so every reader gets the same pairing, filters and change counts.

## 4. File access

| Import from `sternegit:1` | Meaning |
| --- | --- |
| `read_file(path, len) -> i32` | The file's bytes; its length for `take`, or -1 if absent from both trees. Refused with `grant.read = "file"` except for the file itself and its side files. |
| `search(query, len) -> i32` | `{"regex", "glob"}`: `git grep -I -o -E` over the reader's trees, answered as `[{path, text}]` for `take`. |
| `list_files(pattern, len) -> i32` | Paths matching a glob, as JSON. |
| `take`, `finish`, `log`, `now` | As connectors. `send` and `wait` are refused for readers. |

- **Trees:** each call has a commit and the working tree. A commit file looks in that commit first (`git cat-file --batch`, kept open per tab), then in the working tree for what the commit lacks, so an old commit shows its own names and an asset added later still resolves. A working file looks in the working tree only. `search` and `list_files` merge both trees the same way, the commit winning for a path in both.
- **Search:** what Unity's `meta_files_owning` does today (`^guid: (a|b|c)` over `*.meta`, in chunks), now asked by the reader. SterneGit holds no Unity knowledge.
- **What a call used:** SterneGit records every working-tree path read and every search answered from the working tree, for the cache (section 5).
- **File cache:** SterneGit keeps the files a reader read per commit for the tab, so the second side and the next file reuse them.
- **Pictures:** a reader never decodes images. It names a picture file and a rectangle; SterneGit decodes and crops it with its own decoders.

## 5. The model

```rust
pub struct Model {
    pub title: Option<String>,
    pub version: Option<String>,
    pub objects: Vec<Object>,
    pub graph: Option<Graph>,
    pub curves: Option<Curves>,
    pub layout: Option<Layout>,
    pub scopes: Vec<Scope>,
}

pub struct Object {
    pub id: String,
    pub parent: Option<String>,
    pub kind: String,
    pub name: String,
    pub icon: Option<Icon>,
    pub fields: Vec<Field>,
    pub tags: Vec<String>,
}

pub struct Field { pub key: String, pub label: String, pub value: Value, pub tags: Vec<String> }

pub enum Value {
    Text(String), Number(f64), Bool(bool), Flags(Vec<String>),
    Vector(Vec<f64>), Color([f32; 4]), Rotation { degrees: [f64; 3] },
    Reference { target: String, label: String, path: Option<String> },
    List { items: Vec<Item> }, Group(Vec<Field>), Long { digest: u64, preview: String },
}

pub struct Item { pub id: Option<String>, pub value: Value }

pub struct Scope { pub lines: (u32, u32), pub label: String, pub object: Option<String> }
```

- **Ids:** pairing is by `Object.id` and `Item.id`, never by position, which keeps today's list-entry pairing (b70476a).
- **Tags:** option ids from the manifest. Unity tags its noise fields `noise` and its fileID fields `ids`; SterneGit filters by them, so switching an option never calls the reader.
- **Cache:** a model is kept in memory per window under its blob id, the reader's version, the `rereads` option values and the commit, within a size cap, oldest dropped first. An entry that used the working tree also keeps the paths and searches it used, and the file watcher drops it when one of those changes. Reopening a file, or another commit holding the same blob, is instant. Nothing is saved to disk.
- **Sameness:** SterneGit compares values with the rules Unity uses now (numbers within display precision, rotations by degrees, long values by digest), so they move out of `src/unity/value.rs` into `src/read/`.
- **`Graph`:** nodes with ports, edges, parameters and the users of each parameter; today's `GraphNode`, `GraphEdge` and `GraphParameter` without their layout maths.
- **`Curves`:** tracks with keys and tangents; today's `Clip` and `Track`. Sampling (`hermite`, `value_at`) stays in SterneGit.
- **`Layout`:** shapes, sprites as `{ picture, rect, pivot }`, markers and the object each one belongs to; today's `Layout` without the sprite pixels.
- **`scopes`:** line ranges that label the text view, today's `hints.rs` output.
- **Size of a model:** a 14k-row prefab is a few MB of JSON. Readers get a 512 MB memory limit and a CPU budget in slices of 20 M fuel, 1,000 slices per `read`; the connectors' 64 MB and 50 slices are far too small.

## 6. Side files and fixes

- After both sides are read, SterneGit calls `side_files` with the file and the side files the manifest names.
- A warning has a level, a sentence and up to three fixes: Unity's changed GUID offers "Keep the old GUID" and "Point users to the new GUID".
- Picking a fix calls `fixes`, which returns:

```rust
pub struct Fix { pub title: String, pub edits: Vec<Edit> }
pub struct Edit { pub path: String, pub was: Digest, pub text: String }
```

- SterneGit shows the edits as a comparison of each file before and after, with Apply and Cancel. Apply refuses a file whose content no longer matches `was`.
- A fix can only touch files inside the repository; paths are checked, not trusted.

## 7. Merge by object

- In the conflict editor, a file a reader claims with the `merge` job gets a "Merge by object" button.
- `merge` merges every change that doesn't conflict on its own and returns the merged text plus, for each conflicted object, its conflicted fields with both values.
- SterneGit shows those objects in the inspector view. Each conflicted field has Ours / Theirs; each object has a button that takes all its fields from one side. The choices go back to `merge`, which writes the final text.
- Writing the result back is the reader's job, so it keeps the format's own spelling; SterneGit only writes the text after the user accepts it.
- The last milestone: the conflict editor needs inspector panes with choices, drawn in the mockup first.

## 8. Screens

All mockup first.

- **Settings → Readers:** like the Connectors page: name, version, standing, kinds, what it may read, its jobs, Approve, Remove.
- **Approval:** "Unity wants to read any file of the repository being shown, to name scripts and draw sprites. It can't write files, reach the network or run programs."
- **Comparison toolbar:** the views the kind offers, as Unity's modes are today, plus "Reading with Unity…" while a side is being read and the text diff is showing.
- **Options menu:** SterneGit's Show unchanged, then a section per reader with its declared options, replacing today's "Every Unity comparison" section.
- **Text diff labels:** the reader's scopes label the plain patch view wherever it appears, as Unity's annotate does today.
- **File lists:** the changes list and commit files show the kind ("Prefab", "Scene") as a muted tag, chosen by extension alone; the first-bytes check waits until the file is opened.
- **Reader stopped:** the text view with a bar: "Unity couldn't read this file: <stop>. Showing text." and a Log button.
- **Fix preview:** a dialog listing the edited files, each opening its comparison, with Apply and Cancel.
- **Two readers for one file:** asked once per extension, changeable in Settings → Readers.

## 9. Speed

wasmi runs plugin code 15–20× slower than native. For connectors that never shows, because web requests dominate. For readers it is all CPU, so it has to be measured before Unity moves for good.

- Milestone 2 ports prefab reading only and times, on the largest prefab available, native read vs wasm read, both sides, cold and warm file cache.
- What the user sees meanwhile: text at once, the reader's view swapped in when ready, cancelled when the file changes.
- If it is too slow, in order: profile the reader (YAML parsing, JSON output size), cache models per blob id across tabs, then reopen the engine question with the numbers.

## 10. Size

- Unity's reader leaves the exe: about 1.37 MB comes out, measured at 447f1f5.
- What comes in: the generic model and comparison in `src/read/`, the file host, the Readers page, and `src/repo/compare/read/` replacing `src/repo/compare/unity/` at about its size.
- Expected: a smaller exe. Measured per milestone with the size-baseline recipe, codegen-units = 1.

## 11. Milestones

1. **Shared engine:** move the engine, pins, grants and discovery from `src/connect/` to `src/plugin/`. Connectors behave the same; the exe stays within noise.
2. **Prefab reader, measured:** `readers/protocol`, `readers/sdk`, the file host, `read` for prefabs only, the inspector view made generic. Time it (section 9). Decide with the numbers whether Unity moves.
3. **All of Unity:** scenes, clips, controllers, script graphs, layouts with sprites, scopes. `src/unity/` deleted. Every Unity test moves with it or into `src/read/`.
4. **Screens:** Settings → Readers, approval, reader stopped, two readers for one file.
5. **Side files and fixes:** `.meta` pairing, GUID warning, fix preview. Replaces `Repository::rewrite_guid`.
6. **Author kit and a second reader:** `docs/readers/authoring.md`, `readers/template/`, and a Godot reader for `.tscn` and `.tres` (sniffed by `[gd_scene` and `[gd_resource`) that must need no SterneGit change.
7. **Merge by object:** conflict editor inspector panes, Unity's `merge`.

## 12. Open items

- **Disk cache:** only if measurements show the first open after a launch is too slow.
- **Readers in a repository:** not in v1; revisit if teams with their own formats ask.
- **Per-commit counts in file history:** left out, since it needs a read per commit.
- **Pictures readers can't name:** formats whose images are embedded, not separate files.
