# SterneGit

A lightweight Git client for Linux, macOS and Windows, written in
Rust with [GPUI](https://www.gpui.rs/) (through [gpui-kit](https://github.com/longbridge/gpui-kit))
and [gitoxide](https://github.com/GitoxideLabs/gitoxide).

This is an early prototype: it browses history and stages uncommitted changes. It
doesn't commit or push yet.

## Run

```sh
cargo run --release -- path/to/repository
```

SterneGit brings back the tabs that were open last time, and a path given on the
command line opens in one more tab. With no tabs to bring back it offers **Open
repository…**. The open tabs are saved after every change, in
`%APPDATA%\SterneGit\tabs` on Windows, `~/Library/Application Support/SterneGit/tabs`
on macOS and `$XDG_STATE_HOME/sternegit/tabs` (or `~/.local/state/sternegit/tabs`) on
Linux. Git 2.26 or newer must be on `PATH` for diffs and staging.

To time how long reading a repository takes, without opening a window:

```sh
cargo run -- --check path/to/repository
```

On Windows, `scripts/build-and-relaunch.ps1` builds a release and copies it to
`build/sternegit.exe`, which Git ignores, replacing the previous copy. If that copy
is running, it's closed and started again on the new build with `--aside`, which
opens the window on the rightmost screen without taking focus. Nothing happens when
the build hasn't changed.

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

Click a branch, remote branch or tag in the sidebar to jump to its commit. In
the graph, a branch on the same commit as its upstream shows as one badge with a
branch and a cloud.

Stashes have their own section in the sidebar, newest first, and a row each in
the graph: a hollow square joined to the commit the stash was made on, with a
`stash@{n}` badge. Select one in either place to see what it changes.
Right-click one to **Apply** it (the stash is kept), **Pop** it, or **Delete** it
after a confirmation.

When the work tree has changes, an **Uncommitted changes** row sits above the
graph. Select it (or press ↑ from the newest commit) to list the unstaged and
staged files. Click a file to compare it with its previous version in the middle
pane, side by side (**Split**) or in one column (**Unified**). The **+** / **−**
next to each file, **Stage all** / **Unstage all**, and the button above the
comparison move files between the two lists. Edits made outside SterneGit show up
within a few seconds.

Right-click a file in a commit, or a changed file, and choose **File history** to
list every commit that changed it in place of the sidebar, newest first and across
renames. Select one to compare the file as that commit changed it; ↑ / ↓ move
through them, and Esc brings the sidebar and the graph back.

Each repository opens in its own tab under the top bar; **+** opens another one.
Every action is also in the File, Edit, View and About menus of the top bar
(macOS's own menu bar on a Mac).

## How it's built

- `src/git/` reads and changes the repository, and knows nothing about the UI.
  Refs and history come from gix inside the process, so a refresh starts no
  processes however large the history is; the newest 10,000 commits are read.
  A commit's patch comes from one `git show`, a file's history from
  `git log --follow`, uncommitted changes from
  `git status`, each file's comparison from `git diff`, staging from
  `git add` / `git reset`, and fetching from `git fetch`, so patches match what
  Git prints in a terminal and fetches use the user's own credentials. The
  background reads pass `--no-optional-locks`, so they never hold the index lock
  while you commit in a terminal. Text is read as UTF-8, and a line that isn't
  valid UTF-8 is read as Windows-1252 (Latin-1), so files and commit messages
  saved in older Western encodings show é, â, ç and € correctly. File names
  reach Git exactly as Git wrote them, so a file stages and compares whatever
  encoding its name is in. `graph.rs`
  assigns each commit a lane and works out the line pieces that cross each row.
- `src/repo/` is one repository's tab. `repository.rs` holds what was read and
  runs Git in the background, `navigation.rs` decides what stays selected when
  the history or the changes are read again, and `mod.rs` lays the panels out and
  connects them. Each panel has its own file: the ref list, the commit list that
  paints the graph, the commit details, the uncommitted changes, the file
  comparison and the status bar.
- `src/ui/` holds the pieces every panel is drawn with: rows and section headers,
  badges, the graph cell, notices, and `Load` / `Job`, which tie a background
  task to the state it fills.
- `src/app/` is the window: the top bar with its menus and a tab per open
  repository. `src/actions.rs` lists every action and its shortcut context.
- The code carries no comments: names, types and tests say what it does, and
  `tests/no_comments.rs` keeps it that way. `cargo test` runs the unit tests.
- `docs/mockup/` is a browser mockup of every screen and of each reusable
  component, kept up to date with the UI. Open `docs/mockup/index.html`; its
  README says how to update it.

Release builds use thin LTO and strip symbols to keep the binary small.
