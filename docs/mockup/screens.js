const Screens = (() => {
  const { history, tracked, stashed, collapsed, changes, partlyStaged, draft, commitDetails, commitFileDiff, fileHistory, fileDiff, partlyStagedDiff, selectedLines, tabs } = Sample;
  const { starfall, assets, slimeDiff, introDiff, themeDiff, modelDiff } = Sample;
  const selected = 2;
  const position = (row) => `Commit ${thousands(row + 1)} of ${thousands(history.total)}`;

  const topBar = (extra = {}) => C.TopBar({ repository: history.name, head: history.head, ...extra });
  const tabStrip = (overrides = {}) => C.TabStrip({ tabs: tabs.map((tab, ix) => ({ ...tab, ...(overrides[ix] || {}) })), active: 0 });

  const repo = ({ left, middle, right, selectedRow = null, wipSelected = false, status = {}, toolbar }) =>
    C.RepoView({
      toolbar,
      left: left ?? C.RefsList({ refs: history.refs, selectedRow, collapsed }),
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
      left: false,
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
      left: false,
      middle: C.FileDiffPanel({ diff: extra.diff || fileDiff, mode, selection: extra.selection, pointed: extra.pointed, menuOpen: extra.menuOpen, hot: extra.hot, look: extra.look }),
      right: C.ChangesList({ changes: extra.changes || changes, open: openFile, view: extra.view }),
    });

  const firstUnstaged = changes.unstaged[0];
  const pickedFiles = { side: 'unstaged', paths: changes.unstaged.slice(1).map((change) => change.path) };
  const ref = (name) => history.refs.find((label) => label.name === name);

  const gameTabs = [...tabs, { name: starfall.name, state: 'ready' }];

  const { rebasing, rebase, starfallMerge, rebaseChanges, rebaseResolved, statusConflicts, statusMerged, slimeConflict, starfallConflicts } = Sample;
  const busyDuringOperation = ['pull', 'push', 'stash', 'pop'];

  const rebaseView = ({ middle, changes, open, conflicts, expanded = false }) =>
    C.SterneGit({
      topBar: C.TopBar({ repository: rebasing.name, head: rebasing.head }),
      tabStrip: tabStrip(),
      body: C.RepoView({
        toolbar: C.RepoToolbar({ unavailable: busyDuringOperation }),
        operation: C.OperationBar({ operation: rebase, conflicts }),
        expanded,
        left: C.RefsList({ refs: rebasing.refs, selectedRow: null, collapsed }),
        middle,
        right: C.ChangesList({ changes, open, operation: rebase }),
        status: C.RepoStatusBar({ summary: historySummary(rebasing) }),
      }),
    });
  const changedFiles = [...assets.staged, ...assets.unstaged];

  const assetComparison = (diff, { mode = 'split', expanded = false, view } = {}) =>
    C.SterneGit({
      topBar: C.TopBar({ repository: starfall.name, head: starfall.head }),
      tabStrip: C.TabStrip({ tabs: gameTabs, active: gameTabs.length - 1 }),
      body: C.RepoView({
        expanded,
        left: false,
        middle: C.FileDiffPanel({ diff, mode, expanded, view, filePosition: [changedFiles.indexOf(diff.change) + 1, changedFiles.length] }),
        right: C.ChangesList({ changes: assets, open: { side: diff.side, path: diff.change.path } }),
        status: C.RepoStatusBar({ summary: historySummary(starfall) }),
      }),
    });

  const { unity } = Sample;
  const unityFiles = [...unity.work.staged, ...unity.work.unstaged];

  const unityComparison = (diff, mode, { menuOpen = false, hot } = {}) =>
    C.SterneGit({
      topBar: C.TopBar({ repository: starfall.name, head: starfall.head }),
      tabStrip: C.TabStrip({ tabs: gameTabs, active: gameTabs.length - 1 }),
      body: C.RepoView({
        left: false,
        middle: C.FileDiffPanel({ diff, mode, menuOpen, hot, filePosition: [unityFiles.indexOf(diff.change) + 1, unityFiles.length] }),
        right: C.ChangesList({ changes: unity.work, open: { side: diff.side, path: diff.change.path } }),
        status: C.RepoStatusBar({ summary: historySummary(starfall) }),
      }),
    });

  return [
    {
      id: 'welcome',
      title: 'Welcome',
      about: 'No repository is open: SterneGit started outside a repository, or the last tab was closed. The tab strip and the status bar are hidden. Clone repository… opens the clone dialog, as File → Clone Repository… does.',
      source: 'src/app/window.rs',
      render: () =>
        C.SterneGit({
          topBar: C.TopBar(),
          body: C.Notice({
            title: Texts.welcomeTitle,
            description: Texts.welcomeDescription,
            action: `<div class="h actions">${C.Button({ label: 'Open repository…', variant: 'primary', size: 'md' })}${C.Button({ label: Texts.cloneRepository, icon: 'download', size: 'md' })}</div>`,
          }),
        }),
    },
    {
      id: 'reading',
      title: 'Reading history',
      about: 'A repository was just opened. Its tab shows a spinner and the whole body waits for the first read of refs and history.',
      source: 'src/repo/mod.rs',
      render: () =>
        C.SterneGit({
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
        C.SterneGit({
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
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitSelected() }),
    },
    {
      id: 'ahead-behind',
      title: 'Branches apart from their upstream',
      about: 'A local branch whose upstream is on another commit says how far apart they are, at the right of its row in the left pane: ↓ and the number of upstream commits the branch lacks, which Pull brings in, then ↑ and the number of its own commits the upstream lacks, which Push sends. A side with nothing to count is left out. A branch level with its upstream, without one, or whose upstream is gone shows nothing, and neither do remote branches and tags. Hovering the numbers says it in words, naming the upstream. The upstream is counted as the last fetch left it, so the numbers move with each fetch, pull, push and commit. Here main has two commits that origin/main lacks, and a fetch brought origin/feature/diff-view a commit that feature/diff-view lacks.',
      source: 'src/repo/refs_list.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({
            left: C.RefsList({ refs: tracked.refs, selectedRow: null, collapsed }),
            middle: C.HistoryList({ history: tracked, selectedRow: null, changes }),
            right: C.Hint({ message: Texts.selectCommit }),
          }),
        }),
    },
    {
      id: 'commit-message-hover',
      title: 'Pointing at a commit message',
      about: "The message of the selected commit can be edited in place. Pointing at its subject or description lights them up, with a text cursor and a tooltip: Click to edit the message. That works for any commit a local branch holds, or a detached HEAD, however old. A commit that only remote branches or tags hold, a stash, and every commit while a rebase, a merge, a cherry-pick or a revert waits or another Git write runs, don't light up and ignore the click.",
      source: 'src/repo/commit_panel.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitSelected(undefined, { pointing: true }) }),
    },
    {
      id: 'edit-message',
      title: 'Edit a commit message',
      about: "A click on the subject or the description turns both into fields, the one clicked taking the focus, as in the CommitForm: Enter in the subject moves to the description, Ctrl+Enter saves, Esc, Cancel or selecting another commit drops the edit. Under them a line says what saving rewrites. Git can't change a commit in place, so saving writes a new commit with the same files, author and date and the new message, then writes every commit after it again on top, merges included, on every local branch that holds it. Each keeps its own files, author, date and message, so nothing can conflict, and the working files, the staged changes and the stashes stay as they are. Those branches then move to the new commits, each only if it hasn't moved meanwhile; the new commits name you as committer, now, as git commit --amend does. Save message stays disabled while the subject is empty or nothing changed. Here main holds the commit and one more after it, neither pushed yet, and a description is being added to a commit that had none.",
      source: 'src/repo/commit_panel.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({
            selectedRow: 1,
            left: C.RefsList({ refs: tracked.refs, selectedRow: 1, collapsed }),
            middle: C.HistoryList({ history: tracked, selectedRow: 1, changes }),
            right: C.CommitPanel({
              commit: tracked.commits[1],
              details: { ...commitDetails, body: '' },
              edit: { description: 'parse_patch moves to patch_view.rs, so the FileDiffPanel\ncan read a patch without the commit panel.', focus: 'description', after: 1, branches: ['main'] },
            }),
          }),
        }),
    },
    {
      id: 'edit-pushed-message',
      title: 'Edit the message of a pushed commit',
      about: "When a remote branch already has the commit, a warning says so: the remote keeps the old commit, so the next push of the branch is refused and its question offers Force push, which replaces the old commit there too. Tags on the commit or on the commits after it stay where they are, on the old commits, and a line names them; so do remote branches. The old commits stay in the graph as long as a tag or a remote branch holds them. Here only feature/diff-view holds the commit, which origin/feature/diff-view already has.",
      source: 'src/repo/commit_panel.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(undefined, {
            edit: { subject: 'Render split diffs with syntax colors', focus: 'subject', after: 0, branches: ['feature/diff-view'], remotes: ['origin/feature/diff-view'] },
          }),
        }),
    },
    {
      id: 'message-saved',
      title: 'Commit message saved',
      about: "Once saved, the commit shows again with its new message and its new hash, still selected, and a notification names the old hash and the new one. The commit after it got a new hash too, and main moved to it: it is still two commits ahead of origin/main, since neither was pushed.",
      source: 'src/repo/commit_panel.rs',
      render: () => {
        const renamed = { [tracked.commits[0].id]: '3fa81c2', [tracked.commits[1].id]: '9d02e6b' };
        const saved = { ...tracked, commits: tracked.commits.map((commit) => ({ ...commit, id: renamed[commit.id] || commit.id })) };
        return C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({
            selectedRow: 1,
            left: C.RefsList({ refs: saved.refs, selectedRow: 1, collapsed }),
            middle: C.HistoryList({ history: saved, selectedRow: 1, changes }),
            right: C.CommitPanel({
              commit: saved.commits[1],
              details: { ...commitDetails, body: 'parse_patch moves to patch_view.rs, so the FileDiffPanel\ncan read a patch without the commit panel.' },
            }),
          }),
          overlays: `<div class="toasts">${C.Notification({ kind: 'success', message: Texts.messageSaved(tracked.commits[1].id, '9d02e6b') })}</div>`,
        });
      },
    },
    {
      id: 'commit-menu',
      title: 'Right-click on a commit',
      about: "A right-click on a commit row in the graph opens its menu where the pointer is, without selecting it. Check out this commit checks out the commit itself: HEAD is detached and the head pill reads detached at its id; uncommitted changes in the way are stashed and brought back, as for a branch. Create branch here… and Create tag here… start a branch or a tag at the commit. Revert this commit adds a commit on main that undoes it; a merge is reverted against its first parent. Reset main to here opens a submenu that moves main to the commit: Soft keeps what main leaves behind staged, Mixed keeps it unstaged, and Hard asks first, then throws every uncommitted change away. Copy hash copies the full hash and Copy message the whole message, each with a notification. The HEAD commit has no Check out and no Reset. Every Git write runs like the branch menu's: one at a time, items disabled while one runs, then a notification such as Reverted b47d0a8 or Reset main to b47d0a8; its changes are unstaged. While a rebase, a merge, a cherry-pick or a revert waits, only Create tag here… and the two Copy items stay enabled, since Git won't switch branches then.",
      source: 'src/repo/commit_menu.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.CommitMenu({ commit: history.commits[1], current: history.head, left: 440, top: 186, hot: Texts.resetHere(history.head), reset: 'mixed' }),
        }),
    },
    {
      id: 'commit-menu-other-branch',
      title: 'Right-click on a commit of another branch',
      about: "On a commit that main doesn't hold, Revert gives way to Cherry-pick onto main, which copies the commit onto main as a new commit with the same message and author. A merge has no Cherry-pick. Git refuses to cherry-pick or revert over staged changes or over changes to the files it touches, so SterneGit stashes the uncommitted changes first and brings them back after. When a cherry-pick or a revert conflicts, the OperationBar shows it as for a merge: the conflicted files open in the MergeEditor, then Commit finishes it, or Abort puts main back. Meanwhile the changes wait in a stash named Set aside by SterneGit until the cherry-pick or revert is done, which Commit and Abort bring back. A commit whose changes main already has, or a resolution that leaves main as it was, adds nothing: the notification says main already has the changes. With a detached HEAD the menu says HEAD where it says main here.",
      source: 'src/repo/commit_menu.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.CommitMenu({ commit: history.commits[4], current: history.head, reachable: false, left: 440, top: 268, hot: Texts.cherryPick(history.head) }),
        }),
    },
    {
      id: 'reset-hard',
      title: 'Reset hard',
      about: "Hard: discard changes… always asks first, because Git keeps no copy of uncommitted changes. The question counts the tracked files whose changes are lost, and the commits that no other branch or tag holds, which leave the graph. Reset then runs git reset --hard; untracked files stay. Soft and Mixed don't ask, since every change stays in the files. When main ends up behind its upstream, the next Push is refused and offers Pull and push or Force push, as usual.",
      source: 'src/repo/commit_menu.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.ResetHardDialog({ branch: history.head, commit: history.commits[1], files: [...changes.staged, ...changes.unstaged].filter((change) => change.kind !== '?').length }),
        }),
    },
    {
      id: 'create-tag',
      title: 'Create a tag',
      about: "Create tag here… asks for a name and an optional message. With a message it makes an annotated tag, which keeps its author and date; without one, a lightweight tag. Create tag stays disabled while the name is empty, already a tag, or refused by Git, and the reason shows under the field. Push it to origin pushes the tag right after, since Push sends branches only; it is offered when the repository has a remote and stays as it was last left. Enter creates, Esc cancels. The tag shows at once in the left pane and on the commit's row.",
      source: 'src/repo/tag_dialog.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.TagDialog({ commit: history.commits[1], value: 'v0.3.0', remote: 'origin' }),
        }),
    },
    {
      id: 'stashes',
      title: 'Stashes',
      about: 'Stashes get a section of their own in the left pane, between Remotes and Tags, newest first, each with its message as Git wrote it; it folds like the others and is left out when there is no stash. In the graph each stash is a row at the time it was made: a hollow square in the lane of the commit it was made on, joined to that commit, with a stash@{n} badge. Git also keeps a commit for the staged part of a stash and one for its untracked files; the graph leaves those out. Clicking a stash in the left pane or in the graph selects it, and the right pane shows what it changes in tracked files against the commit it was made on, as for any commit. The status bar counts the stashes, and says which one is selected; commit positions skip the stash rows.',
      source: 'src/repo/refs_list.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: stashSelected() }),
    },
    {
      id: 'stash-menu',
      title: 'Right-click on a stash',
      about: "A right-click on a stash, in the left pane or on its row in the graph, opens its menu where the pointer is, without selecting it. Apply brings its changes back into the work tree and keeps the stash; Pop brings them back and deletes the stash; both keep what was staged staged. When the changes conflict with the files, the conflicted files are left to resolve and the stash is kept; when they don't apply at all, nothing changes and the stash is kept; an error notification says which. Delete… asks first. Each item runs like the header's Pop: one Git write at a time, disabled while one runs, then a notification such as Popped stash@{1}. The header's Pop still pops the newest stash.",
      source: 'src/repo/stash_menu.rs',
      render: () =>
        C.SterneGit({
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
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: stashSelected(),
          overlays: C.DeleteStashDialog({ stash: stashed.history.stashes[1] }),
        }),
    },
    {
      id: 'commit-file',
      title: 'Commit file comparison',
      about: "A file was clicked in the selected commit's file list. The middle pane swaps the graph for a comparison of that file with the commit's first parent, and the refs pane steps aside; the row above the columns names the parent and the commit by their short ids. The whole file is shown as the commit left it, opened on its first change; with nothing to stage, no row comes between its lines. The file stays highlighted on the right, another click opens another file, and Esc or × goes back to the graph and the refs.",
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitFileComparison() }),
    },
    {
      id: 'history-tree',
      title: 'Commit files as a tree',
      about: "The two buttons at the right end of the file counts show the files as a list or as a tree. The tree splits each path at its slashes: folders first, then files, both by name, and a folder whose only child is a folder shares its row with it, as in src/repo. Each file row keeps its status letter and line counts. Click a folder to fold it. The choice applies to every file list in every tab, the changes list included.",
      source: 'src/repo/commit_files.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitSelected(undefined, { view: 'tree' }) }),
    },
    {
      id: 'commit-file-menu',
      title: "Right-click on a commit's file",
      about: "A right-click on a file in the selected commit's list opens its menu where the pointer is, without opening the file: File history, then Open file, Show in folder and Copy path, as for a changed file. A file the commit deletes has nothing to open or show, but still has a history. The same menu opens on the files of a commit in a file's history.",
      source: 'src/repo/commit_files.rs',
      render: () =>
        C.SterneGit({
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
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileHistoryView() }),
    },
    {
      id: 'nothing-selected',
      title: 'History — nothing selected',
      about: 'History is read but nothing is selected yet, so the right pane asks for a selection.',
      source: 'src/repo/mod.rs',
      render: () =>
        C.SterneGit({
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
        C.SterneGit({
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
        C.SterneGit({
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
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split', { view: 'tree' }) }),
    },
    {
      id: 'file-menu',
      title: 'Right-click on a changed file',
      about: 'A right-click on a file in the changes list opens its menu where the pointer is: move it to the other list, discard its unstaged changes (Delete file… when Git does not track it), see its history, open it with its default app, show it in the file manager or copy its full path. A staged or conflicted file has no Discard; a file that is new or untracked has no history yet; a deleted one has nothing to open or show.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({
            wipSelected: true,
            right: C.ChangesList({ changes, pointed: { side: 'unstaged', path: firstUnstaged.path, left: 168, top: 16, hot: 'Discard changes…' } }),
          }),
        }),
    },
    {
      id: 'files-picked',
      title: 'Several changed files picked',
      about: 'A click on a file picks it alone and opens it, as before. Ctrl+click (Cmd+click on macOS) adds a file to the pick or takes it out; Shift+click picks every file shown from the last one clicked to this one, and Ctrl+Shift+click adds that run to the pick. Runs follow the order on screen, in a list or a tree, and skip files inside folded folders. A pick stays inside one list: Ctrl or Shift on a file in the other list starts a new pick there, and conflicted files are picked one at a time. Ctrl and Shift only change the pick; the middle pane keeps the file opened last. Here patch.rs was clicked, then notes.md Shift-clicked.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({
            wipSelected: true,
            left: false,
            middle: C.FileDiffPanel({ diff: fileDiff, mode: 'split' }),
            right: C.ChangesList({ changes, open: openFile, picked: pickedFiles }),
          }),
        }),
    },
    {
      id: 'files-menu',
      title: 'Right-click on picked files',
      about: 'A right-click on a picked file opens a menu that acts on every picked file: Stage 3 files (Unstage in the staged list), Discard 3 files…, Open 3 files with their default apps and Copy 3 paths, one per line. File history and Show in folder need a single file, so they are left out. Delete 3 files… replaces Discard when none of them is tracked, and Open is left out when one of them is deleted. A right-click on a file outside the pick picks it alone and opens the one-file menu, without opening the file.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({
            wipSelected: true,
            left: false,
            middle: C.FileDiffPanel({ diff: fileDiff, mode: 'split' }),
            right: C.ChangesList({
              changes,
              open: openFile,
              picked: pickedFiles,
              pointed: { side: 'unstaged', path: changes.unstaged[2].path, left: 168, top: 16, hot: Texts.discardFiles(3) },
            }),
          }),
        }),
    },
    {
      id: 'discard-files',
      title: 'Discard several files',
      about: 'Discard 3 files… asks once for all of them. The question counts the files and says how many untracked ones are deleted; Discard puts each tracked file back as it is staged, or as it was committed, and deletes the untracked ones. Cancel or Esc keeps them all.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({
            wipSelected: true,
            left: false,
            middle: C.FileDiffPanel({ diff: fileDiff, mode: 'split' }),
            right: C.ChangesList({ changes, open: openFile, picked: pickedFiles }),
          }),
          overlays: C.DiscardDialog({ changes: changes.unstaged.slice(1) }),
        }),
    },
    {
      id: 'discard-file',
      title: 'Discard a file',
      about: 'Discard changes… asks first, because Git keeps no copy of unstaged changes. Discard puts the file back as it is staged, or as it was committed when nothing of it is staged; Cancel or Esc keeps it.',
      source: 'src/repo/changes_list.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: repo({ wipSelected: true, right: C.ChangesList({ changes }) }),
          overlays: C.DiscardDialog({ change: firstUnstaged }),
        }),
    },
    {
      id: 'file-split',
      title: 'File comparison — Split',
      about: "A file was clicked in the changes list. The middle pane swaps the graph for a side-by-side comparison, and the refs pane steps aside to give it room; Esc or × brings both back. One bar names the file and holds everything that acts on it: Split or Unified, the change count with ↑ and ↓ (Alt+Up, Alt+Down), the view options, Stage, the button that gives the comparison the whole window, and ×. A row above the columns names the two sides: Before, HEAD (Staged once part of the file is staged), and After, Working copy. A changed line has a red or green bar beside a number of the same color, and its changed words are highlighted; a line that exists on one side only leaves a hatched gap on the other. Long lines run past the edge: each column has a horizontal scrollbar, both columns move together and the line numbers stay in place. The whole file is there, and it opens on its first change rather than at line 1: the view starts four lines above that change's hunk, so the change sits near the top with what comes before it just above. ↑ and ↓ bring the other changes to the same place. A thin grey row above each hunk holds Discard hunk and Stage hunk. The strip at the right edge is the whole file: it marks where the changes sit and what part of the file is in view; a click on a mark goes there.",
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split') }),
    },
    {
      id: 'file-unified',
      title: 'File comparison — Unified',
      about: "The same comparison in one column, after clicking Unified: the whole file again, opened on its first change. Both side names sit above it, each line keeps both of its numbers, and one horizontal scrollbar moves the code. The hunk buttons are the same.",
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('unified') }),
    },
    {
      id: 'file-lines',
      title: 'File comparison — lines selected',
      about: 'Clicking a line number selects that line; dragging over line numbers, or Shift-clicking a second one, selects more. The header of the hunk that holds them then reads Discard 3 lines and Stage 3 lines, which act on the selected changed lines only; the other hunks keep their hunk buttons.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split', { selection: selectedLines }) }),
    },
    {
      id: 'line-menu',
      title: 'Right-click on a change',
      about: 'The line under the pointer is highlighted, and a right-click on it opens a menu where the pointer is. On a changed line it offers its hunk, then the line alone; on an unchanged line or a hunk header only the hunk; inside a selection, the selected lines first. Discarding a hunk or lines happens at once, like staging them, without asking.',
      source: 'src/repo/file_diff_panel.rs',
      render: () =>
        C.SterneGit({
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
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: fileComparison('split', { diff: partlyStagedDiff, changes: partlyStaged }),
        }),
    },
    {
      id: 'view-options',
      title: 'File comparison — view options',
      about: 'The ⋯ in the file bar opens the options of every text comparison, in every tab: changed words highlighted, long lines wrapped instead of scrolled (off by default), the whole file shown instead of only the changed parts (on by default), whitespace changes hidden. While whitespace is hidden the hunk buttons and the line menu go away, because Git could not apply those parts to the file.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split', { menuOpen: true, hot: 'whitespace' }) }),
    },
    {
      id: 'file-changes-only',
      title: 'File comparison — only the changes',
      about: 'With Show the whole file unchecked in ⋯, the comparison keeps only the changed parts and opens at the top. Between two hunks a single grey row folds the unchanged lines, names the function or block the next hunk sits in, and ends with Discard hunk and Stage hunk; a click on the fold shows the lines, or 20 at a time when there are more. A file longer than 100,000 lines always opens this way.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: fileComparison('split', { look: { whole: false } }) }),
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
      about: 'A video compares frame by frame. Here the comparison has the whole window: the button left of × in the file bar hides the refs and the file list until it is clicked again, and the file bar gains ‹ and › to move through the changed files (Alt+Left, Alt+Right). Both versions play in step from the same time, and the frame buttons step one frame. The timeline draws how long each version is and where their frames differ, numbered like the changes in the view bar. SterneGit uses the ffmpeg and ffprobe found on PATH; without them a video shows its details and the Open buttons, with a note that says so. Hear picks whose sound plays. Split, Swipe, Onion skin and Difference work as for pictures. LFS says Git LFS stores the file: SterneGit compares the videos, not the pointers.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(introDiff, { expanded: true }),
    },
    {
      id: 'music',
      title: 'Compare music and sounds',
      about: 'A sound shows both waveforms on one time scale, the version before the change on top. Yellow bands mark where they sound different, numbered like the changes in the view bar; the hatching is where the old version has already ended. One playhead drives both: Hear switches which version plays at once, without stopping, so the same bar can be heard one way and then the other (Tab switches too). Loop change repeats the current band. Difference shows only what changed. The details add loudness (EBU R128) and peak. SterneGit reads WAV, AIFF, FLAC, Ogg Vorbis, MP3 and M4A (AAC).',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(themeDiff),
    },
    {
      id: 'other-file',
      title: 'Compare a file SterneGit can’t draw',
      about: 'A file SterneGit can’t draw, such as a 3D model, still says what changed: its type, its size and when it last changed. Open Before gives a copy of the old version to its default app, Open After the file on disk, and Compare in difftool hands both to the tool set in Git’s diff.tool; it only shows when one is set.',
      source: 'src/repo/file_diff_panel.rs',
      render: () => assetComparison(modelDiff),
    },
    {
      id: 'unity-prefab',
      title: 'Unity — prefab, Inspector',
      about: "A file Unity saved as text (its first lines are %YAML 1.1 and %TAG !u! tag:unity3d.com,2011:) opens like the Inspector instead of as YAML: prefabs, scenes, materials, animation clips, Animator controllers, ScriptableObject assets, project settings and .meta files. On the left, the Hierarchy of both versions merged: A for an object added, D for one removed (struck through), M for one with changed properties; unchanged objects are grey, and Changed only hides them. On the right, every changed object in Hierarchy order, each component as a card, each changed property as a row: the name, the value before on a red tint, the value after on a green tint, the part that changed in a stronger tint, like changed words in code. A property that only one side has leaves a hatched cell, like a missing line. References read as Unity shows them, never as a fileID or a GUID: an object of the same file by its name and type, an asset by its name through the GUID in its .meta file, a script by its class. Lists show the elements that changed. A removed object folds to one line. ↑ and ↓ move through the changes; a click in the Hierarchy scrolls to the object. Stage acts on the whole file: parts of it can be staged in YAML only.",
      source: 'src/repo/compare/unity/inspector.rs',
      render: () => unityComparison(unity.prefabDiff, 'inspector'),
    },
    {
      id: 'unity-options',
      title: 'Unity — view options',
      about: "The ⋯ of a Unity comparison holds its own options, for every Unity file in every tab. Hide what Unity changes on its own is on by default: Unity rewrites values on save that nobody changed, such as floats that round differently (0.99999994 for 1), prefab overrides in another order, a serializedVersion bump or editor-only fields; the view bar says how many such changes are hidden. Show unchanged properties lists the whole component, not only its changed rows. Show file IDs adds the fileID of each object and component, for when YAML has to be read next to it.",
      source: 'src/repo/compare/unity/options.rs',
      render: () => unityComparison(unity.prefabDiff, 'inspector', { menuOpen: true, hot: 'noise' }),
    },
    {
      id: 'unity-prefab-yaml',
      title: 'Unity — prefab, YAML',
      about: "YAML shows the text Unity saved, as a text comparison with only the changed parts. The grey row above each part names the object and the component it belongs to (Slime › Slime Controller) instead of a YAML header, and every fileID and GUID has its name after the line, as a # comment: the object, the script or the asset it points to. The names are only shown, never written into the file. This is where parts of the file can be staged or discarded with Stage hunk and Discard hunk, as in any text file.",
      source: 'src/unity/hints.rs',
      render: () => unityComparison(unity.prefabYaml, 'yaml'),
    },
    {
      id: 'unity-prefab-scene',
      title: 'Unity — prefab, Scene',
      about: "A prefab has the Scene view too, the way Unity opens it in prefab mode: the prefab alone, zoomed to fit, each child drawn with its own sprite. A child added is outlined in green (the crown), one removed is drawn faded at its place with a dashed red outline and a cross (the particles), and one whose position, rotation or scale changed shows its old shape dashed in red under the new one, outlined in yellow (the shadow, now smaller). Colliders are drawn as Unity draws their gizmos, the old size dashed in red and the new one in green. Changes that have no place, such as Mass or Move Speed, are not drawn here: Inspector lists them, and so does the panel underneath for the picked object. A click picks an object, outlined in blue, and its changes show underneath.",
      source: 'src/repo/compare/unity/scene.rs',
      render: () => unityComparison(unity.prefabDiff, 'scene'),
    },
    {
      id: 'unity-scene',
      title: 'Unity — scene, Inspector',
      about: "A scene compares like a prefab. A prefab instance has a blue cube and says which prefab it comes from; only its overrides are listed, each card marked Override with a blue edge as in Unity. Here the camera size and background, the light and one slime changed, a slime was added and a bat removed. Five changes Unity made on its own are hidden.",
      source: 'src/repo/compare/unity/inspector.rs',
      render: () => unityComparison(unity.levelDiff, 'inspector'),
    },
    {
      id: 'unity-scene-view',
      title: 'Unity — scene, Scene',
      about: "Scene draws where things are, from the front for a 2D scene (X and Y) and from above for a 3D one (X and Z): every object with a sprite is drawn with it, the others with their icon. What is gone or moved away is drawn faded with a dashed red outline at its old place, an object that moved has a yellow arrow to its new place, an added one a green outline. The camera's view is a dashed red frame before and a green frame after, so a new size or position shows at once. A click on an object picks it, outlined in blue, and its changes show underneath. SterneGit draws sprites and positions only: no lighting, meshes or shaders.",
      source: 'src/repo/compare/unity/scene.rs',
      render: () => unityComparison(unity.levelDiff, 'scene'),
    },
    {
      id: 'unity-material',
      title: 'Unity — material',
      about: "A file that holds one thing, such as a material, a ScriptableObject asset or project settings, has no Hierarchy: the Inspector takes the whole width. A material says its shader, then compares its properties grouped as Textures, Numbers, Colors, Keywords and Settings: colors as swatches with their hex value and HDR intensity, textures by name, numbers and keywords as text. The shader is not run, so there is no preview sphere.",
      source: 'src/repo/compare/unity/inspector.rs',
      render: () => unityComparison(unity.materialDiff, 'inspector'),
    },
    {
      id: 'unity-animation',
      title: 'Unity — animation clip',
      about: "An animation clip draws one lane per animated property, both versions on the same time scale: Before as a dashed red curve, After as a solid green one. A diamond is a key, white when both versions have it at the same time and value. Sprite properties show the sprite of each key, events show their flags. The ruler marks where each version ends. A property only one version animates has an A or D badge. Under it, the clip settings in one table.",
      source: 'src/repo/compare/unity/curves.rs',
      render: () => unityComparison(unity.jumpDiff, 'curves'),
    },
    {
      id: 'unity-animator',
      title: 'Unity — Animator controller',
      about: "An Animator controller draws its state machine as Unity lays it out, one layer at a time: states added have a green outline, states removed are faded with a dashed red outline, transitions are green when added, dashed red when removed and yellow when their settings changed. The list on the right counts every change, parameters included; a click on one, or on a state or a transition, shows its settings before and after underneath.",
      source: 'src/repo/compare/unity/graph.rs',
      render: () => unityComparison(unity.animatorDiff, 'graph'),
    },
    {
      id: 'unity-script-graph',
      title: 'Unity — Script Graph',
      about: "A Visual Scripting Script Graph asset is drawn as its graph, nodes where the graph editor puts them. Nodes added have a green outline, nodes removed are faded with a dashed red outline, nodes with a changed setting a yellow one, with the changed value filled in yellow in the node; the old value is in the settings on the right. Connections are green when added and dashed red when removed; the others keep the color of what they carry: white for flow, then one color per type, as in the graph editor. A value that used to be typed in and is now connected shows in the node's settings on the right: Y was 6, it now comes from Multiply. The list on the right counts every node, connection and graph variable that changed. The selector at the top left moves into subgraphs; a State Graph asset is drawn like an Animator controller. Unity keeps the whole graph as JSON on a single line, so the second mode is JSON, laid out one value per line, rather than YAML.",
      source: 'src/repo/compare/unity/graph.rs',
      render: () => unityComparison(unity.brainDiff, 'graph'),
    },
    {
      id: 'unity-texture-meta',
      title: 'Unity — an asset and its .meta',
      about: "In a Unity project a .meta file moves with its asset: the changes list shows slime.png once, with a meta tag, and staging, unstaging or discarding it does the same to slime.png.meta. The comparison is the asset's own, here the picture, with the import settings that changed in the .meta under its details: here the sprite was made crisper and smaller. A .meta that changed alone is listed on its own.",
      source: 'src/repo/changes_list.rs',
      render: () => unityComparison(unity.slimeMetaDiff, 'split'),
    },
    {
      id: 'unity-guid',
      title: 'Unity — a GUID that changed',
      about: "A .meta whose GUID changed is the usual way a Unity project loses references: the asset was deleted and imported again, or its .meta was never committed. Its comparison starts with what breaks: SterneGit searches the repository for the old GUID (git grep) and lists every file and property still using it. Keep the old GUID writes it back into the .meta, so nothing breaks; Point the files to the new GUID changes them instead. Either way the changes list reads the files again, and the warning goes once nothing uses the old GUID. The import settings, reset to Unity's defaults by the new import, are compared underneath.",
      source: 'src/repo/compare/unity/guid.rs',
      render: () => unityComparison(unity.guidDiff, 'inspector'),
    },
    {
      id: 'pulling',
      title: 'Header — pulling',
      about: 'Pull was clicked. Its icon turns into a spinner and the other header buttons wait, like the stage and commit buttons, because SterneGit runs one Git write at a time. Pull and Push need a checked-out branch, except that Pull in Fetch only mode works anywhere; Stash needs uncommitted changes; Pop needs a stash.',
      source: 'src/repo/toolbar.rs',
      render: () => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitSelected(undefined, {}, C.RepoToolbar({ running: 'pull' })) }),
    },
    {
      id: 'pull-modes',
      title: 'Header — pull mode',
      about: "The caret beside Pull opens its modes. The checked one is what a click on Pull does; picking another makes it Pull's default for this repository, without pulling. SterneGit saves it in the repository's Git config as sternegit.pullMode, which Git itself ignores. Until a mode is picked, Pull follows Git's own settings: Rebase when pull.rebase is true, Fast-forward only when pull.ff is only, Fast-forward if possible otherwise.",
      source: 'src/repo/toolbar.rs',
      render: () =>
        C.SterneGit({
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
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'success', message: 'Pushed main to origin/main' })}</div>`,
        }),
    },
    {
      id: 'push-behind',
      title: 'Header — push behind its upstream',
      about: "When the remote refuses a push because origin/main has commits main doesn't have, SterneGit fetches that remote, counts the commits on each side and asks what to do; the graph behind the dialog already shows the fetched commits. Pull and push pulls in Fast-forward if possible mode, whatever the header's mode is, then pushes; a pull that stops on a conflict shows the OperationBar and pushes nothing, so Push is clicked again once the merge is committed. Force push overwrites origin/main with main, but only if origin/main is still where the dialog counted from: if someone pushed again in between, nothing is overwritten and a notification says so. Cancel or Esc pushes nothing. When main has nothing of its own to push, the primary button is just Pull. Push in a branch's menu asks the same way; for a branch that isn't checked out, Pull is only offered when it can fast-forward.",
      source: 'src/repo/behind_dialog.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.BehindDialog({ branch: 'main', upstream: 'origin/main', theirs: 2, ours: 1 }),
        }),
    },
    {
      id: 'push-rewritten',
      title: 'Header — push after rewriting pushed commits',
      about: "When every commit origin/main has and main doesn't makes the same changes as one of main's, those commits are older versions: main rewrote them after they were pushed, with Edit message or a rebase. SterneGit checks it with git rev-list --cherry-pick right after counting. The question then says so and offers only Cancel and Force push, which is the primary button and what Enter picks: pulling would bring the old versions back beside the new ones, so Pull and push is left out. Force push keeps its lease on the counted upstream commit, so it still overwrites nothing someone pushed in between. A rewritten merge has no equivalent by Git's rules, so when the old versions include a merge the usual question shows.",
      source: 'src/repo/behind_dialog.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.BehindDialog({ branch: 'main', upstream: 'origin/main', theirs: 2, ours: 2, rewritten: true }),
        }),
    },
    {
      id: 'pulled-and-pushed',
      title: 'Header — pulled, then pushed',
      about: 'Pull and push says both steps in one notification. Force push says Force pushed main to origin/main. When the upstream moved again before Force push ran, the push fails and the notification stays until it is closed; pushing again asks with fresh counts.',
      source: 'src/repo/toolbar.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'success', message: Texts.pulledAndPushed('main', 'origin/main') })}</div>`,
        }),
    },
    {
      id: 'branch-dialog',
      title: 'Header — create a branch',
      about: 'Branch asks for a name, then creates the branch where HEAD is and checks it out. Create branch stays disabled while the name is empty, taken or refused by Git; Enter creates, Esc cancels.',
      source: 'src/repo/branch_dialog.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.BranchDialog({ head: `main (${history.commits[0].id})`, value: 'feature/toolbar' }),
        }),
    },
    {
      id: 'branch-menu',
      title: 'Right-click on a branch',
      about: "A right-click on a branch, in the left list or on its badge in the graph, opens its menu where the pointer is, without selecting its commit. Here feature/blame is not checked out, so it offers Check out, Pull, Push, Merge it into main or Rebase main onto it, Create branch here…, New worktree for feature/blame… and Delete…. Pull is fast-forward only, because Git can't merge into or rebase a branch that isn't checked out. The checked-out branch offers Pull with the header's pull mode, Push, Create branch here… and New worktree from main…. A remote branch offers Check out, Merge, Rebase, Create branch here…, New worktree for … and Delete from its remote. With a detached HEAD there is no Merge or Rebase. Folders have no menu, and a tag has its own; a badge that holds a branch and its upstream opens the menu of the local branch. Each action runs like the header buttons: one Git write at a time, then a notification; Pull and Push spin the header's button while they run. A double-click on a branch, in the left list or on its badge in the graph, checks it out straight away; it does nothing on the checked-out branch, on a tag, or while Git writes.",
      source: 'src/repo/branch_menu.rs',
      render: () =>
        C.SterneGit({
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
        C.SterneGit({
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
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.DeleteBranchDialog({ label: ref('feature/blame'), only: 1 }),
        }),
    },
    {
      id: 'tag-menu',
      title: 'Right-click on a tag',
      about: "A right-click on a tag in the left list opens its menu where the pointer is, without selecting its commit. Check out v0.2.0 checks out the tag's commit with HEAD detached, as Check out this commit does. Push v0.2.0 to origin sends the tag to the push remote, since the header's Push sends branches only. Create branch here… starts a branch at the tag. Copy name copies the tag's name, with a notification. Delete v0.2.0… deletes the tag here, and Delete v0.2.0 from origin… deletes it on origin; both ask first. Without a remote, Push and Delete from origin are left out. Every item but Copy name is disabled while Git writes, and Check out and Create branch here… also while a rebase, a merge, a cherry-pick or a revert waits. A double-click on a tag does nothing.",
      source: 'src/repo/tag_menu.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.TagMenu({ label: ref('v0.2.0'), remote: 'origin', left: 70, top: 612, hot: Texts.pushTagTo('v0.2.0', 'origin') }),
        }),
    },
    {
      id: 'tag-menu-badge',
      title: 'Right-click on a tag badge',
      about: "The same menu opens from the tag's badge in the graph; a right-click elsewhere on the row still opens the commit's menu. A tag hidden behind +n has no badge to right-click; its row in the left list has the menu.",
      source: 'src/repo/history_list.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.TagMenu({ label: ref('v0.2.0'), remote: 'origin', left: 450, top: 335, hot: Texts.pushTagTo('v0.2.0', 'origin') }),
        }),
    },
    {
      id: 'tag-pushed',
      title: 'Tag pushed',
      about: "Push spins the header's Push button while it runs, then says what happened: Pushed v0.2.0 to origin, or origin already has v0.2.0 when it was there already. When origin has a different v0.2.0, Git refuses to replace it and the error says to delete it from origin first.",
      source: 'src/repo/operation.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'success', message: Texts.tagPushedTo('v0.2.0', 'origin') })}</div>`,
        }),
    },
    {
      id: 'delete-tag',
      title: 'Delete a tag',
      about: "Delete v0.2.0… asks first, like a branch: it counts the commits that no branch or other tag holds and says they are lost, or that nothing is lost. Fetching brings back the tags a remote has, so when the repository has a remote the question says so and points to Delete from origin. Delete then runs git tag --delete and says Deleted tag v0.2.0.",
      source: 'src/repo/tag_menu.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.DeleteTagDialog({ label: ref('v0.2.0'), remote: 'origin' }),
        }),
    },
    {
      id: 'delete-tag-remote',
      title: 'Delete a tag from its remote',
      about: "Delete v0.2.0 from origin… asks first, since everyone who uses origin loses the tag. Delete then runs git push origin --delete refs/tags/v0.2.0 and says Deleted v0.2.0 from origin. The tag stays here, so a later Push v0.2.0 to origin sends it back.",
      source: 'src/repo/tag_menu.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.DeleteTagDialog({ label: ref('v0.2.0'), remote: 'origin', there: true }),
        }),
    },
    {
      id: 'merged',
      title: 'Branch merged',
      about: 'Check out, Merge, Rebase and Delete say what they did in a notification. A merge or a rebase that stops on a conflict shows the OperationBar instead, until it is finished or aborted.',
      source: 'src/repo/operation.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'success', message: 'Merged feature/blame into main' })}</div>`,
        }),
    },
    {
      id: 'checked-out-stash-kept',
      title: 'Checked out, changes left in the stash',
      about: "When uncommitted changes are in the way of a check-out, SterneGit stashes them, switches and pops the stash, so what was staged stays staged. When they conflict with the branch or don't apply on it, the check-out still happens, the stash keeps them and a warning says so until it is closed.",
      source: 'src/git/branch.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'warning', title: 'Switched to main', message: 'Your uncommitted changes conflict with main: resolve the conflicted files. The stash keeps a copy of them.' })}</div>`,
        }),
    },
    {
      id: 'rebase-stopped',
      title: 'Rebase stopped on a conflict',
      about: "Rebase in a branch's menu and Pull in Rebase mode go through on their own: uncommitted changes in the way are stashed first and brought back at the end, and a commit whose changes are already upstream is left out. SterneGit stops only when a commit conflicts. The OperationBar under the header then says what is being rebased, which commit stopped and how many files conflict. The uncommitted changes get selected, the conflicted files get their own section at the top of the list, and the first one opens with the whole window, because three views of the code need the width; ⤡ shows the panes again and ‹ › step through the conflicted files. The head pill reads rebasing main while Git replays the commits on a detached HEAD. Pull, Push, Stash and Pop wait, and the commit form is hidden, because Continue commits each step with its own message. A conflicted text file opens in the MergeEditor: the current version (what the branch already has: during a rebase, origin/main and the commits replayed so far) and the incoming version (the commit being replayed) side by side on top, each conflict under a row with its two Accept checkboxes, and below them the result, the file as it will be saved, still holding the conflict markers. ↑ and ↓ step through the conflicts and scroll the three together.",
      source: 'src/repo/operation_bar.rs',
      render: () =>
        rebaseView({
          middle: C.FileDiffPanel({ diff: statusConflicts, expanded: true, filePosition: [1, 2] }),
          changes: rebaseChanges,
          open: { side: 'conflicted', path: statusConflicts.change.path },
          conflicts: rebaseChanges.conflicted.length,
          expanded: true,
        }),
    },
    {
      id: 'conflict-chosen',
      title: 'Conflicts resolved in the result',
      about: 'The same file, scrolled to its second conflict with ↓. Ticking Accept current, Accept incoming or both puts those lines in the result in place of the markers; unticking takes them out again. The result is an ordinary editor: here the incoming change was ticked for the second conflict, then two of its lines were edited by hand to keep the current String path, so they turn yellow and that conflict’s checkboxes are disabled. The result counts the conflicts left, and Mark resolved becomes primary once no markers are left, ticked or typed away. Mark resolved writes the result, stages it and opens the next conflicted file; until then nothing is written. Start over puts the markers back.',
      source: 'src/repo/merge_editor.rs',
      render: () =>
        rebaseView({
          middle: C.FileDiffPanel({ diff: statusMerged, expanded: true, filePosition: [1, 2] }),
          changes: rebaseChanges,
          open: { side: 'conflicted', path: statusConflicts.change.path },
          conflicts: rebaseChanges.conflicted.length,
          expanded: true,
        }),
    },
    {
      id: 'rebase-ready',
      title: 'Rebase ready to continue',
      about: "After the last Mark resolved the middle pane goes back to the graph. The replayed commit sits on origin/main, main still points at the old commits, and the uncommitted changes hold the resolved commit. The bar says no conflicts are left and Continue is ready: it commits them with the stopped commit's message and replays the rest, leaving out a commit the resolution emptied. The next conflict stops the same way. When every commit is replayed, main moves to the result, the bar goes away and a notification says Rebased main onto origin/main. Abort rebase puts main back where it was without asking, and brings back the stashed changes.",
      source: 'src/repo/operation_bar.rs',
      render: () =>
        rebaseView({
          middle: C.HistoryList({ history: rebasing, selectedRow: null, changes: rebaseResolved, wipSelected: true }),
          changes: rebaseResolved,
          conflicts: 0,
        }),
    },
    {
      id: 'merge-picture-conflict',
      title: 'Merge stopped on a picture',
      about: "The same bar serves a merge that stops, from a branch's menu or from Pull in Fast-forward if possible mode: Abort merge, and Commit merge, which commits with Git's usual message. A conflicted file that isn't text has no markers to choose between, so its comparison shows the current version as Before and the incoming one as After, side by side, with the comparison modes of its kind. Keep current or Keep incoming stages that version as it is.",
      source: 'src/repo/file_diff_panel.rs',
      render: () =>
        C.SterneGit({
          topBar: C.TopBar({ repository: starfall.name, head: starfall.head }),
          tabStrip: C.TabStrip({ tabs: gameTabs, active: gameTabs.length - 1 }),
          body: C.RepoView({
            toolbar: C.RepoToolbar({ unavailable: busyDuringOperation }),
            operation: C.OperationBar({ operation: starfallMerge, conflicts: starfallConflicts.conflicted.length }),
            left: false,
            middle: C.FileDiffPanel({ diff: slimeConflict, mode: 'split' }),
            right: C.ChangesList({ changes: starfallConflicts, open: { side: 'conflicted', path: slimeConflict.change.path }, operation: starfallMerge }),
            status: C.RepoStatusBar({ summary: historySummary(starfall) }),
          }),
        }),
    },
    {
      id: 'menu',
      title: 'View menu open',
      about: 'On Windows and Linux SterneGit draws its own menu bar in the top bar; on macOS the same menus go to the system menu bar. Zoom In, Zoom Out and Reset Zoom step the zoom of the whole window, like Ctrl + mouse wheel.',
      source: 'src/app/mod.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar({ openMenu: 'View' }),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.Menu({ items: Menus.View, left: 114, top: 32, hot: 'Fetch All Remotes' }),
        }),
    },
    {
      id: 'fetching',
      title: 'Copy hash and fetch',
      about: 'Copy hash shows a notification. A fetch runs in the background every five minutes, when the window comes back, when the network comes back (a new address settles, or the computer wakes), when a remote is added or its URL changes, and when an account that serves the remote signs in or is chosen for its owner; the status bar shows it.',
      source: 'src/repo/status_bar.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected({ fetching: true }),
          overlays: `<div class="toasts">${C.Notification({ message: 'Commit hash copied' })}</div>`,
        }),
    },
    {
      id: 'about',
      title: 'About dialog',
      about: 'About → About SterneGit opens an alert dialog with the version.',
      source: 'src/app/window.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.AlertDialog({ title: 'SterneGit 0.1.0', description: Texts.aboutDescription }),
        }),
    },
    {
      id: 'update-ready',
      title: 'An update is ready',
      about: "A copy from a SterneGit release looks for a new release when it starts and every six hours. With Update automatically on, it downloads the setup (installed copies) or the portable zip (portable copies) in the background, checks its SHA-256 and says nothing until it is ready. Then Restart to update appears at the right of the top bar: it installs the new version silently, closes every SterneGit window and reopens this one with its tabs. Quitting installs it too, once the last SterneGit window closes. What's new opens the release page on GitHub.",
      source: 'src/app/top_bar.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar({ update: { state: 'ready', version: '0.2.0' } }),
          tabStrip: tabStrip(),
          body: commitSelected(),
        }),
    },
    {
      id: 'update-available',
      title: 'A release is out, Update automatically off',
      about: 'With Update automatically off in Settings → Updates, SterneGit downloads nothing on its own: the top bar offers Update to 0.2.0 instead. Clicking it downloads the release (the button spins meanwhile), then Restart to update replaces it. A portable copy in a folder it cannot write to shows SterneGit 0.2.0 is out, which opens the release page.',
      source: 'src/app/top_bar.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar({ update: { state: 'available', version: '0.2.0' } }),
          tabStrip: tabStrip(),
          body: commitSelected(),
        }),
    },
    {
      id: 'about-menu',
      title: 'About menu',
      about: 'About → Check for Updates… looks for a new release at once and says what it found in a notification. In a copy built on this computer it says the copy never updates itself.',
      source: 'src/app/mod.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar({ openMenu: 'About' }),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: C.Menu({ items: Menus.About, left: 158, top: 32, hot: 'Check for Updates…' }),
        }),
    },
    {
      id: 'check-up-to-date',
      title: 'Check for Updates: up to date',
      about: 'When the running version is the latest release.',
      source: 'src/app/window.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'success', message: HostTexts.upToDateToast('0.1.0') })}</div>`,
        }),
    },
    {
      id: 'check-found',
      title: 'Check for Updates: a release is out',
      about: "With Update automatically on, the release starts downloading and the notification says Restart to update will appear. With it off, the notification has an Update button instead. When the check fails, an error notification gives curl's reason.",
      source: 'src/app/window.rs',
      render: () =>
        C.SterneGit({
          topBar: topBar(),
          tabStrip: tabStrip(),
          body: commitSelected(),
          overlays: `<div class="toasts">${C.Notification({ kind: 'info', title: HostTexts.foundTitle('0.2.0'), message: HostTexts.foundDownloading })}${C.Notification({ kind: 'info', title: HostTexts.foundTitle('0.2.0'), message: HostTexts.foundManual, action: C.Button({ label: HostTexts.update, icon: 'download', size: 'sm', variant: 'update' }) })}</div>`,
        }),
    },
    ...(() => {
      const { accounts, serverKeys, connectors, gitlabUpdate, owners, repositories } = Sample.hosting;
      const [personal, work, enterprise] = accounts;
      const settingsTabs = [...tabs, { name: HostTexts.settings, icon: 'settings' }];
      const settings = (page, content, overlays = '', zoom = 100, update) =>
        C.SterneGit({
          topBar: C.TopBar(update ? { update: { state: 'ready', version: update } } : {}),
          tabStrip: C.TabStrip({ tabs: settingsTabs, active: settingsTabs.length - 1 }),
          body: C.SettingsTab({ page, content, nav: { accounts: accounts.length, waiting: 1, zoom, update } }),
          overlays,
          zoom,
        });
      const accountsPage = () => C.AccountsPage({ accounts });
      const personalPage = (account = personal) => C.AccountPage({ account, keys: serverKeys.filter((key) => account.key || !key.sternegit) });
      const overHistory = (overlays) => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: commitSelected(), overlays });
      const withoutRemotes = {
        ...history,
        refs: history.refs.filter((label) => label.kind !== 'remote'),
        commits: history.commits.map((commit) => (commit.refs ? { ...commit, refs: commit.refs.filter(([, kind]) => kind !== 'remote') } : commit)),
      };
      const pair = [personal, work];
      const choiceBar = (open) => C.AccountChoiceBar({ host: 'github.com', owner: 'sternegit-org', accounts: pair, chosen: 'alex-martin', open });
      return [
        {
          id: 'app-file-menu',
          title: 'File menu open',
          about: 'File gains Clone Repository… (Ctrl+Shift+O), New Repository… (Ctrl+N) and Settings… (Ctrl+,). Settings… opens the Settings tab, or switches to it when it is already open.',
          source: 'src/app/mod.rs',
          render: () =>
            C.SterneGit({
              topBar: topBar({ openMenu: 'File' }),
              tabStrip: tabStrip(),
              body: commitSelected(),
              overlays: C.Menu({ items: Menus.File, left: 26, top: 32, hot: 'Clone Repository…' }),
            }),
        },
        {
          id: 'settings-accounts',
          title: 'Settings — accounts',
          about: "Settings is a tab after the repository tabs, with a gear; it isn't saved with the open tabs, and Ctrl+W closes it like any tab. Its left column picks a page. Accounts lists every account SterneGit knows, with its service, its server and how it signed in. An account whose token the server refused says so in red and offers Sign in again; nothing is asked until then, so a refused token never interrupts a fetch. Click an account to open its page. The yellow badge on Connectors counts connectors waiting for approval.",
          source: 'src/accounts/book.rs',
          render: () => settings('accounts', accountsPage()),
        },
        {
          id: 'settings-account',
          title: "Settings — an account's page",
          about: "How the account signed in, its scopes and where its token is kept; its SSH key and every key the server lists for it, read when the page opens; and the remote owners SterneGit remembers for it. Sign in again replaces the token after checking who it belongs to: a token of another user is refused. Forget drops a remembered owner, so SterneGit asks again the next time two accounts fit.",
          source: 'src/accounts/account.rs',
          render: () => settings('accounts', personalPage()),
        },
        {
          id: 'add-account',
          title: 'Add an account',
          about: "Add account… asks which service, among every started connector's services, then how to sign in. The connector's manifest lists the methods each service offers: GitHub Enterprise Server asks for its address first and has no browser sign-in. Choosing a method moves to its step in the same dialog; Esc or Cancel closes it.",
          source: 'src/accounts/account.rs',
          render: () => settings('accounts', accountsPage(), C.SignInDialog({ step: 'choose', hot: 'device' })),
        },
        {
          id: 'sign-in-browser',
          title: 'Sign in with the browser',
          about: "GitHub's device flow: SterneGit asks GitHub for a code, opens github.com/login/device in the browser and asks again every few seconds inside the dialog, slower when GitHub says so, until the user approves. The token is checked with account.who_am_i and stored only then; the dialog closes and a notification says Signed in to GitHub as alex-martin. Cancel or Esc stops asking. It needs the client ID of SterneGit's OAuth App in GitHub's manifest.",
          source: 'src/accounts/account.rs',
          render: () => settings('accounts', accountsPage(), C.SignInDialog({ step: 'device' })),
        },
        {
          id: 'sign-in-expired',
          title: 'Sign in with the browser — code expired',
          about: "A code lasts 15 minutes. Get a new code starts over. When the user declines on GitHub, the dialog says GitHub says you declined, with Try again.",
          source: 'src/accounts/account.rs',
          render: () => settings('accounts', accountsPage(), C.SignInDialog({ step: 'expired' })),
        },
        {
          id: 'sign-in-token',
          title: 'Sign in with a pasted token',
          about: "Create a token on GitHub opens the manifest's create_url, with the scopes SterneGit needs already ticked. Sign in checks the token with account.who_am_i before saving it; a refused token says so under the field, and Sign in waits for the field to change. A token missing write:public_key signs in, and the Signed in notification adds that SSH keys won't work with it. The field never shows the token once pasted.",
          source: 'src/accounts/account.rs',
          render: () => settings('accounts', accountsPage(), C.SignInDialog({ step: 'token', problem: HostTexts.tokenRefused('GitHub') })),
        },
        {
          id: 'sign-in-credential-manager',
          title: "Use Git Credential Manager's token",
          about: "SterneGit reads Git Credential Manager's token only after this question, with git credential fill and no prompt. When it has none, the dialog says so and Use this token stays off.",
          source: 'src/accounts/account.rs',
          render: () => settings('accounts', accountsPage(), C.SignInDialog({ step: 'credential' })),
        },
        {
          id: 'ssh-key',
          title: 'Create an SSH key',
          about: "Create SSH key… on an account's page. SterneGit runs ssh-keygen beside git's ssh, adds the public key with ssh_keys.add and fills its own known_hosts from ssh_keys.host_keys, so the first SSH clone checks GitHub's host key. When the token lacks write:public_key, the dialog says so and offers Sign in with the browser instead of Create and add.",
          source: 'src/accounts/account.rs',
          render: () => settings('accounts', personalPage({ ...personal, key: null }), C.SshKeyDialog({ account: personal })),
        },
        {
          id: 'remove-account',
          title: 'Remove an account',
          about: "Remove… on an account's page. The repositories whose core.sshCommand names the account's key are listed, and ticked to have it removed. Remove forgets the account, its token, its key file and its remembered owners.",
          source: 'src/accounts/book.rs',
          render: () => settings('accounts', personalPage(), C.RemoveAccountDialog({ account: personal })),
        },
        {
          id: 'settings-connectors',
          title: 'Settings — connectors',
          about: "Every connector SterneGit found beside its exe (Bundled) or in the user's connectors folder (Installed), with what it reaches and its standing. Ready runs. Updated is a new version within what was approved, until its row is opened. Waiting for approval and Files changed (a bundled connector whose files don't match what this SterneGit ships) wait for Review…. Can't run needs a newer SterneGit or has a broken manifest. Stopped trapped, ran out of time or memory while starting; Show log shows its last log lines.",
          source: 'src/connect/catalog.rs',
          render: () => settings('connectors', C.ConnectorsPage({ connectors })),
        },
        {
          id: 'connector-approval',
          title: 'Approve a connector',
          about: "Exactly what the connector asks for, from its manifest: the servers it can reach and whether they get the token, picture servers, the remotes it recognizes (and any other connector that already does), sign-in pages and features. Approve records the grant and the files' SHA-256 in connectors.json and starts it at once. Not now leaves it waiting, and the next launch asks again. An installed connector with the id of a bundled one says it replaces it; once approved, the bundled one stops being listed.",
          source: 'src/connect/grant.rs',
          render: () => settings('connectors', C.ApprovalPage({ connector: connectors[1] })),
        },
        {
          id: 'connector-update',
          title: 'Approve an update that asks for more',
          about: 'A new version that asks for nothing new starts on its own and shows Updated. One that asks for more waits, and its page lists only the additions; the old version keeps running meanwhile.',
          source: 'src/connect/grant.rs',
          render: () => settings('connectors', C.ApprovalPage({ connector: gitlabUpdate, update: true })),
        },
        {
          id: 'settings-interface',
          title: 'Settings — interface',
          about: "Interface is the third page of Settings. Zoom scales the whole window, like a browser's zoom: text, rows, icons, menus, dialogs and the top bar, with crisp text at every size. − and + step through 50, 67, 75, 80, 90, 100, 110, 125, 150, 175 and 200 %; each end turns its button off. Reset goes back to 100 % and is off there. SterneGit saves the zoom in %APPDATA%\\SterneGit\\settings.json and opens at it next time.",
          source: 'src/hosting/interface_page.rs',
          render: () => settings('interface', C.InterfacePage({ zoom: 100 })),
        },
        {
          id: 'settings-interface-zoomed',
          title: 'Settings — interface, zoomed to 125 %',
          about: 'The same page at 125 %: everything is a quarter bigger, so less fits in the window. Beside Interface, the nav shows the zoom whenever it is not 100 %.',
          source: 'src/hosting/interface_page.rs',
          render: () => settings('interface', C.InterfacePage({ zoom: 125 }), '', 125),
        },
        {
          id: 'settings-updates',
          title: 'Settings — updates',
          about: "Updates is the fourth page of Settings. It names the running version and how this copy was installed: by the setup (for this Windows user, in %LOCALAPPDATA%\\Programs\\SterneGit) or portable, from a zip unpacked anywhere. Check for updates looks at once. Update automatically is saved in %APPDATA%\\SterneGit\\settings.json and is on unless switched off.",
          source: 'src/hosting/updates_page.rs',
          render: () => settings('updates', C.UpdatesPage()),
        },
        {
          id: 'settings-updates-ready',
          title: 'Settings — updates, a release is ready',
          about: 'Once the new release is downloaded and checked, the page and the nav name it, and the top bar shows Restart to update.',
          source: 'src/hosting/updates_page.rs',
          render: () => settings('updates', C.UpdatesPage({ status: 'ready' }), '', 100, '0.2.0'),
        },
        {
          id: 'settings-updates-portable',
          title: 'Settings — updates, portable in a read-only folder',
          about: "A portable copy updates by unpacking the new zip and swapping its own sternegit.exe and connectors folder when it restarts or quits; the files it replaces are deleted at the next launch. When its folder can't be written to, it only points at the release page.",
          source: 'src/hosting/updates_page.rs',
          render: () => settings('updates', C.UpdatesPage({ installation: 'portable', folder: 'D:\\Tools\\SterneGit', status: 'read-only' })),
        },
        {
          id: 'settings-updates-built',
          title: 'Settings — updates, built on this computer',
          about: 'Only exes built by the release workflow know where to look for releases. One built with cargo, like build\\sternegit.exe, never updates itself, wherever it sits.',
          source: 'src/hosting/updates_page.rs',
          render: () => settings('updates', C.UpdatesPage({ installation: 'built' })),
        },
        {
          id: 'wheel-zoom',
          title: 'Ctrl + wheel zooms the window',
          about: "Holding Ctrl and turning the mouse wheel steps one zoom level per notch, anywhere in the window: lists don't scroll meanwhile. A touchpad steps once per 50 px of scrolling. Over a picture or a sound being compared, Ctrl + wheel keeps zooming that instead, as before. View → Zoom In (Ctrl+=), Zoom Out (Ctrl+-) and Reset Zoom (Ctrl+0) do the same from the keyboard. Each key also works where it sits on a US keyboard, so on a French one Ctrl+0 is the key that also reads à.",
          source: 'src/app/zoom.rs',
          render: () =>
            C.SterneGit({
              topBar: topBar(),
              tabStrip: tabStrip(),
              body: commitSelected(),
              zoom: 110,
            }),
        },
        {
          id: 'connector-waiting',
          title: 'A connector waits for approval',
          about: 'At launch, once connectors are found, a notification names each connector waiting for approval (or says how many). Review opens the Settings tab on its approval page. It shows again at every launch until the connector is approved.',
          source: 'src/app/connectors.rs',
          render: () =>
            overHistory(
              `<div class="toasts">${C.Notification({ kind: 'info', title: HostTexts.waitingTitle('GitLab', '1.2.0'), message: HostTexts.waitingMessage, action: C.Button({ label: HostTexts.review, size: 'sm', variant: 'primary' }) })}</div>`,
            ),
        },
        {
          id: 'account-choice',
          title: 'Which account?',
          about: "When a remote fits several accounts and SterneGit hasn't been told which one its owner uses, a bar under the toolbar asks. Use remembers the choice for every repository of that owner and closes the bar. Until then Git uses the user's own credentials, so fetches and pushes work as before; × hides the bar until the next launch.",
          source: 'src/accounts/access.rs',
          render: () =>
            C.SterneGit({
              topBar: topBar(),
              tabStrip: tabStrip(),
              body: C.RepoView({
                operation: choiceBar(true),
                left: C.RefsList({ refs: history.refs, selectedRow: selected, collapsed }),
                middle: C.HistoryList({ history, selectedRow: selected, changes }),
                right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails }),
                status: C.RepoStatusBar({ summary: historySummary(history), position: position(selected) }),
              }),
              overlays: C.AccountMenu({ accounts: pair, chosen: 'alex-martin', hot: 'alex-acme', left: 1137, top: 155 }),
            }),
        },
        {
          id: 'clone-url',
          title: 'Clone — from a URL',
          about: 'File → Clone Repository…, or Clone repository… on the welcome screen. URL clones anything git clone accepts, with no account; Clone into fills in the folder name from the URL and Browse… picks the parent folder. When an account fits the URL, SterneGit lends Git its token.',
          source: 'src/app/window.rs',
          render: () => overHistory(C.CloneDialog({ from: 'url', accounts, url: 'https://github.com/sternegit-org/connectors.git' })),
        },
        {
          id: 'clone-account',
          title: 'Clone — from an account',
          about: "Each account lists the repositories it can reach, newest first, a hundred at a time; Load more reads the next page. The owner filter lists the account's user and organizations, and search asks the server. Clone into ends with the selected repository's name. HTTPS is the default; SSH is offered once the account has a SterneGit key, and the choice is remembered per account.",
          source: 'src/connect/forge.rs',
          render: () => overHistory(C.CloneDialog({ from: 'alex-martin', accounts, repositories, selected: 1 })),
        },
        {
          id: 'clone-signed-out',
          title: 'Clone — account signed out',
          about: "When a connector call fails, the place that asked shows why, with the way out: signed out → Sign in again; a missing scope → Sign in with the browser; rate limited → when it resets; unreachable server → Retry; a stopped connector → its last log lines.",
          source: 'src/connect/stop.rs',
          render: () => overHistory(C.CloneDialog({ from: 'alex-acme', accounts, failure: Failures.signedOut })),
        },
        {
          id: 'cloning',
          title: 'Cloning',
          about: "Clone opens a tab for the new repository at once and runs git clone in it, showing git's progress; the tab reads history as soon as the clone ends. Cancel stops git and deletes the half-made folder. A failed clone keeps the tab with git's message and Retry. A clone over SSH gets core.sshCommand with the account's key; over HTTPS, SterneGit is Git's credential helper for that remote.",
          source: 'src/app/window.rs',
          render: () =>
            C.SterneGit({
              topBar: C.TopBar({ repository: 'connectors' }),
              tabStrip: C.TabStrip({ tabs: [...tabs, { name: 'connectors', state: 'reading' }], active: tabs.length }),
              body: C.Loading({
                message: HostTexts.cloning('sternegit-org/connectors', 'E:\\Projects\\connectors'),
                detail: HostTexts.receiving(45, 1234, 2741),
                action: C.Button({ label: HostTexts.cancel, size: 'md' }),
              }),
            }),
        },
        {
          id: 'new-repository',
          title: 'New repository',
          about: 'File → New Repository…. Name and folder make a local repository with git init, opened in a new tab. Also create it on, off by default, creates it empty on the chosen account and adds it as origin, so the first push sends main there. Owners that can\'t create repositories are listed but greyed out, and Visibility offers only what the owner allows.',
          source: 'src/app/window.rs',
          render: () => overHistory(C.NewRepositoryDialog({ hosted: true, account: personal, owners })),
        },
        {
          id: 'publish',
          title: 'Publish a repository',
          about: "Push in a repository with no remote opens Publish instead of failing. Publish creates the repository on the account, empty, adds it as origin, pushes the checked-out branch and makes origin's copy its upstream. A name the server refuses is shown under the field. With no account yet, the dialog offers Add account…, which opens Settings.",
          source: 'src/repo/toolbar.rs',
          render: () =>
            C.SterneGit({
              topBar: topBar(),
              tabStrip: tabStrip(),
              body: C.RepoView({
                left: C.RefsList({ refs: withoutRemotes.refs, selectedRow: null, collapsed }),
                middle: C.HistoryList({ history: withoutRemotes, selectedRow: null, changes }),
                right: C.Hint({ message: Texts.selectCommit }),
                status: C.RepoStatusBar({ summary: historySummary(withoutRemotes) }),
              }),
              overlays: C.PublishDialog({ repository: 'sternegit', account: personal, owner: owners[0] }),
            }),
        },
      ];
    })(),
    ...(() => {
      const { list, pulled, shown, details, failing, failingDetails, fork, forkDetails, mergedSha } = Sample.pulls;
      const refsWithPulls = (pulls, refs = pulled.refs, selectedRow = null) => C.RefsList({ refs, selectedRow, collapsed, pulls: { pulls: list.open, ...pulls } });
      const graph = (selectedRow = selected) => C.HistoryList({ history: pulled, selectedRow, changes });
      const pullView = ({ pull = shown, info = details, middle, right, pulls = {}, refs, overlays = '', position, head = history.head, left }) =>
        C.SterneGit({
          topBar: topBar({ head }),
          tabStrip: tabStrip(),
          body: C.RepoView({
            left: left ?? refsWithPulls({ selected: pull.number, ...pulls }, refs),
            middle: middle || C.PullOverview({ pull, details: info }),
            right: right || C.PullFiles({ pull, details: info }),
            status: C.RepoStatusBar({ summary: historySummary(history), position: position || PullTexts.position(pull.number, list.open.length, PullTexts.filters.open) }),
          }),
          overlays,
        });
      const topicTabs = ref('topic/tabs');
      const forkBranch = 'pr/9-patch-1';
      const forkRefs = [...pulled.refs.map((label) => (label.head ? { ...label, head: false } : label)), { name: forkBranch, kind: 'local', row: 0, head: true }];
      const toast = (notification) => `<div class="toasts">${notification}</div>`;
      return [
        {
          id: 'pulls',
          title: 'Pull requests in the left pane',
          about: "When a started connector claims the repository's remote and an account fits it, the left pane starts with Pull requests, above Branches. SterneGit reads them with pulls.list when the tab opens and after each fetch finishes, never on a timer of its own. Each row has its state's icon (open in green, a draft in grey), the title, the overall state of its checks and its number; hovering it names the author and the branches. A local branch that is the head of an open pull request carries its number, in the left pane and on its badge in the graph: here feature/diff-view (#12), feature/stash (#14) and the draft feature/blame (#11). A pull request from a fork has no branch here, so it is only in the list. The section folds like the others and is left out when no connector claims the remote.",
          source: 'src/repo/pulls/section.rs',
          render: () =>
            C.SterneGit({
              topBar: topBar(),
              tabStrip: tabStrip(),
              body: C.RepoView({
                left: refsWithPulls({}, pulled.refs, selected),
                middle: graph(),
                right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails }),
                status: C.RepoStatusBar({ summary: historySummary(history), position: position(selected) }),
              }),
            }),
        },
        {
          id: 'pull-filter',
          title: 'Closed pull requests',
          about: 'The button at the right of the section header picks which pull requests to list: Open, Closed (merged or closed, newest first) or All. Away from Open, the button says which. A hundred are read at a time; Load more, the last row, reads the next hundred. The choice belongs to the tab and isn\'t saved. Branch badges keep showing the open pull requests read last.',
          source: 'src/repo/pulls/section.rs',
          render: () =>
            C.SterneGit({
              topBar: topBar(),
              tabStrip: tabStrip(),
              body: C.RepoView({
                left: refsWithPulls({ pulls: list.closed, filter: 'closed', more: true }, pulled.refs, selected),
                middle: graph(),
                right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails }),
                status: C.RepoStatusBar({ summary: historySummary(history), position: position(selected) }),
              }),
              overlays: C.PullFilterMenu({ filter: 'closed', left: 150, top: 144, hot: 'all' }),
            }),
        },
        {
          id: 'pull-overview',
          title: 'Pull request selected',
          about: "Clicking a pull request selects it. The middle pane swaps the graph for its overview: title and number; its state, review, author and branches; Check out and Open on GitHub. Then its description as plain text, its checks (pulls.checks on the head commit), whether it can merge with Merge…, and the conversation with a box to comment. The right pane fetches the head into refs/sternegit/pull/12 and lists the files it changes since the commit where it branched off, as git diff main...head would; the list or tree choice is the one every file list shares. The overview is read again after each fetch. Esc, or a click on a branch, a tag or a stash, brings the graph back.",
          source: 'src/repo/pulls/overview.rs',
          render: () => pullView({}),
        },
        {
          id: 'pull-reading',
          title: 'Pull request opening',
          about: "The row's facts show at once. The details come from GitHub while Git fetches the head; each part shows when it is ready. A failure stays in its part, with the way out: Retry for a fetch or an unreachable server, Sign in again for a refused token, the connector's last log lines when it stopped.",
          source: 'src/repo/pulls/overview.rs',
          render: () => pullView({ middle: C.PullOverview({ pull: shown, details, state: 'loading' }), right: C.PullFiles({ pull: shown, details, state: 'loading' }) }),
        },
        {
          id: 'pull-file',
          title: "Pull request file comparison",
          about: 'A click on a file compares it in the middle pane: the file where the branch started on the left, at the head on the right, with every view the commit comparison has. Esc or × in the file bar goes back to the overview; another file opens another comparison; Alt+Left and Alt+Right step through the files.',
          source: 'src/repo/file_diff_panel.rs',
          render: () =>
            pullView({
              left: false,
              middle: C.FileDiffPanel({ diff: { ...commitFileDiff, commit: `#${shown.number}` }, mode: 'split', filePosition: [1, details.files.length] }),
              right: C.PullFiles({ pull: shown, details, open: commitFileDiff.change.path }),
            }),
        },
        {
          id: 'pull-checks-failing',
          title: 'Checks failing',
          about: 'One row per check run and commit status on the head commit, each opening its page on GitHub. The summary counts them, worst first: failing, waiting for you, running, waiting, passed, then the rest. Running and waiting checks are read again with the next fetch. A yellow merge box can still merge: GitHub says none of the failing checks is required.',
          source: 'src/repo/pulls/overview.rs',
          render: () => pullView({ pull: failing, info: failingDetails }),
        },
        {
          id: 'pull-merge',
          title: 'Merge a pull request',
          about: "Merge… asks how, offering only the methods the repository allows, with the first one picked. Squash and merge and Create a merge commit take a commit title and message, filled in as GitHub would; Rebase and merge takes none. SterneGit sends the head it showed with pulls.merge: if someone pushed since, GitHub refuses, nothing is merged, and the dialog says so. Enter merges, Esc cancels.",
          source: 'src/repo/pulls/merge.rs',
          render: () => pullView({ overlays: C.MergePullDialog({ pull: shown, details }) }),
        },
        {
          id: 'pull-merged',
          title: 'Pull request merged',
          about: 'Once the server merges it, SterneGit fetches; the overview shows Merged and the pull request leaves the Open list, but stays selected. A notification offers Delete branch for a few seconds, and the merge box keeps offering it while origin still has the branch: git push --delete removes feature/diff-view from origin, then the local branch goes too unless it is checked out or points somewhere else than the merged head.',
          source: 'src/repo/pulls/overview.rs',
          render: () =>
            pullView({
              pull: { ...shown, state: 'merged' },
              info: { ...details, mergedSha },
              pulls: { pulls: list.open.filter((pull) => pull.number !== shown.number) },
              refs: pulled.refs.map((label) => (label.pull && label.pull.number === shown.number ? { ...label, pull: undefined } : label)),
              position: `Pull request #${shown.number} · merged`,
              overlays: toast(C.Notification({ kind: 'success', title: PullTexts.merged(shown.number, shown.base), message: PullTexts.mergedMessage(shown.head), action: C.Button({ label: PullTexts.deleteBranch, icon: 'trash-2', size: 'sm', tooltip: PullTexts.deleteBranchTooltip }) })),
            }),
        },
        {
          id: 'pull-checked-out-fork',
          title: 'Check out a pull request from a fork',
          about: 'Check out on a pull request from this repository checks out its branch tracking origin\'s copy, as Check out on a remote branch does. One from a fork has no branch on origin: SterneGit fetches refs/pull/9/head and makes a local pr/9-patch-1 from it, without an upstream; the branch carries the pull request\'s number. Uncommitted changes come along unless they clash, as for any check out. Here the merge is blocked until someone with write access approves.',
          source: 'src/repo/pulls/overview.rs',
          render: () =>
            pullView({
              pull: fork,
              info: forkDetails,
              head: forkBranch,
              refs: forkRefs,
              middle: C.PullOverview({ pull: fork, details: forkDetails, checkedOut: true }),
              overlays: toast(C.Notification({ kind: 'success', message: PullTexts.checkedOutFork(forkBranch, fork.headOwner) })),
            }),
        },
        {
          id: 'create-pull-menu',
          title: 'Right-click on a branch — pull request',
          about: "When the remote has a connector and an account, the branch menu gains Create pull request…, or Show pull request #12 for a branch that already has an open one, which selects it. It is offered on local and remote branches, in the left pane and on badges in the graph, but not on the default branch.",
          source: 'src/repo/branch_menu.rs',
          render: () =>
            C.SterneGit({
              topBar: topBar(),
              tabStrip: tabStrip(),
              body: C.RepoView({
                left: refsWithPulls({}, pulled.refs, selected),
                middle: graph(),
                right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails }),
                status: C.RepoStatusBar({ summary: historySummary(history), position: position(selected) }),
              }),
              overlays: C.BranchMenu({ label: topicTabs, current: history.head, left: 92, top: 470, hot: PullTexts.createItem, pull: 'create' }),
            }),
        },
        {
          id: 'create-pull',
          title: 'Create a pull request',
          about: "From is the branch that was right-clicked, Into lists origin's branches with the default one picked. The title is the newest commit's subject, and the description lists the subjects when there are several. A branch that isn't on origin yet, or has commits origin lacks, is pushed first, and the dialog says so. Create sends pulls.create, selects the new pull request and says Created pull request #15; a refusal, such as one already open for the branch, shows under the title. Ctrl+Enter creates, Esc cancels.",
          source: 'src/repo/pulls/create.rs',
          render: () =>
            C.SterneGit({
              topBar: topBar(),
              tabStrip: tabStrip(),
              body: C.RepoView({
                left: refsWithPulls({}, pulled.refs, selected),
                middle: graph(),
                right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails }),
                status: C.RepoStatusBar({ summary: historySummary(history), position: position(selected) }),
              }),
              overlays: C.CreatePullDialog({
                head: topicTabs.name,
                title: 'Keep one tab per repository',
                body: '- Keep one tab per repository\n- Open each repository in its own tab',
                unpublished: true,
                commits: 2,
                files: 4,
              }),
            }),
        },
        {
          id: 'pulls-no-account',
          title: 'Pull requests without an account',
          about: 'A connector claims the remote but no account fits it: the section\'s one row says so, and Add account… opens the Add an account dialog. A refused token says the account is signed out, with Sign in again; nothing is asked before, and fetches keep using Git\'s own credentials. The first read shows a spinner; another failure shows its reason with Retry. When two accounts fit and SterneGit asks which one, the row says to pick one above. Hovering a row shows its whole text.',
          source: 'src/repo/pulls/section.rs',
          render: () =>
            C.SterneGit({
              topBar: topBar(),
              tabStrip: tabStrip(),
              body: C.RepoView({
                left: C.RefsList({ refs: history.refs, selectedRow: selected, collapsed, pulls: { state: 'none' } }),
                middle: C.HistoryList({ history, selectedRow: selected, changes }),
                right: C.CommitPanel({ commit: history.commits[selected], details: commitDetails }),
                status: C.RepoStatusBar({ summary: historySummary(history), position: position(selected) }),
              }),
            }),
        },
      ];
    })(),
    ...(() => {
      const { list, diffViewChanges, inGraph, inGraphFromDiffView } = Sample.worktrees;
      const [, diffView, , dreamy] = list;
      const diffViewRow = inGraph.wipRow('diff-view');
      const graphView = (section = {}) =>
        C.RepoView({
          left: C.RefsList({ refs: inGraph.refs, selectedRow: null, collapsed, worktrees: { list, here: 'sternegit', selected: 'diff-view', ...section } }),
          middle: C.HistoryList({ history: inGraph, selectedRow: diffViewRow, changes }),
          right: C.ChangesList({ changes: diffViewChanges, worktree: diffView }),
          status: C.RepoStatusBar({ summary: historySummary(inGraph) }),
        });
      const overGraph = (overlays = '', section) => C.SterneGit({ topBar: topBar(), tabStrip: tabStrip(), body: graphView(section), overlays });
      const worktreeTabs = [tabs[0], { name: diffView.name, state: 'ready', icon: 'folder-git' }, ...tabs.slice(1)];
      return [
        {
          id: 'worktrees-in-graph',
          title: 'Worktrees — the section and the graph',
          about: "When the repository has more than one worktree, the left pane gets a Worktrees section under Branches. Each row is a worktree folder: its name, then its branch, or detached at and the commit, then a yellow dot with its number of changed files, a lock when it is locked, or a warning when its folder is gone; hovering the lock or the warning says why. This tab's worktree is in bold. A click on a row selects that worktree in the graph: its row of changes when it has some, otherwise the commit its HEAD is on. A double-click opens it in a new tab. A click on the section's header opens the Worktrees panel; its chevron folds the section as for the others. The graph shows the uncommitted changes of every worktree, not only this tab's, which stay pinned on top. Each other worktree with changed files gets its own row right above the commit its HEAD is on: a dashed hollow dot joined to that commit, a badge with the folder name, and what changed. A detached worktree whose lane is taken gets a lane of its own, like dreamy-grothendieck. A clean worktree shows in the graph only through the folder icon on its branch, or a badge of its own when it is detached. Selecting a worktree's changes shows them on the right, under a bar with its name, branch and path, and buttons to show the folder or open it in a new tab; staging, discarding and Commit act in that folder. Pull, Push, Branch and Stash still act on this tab's worktree. SterneGit reads the status of every worktree, side by side, along with its own every few seconds; a worktree that gains or loses changes, or moves its HEAD, rereads the graph.",
          source: 'src/repo/refs_list.rs',
          render: () => overGraph(),
        },
        {
          id: 'worktree-row-menu',
          title: 'Worktrees — right-click on a worktree',
          about: "A right-click on a worktree, in the section, on its row of changes in the graph or on its badge, opens its menu where the pointer is. Open in a new tab opens the folder in a tab right after the repository's, or switches to that tab when it is already open. Show in folder and Copy path work as for a file. Lock… asks for a reason and keeps git worktree prune from forgetting it, for a folder on a drive that isn't always there; Unlock undoes it. Remove worktree… asks first. The main worktree has no Lock and no Remove, a locked one no Remove until it is unlocked, and this tab's worktree no Open and no Remove. A worktree whose folder is gone has only Forget this worktree, which forgets that one worktree and frees its branch, and Copy path.",
          source: 'src/repo/worktree_menu.rs',
          render: () => overGraph(C.WorktreeMenu({ tree: diffView, left: 150, top: 420, hot: WorktreeTexts.openInTab })),
        },
        {
          id: 'worktrees-manage',
          title: 'Worktrees — the Worktrees panel',
          about: "A click on the Worktrees header opens this panel: every worktree of the repository, to add one or clean up the ones tools and old reviews leave behind. Each row has its folder name, branch and path, then what to know before removing it: changed files, merged into the current branch, locked, or folder missing, then a button to open it in a new tab. Select merged ticks every worktree whose branch or detached commit is merged; Forget missing folders runs git worktree prune. Remove deletes the ticked folders, and warns first when some have uncommitted changes. The main worktree, this tab's and locked ones can't be ticked. New worktree… opens the New worktree dialog on a new branch starting from HEAD's branch. Esc or Close goes back.",
          source: 'src/repo/worktrees_panel.rs',
          render: () => overGraph(C.WorktreesDialog({ repository: inGraph.name, list, here: 'sternegit', picked: [dreamy.name, 'stash-ui'] }), { hot: true }),
        },
        {
          id: 'worktree-tab',
          title: 'Worktrees — a worktree in its own tab',
          about: "Open in a new tab opens the worktree as Open Repository… opens a folder, in a tab right after its repository's, with a folder icon and the folder's name. The top bar shows the repository, the worktree and its branch. The tab reads history and refs itself; HEAD, the uncommitted changes pinned on top and the staged files are the worktree's own, so Commit, Pull, Push and Stash act there. Its Worktrees section puts it in bold. The other worktrees, the main one included, show in its graph as everywhere: here main has its own row of changes, and its badge carries the folder icon. Closing the tab leaves the folder alone.",
          source: 'src/app/window.rs',
          render: () =>
            C.SterneGit({
              topBar: C.TopBar({ repository: inGraphFromDiffView.name, worktree: diffView.name, head: inGraphFromDiffView.head }),
              tabStrip: C.TabStrip({ tabs: worktreeTabs, active: 1 }),
              body: C.RepoView({
                left: C.RefsList({ refs: inGraphFromDiffView.refs, selectedRow: null, collapsed, worktrees: { list, here: 'diff-view' } }),
                middle: C.HistoryList({ history: inGraphFromDiffView, selectedRow: null, changes: diffViewChanges, wipSelected: true }),
                right: C.ChangesList({ changes: diffViewChanges }),
                status: C.RepoStatusBar({ summary: historySummary(inGraphFromDiffView) }),
              }),
            }),
        },
        {
          id: 'worktree-branch-menu',
          title: 'Worktrees — new worktree from a branch',
          about: "The BranchMenu of every local or remote branch has a worktree item under Create branch here…. A branch checked out nowhere offers New worktree for …, which opens the New worktree dialog with that branch picked. Git won't check out one branch in two worktrees, so a branch already checked out in a worktree has Open the worktree in place of Check out, and no Delete.",
          source: 'src/repo/branch_menu.rs',
          render: () => overGraph(C.BranchMenu({ label: inGraph.refs.find((label) => label.name === 'topic/tabs'), current: inGraph.head, left: 92, top: 336, hot: WorktreeTexts.newFor('topic/tabs') })),
        },
        {
          id: 'worktree-branch-menu-taken',
          title: 'Worktrees — new worktree from a checked-out branch',
          about: "A branch already checked out, in this tab or in another worktree, can't get a second worktree of its own, so its BranchMenu offers New worktree from … instead. The same goes for a remote branch whose local branch is checked out. It opens the New worktree dialog on New branch, starting from that branch, with the focus in Name.",
          source: 'src/repo/branch_menu.rs',
          render: () => overGraph(C.BranchMenu({ label: inGraph.refs.find((label) => label.name === inGraph.head), current: inGraph.head, left: 352, top: 166, hot: WorktreeTexts.newFrom(inGraph.head) })),
        },
        {
          id: 'create-worktree-from',
          title: 'Worktrees — the New worktree dialog from a checked-out branch',
          about: "New worktree from main… opens the dialog on New branch, Starting from main, with the focus in Name. The folder follows the name as it is typed, as long as it wasn't edited. Existing branch is still there, with main listed as taken.",
          source: 'src/repo/worktree_dialog.rs',
          render: () => overGraph(C.WorktreeDialog({ fresh: true, name: 'fix/tab-order', from: inGraph.head, folder: 'E:\\Projects\\sternegit-worktrees\\tab-order', parent: 'E:\\Projects\\sternegit-worktrees' })),
        },
        {
          id: 'create-worktree',
          title: 'Worktrees — the New worktree dialog',
          about: "Opened by New worktree for … or New worktree from … in a BranchMenu, or New worktree… in the Worktrees panel. Existing branch checks out a branch, local or remote, a remote one through its local branch when it has one; branches already checked out in a worktree are listed but can't be picked, and say where. New branch asks for a name and where it starts, HEAD by default. The folder is suggested next to the repository, in sternegit-worktrees, named after the last part of the branch, so it never shows up as untracked files in the main worktree; Browse… picks another. Create worktree runs git worktree add; the new worktree then shows in the section and in the graph through its branch, and in a new tab too when Open it in a new tab is ticked, which stays as it was last left. Create stays disabled while the branch is taken, the name is refused by Git, or the folder exists and isn't empty; the reason shows under the field. Enter creates, Esc cancels.",
          source: 'src/repo/worktree_dialog.rs',
          render: () => overGraph(C.WorktreeDialog({ branch: 'topic/tabs', folder: 'E:\\Projects\\sternegit-worktrees\\tabs', parent: 'E:\\Projects\\sternegit-worktrees' })),
        },
        {
          id: 'lock-worktree',
          title: 'Worktrees — lock a worktree',
          about: "Lock… asks for an optional reason, then runs git worktree lock. The worktree's row then shows a lock, whose tooltip gives the reason, its menu offers Unlock in place of Lock, and Remove worktree… waits until it is unlocked.",
          source: 'src/repo/worktree_menu.rs',
          render: () => overGraph(C.LockWorktreeDialog({ tree: dreamy, reason: 'on the backup drive' })),
        },
        {
          id: 'remove-worktree',
          title: 'Worktrees — remove a worktree',
          about: "Remove worktree… always asks, because it deletes the folder and Git keeps no copy of its uncommitted changes. The question names the folder, counts the changes that are lost, and says the commits stay on the branch. Also delete the branch is offered when the branch is merged into the current one, and stays off otherwise. Remove runs git worktree remove --force, then deletes the branch if asked; a tab open on the worktree closes.",
          source: 'src/repo/worktree_menu.rs',
          render: () => overGraph(C.RemoveWorktreeDialog({ tree: diffView })),
        },
      ];
    })(),
  ];
})();
