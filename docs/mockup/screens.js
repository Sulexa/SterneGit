const Screens = (() => {
  const { history, collapsed, changes, partlyStaged, draft, commitDetails, commitFileDiff, fileDiff, partlyStagedDiff, selectedLines, tabs } = Sample;
  const selected = 2;
  const position = (row) => `Commit ${thousands(row + 1)} of ${thousands(history.total)}`;

  const topBar = (extra = {}) => C.TopBar({ repository: history.name, head: history.head, ...extra });
  const tabStrip = (overrides = {}) => C.TabStrip({ tabs: tabs.map((tab, ix) => ({ ...tab, ...(overrides[ix] || {}) })), active: 0 });

  const repo = ({ middle, right, selectedRow = null, wipSelected = false, status = {}, toolbar }) =>
    C.RepoView({
      toolbar,
      left: C.RefsList({ refs: history.refs, selectedRow, collapsed }),
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

  const commitFileComparison = () =>
    repo({
      selectedRow: selected,
      middle: C.FileDiffPanel({ diff: commitFileDiff, mode: 'split' }),
      right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails, open: commitFileDiff.change.path }),
    });

  const openFile = { side: fileDiff.side, path: fileDiff.change.path };

  const fileComparison = (mode, extra = {}) =>
    repo({
      wipSelected: true,
      middle: C.FileDiffPanel({ diff: extra.diff || fileDiff, mode, selection: extra.selection, pointed: extra.pointed }),
      right: C.ChangesList({ changes: extra.changes || changes, open: openFile, view: extra.view }),
    });

  const firstUnstaged = changes.unstaged[0];
  const ref = (name) => history.refs.find((label) => label.name === name);

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
      about: 'A right-click on a file in the changes list opens its menu where the pointer is: move it to the other list, discard its unstaged changes (Delete file… when Git does not track it), open it with its default app, show it in the file manager or copy its full path. A staged or conflicted file has no Discard; a deleted one has nothing to open or show.',
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
      about: 'A file was clicked in the changes list. The middle pane swaps the graph for a side-by-side comparison; Esc or × goes back to the graph. Each hunk header ends with Discard hunk and Stage hunk, so one part of the file can be staged or thrown away without the rest.',
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
