const Screens = (() => {
  const { history, collapsed, changes, partlyStaged, draft, commitDetails, commitFileDiff, fileDiff, partlyStagedDiff, selectedLines, tabs } = Sample;
  const selected = 2;
  const position = (row) => `Commit ${thousands(row + 1)} of ${thousands(history.total)}`;

  const topBar = (extra = {}) => C.TopBar({ repository: history.name, head: history.head, ...extra });
  const tabStrip = (overrides = {}) => C.TabStrip({ tabs: tabs.map((tab, ix) => ({ ...tab, ...(overrides[ix] || {}) })), active: 0 });

  const repo = ({ middle, right, selectedRow = null, wipSelected = false, status = {} }) =>
    C.RepoView({
      left: C.RefsList({ refs: history.refs, selectedRow, collapsed }),
      middle: middle || C.HistoryList({ history, selectedRow, changes, wipSelected }),
      right,
      status: C.RepoStatusBar({
        summary: historySummary(history),
        position: selectedRow === null ? '' : position(selectedRow),
        ...status,
      }),
    });

  const commitSelected = (status) =>
    repo({
      selectedRow: selected,
      right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails }),
      status,
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
      right: C.ChangesList({ changes: extra.changes || changes, open: openFile }),
    });

  const firstUnstaged = changes.unstaged[0];

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
      about: 'The main screen. Refs on the left, split into folders at each slash, the commit graph in the middle, the selected commit and the files it changes on the right. The refs pointing at the selected commit are highlighted too. Click a section or a folder to fold it.',
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
      about: 'The Uncommitted changes row is selected. The right pane starts with the commit message, its description and the Commit button, then the staged files, then the unstaged files at the bottom. Each file has a button that moves it to the other list. Commit stays disabled until there is a message and at least one staged file.',
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
