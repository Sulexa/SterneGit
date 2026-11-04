# FastGit

A lightweight Git client for Linux, macOS and Windows, written in
Rust with [GPUI](https://www.gpui.rs/) (through [gpui-kit](https://github.com/longbridge/gpui-kit))
and [gitoxide](https://github.com/GitoxideLabs/gitoxide).

This is an early prototype: it browses history and stages uncommitted changes. It
doesn't commit or push yet.

## Run

```sh
cargo run --release -- path/to/repository
```

Without a path it opens the current directory if that's a repository, and otherwise
offers **Open repository…**. Git must be on `PATH` for diffs.

To time how long reading a repository takes, without opening a window:

```sh
cargo run -- --check path/to/repository
```

On Linux, building needs the same system packages Zed needs (Wayland or X11,
xkbcommon, Vulkan, fontconfig).

## Using it

| Key | Action |
| --- | --- |
| ↑ / ↓ or k / j | Select the newer / older commit |
| Ctrl+O (Cmd+O) | Open a repository |
| Ctrl+R (Cmd+R) | Refresh, keeping the selected commit |
| Ctrl+Shift+C (Cmd+Shift+C) | Copy the selected commit's full hash |
| Esc | Close the open file comparison |
| Ctrl+W (Cmd+W) | Close the current tab |
| Ctrl+Tab / Ctrl+Shift+Tab | Switch to the next / previous tab |

Click a branch, remote branch or tag in the sidebar to jump to its commit.

When the work tree has changes, an **Uncommitted changes** row sits above the
graph. Select it (or press ↑ from the newest commit) to list the unstaged and
staged files. Click a file to compare it with its previous version in the middle
pane, side by side (**Split**) or in one column (**Unified**). The **+** / **−**
next to each file, **Stage all** / **Unstage all**, and the button above the
comparison move files between the two lists. Edits made outside FastGit show up
within a few seconds.

Each repository opens in its own tab under the top bar; **+** opens another one.
Every action is also in the File, Edit, View and About menus of the top bar
(macOS's own menu bar on a Mac).

## How it's built

- `src/repository.rs` reads refs and history with gix inside the process, so a
  refresh starts no processes however large the history is. The newest 10,000
  commits are read. Selecting a commit runs one `git show` for its patch, so rename
  detection and the patch itself match what Git prints in a terminal. Uncommitted
  changes come from `git status`, each file's comparison from `git diff`, and
  staging from `git add` / `git reset`.
- `src/graph.rs` assigns each commit a lane and works out the line pieces that
  cross each row. It has unit tests.
- `src/app.rs` is the window: the top bar with its menus, a tab per open
  repository, and the keyboard shortcuts.
- `src/repo_view.rs` is one repository's view: a virtualized ref list, a
  virtualized commit list that paints each row's piece of the graph, and
  gpui-kit's `Diff` for the selected commit.

Release builds use thin LTO and strip symbols to keep the binary small.
