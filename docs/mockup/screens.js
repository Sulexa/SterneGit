const Screens = (() => {
  const { history, stashed, collapsed, changes, partlyStaged, draft, commitDetails, commitFileDiff, fileHistory, fileDiff, partlyStagedDiff, selectedLines, tabs } = Sample;
  const { starfall, assets, slimeDiff, introDiff, themeDiff, modelDiff } = Sample;
  const selected = 2;
  const position = (row) => `Commit ${thousands(row + 1)} of ${thousands(history.total)}`;

  const topBar = (extra = {}) => C.TopBar({ repository: history.name, head: history.head, ...extra });
  const tabStrip = (overrides = {}) => C.TabStrip({ tabs: tabs.map((tab, ix) => ({ ...tab, ...(overrides[ix] || {}) })), active: 0 });

  const repo = ({ left, middle, right, selectedRow = null, wipSelected = false, status = {}, toolbar }) =>
    C.RepoView({
      toolbar,
      left: left || C.RefsList({ refs: history.refs, selectedRow, collapsed }),
      middle: middle || C.HistoryList({ history, selectedRow, changes, wipSelected }),
      right,
      status: C.RepoStatusBar({
        summary: historySummary(history),
        position: selectedRow === null ? '' : position(selectedRow),
        ...status,
      }),
    });

  const commitSelected = (status, panel = {}, toolbar) =>
    repo({
      selectedRow: selected,
      right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails, ...panel }),
      status,
      toolbar,
    });

  const stashSelected = () =>
    repo({
      left: C.RefsList({ refs: stashed.history.refs, stashes: stashed.history.stashes, selectedRow: 3, collapsed }),
      middle: C.HistoryList({ history: stashed.history, selectedRow: 3, changes }),
      right: C.CommitPanel({ commit: stashed.history.commits[3], details: stashed.details }),
      status: { summary: historySummary(stashed.history), position: 'Stash 2 of 2' },
      toolbar: C.RepoToolbar({ unavailable: [] }),
    });

  const commitFileComparison = () =>
    repo({
      selectedRow: selected,
      middle: C.FileDiffPanel({ diff: commitFileDiff, mode: 'split' }),
      right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails, open: commitFileDiff.change.path }),
    });

  const fileHistoryView = () =>
    repo({
      selectedRow: selected,
      left: C.FileHistoryList({ history: fileHistory, selected: 2 }),
      middle: C.FileDiffPanel({ diff: commitFileDiff, mode: 'split' }),
      right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails, open: fileHistory.path }),
      status: { position: Texts.historyPosition(3, fileHistory.entries.length, splitPath(fileHistory.path)[0]) },
    });

  const openFile = { side: fileDiff.side, path: fileDiff.change.path };

  const fileComparison = (mode, extra = {}) =>
    repo({
      wipSelected: true,
      middle: C.FileDiffPanel({ diff: extra.diff || fileDiff, mode, selection: extra.selection, pointed: extra.pointed, menuOpen: extra.menuOpen, hot: extra.hot }),
      right: C.ChangesList({ changes: extra.changes || changes, open: openFile, view: extra.view }),
    });

  const firstUnstaged = changes.unstaged[0];
  const ref = (name) => history.refs.find((label) => label.name === name);

  const gameTabs = [...tabs, { name: starfall.name, state: 'ready' }];
  const changedFiles = [...assets.staged, ...assets.unstaged];

  const assetComparison = (diff, { mode = 'split', expanded = false, view } = {}) =>
    C.FastGit({
      topBar: C.TopBar({ repository: starfall.name, head: starfall.head }),
      tabStrip: C.TabStrip({ tabs: gameTabs, active: gameTabs.length - 1 }),
      body: C.RepoView({
        expanded,
        left: C.RefsList({ refs: starfall.refs, selectedRow: null }),
        middle: C.FileDiffPanel({ diff, mode, expanded, view, filePosition: [changedFiles.indexOf(diff.change) + 1, changedFiles.length] }),
        right: C.ChangesList({ changes: assets, open: { side: diff.side, path: diff.change.path } }),
        status: C.RepoStatusBar({ summary: historySummary(starfall) }),
      }),
    });

  return [
    {
      id: 'welcome',
      title: 'Welcome',
      about: 'No repository is open: FastGit started outside a repository, or the last tab was closed. The tab strip and the status bar are hidden.',
      source: 'src/app/window.rs',
      render: () =>
        C.FastGit({
          topBar: C.TopBar(),
          body: C.Notice({
            title: Texts.welcomeTitle,
            description: Texts.welcomeDescription,
            action: C.Button({ label: 'Open repository…', variant: 'primary', size: 'md' }),
          }),
        }),
    },
    {
      id: 'reading',
      title: 'Reading history',
      about: 'A repository was just opened. Its tab shows a spinner and the whole body waits for the first read of refs and history.',
      source: 'src/repo/mod.rs',
      render: () =>
        C.FastGit({
          topBar: C.TopBar({ repository: 'linux' }),
          tabStrip: C.TabStrip({ tabs, active: 2 }),
          body: C.RepoView({
            toolbar: C.RepoToolbar({ ready: false }),
            body: C.Loading({ message: Texts.readingHistory }),
            status: C.RepoStatusBar({ summary: Texts.readingHistory }),
          }),
        }),
    },
    {
      id: 'open-failed',
      title: "Couldn't open repository",
      about: 'Reading the repository failed. The tab shows a warning and the body explains why, with a Retry button.',
      source: 'src/repo/mod.rs',
      render: () =>
        C.FastGit({
          topBar: C.TopBar({ repository: 'old-site' }),
          tabStrip: C.TabStrip({ tabs: [...tabs.slice(0, 2), { name: 'old-site', state: 'failed' }], active: 2 }),
          body: C.RepoView({
            toolbar: C.RepoToolbar({ ready: false }),
            body: C.Notice({
              title: Texts.openFailedTitle,
              description: "could not open repository at 'D:\\Archive\\old-site': not a git repository",
              action: C.Button({ label: 'Retry', size: 'md' }),
            }),
            status: C.RepoStatusBar({}),
          }),
        }),
    },
    {
      id: 'history',
      title: 'History — commit selected',
      about: 'The main screen. Under the tabs, the header holds Pull, Push, Branch, Stash and Pop. Then refs on the left, split into folders at each slash, the commit graph in the middle, the selected commit and the files it changes on the right. The refs pointing at the selected commit are highlighted too. Click a section or a folder to fold it.',
      source: 'src/repo/mod.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitSelected() }),
    },
    {
      id: 'stashes',
      title: 'Stashes',
      about: 'Stashes get a section of their own in the left pane, between Remotes and Tags, newest first, each with its message as Git wrote it; it folds like the others and is left out when there is no stash. In the graph each stash is a row at the time it was made: a hollow square in the lane of the commit it was made on, joined to that commit, with a stash@{n} badge. Git also keeps a commit for the staged part of a stash and one for its untracked files; the graph leaves those out. Clicking a stash in the left pane or in the graph selects it, and the right pane shows what it changes in tracked files against the commit it was made on, as for any commit. The status bar counts the stashes, and says which one is selected; commit positions skip the stash rows.',
      source: 'src/repo/refs_list.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: stashSelected() }),
    },
    {
      id: 'stash-menu',
      title: 'Right-click on a stash',
      about: "A right-click on a stash, in the left pane or on its row in the graph, opens its menu where the pointer is, without selecting it. Apply brings its changes back into the work tree and keeps the stash; Pop brings them back and deletes the stash; both keep what was staged staged. When the changes conflict with the files, the conflicted files are left to resolve and the stash is kept; when they don't apply at all, nothing changes and the stash is kept; an error notification says which. Delete… asks first. Each item runs like the header's Pop: one Git write at a time, disabled while one runs, then a notification such as Popped stash@{1}. The header's Pop still pops the newest stash.",
      source: 'src/repo/stash_menu.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: stashSelected(),
          overlays: C.StashMenu({ stash: stashed.history.stashes[1], left: 120, top: 608, hot: Texts.popStash(stashed.history.stashes[1].name) }),
        }),
    },
    {
      id: 'delete-stash',
      title: 'Delete a stash',
      about: 'Delete… always asks first, because Git keeps no other copy of a stash. Delete then runs git stash drop on that stash, and the stashes after it move up by one; Cancel or Esc keeps it.',
      source: 'src/repo/stash_menu.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: stashSelected(),
          overlays: C.DeleteStashDialog({ stash: stashed.history.stashes[1] }),
        }),
    },
    {
      id: 'commit-file',
      title: 'Commit file comparison',
      about: "A file was clicked in the selected commit's file list. The middle pane swaps the graph for a comparison of that file with the commit's first parent; the file stays highlighted on the right, another click opens another file, and Esc or × goes back to the graph.",
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitFileComparison() }),
    },
    {
      id: 'history-tree',
      title: 'Commit files as a tree',
      about: "The two buttons at the right end of the file counts show the files as a list or as a tree. The tree splits each path at its slashes: folders first, then files, both by name, and a folder whose only child is a folder shares its row with it, as in src/repo. Each file row keeps its status letter and line counts. Click a folder to fold it. The choice applies to every file list in every tab, the changes list included.",
      source: 'src/repo/commit_files.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitSelected(undefined, { view: 'tree' }) }),
    },
    {
      id: 'commit-file-menu',
      title: "Right-click on a commit's file",
      about: "A right-click on a file in the selected commit's list opens its menu where the pointer is, without opening the file: File history, then Open file, Show in folder and Copy path, as for a changed file. A file the commit deletes has nothing to open or show, but still has a history. The same menu opens on the files of a commit in a file's history.",
      source: 'src/repo/commit_files.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(undefined, { pointed: { path: fileHistory.path, left: 150, top: 16, hot: Texts.fileHistory } }),
        }),
    },
    {
      id: 'file-history',
      title: 'File history',
      about: "File history puts every commit that changed the file in the left pane, in place of the refs, newest first, and follows the file through renames as git log --follow does. Each row says what the commit did to the file (added, modified, renamed, deleted) and its subject, then its author and day, or the old path for a rename, and the lines it added and removed in that file. The middle pane compares the file as the selected commit changed it with its first parent, as for any file of a commit. Opened from a commit, that commit is selected; opened from the changes list, the newest one. The right pane shows the selected commit with the file highlighted; a click on another of its files compares that file and leaves the history. Up and Down move through the file's commits; Esc or × in the file bar brings back the refs and the graph, with the same commit selected. While Git reads, the header spins and the list waits; opened from a commit, that commit is compared at once. A file no commit changed says so, and the history is read again when the branches move.",
      source: 'src/repo/file_history.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileHistoryView() }),
    },
    {
      id: 'nothing-selected',
      title: 'History — nothing selected',
      about: 'History is read but nothing is selected yet, so the right pane asks for a selection.',
      source: 'src/repo/mod.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({ right: C.Hint({ message: Texts.selectCommit }) }),
        }),
    },
    {
      id: 'uncommitted',
      title: 'Uncommitted changes',
      about: 'The Uncommitted changes row is selected. The right pane starts with the commit message, its description and the Commit button, then the number of changed files with the list and tree buttons, then the staged files, then the unstaged files at the bottom. Each file has a button that moves it to the other list. Commit stays disabled until there is a message and at least one staged file.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({ wipSelected: true, right: C.ChangesList({ changes }) }),
        }),
    },
    {
      id: 'commit-ready',
      title: 'Uncommitted changes — ready to commit',
      about: 'A message and a description are typed and two files are staged, so Commit is enabled and names how many files it commits. Ctrl+Enter commits from either field.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({ wipSelected: true, right: C.ChangesList({ changes, draft: { ...draft, focus: 'description' } }) }),
        }),
    },
    {
      id: 'changes-tree',
      title: 'Uncommitted changes as a tree',
      about: 'The same switch sits between the commit form and the file lists, beside the number of changed files. In a tree, the staged and the unstaged files each get their own folders, and a folder row ends with a + or − that moves every file under it to the other list. Here patch.rs is open in the middle pane and highlighted in its folder.',
      source: 'src/repo/changes_list.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split', { view: 'tree' }) }),
    },
    {
      id: 'file-menu',
      title: 'Right-click on a changed file',
      about: 'A right-click on a file in the changes list opens its menu where the pointer is: move it to the other list, discard its unstaged changes (Delete file… when Git does not track it), see its history, open it with its default app, show it in the file manager or copy its full path. A staged or conflicted file has no Discard; a file that is new or untracked has no history yet; a deleted one has nothing to open or show.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({
            wipSelected: true,
            right: C.ChangesList({ changes, pointed: { side: 'unstaged', path: firstUnstaged.path, left: 168, top: 16, hot: 'Discard changes…' } }),
          }),
        }),
    },
    {
      id: 'discard-file',
      title: 'Discard a file',
      about: 'Discard changes… asks first, because Git keeps no copy of unstaged changes. Discard puts the file back as it is staged, or as it was committed when nothing of it is staged; Cancel or Esc keeps it.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({ wipSelected: true, right: C.ChangesList({ changes }) }),
          overlays: C.DiscardDialog({ change: firstUnstaged }),
        }),
    },
    {
      id: 'file-split',
      title: 'File comparison — Split',
      about: 'A file was clicked in the changes list. The middle pane swaps the graph for a side-by-side comparison; Esc or × goes back to the graph. The file bar names the file and holds what acts on all of it: Stage, the button that gives the comparison the whole window, and ×. The view bar under it picks Split or Unified, counts the changes and steps through them with ↑ and ↓ (Alt+Up, Alt+Down), and opens the view options. Changed words are highlighted inside changed lines. FastGit reads the whole file, so each hunk header is followed by the unchanged lines above it, folded: a click shows them all, or 20 at a time when there are more. The strip at the right edge marks where the changes sit and what part of the comparison is in view; a click on a mark goes there. Each hunk header ends with Discard hunk and Stage hunk, so one part of the file can be staged or thrown away without the rest.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split') }),
    },
    {
      id: 'file-unified',
      title: 'File comparison — Unified',
      about: 'The same comparison in one column, after clicking Unified. The hunk buttons are the same.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('unified') }),
    },
    {
      id: 'file-lines',
      title: 'File comparison — lines selected',
      about: 'Clicking a line number selects that line; dragging over line numbers, or Shift-clicking a second one, selects more. The header of the hunk that holds them then reads Discard 3 lines and Stage 3 lines, which act on the selected changed lines only; the other hunks keep their hunk buttons.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split', { selection: selectedLines }) }),
    },
    {
      id: 'line-menu',
      title: 'Right-click on a change',
      about: 'The line under the pointer is highlighted, and a right-click on it opens a menu where the pointer is. On a changed line it offers its hunk, then the line alone; on an unchanged line or a hunk header only the hunk; inside a selection, the selected lines first. Discarding a hunk or lines happens at once, like staging them, without asking.',
      source: 'src/repo/file_diff_panel.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: fileComparison('split', { pointed: { hunk: 1, line: 2, left: 132, top: 12, hot: 'Stage hunk' } }),
        }),
    },
    {
      id: 'partly-staged',
      title: 'Part of a file staged',
      about: 'After Stage hunk on the first hunk, patch.rs is in both lists: the staged copy holds that hunk, the open unstaged comparison holds the two hunks left. Unstage hunk on the staged side brings it back.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: fileComparison('split', { diff: partlyStagedDiff, changes: partlyStaged }),
        }),
    },
    {
      id: 'view-options',
      title: 'File comparison — view options',
      about: 'The ⋯ at the right end of the view bar opens the options of every text comparison, in every tab: changed words highlighted, long lines wrapped, the whole file shown instead of the changed parts, whitespace changes hidden. While whitespace is hidden the hunk buttons and the line menu go away, because Git could not apply those parts to the file.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split', { menuOpen: true, hot: 'whitespace' }) }),
    },
    {
      id: 'picture-split',
      title: 'Compare pictures — Split',
      about: 'A picture opens as pictures, not as “binary”: PNG, JPEG, GIF, WebP, BMP, TGA, ICO and QOI. Other formats show their details and the Open buttons. Split puts the version before the change on the left and the file on disk on the right, at the same zoom, on a checkerboard where they are transparent; zooming or dragging one moves the other. The view bar picks the way to compare, steps through the changed areas (each step zooms both sides onto one), zooms (Fit, − and +, Ctrl+wheel) and picks what shows behind transparent pixels. Past 400 % pixels stay square, which suits pixel art. Under the pictures, the details put both versions in one table, what changed in yellow. A new or deleted picture shows its one version and no modes.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(slimeDiff),
    },
    {
      id: 'picture-swipe',
      title: 'Compare pictures — Swipe',
      about: 'Swipe draws one picture: the version before the change left of the line, the file on disk right of it. Drag the handle, or click anywhere on the picture, to sweep the line across a change.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(slimeDiff, { mode: 'swipe', view: { at: 0.47 } }),
    },
    {
      id: 'picture-onion',
      title: 'Compare pictures — Onion skin',
      about: 'Onion skin lays the file on disk over the version before it. The slider sets how much of each shows; Space jumps between its two ends, so a change blinks in place.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(slimeDiff, { mode: 'onion', view: { mix: 0.5 } }),
    },
    {
      id: 'picture-difference',
      title: 'Compare pictures — Difference',
      about: 'Difference greys the picture out and paints only the pixels that changed, with the colors of the file letters: green where a pixel appeared, yellow where its color changed, red where it went transparent. Each changed area gets a numbered frame, and the view bar steps through them. Hovering a pixel reads its color in both versions.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(slimeDiff, { mode: 'difference', view: { probe: { x: 11, y: 17 } } }),
    },
    {
      id: 'video-whole-window',
      title: 'Compare videos — whole window',
      about: 'A video compares frame by frame. Here the comparison has the whole window: the button left of × in the file bar hides the refs and the file list until it is clicked again, and the file bar gains ‹ and › to move through the changed files (Alt+Left, Alt+Right). Both versions play in step from the same time, and the frame buttons step one frame. The timeline draws how long each version is and where their frames differ, numbered like the changes in the view bar. FastGit uses the ffmpeg and ffprobe found on PATH; without them a video shows its details and the Open buttons, with a note that says so. Hear picks whose sound plays. Split, Swipe, Onion skin and Difference work as for pictures. LFS says Git LFS stores the file: FastGit compares the videos, not the pointers.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(introDiff, { expanded: true }),
    },
    {
      id: 'music',
      title: 'Compare music and sounds',
      about: 'A sound shows both waveforms on one time scale, the version before the change on top. Yellow bands mark where they sound different, numbered like the changes in the view bar; the hatching is where the old version has already ended. One playhead drives both: Hear switches which version plays at once, without stopping, so the same bar can be heard one way and then the other (Tab switches too). Loop change repeats the current band. Difference shows only what changed. The details add loudness (EBU R128) and peak. FastGit reads WAV, AIFF, FLAC, Ogg Vorbis, MP3 and M4A (AAC).',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(themeDiff),
    },
    {
      id: 'other-file',
      title: 'Compare a file FastGit can’t draw',
      about: 'A file FastGit can’t draw, such as a 3D model, still says what changed: its type, its size and when it last changed. Open Before gives a copy of the old version to its default app, Open After the file on disk, and Compare in difftool hands both to the tool set in Git’s diff.tool; it only shows when one is set.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(modelDiff),
    },
    {
      id: 'pulling',
      title: 'Header — pulling',
      about: 'Pull was clicked. Its icon turns into a spinner and the other header buttons wait, like the stage and commit buttons, because FastGit runs one Git write at a time. Pull and Push need a checked-out branch, except that Pull in Fetch only mode works anywhere; Stash needs uncommitted changes; Pop needs a stash.',
      source: 'src/repo/toolbar.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitSelected(undefined, {}, C.RepoToolbar({ running: 'pull' })) }),
    },
    {
      id: 'pull-modes',
      title: 'Header — pull mode',
      about: "The caret beside Pull opens its modes. The checked one is what a click on Pull does; picking another makes it Pull's default for this repository, without pulling. FastGit saves it in the repository's Git config as fastgit.pullMode, which Git itself ignores. Until a mode is picked, Pull follows Git's own settings: Rebase when pull.rebase is true, Fast-forward only when pull.ff is only, Fast-forward if possible otherwise.",
      source: 'src/repo/toolbar.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(undefined, {}, C.RepoToolbar({ menuOpen: true })),
          overlays: C.PullModeMenu({ mode: 'merge', left: 607, top: 112, hot: 'rebase' }),
        }),
    },
    {
      id: 'pushed',
      title: 'Header — pushed',
      about: 'A pull, a push, a stash or a pop that worked says so in a notification. The first push of a branch that has no upstream sends it to origin, or to the only remote, and makes that copy its upstream.',
      source: 'src/repo/toolbar.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'success', message: 'Pushed main to origin/main' })}</div>`,
        }),
    },
    {
      id: 'push-failed',
      title: 'Header — push rejected',
      about: 'When Git refuses, the notification says why and stays until it is closed. A push the remote rejects because it has commits main does not have yet asks for a pull first.',
      source: 'src/repo/toolbar.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'error', title: 'Push failed', message: "origin/main has commits that main doesn't have yet. Pull them, then push again." })}</div>`,
        }),
    },
    {
      id: 'branch-dialog',
      title: 'Header — create a branch',
      about: 'Branch asks for a name, then creates the branch where HEAD is and checks it out. Create branch stays disabled while the name is empty, taken or refused by Git; Enter creates, Esc cancels.',
      source: 'src/repo/branch_dialog.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.BranchDialog({ head: `main (${history.commits[0].id})`, value: 'feature/toolbar' }),
        }),
    },
    {
      id: 'branch-menu',
      title: 'Right-click on a branch',
      about: "A right-click on a branch in the left list opens its menu where the pointer is, without selecting its commit. Here feature/blame is not checked out, so it offers Check out, Pull, Push, Merge it into main or Rebase main onto it, Create branch here… and Delete…. Pull is fast-forward only, because Git can't merge into or rebase a branch that isn't checked out. The checked-out branch offers Pull with the header's pull mode, Push and Create branch here…. A remote branch offers Check out, Merge, Rebase, Create branch here… and Delete from its remote. With a detached HEAD there is no Merge or Rebase. Tags and folders have no menu. Each action runs like the header buttons: one Git write at a time, then a notification; Pull and Push spin the header's button while they run.",
      source: 'src/repo/branch_menu.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.BranchMenu({ label: ref('feature/blame'), current: history.head, left: 92, top: 188, hot: Texts.merge('feature/blame', history.head) }),
        }),
    },
    {
      id: 'branch-from-ref',
      title: 'Create a branch from another ref',
      about: 'Create branch here… opens the same dialog as the header’s Branch, but the new branch starts at the ref that was right-clicked. It is checked out right away; uncommitted changes come along unless they clash with it, and then Git refuses and says why.',
      source: 'src/repo/branch_dialog.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.BranchDialog({ from: `feature/blame (${history.commits[ref('feature/blame').row].id})`, value: 'fix/blame-tabs' }),
        }),
    },
    {
      id: 'delete-branch',
      title: 'Delete a branch',
      about: 'Delete… always asks first. The question counts the commits that no other branch or tag holds and says they are lost; when there are none it says nothing is lost. Delete then removes the branch, merged or not. A remote branch is deleted on its remote, after a question that says everyone who uses it loses it.',
      source: 'src/repo/branch_menu.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.DeleteBranchDialog({ label: ref('feature/blame'), only: 1 }),
        }),
    },
    {
      id: 'merged',
      title: 'Branch merged',
      about: 'Check out, Merge, Rebase and Delete say what they did in a notification. A merge that stops on conflicts explains what to do in an error notification instead.',
      source: 'src/repo/operation.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'success', message: 'Merged feature/blame into main' })}</div>`,
        }),
    },
    {
      id: 'checked-out-stash-kept',
      title: 'Checked out, changes left in the stash',
      about: "When uncommitted changes are in the way of a check-out, FastGit stashes them, switches and pops the stash, so what was staged stays staged. When they conflict with the branch or don't apply on it, the check-out still happens, the stash keeps them and a warning says so until it is closed.",
      source: 'src/git/branch.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'warning', title: 'Switched to main', message: 'Your uncommitted changes conflict with main: resolve the conflicted files. The stash keeps a copy of them.' })}</div>`,
        }),
    },
    {
      id: 'menu',
      title: 'View menu open',
      about: 'On Windows and Linux FastGit draws its own menu bar in the top bar; on macOS the same menus go to the system menu bar.',
      source: 'src/app/mod.rs',
      render: () =>
        C.FastGit({
          topBar: topBar({ openMenu: 'View' }),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.Menu({ items: Menus.View, left: 114, top: 32, hot: 'Fetch All Remotes' }),
        }),
    },
    {
      id: 'fetching',
      title: 'Copy hash and fetch',
      about: 'Copy hash shows a notification. A fetch runs in the background every five minutes and when the window comes back; the status bar shows it.',
      source: 'src/repo/status_bar.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected({ fetching: true }),
          overlays: `<div class="toasts">${C.Notification({ message: 'Commit hash copied' })}</div>`,
        }),
    },
    {
      id: 'about',
      title: 'About dialog',
      about: 'About → About FastGit opens an alert dialog with the version.',
      source: 'src/app/window.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.AlertDialog({ title: 'FastGit 0.1.0', description: Texts.aboutDescription }),
        }),
    },
  ];
})();
