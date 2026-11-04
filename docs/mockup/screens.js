const Screens = (() => {
  const { history, changes, commitDetails, fileDiff, tabs } = Sample;
  const selected = 2;
  const position = (row) => `Commit ${thousands(row + 1)} of ${thousands(history.total)}`;

  const topBar = (extra = {}) => C.TopBar({ repository: history.name, head: history.head, ...extra });
  const tabStrip = (overrides = {}) => C.TabStrip({ tabs: tabs.map((tab, ix) => ({ ...tab, ...(overrides[ix] || {}) })), active: 0 });

  const repo = ({ middle, right, selectedRow = null, wipSelected = false, status = {} }) =>
    C.RepoView({
      left: C.RefsList({ refs: history.refs, selectedRow }),
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

  const openFile = { side: fileDiff.side, path: fileDiff.change.path };

  const fileComparison = (mode) =>
    repo({
      wipSelected: true,
      middle: C.FileDiffPanel({ diff: fileDiff, mode }),
      right: C.ChangesList({ changes, open: openFile }),
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
      about: 'The main screen. Refs on the left, the commit graph in the middle, the selected commit with its patch on the right. The refs pointing at the selected commit are highlighted too.',
      source: 'src/repo/mod.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitSelected() }),
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
      about: 'The Uncommitted changes row is selected. The right pane lists unstaged and staged files, each with a button that moves it to the other list.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.FastGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({ wipSelected: true, right: C.ChangesList({ changes }) }),
        }),
    },
    {
      id: 'file-split',
      title: 'File comparison — Split',
      about: 'A file was clicked in the changes list. The middle pane swaps the graph for a side-by-side comparison; Esc or × goes back to the graph.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split') }),
    },
    {
      id: 'file-unified',
      title: 'File comparison — Unified',
      about: 'The same comparison in one column, after clicking Unified.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.FastGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('unified') }),
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
          overlays: C.Menu({ items: Menus.View, left: 122, top: 32, hot: 'Fetch All Remotes' }),
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
