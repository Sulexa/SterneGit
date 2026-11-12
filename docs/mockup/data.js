const Sample = (() => {
  const linear = [
    'Read refs and history with gix',
    'Paint lanes with six theme colors',
    'Show the newest 10,000 commits',
    'Time history reads with --check',
    'Group thousands in counts',
    'Open the current directory by default',
    'Keep the selection on refresh',
    'Draw merge nodes smaller',
    'Ring the HEAD commit',
    'Truncate long branch names',
    'Move through history with j and k',
    'Copy the full hash with Ctrl+Shift+C',
    'Show commit bodies under the subject',
    'List parents in the commit details',
    'Strip symbols from release builds',
    'Use thin LTO for release builds',
    'Set up the GPUI window',
    'Initial commit',
  ];
  const authors = ['Alex Martin', 'Sam Lee', 'Robin Chen'];
  const emails = { 'Alex Martin': 'alex@sternegit.dev', 'Sam Lee': 'sam@sternegit.dev', 'Robin Chen': 'robin@sternegit.dev' };
  const hex = (seed) => ((seed * 2654435761) >>> 0).toString(16).padStart(8, '0').slice(0, 7);

  const commits = [
    { subject: 'Show, stage and unstage uncommitted changes', author: 'Alex Martin', date: '2026-11-04 10:31', graph: [0, 1, 'O0'], head: true, refs: [['main', 'local', true], ['origin/main', 'remote']] },
    { subject: 'Split the diff parser out of the commit panel', author: 'Alex Martin', date: '2026-11-04 09:12', graph: [0, 2, 'I0 O0'] },
    { subject: 'Render side-by-side diffs with syntax colors', author: 'Sam Lee', date: '2026-11-03 18:42', graph: [1, 2, 'P0 O1'], refs: [['feature/diff-view', 'local'], ['origin/feature/diff-view', 'remote']] },
    { subject: "Merge branch 'topic/tabs'", author: 'Alex Martin', date: '2026-11-03 17:05', graph: [0, 3, 'I0 P1 O0 O2'], merge: true },
    { subject: 'Add Split and Unified toggles', author: 'Sam Lee', date: '2026-11-03 15:20', graph: [1, 3, 'P0 I1 O1 P2'] },
    { subject: 'Keep one tab per repository', author: 'Robin Chen', date: '2026-11-03 11:47', graph: [2, 3, 'P0 P1 I2 O2'], refs: [['topic/tabs', 'local']] },
    { subject: 'Add a menu top bar', author: 'Alex Martin', date: '2026-11-02 20:13', graph: [0, 3, 'I0 O0 P1 P2'], refs: [['upstream/main', 'remote'], ['v0.2.0', 'tag']] },
    { subject: 'Open each repository in its own tab', author: 'Robin Chen', date: '2026-11-02 16:58', graph: [2, 3, 'P0 P1 I2 O0'] },
    { subject: 'Fetch remotes and pick up outside changes', author: 'Alex Martin', date: '2026-11-02 10:04', graph: [0, 2, 'I0 O0 P1'], refs: [['feature/stash', 'local'], ['origin/feature/stash', 'remote']] },
    { subject: 'Parse git diff output into hunks', author: 'Sam Lee', date: '2026-11-01 22:36', graph: [1, 2, 'P0 I1 O0'], refs: [['feature/blame', 'local']] },
    { subject: 'Add read-only commit graph prototype', author: 'Alex Martin', date: '2026-11-01 14:09', graph: [0, 1, 'I0 O0'], refs: [['v0.1.0', 'tag']] },
    ...linear.map((subject, ix) => ({
      subject,
      author: authors[ix % authors.length],
      date: `2026-10-${String(31 - ix).padStart(2, '0')} ${String(9 + (ix * 5) % 12).padStart(2, '0')}:${String((ix * 17) % 60).padStart(2, '0')}`,
      graph: [0, 1, ix === linear.length - 1 ? 'I0' : 'I0 O0'],
    })),
  ].map((commit, ix) => ({ ...commit, email: emails[commit.author], id: hex(ix + 7), refs: commit.refs || [] }));

  const refs = [
    { name: 'feature/blame', kind: 'local', row: 9 },
    { name: 'feature/diff-view', kind: 'local', row: 2 },
    { name: 'feature/stash', kind: 'local', row: 8 },
    { name: 'main', kind: 'local', row: 0, head: true },
    { name: 'topic/tabs', kind: 'local', row: 5 },
    { name: 'origin/feature/diff-view', kind: 'remote', row: 2 },
    { name: 'origin/feature/stash', kind: 'remote', row: 8 },
    { name: 'origin/main', kind: 'remote', row: 0 },
    { name: 'upstream/main', kind: 'remote', row: 6 },
    { name: 'v0.1.0', kind: 'tag', row: 10 },
    { name: 'v0.2.0', kind: 'tag', row: 6 },
  ];

  const collapsed = ['remote:upstream'];

  const history = {
    name: 'sternegit',
    head: 'main',
    total: 1284,
    readMs: 38,
    truncated: false,
    commits,
    refs,
  };

  const tracked = (() => {
    const apart = {
      main: { ahead: 2, upstream: 'origin/main' },
      'origin/main': { row: 3 },
      'feature/diff-view': { row: 4, behind: 1, upstream: 'origin/feature/diff-view' },
    };
    const moved = refs.map((label) => ({ ...label, ...apart[label.name] }));
    const badges = (row) => moved.filter((label) => label.row === row).map(({ name, kind, head }) => (head ? [name, kind, true] : [name, kind]));
    return { ...history, refs: moved, commits: commits.map((commit, row) => ({ ...commit, refs: badges(row) })) };
  })();

  const stashed = (() => {
    const entries = [
      { message: `WIP on main: ${commits[0].id} ${commits[0].subject}`, author: 'Alex Martin', date: '2026-11-04 11:02', base: 0, graph: [0, 1, 'O0'] },
      { message: 'On feature/diff-view: Try a wider gutter', author: 'Sam Lee', date: '2026-11-03 19:10', base: 2, graph: [1, 2, 'P0 O1'] },
    ];
    const rowOf = (row) => row + entries.filter((stash) => stash.base <= row).length;
    const stashRows = entries.map((stash, ix) => ({
      subject: stash.message,
      author: stash.author,
      email: emails[stash.author],
      date: stash.date,
      graph: stash.graph,
      stash: `stash@{${ix}}`,
      id: hex(100 + ix),
      refs: [],
    }));
    const rejoined = { 0: [0, 1, 'I0 O0'], 2: [1, 2, 'P0 I1 O1'] };
    const shown = [];
    commits.forEach((commit, row) => {
      entries.forEach((stash, ix) => stash.base === row && shown.push(stashRows[ix]));
      shown.push(rejoined[row] ? { ...commit, graph: rejoined[row] } : commit);
    });
    const stashes = entries.map((stash, ix) => ({ name: stashRows[ix].stash, message: stash.message, row: shown.indexOf(stashRows[ix]) }));
    return {
      history: { ...history, commits: shown, refs: refs.map((label) => ({ ...label, row: rowOf(label.row) })), stashes },
      details: {
        body: '',
        parents: [commits[2].id],
        files: [
          { path: 'src/git/command.rs', kind: 'M', additions: 12, deletions: 3, hunks: [] },
          { path: 'src/git/patch.rs', kind: 'M', additions: 4, deletions: 1, hunks: [] },
          { path: 'src/ui/toast.rs', kind: 'A', additions: 40, deletions: 0, hunks: [] },
        ],
      },
    };
  })();

  const pickedLine = {
    rows: [2, 4, 9],
    from: commits[10].id,
    files: [
      { path: 'Cargo.toml', kind: 'M', additions: 2, deletions: 0, hunks: [] },
      { path: 'src/git/patch.rs', kind: 'A', additions: 142, deletions: 0, hunks: [] },
      { path: 'src/repo/commit_files.rs', kind: 'A', additions: 98, deletions: 0, hunks: [] },
      { path: 'src/repo/commit_panel.rs', kind: 'M', additions: 12, deletions: 64, hunks: [] },
      { path: 'src/repo/file_diff_panel.rs', kind: 'M', additions: 87, deletions: 15, hunks: [] },
      { path: 'src/repo/view_bar.rs', kind: 'A', additions: 46, deletions: 0, hunks: [] },
    ],
  };

  const pickedApart = { rows: [1, 2, 3, 4] };

  const pickedHead = {
    rows: [0, 1],
    from: commits[3].id,
    files: [
      { path: 'src/git/status.rs', kind: 'A', additions: 96, deletions: 0, hunks: [] },
      { path: 'src/repo/changes_list.rs', kind: 'A', additions: 210, deletions: 0, hunks: [] },
      { path: 'src/repo/commit_panel.rs', kind: 'M', additions: 8, deletions: 112, hunks: [] },
      { path: 'src/repo/mod.rs', kind: 'M', additions: 34, deletions: 6, hunks: [] },
      { path: 'src/repo/patch_view.rs', kind: 'A', additions: 120, deletions: 0, hunks: [] },
    ],
  };

  const changes = {
    unstaged: [
      { kind: 'M', path: 'src/git/command.rs' },
      { kind: 'M', path: 'src/git/patch.rs' },
      { kind: 'M', path: 'src/git/status.rs' },
      { kind: '?', path: 'docs/notes.md' },
    ],
    staged: [
      { kind: 'A', path: 'src/ui/toast.rs' },
      { kind: 'R', path: 'src/repo/patch_view.rs', original: 'src/repo/diff_view.rs' },
    ],
  };

  const commitDetails = {
    body: 'The file comparison now uses the kit\'s diff view in split mode.\nSyntax colors come from syntect, which the kit already bundles.',
    parents: ['b47d0a8'],
    files: [
      {
        path: 'src/repo/file_diff_panel.rs',
        kind: 'M',
        additions: 31,
        deletions: 4,
        hunks: [
          {
            header: '@@ -48,9 +48,12 @@ impl FileDiffPanel {',
            lines: [
              [' ', 48, 48, 'pub(crate) fn new(repository: Entity<Repository>, cx: &mut Context<Self>) -> Self {'],
              ['-', 49, null, '    let diff = cx.new(|cx| DiffState::new([], cx));'],
              ['+', null, 49, '    let diff = cx.new(|cx| DiffState::new([], cx).with_mode(DiffMode::Split));'],
              [' ', 50, 50, '    let diff_changed = cx.observe(&diff, |_, _, cx| cx.notify());'],
              [' ', 51, 51, '    Self {'],
              [' ', 52, 52, '        repository,'],
              ['+', null, 53, '        open: None,'],
              ['+', null, 54, '        patch: Load::Idle,'],
              [' ', 53, 55, '        diff,'],
            ],
          },
          {
            header: '@@ -112,6 +115,14 @@ impl FileDiffPanel {',
            lines: [
              [' ', 112, 115, '    fn set_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>) {'],
              ['-', 113, null, '        todo!()'],
              ['+', null, 116, '        self.diff.update(cx, |state, cx| state.set_mode(mode, cx));'],
              [' ', 114, 117, '    }'],
            ],
          },
        ],
      },
      {
        path: 'src/repo/commit_files.rs',
        kind: 'A',
        additions: 98,
        deletions: 0,
        hunks: [],
      },
      {
        path: 'src/repo/patch_view.rs',
        kind: 'R',
        original: 'src/repo/diff_view.rs',
        additions: 12,
        deletions: 6,
        hunks: [
          {
            header: '@@ -21,7 +21,9 @@ pub(crate) fn patch_content<T>(',
            lines: [
              [' ', 21, 21, '    match load {'],
              ['-', 22, null, '        Load::Idle => div(),'],
              ['+', null, 22, '        Load::Idle => div().into_any_element(),'],
              ['+', null, 23, '        Load::Loading { .. } => ui::loading(texts.loading),'],
              [' ', 23, 24, '        Load::Failed(failure) => ui::failure(texts.failed, failure.message(), cx),'],
            ],
          },
        ],
      },
      {
        path: 'docs/mockup/history.png',
        kind: 'D',
        binary: true,
        additions: 0,
        deletions: 0,
        hunks: [],
      },
      {
        path: 'Cargo.toml',
        kind: 'M',
        additions: 5,
        deletions: 2,
        hunks: [
          {
            header: '@@ -6,4 +6,7 @@ edition = "2024"',
            lines: [
              [' ', 6, 6, '[dependencies]'],
              [' ', 7, 7, 'anyhow = "1.0.104"'],
              ['-', 8, null, 'gpui-kit = "0.9.0"'],
              ['+', null, 8, 'gpui-kit = { version = "0.9.0", features = ["syntect"] }'],
            ],
          },
        ],
      },
    ],
  };

  const panelSource = [
    '',
    'impl FileDiffPanel {',
    'const ROWS_ABOVE_A_CHANGE: usize = 2;',
    '',
    'pub(crate) fn new(repository: Entity<Repository>, cx: &mut Context<Self>) -> Self {',
    '    let diff = cx.new(|cx| DiffState::new([], cx).with_mode(DiffMode::Split));',
    '    let diff_changed = cx.observe(&diff, |_, _, cx| cx.notify());',
    '    Self {',
    '        repository,',
    '        open: None,',
    '        patch: Load::Idle,',
    '        diff,',
    '        stepped: None,',
    '        _diff_changed: diff_changed,',
    '    }',
    '}',
    '',
    'pub(crate) fn open_working_file(&mut self, repo: &Repo, change: FileChange, side: Side, cx: &mut Context<Self>) {',
    '    let path = change.path.clone();',
    '    self.open = Some(Open { side, path: path.clone() });',
    '    self.patch = Load::Loading { since: Instant::now() };',
    '    let repo = repo.clone();',
    '    let task = cx.background_spawn(async move { repo.file_patch(&change, side) });',
    '    cx.spawn(async move |this, cx| {',
    '        let patch = task.await;',
    '        this.update(cx, |this, cx| this.show_patch(&path, patch, cx))',
    '    })',
    '    .detach();',
    '    cx.notify();',
    '}',
    '',
    'fn show_patch(&mut self, path: &str, patch: Result<FilePatch>, cx: &mut Context<Self>) {',
    '    if self.open.as_ref().is_none_or(|open| open.path != path) {',
    '        return;',
    '    }',
    '    self.patch = match patch {',
    '        Ok(patch) => Load::Ready(patch),',
    '        Err(error) => Load::Failed(Failure::new(error)),',
    '    };',
    '    self.stepped = None;',
    '    cx.notify();',
    '}',
    '',
    'fn close(&mut self, cx: &mut Context<Self>) {',
    '    self.open = None;',
    '    self.patch = Load::Idle;',
    '    self.diff.update(cx, |state, cx| state.clear(cx));',
    '    cx.emit(Closed);',
    '}',
    '',
    'fn mode(&self, cx: &App) -> DiffMode {',
    '    self.diff.read(cx).mode()',
    '}',
  ];

  const commitFile = commitDetails.files[0];

  const commitFileDiff = {
    change: { kind: commitFile.kind, path: commitFile.path },
    commit: commits[2].id,
    parent: commitDetails.parents[0],
    additions: commitFile.additions,
    deletions: commitFile.deletions,
    hunks: commitFile.hunks,
    lines: 404,
    unchanged: (n) => panelSource[n - 44],
  };

  const fileHistory = {
    path: commitFile.path,
    entries: [
      { row: 0, kind: 'M', additions: 64, deletions: 12 },
      { row: 1, kind: 'M', additions: 18, deletions: 41 },
      { row: 2, kind: 'M', additions: commitFile.additions, deletions: commitFile.deletions },
      { row: 4, kind: 'M', additions: 22, deletions: 3 },
      { row: 7, kind: 'M', additions: 6, deletions: 6 },
      { row: 9, kind: 'R', original: 'src/diff_panel.rs', additions: 40, deletions: 8 },
      { row: 10, kind: 'A', additions: 87, deletions: 0 },
    ].map((entry) => ({ ...entry, commit: commits[entry.row] })),
  };

  const hunk = (oldStart, newStart, scope, lines) => {
    let oldNo = oldStart;
    let newNo = newStart;
    const numbered = lines.map(([sign, text]) => [sign, sign === '+' ? null : oldNo++, sign === '-' ? null : newNo++, text]);
    const oldCount = lines.filter(([sign]) => sign !== '+').length;
    const newCount = lines.filter(([sign]) => sign !== '-').length;
    return { header: `@@ -${oldStart},${oldCount} +${newStart},${newCount} @@ ${scope}`, lines: numbered };
  };

  const counts = (hunks) => {
    const lines = hunks.flatMap((h) => h.lines);
    return { additions: lines.filter(([sign]) => sign === '+').length, deletions: lines.filter(([sign]) => sign === '-').length };
  };

  const conflictedBranch = [
    [' ', 'impl Repo {'],
    [' ', '    pub(crate) fn file_patch(&self, change: &FileChange, side: Side) -> Result<FilePatch> {'],
    ['-', '        let args = diff_args(change, side);'],
    ['+', '        if change.kind == ChangeKind::Conflicted {'],
    ['+', '            return self.conflicted(change);'],
    ['+', '        }'],
    ['+', '        let args = diff_args(change, side)?;'],
    [' ', '        let output = self.git(&args)?;'],
    [' ', '        Ok(FilePatch::Diff(output))'],
    [' ', '    }'],
  ];

  const conflictedReader = [
    [' ', '    }'],
    [' ', ''],
    ['+', '    fn conflicted(&self, change: &FileChange) -> Result<FilePatch> {'],
    ['+', '        let contents = std::fs::read_to_string(self.workdir().join(&change.path))?;'],
    ['+', '        Ok(FilePatch::Conflicted { path: change.path.clone(), contents })'],
    ['+', '    }'],
    ['+', ''],
    [' ', '    fn untracked(&self, path: &str) -> Result<String> {'],
    [' ', '        let null = if cfg!(windows) { "NUL" } else { "/dev/null" };'],
  ];

  const pathArguments = [
    [' ', '    let mut args = vec!["diff".to_owned(), "--no-color".to_owned()];'],
    [' ', '    if side == Side::Staged {'],
    [' ', '        args.push("--cached".to_owned());'],
    [' ', '    }'],
    ['-', '    args.push("--".to_owned());'],
    ['-', '    args.push(change.path.clone());'],
    ['+', '    args.extend(["--".to_owned(), change.path.clone()]);'],
    [' ', '    Ok(args)'],
  ];

  const patchFile = { kind: 'M', path: 'src/git/patch.rs' };

  const patchSource = [
    'use std::path::Path;',
    '',
    'use super::{FileChange, Repo, Result, Side};',
    'use crate::git::{ChangeKind, context::with_whole_file};',
    '',
    '#[derive(Clone, Debug)]',
    'pub(crate) enum FilePatch {',
    '    Diff(String),',
    '    Binary { path: String },',
    '    Untracked(String),',
    '    Conflicted { path: String, contents: String },',
    '}',
    '',
    'impl Repo {',
    '    pub(crate) fn file_patch(&self, change: &FileChange, side: Side) -> Result<FilePatch> {',
    '        if change.kind == ChangeKind::Conflicted {',
    '            return self.conflicted(change);',
    '        }',
    '        let args = diff_args(change, side)?;',
    '        let output = self.git(&args)?;',
    '        Ok(FilePatch::Diff(output))',
    '    }',
    '',
    '    pub(crate) fn file_text(&self, path: &Path) -> Result<String> {',
    '        let bytes = std::fs::read(self.workdir().join(path))?;',
    '        if bytes.contains(&0) {',
    '            return Ok(String::new());',
    '        }',
    '        Ok(String::from_utf8_lossy(&bytes).into_owned())',
    '    }',
    '',
    '    fn conflicted(&self, change: &FileChange) -> Result<FilePatch> {',
    '        let contents = std::fs::read_to_string(self.workdir().join(&change.path))?;',
    '        Ok(FilePatch::Conflicted { path: change.path.clone(), contents })',
    '    }',
    '',
    '    fn untracked(&self, path: &str) -> Result<String> {',
    '        let null = if cfg!(windows) { "NUL" } else { "/dev/null" };',
    '        let args = ["diff", "--no-color", "--no-index", "--", null, path];',
    '        self.git_allowing_changes(&args)',
    '    }',
    '',
    '    pub(crate) fn with_whole_file(&self, patch: String, side: Side, path: &Path) -> String {',
    '        match self.file_text(path) {',
    '            Ok(text) if side == Side::Unstaged => with_whole_file(&patch, &text).unwrap_or(patch),',
    '            _ => patch,',
    '        }',
    '    }',
    '}',
    '',
    'fn diff_args(change: &FileChange, side: Side) -> Result<Vec<String>> {',
    '    if change.path.is_empty() {',
    '        anyhow::bail!("a change without a path");',
    '    }',
    '    if side == Side::Staged && change.kind == ChangeKind::Untracked {',
    '        anyhow::bail!("{} is not in the index", change.path);',
    '    }',
    '    log::trace!("diff {} ({side:?})", change.path);',
    '',
    '    let mut args = vec!["diff".to_owned(), "--no-color".to_owned()];',
    '    if side == Side::Staged {',
    '        args.push("--cached".to_owned());',
    '    }',
    '    args.extend(["--".to_owned(), change.path.clone()]);',
    '    Ok(args)',
    '}',
    '',
    '#[cfg(test)]',
    'mod tests {',
    '    use super::{FileChange, Side, diff_args};',
    '    use crate::git::ChangeKind;',
    '',
    '    #[test]',
    '    fn staged_changes_compare_the_index_with_head() {',
    '        let change = FileChange { kind: ChangeKind::Modified, path: "src/main.rs".into() };',
    '        let args = diff_args(&change, Side::Staged).unwrap();',
    '        assert_eq!(args, ["diff", "--no-color", "--cached", "--", "src/main.rs"]);',
    '    }',
    '',
    '    #[test]',
    '    fn a_change_without_a_path_has_no_arguments() {',
    '        let change = FileChange { kind: ChangeKind::Modified, path: String::new() };',
    '        assert!(diff_args(&change, Side::Unstaged).is_err());',
    '    }',
    '}',
  ];

  const patchLine = (n) => patchSource[n - 1];

  const workingDiff = (side, hunks, lines, unchanged = patchLine) => ({ change: patchFile, side, ...counts(hunks), hunks, lines, unchanged });

  const fileDiff = workingDiff('unstaged', [
    hunk(14, 14, 'pub(crate) enum FilePatch {', conflictedBranch),
    hunk(27, 30, 'impl Repo {', conflictedReader),
    hunk(52, 60, 'fn diff_args(change: &FileChange, side: Side) -> Result<Vec<String>> {', pathArguments),
  ], 183);

  const stagedHunkDiff = workingDiff('staged', [hunk(14, 14, 'pub(crate) enum FilePatch {', conflictedBranch)], 179, (n) => patchLine(n <= 31 ? n : n + 5));

  const partlyStagedDiff = {
    ...workingDiff('unstaged', [
      hunk(30, 30, 'impl Repo {', conflictedReader),
      hunk(55, 60, 'fn diff_args(change: &FileChange, side: Side) -> Result<Vec<String>> {', pathArguments),
    ], 183),
    stagedToo: true,
  };

  const partlyStaged = {
    unstaged: changes.unstaged,
    staged: [...changes.staged, patchFile],
  };

  const selectedLines = { hunk: 0, from: 3, to: 5 };

  const draft = {
    message: 'Show conflicted files in the changes list',
    description: 'git status marks them UU. The list gives them a U badge\nand the comparison shows the file with its markers.',
  };

  const worktrees = (() => {
    const none = { unstaged: [], staged: [] };
    const diffViewChanges = {
      unstaged: [
        { kind: 'M', path: 'src/repo/file_diff_panel.rs' },
        { kind: 'M', path: 'src/repo/patch_view.rs' },
        { kind: '?', path: 'docs/mockup/split.png' },
      ],
      staged: [],
    };
    const list = [
      { name: 'sternegit', path: 'E:\\Projects\\sternegit', branch: 'main', row: 0, main: true, changes, changed: '2026-11-04 11:02' },
      { name: 'diff-view', path: 'E:\\Projects\\sternegit-worktrees\\diff-view', branch: 'feature/diff-view', row: 2, changes: diffViewChanges, changed: '2026-11-04 10:48' },
      { name: 'blame', path: 'E:\\Projects\\sternegit-worktrees\\blame', branch: 'feature/blame', row: 9, locked: 'kept for the blame benchmark', changes: none, changed: '2026-11-01 22:40' },
      { name: 'dreamy-grothendieck', path: 'E:\\Projects\\sternegit\\.claude\\worktrees\\dreamy-grothendieck', row: 5, merged: true, changes: { unstaged: [{ kind: '?', path: 'notes/tab-order.md' }], staged: [] }, changed: '2026-11-03 12:15' },
      { name: 'stash-ui', path: 'D:\\Temp\\stash-ui', branch: 'feature/stash', row: 8, merged: true, missing: true, changes: none, changed: '2026-11-02 10:30' },
    ].map((tree) => ({ ...tree, id: commits[tree.row].id }));

    const seenFrom = (name) => {
      const here = list.find((tree) => tree.name === name);
      const others = list.filter((tree) => tree !== here);
      const elsewhere = Object.fromEntries(others.filter((tree) => tree.branch).map((tree) => [tree.branch, tree.name]));
      const detached = Object.fromEntries(others.filter((tree) => !tree.branch).map((tree) => [tree.row, tree.name]));
      return {
        ...history,
        head: here.branch,
        elsewhere,
        refs: refs.map((label) => ({ ...label, head: label.name === here.branch, worktree: elsewhere[label.name] })),
        commits: commits.map((commit, row) => ({
          ...commit,
          head: row === here.row,
          worktree: detached[row],
          refs: commit.refs.map(([refName, kind]) => (refName === here.branch ? [refName, kind, true] : [refName, kind])),
        })),
      };
    };

    const withChanges = (view, wips) => {
      const shown = [];
      view.commits.forEach((commit, row) => {
        const wip = wips.find((each) => each.at === row);
        if (wip) shown.push({ wip: wip.tree, graph: wip.graph, refs: [], subject: '', id: '' });
        shown.push(wip ? { ...commit, graph: wip.rejoined, worktree: undefined } : commit);
      });
      const rowOf = (row) => row + wips.filter((each) => each.at <= row).length;
      return { ...view, commits: shown, refs: view.refs.map((label) => ({ ...label, row: rowOf(label.row) })), wipRow: (name) => shown.findIndex((commit) => commit.wip && commit.wip.name === name) };
    };

    const dreamyChanges = { tree: list[3], at: 5, graph: [3, 4, 'P0 P1 P2 O3'], rejoined: [2, 4, 'P0 P1 I2 I3 O2'] };
    const inGraph = withChanges(seenFrom('sternegit'), [{ tree: list[1], at: 2, graph: [1, 2, 'P0 O1'], rejoined: [1, 2, 'P0 I1 O1'] }, dreamyChanges]);
    const inGraphFromDiffView = withChanges(seenFrom('diff-view'), [{ tree: list[0], at: 0, graph: [0, 1, 'O0'], rejoined: [0, 1, 'I0 O0'] }, dreamyChanges]);

    return { list, diffViewChanges, inGraph, inGraphFromDiffView };
  })();

  const tabs = [
    { name: 'sternegit', state: 'ready' },
    { name: 'website', state: 'ready' },
    { name: 'linux', state: 'reading' },
  ];

  const starfall = {
    name: 'starfall',
    head: 'feature/slime-crown',
    total: 3906,
    readMs: 61,
    truncated: false,
    refs: [
      { name: 'feature/boss-music', kind: 'local', row: 4 },
      { name: 'feature/slime-crown', kind: 'local', row: 0, head: true },
      { name: 'main', kind: 'local', row: 2 },
      { name: 'release/0.4', kind: 'local', row: 7 },
      { name: 'origin/feature/boss-music', kind: 'remote', row: 4 },
      { name: 'origin/main', kind: 'remote', row: 2 },
      { name: 'origin/release/0.4', kind: 'remote', row: 7 },
      { name: 'v0.3.0', kind: 'tag', row: 12 },
      { name: 'v0.4.0-beta', kind: 'tag', row: 7 },
    ],
  };

  const assets = {
    unstaged: [
      { kind: 'M', path: 'Assets/Art/Characters/slime.png' },
      { kind: 'M', path: 'Assets/Audio/Music/boss-theme.ogg' },
      { kind: 'M', path: 'Assets/Video/intro.mp4' },
      { kind: 'M', path: 'Assets/Models/slime.fbx' },
      { kind: 'M', path: 'Assets/Scripts/Enemies/SlimeController.cs' },
    ],
    staged: [
      { kind: 'A', path: 'Assets/Art/UI/crown-icon.png' },
      { kind: 'M', path: 'ProjectSettings/QualitySettings.asset' },
    ],
  };

  const onDisk = ['HEAD', 'Working copy'];

  const slime = {
    palette: { k: '#14532d', g: '#4ade80', d: '#16a34a', h: '#bbf7d0', w: '#f8fafc', p: '#0f172a', m: '#14532d', s: '#00000059', y: '#facc15', o: '#ca8a04', r: '#ef4444' },
    before: [
      '........................',
      '........................',
      '........................',
      '........................',
      '........................',
      '........................',
      '........................',
      '........................',
      '.........kkkkkk.........',
      '.......kkggggggkk.......',
      '......kghhhggggggk......',
      '.....kghhgggggggggk.....',
      '....kgghggggggggggdk....',
      '....kgggwwggggwwggdk....',
      '...kggggwpggggpwggddk...',
      '...kggggwpggggpwggddk...',
      '...kggggggggggggggddk...',
      '...kggggggmggmggggddk...',
      '...kdddddddmmdddddddk...',
      '....kddddddddddddddk....',
      '.....kkkkkkkkkkkkkk.....',
      '.....ssssssssssssss.....',
      '.......ssssssssss.......',
      '........................',
    ],
    after: [
      '........................',
      '........................',
      '........................',
      '........................',
      '........y..yy..y........',
      '........yy.yy.yy........',
      '........yyyrryyy........',
      '........yyyrryyy........',
      '........oooooooo........',
      '.......kkggggggkk.......',
      '......kghhhggggggk......',
      '.....kghhgggggggggk.....',
      '....kgghggggggggggdk....',
      '....kgggwwggggwwggdk....',
      '...kggggwpggggpwggddk...',
      '...kggggwpggggpwggddk...',
      '...kgggggmmmmmmgggddk...',
      '...kgggggmwwwwmgggddk...',
      '...kddddddmmmmddddddk...',
      '....kddddddddddddddk....',
      '.....kkkkkkkkkkkkkk.....',
      '......ssssssssssss......',
      '........ssssssss........',
      '........................',
    ],
    areas: [
      { x: 8, y: 4, w: 8, h: 5 },
      { x: 9, y: 16, w: 6, h: 3 },
      { x: 5, y: 21, w: 14, h: 2 },
    ],
  };

  const slimeDiff = {
    kind: 'image',
    change: assets.unstaged[0],
    side: 'unstaged',
    sources: onDisk,
    facts: '24 × 24 PNG',
    image: slime,
    details: {
      columns: ['Size', 'Color', 'Format', 'File size'],
      before: ['24 × 24 px', 'RGBA, 8 bits', 'PNG', '742 B'],
      after: ['24 × 24 px', 'RGBA, 8 bits', 'PNG', '913 B'],
      deltas: { 3: '+171 B' },
    },
  };

  const crownIcon = {
    kind: 'image',
    change: assets.staged[0],
    side: 'staged',
    sources: ['', 'Staged'],
    facts: '24 × 24 PNG',
    image: slime,
    details: {
      columns: ['Size', 'Color', 'Format', 'File size'],
      before: ['', '', '', ''],
      after: ['24 × 24 px', 'RGBA, 8 bits', 'PNG', '913 B'],
    },
  };

  const introDiff = {
    kind: 'video',
    change: assets.unstaged[2],
    side: 'unstaged',
    sources: onDisk,
    lfs: true,
    facts: '0:41.00 → 0:44.50',
    duration: { before: 41, after: 44.5 },
    at: 12.48,
    changes: [[0, 3.2], [11.5, 16], [28, 31], [41, 44.5]],
    current: 1,
    hearing: 'after',
    details: {
      columns: ['Resolution', 'Frame rate', 'Duration', 'Video', 'Audio', 'Bit rate', 'File size'],
      before: ['1920 × 1080', '30 fps', '0:41.00', 'H.264', 'AAC, 48 kHz', '8.2 Mb/s', '42.1 MB'],
      after: ['1920 × 1080', '60 fps', '0:44.50', 'H.264', 'AAC, 48 kHz', '9.6 Mb/s', '53.4 MB'],
      deltas: { 2: '+3.50 s', 6: '+11.3 MB' },
    },
  };

  const themeDiff = {
    kind: 'audio',
    change: assets.unstaged[1],
    side: 'unstaged',
    sources: onDisk,
    lfs: true,
    facts: '2:41.3 → 2:47.9',
    duration: { before: 161.3, after: 167.9 },
    at: 78.4,
    changes: [[31.5, 38], [72, 89.5], [156, 167.9]],
    current: 1,
    hearing: 'after',
    song: {
      before: [[0, 16, 0.2, 0.45], [16, 31.5, 0.5, 0.55], [31.5, 38, 0.55, 0.6], [38, 72, 0.78, 0.82], [72, 89.5, 0.3, 0.36], [89.5, 136, 0.84, 0.86], [136, 156, 0.56, 0.5], [156, 161.3, 0.45, 0]],
      after: [[0, 16, 0.2, 0.45], [16, 31.5, 0.5, 0.55], [31.5, 38, 0.86, 0.95], [38, 72, 0.78, 0.82], [72, 89.5, 0.6, 0.68], [89.5, 136, 0.84, 0.86], [136, 156, 0.56, 0.5], [156, 162, 0.55, 0.64], [162, 167.9, 0.64, 0]],
    },
    details: {
      columns: ['Duration', 'Sample rate', 'Channels', 'Codec', 'Bit rate', 'Loudness', 'Peak', 'File size'],
      before: ['2:41.3', '48 kHz', 'Stereo', 'Vorbis', '192 kb/s', '−14.2 LUFS', '−1.0 dB', '3.7 MB'],
      after: ['2:47.9', '48 kHz', 'Stereo', 'Vorbis', '192 kb/s', '−12.8 LUFS', '−0.3 dB', '3.9 MB'],
      deltas: { 0: '+6.6 s', 5: '+1.4' },
    },
  };

  const modelDiff = {
    kind: 'binary',
    change: assets.unstaged[3],
    side: 'unstaged',
    sources: onDisk,
    lfs: true,
    type: 'FBX',
    facts: '2.4 MB → 2.6 MB',
    details: {
      columns: ['Type', 'File size', 'Last changed'],
      before: ['FBX 7.4, binary', '2.4 MB', '2026-10-28 by Robin Chen'],
      after: ['FBX 7.4, binary', '2.6 MB', 'Not committed yet'],
      deltas: { 1: '+212 KB' },
    },
  };

  const unityWork = {
    staged: [{ kind: 'M', path: 'Assets/Scripts/Enemies/SlimeController.cs' }],
    unstaged: [
      { kind: 'M', path: 'Assets/Prefabs/Enemies/Slime.prefab' },
      { kind: 'M', path: 'Assets/Scenes/Level1.unity' },
      { kind: 'M', path: 'Assets/Materials/SlimeGlow.mat' },
      { kind: 'M', path: 'Assets/Animations/Slime/SlimeJump.anim' },
      { kind: 'M', path: 'Assets/Animations/Slime/Slime.controller' },
      { kind: 'M', path: 'Assets/Graphs/SlimeBrain.asset' },
      { kind: 'M', path: 'Assets/Art/Characters/slime.png', meta: true },
      { kind: 'M', path: 'Assets/Audio/Sfx/slime-hit.wav.meta' },
      { kind: 'M', path: 'Assets/UI/MainMenu_Canvas.prefab' },
    ],
  };

  const v = (...axes) => ({ vec: axes });
  const rgba = (hex, alpha = 1, hdr) => ({ color: hex, alpha, hdr });
  const ref = (name, type, glyph = 'circle-dot') => ({ ref: name, type, glyph });
  const none = (type) => ({ none: type });
  const on = { bool: true };
  const off = { bool: false };
  const tex = (name, art, palette) => ({ texture: name, art, palette });

  const noiseArt = {
    palette: { a: '#1f2937', b: '#4b5563', c: '#9ca3af', d: '#e5e7eb' },
    rows: ['abcbdcba', 'bcdcbabc', 'cdcbabcd', 'bcbabcdc', 'dbabcdcb', 'cabcdcba', 'babcdcbd', 'abcdcbdc'],
  };

  const mask = (rows, keep) => rows.map((row, y) => [...row].map((cell, x) => (cell !== '.' && keep(cell, x, y) ? cell : '.')).join(''));
  const isCrown = (cell) => 'yor'.includes(cell);
  const isShadow = (cell) => cell === 's';
  const isEye = (cell, x, y) => 'wp'.includes(cell) && y >= 13 && y <= 15;
  const isBody = (cell, x, y) => !isCrown(cell) && !isShadow(cell) && !isEye(cell, x, y);
  const sparks = [[2, 16], [4, 18], [1, 19], [3, 21], [5, 20], [0, 17]];
  const particleRows = slime.after.map((row, y) => [...row].map((cell, x) => (sparks.some(([sx, sy]) => sx === x && sy === y) ? 'h' : '.')).join(''));
  const partOf = (keep, scale = 12) => ({ palette: slime.palette, rows: mask(slime.after, keep), old: mask(slime.before, keep), scale });

  const prefabDiff = {
    kind: 'objects',
    unity: 'Prefab',
    change: unityWork.unstaged[0],
    side: 'unstaged',
    sources: onDisk,
    facts: 'Prefab · 4 objects changed',
    hidden: 3,
    tree: [
      { name: 'Slime', depth: 0, state: 'changed', open: true, prefab: true },
      { name: 'Body', depth: 1, open: true },
      { name: 'Highlight', depth: 2 },
      { name: 'Eyes', depth: 1 },
      { name: 'Crown', depth: 1, state: 'added' },
      { name: 'Shadow', depth: 1, state: 'changed' },
      { name: 'SquishParticles', depth: 1, state: 'removed' },
    ],
    selected: 'Slime',
    layout: {
      unit: 192,
      grid: 0.25,
      origin: [-2.45, 1.3],
      view: 'Prefab mode, seen from the front · 5 changes without a place are in Inspector',
      gizmos: [{ name: 'Circle Collider 2D', at: [-0.03, -0.22], before: 0.45, after: 0.5, note: 'Radius 0.45 → 0.5' }],
      items: [
        { name: 'Shadow', art: 'shadow', from: [0, 0], at: [0, 0], state: 'changed', note: 'Scale 1 × 0.3 → 0.9 × 0.25' },
        { name: 'Body', art: 'body', at: [0, 0], silent: true },
        { name: 'Eyes', art: 'eyes', at: [0, 0], silent: true },
        { name: 'Crown', art: 'crown', at: [0, 0], state: 'added', label: 'right', note: 'Sprite crown-icon' },
        { name: 'SquishParticles', art: 'particles', from: [0, 0], state: 'removed', label: 'left' },
      ],
      picked: 'Shadow',
    },
    sprites: {
      shadow: partOf(isShadow),
      body: partOf(isBody),
      eyes: partOf(isEye),
      crown: partOf(isCrown),
      particles: { palette: slime.palette, rows: particleRows, scale: 12 },
    },
    objects: [
      {
        name: 'Slime',
        state: 'changed',
        prefab: 'Root of Slime.prefab',
        components: [
          { name: 'Rigidbody 2D', props: [['Mass', '1', '1.4']] },
          {
            name: 'Slime Controller',
            script: 'SlimeController.cs',
            props: [
              ['Move Speed', '2.5', '3'],
              ['Jump Force', '6', '7.5'],
              ['Crown', none('Transform'), ref('Crown', 'Transform', 'box')],
              ['Drops', '2 items', '3 items', [['Element 2', undefined, ref('CrownGem', 'Item Data', 'file-box')]]],
            ],
          },
          { name: 'Circle Collider 2D', props: [['Radius', '0.45', '0.5']] },
        ],
      },
      {
        name: 'Crown',
        parent: 'Slime',
        state: 'added',
        components: [
          { name: 'Transform', props: [['Position', undefined, v(0, 0.62, 0)], ['Scale', undefined, v(0.8, 0.8, 1)]] },
          { name: 'Sprite Renderer', props: [['Sprite', undefined, ref('crown-icon', 'Sprite', 'image')], ['Order in Layer', undefined, '3']] },
        ],
      },
      {
        name: 'Shadow',
        parent: 'Slime',
        state: 'changed',
        components: [{ name: 'Transform', props: [['Scale', v(1, 0.3, 1), v(0.9, 0.25, 1)]] }],
      },
      {
        name: 'SquishParticles',
        parent: 'Slime',
        state: 'removed',
        folded: true,
        components: [{ name: 'Transform', props: [] }, { name: 'Particle System', props: [] }],
      },
    ],
  };

  const prefabYamlAround = [
    [208, ['  m_SortingLayerID: 0', '  m_SortingOrder: 2', '  m_Sprite: {fileID: 21300000, guid: 9a8b7c6d5e4f40312a1b0c9d8e7f6a5b, type: 3}', '--- !u!50 &5208833104513330127']],
    [
      219,
      [
        '  m_AngularDamping: 0.05',
        '  m_Material: {fileID: 0}',
        '  m_IncludeLayers:',
        '    serializedVersion: 2',
        '    m_Bits: 0',
        '  m_ExcludeLayers:',
        '    serializedVersion: 2',
        '    m_Bits: 0',
        '  m_Interpolate: 1',
        '  m_SleepingMode: 1',
        '  m_CollisionDetection: 1',
        '  m_Constraints: 4',
        '--- !u!95 &1592374480263370021',
        'Animator:',
        '  serializedVersion: 7',
        '  m_ObjectHideFlags: 0',
        '  m_GameObject: {fileID: 3381795302254875041}',
        '  m_Enabled: 1',
        '  m_Avatar: {fileID: 0}',
        '  m_Controller: {fileID: 9100000, guid: 5e4d3c2b1a0948f7e6d5c4b3a2918070, type: 2}',
        '  m_CullingMode: 0',
        '  m_UpdateMode: 0',
        '  m_ApplyRootMotion: 0',
        '  m_LinearVelocityBlending: 0',
        '  m_StabilizeFeet: 0',
        '  m_WarningMessage: ',
        '  m_KeepAnimatorStateOnDisable: 0',
        '--- !u!114 &8045172693318120554',
      ],
    ],
    [
      257,
      [
        '  hurtSound: {fileID: 8300000, guid: 1b2c3d4e5f60718293a4b5c6d7e8f901, type: 3}',
        '  invincibleTime: 0.6',
        '  knockback: {x: 4, y: 3}',
        '--- !u!4 &1148820317265581290',
        'Transform:',
        '  m_ObjectHideFlags: 0',
        '  m_GameObject: {fileID: 3381795302254875041}',
        '  serializedVersion: 2',
        '  m_LocalRotation: {x: 0, y: 0, z: 0, w: 1}',
        '  m_LocalPosition: {x: 0, y: 0, z: 0}',
        '  m_LocalScale: {x: 1, y: 1, z: 1}',
        '  m_ConstrainProportionsScale: 0',
        '  m_Children: []',
        '  m_Father: {fileID: 0}',
      ],
    ],
  ];

  const prefabYaml = {
    ...prefabDiff,
    kind: 'text',
    lines: 412,
    unchanged: (n) => {
      if (n < 1 || n > 412) return undefined;
      const block = prefabYamlAround.find(([from, lines]) => n >= from && n < from + lines.length);
      return block ? block[1][n - block[0]] : '  m_ObjectHideFlags: 0';
    },
    hints: [
      ['fileID: 3381795302254875041', 'Slime'],
      ['guid: 9a8b7c6d5e4f40312a1b0c9d8e7f6a5b', 'Slime.png'],
      ['guid: 5e4d3c2b1a0948f7e6d5c4b3a2918070', 'Slime.controller'],
      ['guid: 1b2c3d4e5f60718293a4b5c6d7e8f901', 'Hurt.wav'],
      ['guid: 4f1d0c7a2b9e4e3f8a6b5c4d3e2f1a09', 'SlimeController.cs'],
      ['fileID: 4823015530731772630', 'Crown (Transform)'],
      ['fileID: 2290457312861453377', 'Crown (Sprite Renderer)'],
      ['guid: 0c2b7e4d9a1f4b8e9d3c2a1b0f9e8d7c', 'Coin.asset'],
      ['guid: 7e6d5c4b3a291807f6e5d4c3b2a19087', 'Heart.asset'],
      ['guid: c3a1e9f07b2d4c5e8f6a9b0c1d2e3f4a', 'CrownGem.asset'],
    ],
    hunks: [
      hunk(212, 212, 'Slime › Rigidbody 2D', [
        [' ', 'Rigidbody2D:'],
        [' ', '  m_GameObject: {fileID: 3381795302254875041}'],
        [' ', '  m_BodyType: 0'],
        [' ', '  m_UseAutoMass: 0'],
        ['-', '  m_Mass: 1'],
        ['+', '  m_Mass: 1.4'],
        [' ', '  m_LinearDamping: 0'],
        [' ', '  m_GravityScale: 3'],
      ]),
      hunk(247, 247, 'Slime › Slime Controller', [
        [' ', 'MonoBehaviour:'],
        [' ', '  m_GameObject: {fileID: 3381795302254875041}'],
        [' ', '  m_Script: {fileID: 11500000, guid: 4f1d0c7a2b9e4e3f8a6b5c4d3e2f1a09, type: 3}'],
        ['-', '  moveSpeed: 2.5'],
        ['-', '  jumpForce: 6'],
        ['-', '  crown: {fileID: 0}'],
        ['+', '  moveSpeed: 3'],
        ['+', '  jumpForce: 7.5'],
        ['+', '  crown: {fileID: 4823015530731772630}'],
        [' ', '  drops:'],
        [' ', '  - {fileID: 11400000, guid: 0c2b7e4d9a1f4b8e9d3c2a1b0f9e8d7c, type: 2}'],
        [' ', '  - {fileID: 11400000, guid: 7e6d5c4b3a291807f6e5d4c3b2a19087, type: 2}'],
        ['+', '  - {fileID: 11400000, guid: c3a1e9f07b2d4c5e8f6a9b0c1d2e3f4a, type: 2}'],
      ]),
      hunk(301, 302, 'Slime › Circle Collider 2D', [
        [' ', '  m_Offset: {x: 0, y: 0.1}'],
        ['-', '  m_Radius: 0.45'],
        ['+', '  m_Radius: 0.5'],
      ]),
      hunk(330, 331, 'Crown', [
        ['+', '--- !u!1 &6106237841907316552'],
        ['+', 'GameObject:'],
        ['+', '  m_Component:'],
        ['+', '  - component: {fileID: 4823015530731772630}'],
        ['+', '  - component: {fileID: 2290457312861453377}'],
        ['+', '  m_Name: Crown'],
      ]),
    ],
  };
  Object.assign(prefabYaml, counts(prefabYaml.hunks));

  const menuDiff = {
    kind: 'objects',
    unity: 'Prefab',
    change: unityWork.unstaged[8],
    side: 'unstaged',
    sources: onDisk,
    facts: 'Prefab · UI · 4 objects changed',
    hidden: 0,
    tree: [
      { name: 'MainMenu_Canvas', depth: 0, open: true, prefab: true },
      { name: 'Background_Image', depth: 1 },
      { name: 'Logo', depth: 1, state: 'changed' },
      { name: 'Buttons', depth: 1, open: true },
      { name: 'Play_Button', depth: 2 },
      { name: 'Characters_Button', depth: 2, state: 'added' },
      { name: 'Options_Button', depth: 2 },
      { name: 'Quit_Button', depth: 2 },
      { name: 'Discord_Label', depth: 1, state: 'changed' },
      { name: 'Instagram_Button', depth: 1, state: 'removed' },
      { name: 'Version_Label', depth: 1 },
    ],
    selected: 'Discord_Label',
    layout: {
      canvas: { name: 'MainMenu_Canvas', size: [1920, 1080], note: 'Screen Space - Overlay · 1920 × 1080 from Canvas Scaler' },
      view: 'UI, drawn as Unity draws a canvas: one unit per pixel',
      items: [
        { name: 'Background_Image', rect: [0, 0, 1920, 1080], fill: '#2a1f4a' },
        { name: 'Version_Label', rect: [1560, 990, 320, 60], text: { value: 'v0.4.2', size: 30, color: '#a1a1aa', align: 'end' } },
        { name: 'Logo', from: [610, 700, 700, 230], rect: [522, 671, 876, 288], fill: '#f472b6', radius: 36, text: { value: 'STICKERINO', size: 120, oldSize: 96, color: '#ffffff', bold: true }, state: 'changed', note: 'Scale 1 → 1.25', label: 'right' },
        { name: 'Play_Button', rect: [710, 534, 500, 96], fill: '#fde047', radius: 20, text: { value: 'Play', size: 46, color: '#1f1535', bold: true } },
        { name: 'Characters_Button', rect: [710, 406, 500, 96], fill: '#fde047', radius: 20, text: { value: 'Characters', size: 46, color: '#1f1535', bold: true }, state: 'added', label: 'right' },
        { name: 'Options_Button', rect: [710, 278, 500, 96], fill: '#fde047', radius: 20, text: { value: 'Options', size: 46, color: '#1f1535', bold: true } },
        { name: 'Quit_Button', rect: [710, 150, 500, 96], fill: '#fde047', radius: 20, text: { value: 'Quit', size: 46, color: '#1f1535', bold: true } },
        { name: 'Discord_Label', from: [60, 40, 760, 100], rect: [60, 40, 760, 100], text: { value: 'Found a bug? Join our Discord', old: 'Join our Discord', size: 38, color: '#e9d5ff', align: 'start' }, state: 'changed', note: 'Text', label: 'above' },
        { name: 'Instagram_Button', from: [1790, 40, 90, 90], fill: '#a78bfa', radius: 22, text: { value: 'IG', size: 34, color: '#1f1535', bold: true }, state: 'removed', label: 'left' },
      ],
      picked: 'Discord_Label',
    },
    objects: [
      {
        name: 'Logo',
        parent: 'MainMenu_Canvas',
        state: 'changed',
        components: [{ name: 'Rect Transform', props: [['Scale', v(1, 1, 1), v(1.25, 1.25, 1)]] }],
      },
      {
        name: 'Characters_Button',
        parent: 'Buttons',
        state: 'added',
        components: [
          { name: 'Rect Transform', props: [['Size', undefined, v(500, 96)]] },
          { name: 'Image', props: [['Color', undefined, rgba('#fde047')]] },
          { name: 'Button', props: [] },
        ],
      },
      {
        name: 'Discord_Label',
        parent: 'MainMenu_Canvas',
        state: 'changed',
        components: [{ name: 'TextMeshPro - Text (UI)', props: [['Text', 'Join our Discord', 'Found a bug? Join our Discord']] }],
      },
      {
        name: 'Instagram_Button',
        parent: 'MainMenu_Canvas',
        state: 'removed',
        folded: true,
        components: [{ name: 'Rect Transform', props: [] }, { name: 'Image', props: [] }, { name: 'Button', props: [] }],
      },
    ],
  };

  const levelDiff = {
    kind: 'objects',
    unity: 'Scene',
    change: unityWork.unstaged[1],
    side: 'unstaged',
    sources: onDisk,
    facts: 'Scene · 5 objects changed',
    hidden: 5,
    tree: [
      { name: 'Main Camera', depth: 0, state: 'changed' },
      { name: 'Global Light 2D', depth: 0, state: 'changed' },
      { name: 'Grid', depth: 0, open: false },
      { name: 'Player', depth: 0, prefab: true },
      { name: 'Enemies', depth: 0, open: true },
      { name: 'Slime (1)', depth: 1, prefab: true },
      { name: 'Slime (2)', depth: 1, prefab: true, state: 'changed' },
      { name: 'Slime (3)', depth: 1, prefab: true, state: 'added' },
      { name: 'Bat (1)', depth: 1, prefab: true, state: 'removed' },
      { name: 'UI', depth: 0, open: false },
    ],
    selected: 'Main Camera',
    objects: [
      {
        name: 'Main Camera',
        state: 'changed',
        components: [{ name: 'Camera', props: [['Size', '5', '5.5'], ['Background', rgba('#0f172a'), rgba('#1e1b4b')]] }],
      },
      {
        name: 'Global Light 2D',
        state: 'changed',
        components: [{ name: 'Light 2D', props: [['Intensity', '1', '0.85'], ['Color', rgba('#ffffff'), rgba('#fde68a')]] }],
      },
      {
        name: 'Slime (2)',
        parent: 'Enemies',
        state: 'changed',
        prefab: 'Slime.prefab',
        components: [{ name: 'Transform', overrides: true, props: [['Position', v(4, -1.5, 0), v(7, -1.5, 0)]] }],
      },
      {
        name: 'Slime (3)',
        parent: 'Enemies',
        state: 'added',
        prefab: 'Slime.prefab',
        components: [
          { name: 'Transform', overrides: true, props: [['Position', undefined, v(9, -1.5, 0)]] },
          { name: 'Slime Controller', overrides: true, script: 'SlimeController.cs', props: [['Move Speed', undefined, '3.5']] },
        ],
      },
      {
        name: 'Bat (1)',
        parent: 'Enemies',
        state: 'removed',
        prefab: 'Bat.prefab',
        folded: true,
        components: [{ name: 'Transform', props: [] }],
      },
    ],
    layout: {
      unit: 40,
      origin: [-5.75, 6.6],
      camera: { at: [6, 0.5], aspect: 16 / 9, before: 5, after: 5.5 },
      ground: { top: -2, bottom: -6, from: -6, to: 18 },
      items: [
        { name: 'Player', at: [-1, -1.5], art: 'player' },
        { name: 'Slime (1)', at: [1.5, -1.5], art: 'slime' },
        { name: 'Slime (2)', from: [4, -1.5], at: [7, -1.5], art: 'slime', state: 'changed', note: 'X 4 → 7' },
        { name: 'Slime (3)', at: [11, -1.5], art: 'slime', state: 'added' },
        { name: 'Bat (1)', from: [9.5, 2.5], art: 'bat', state: 'removed' },
        { name: 'Global Light 2D', at: [1, 3.6], art: 'light', state: 'changed', note: 'Intensity 1 → 0.85' },
        { name: 'Main Camera', at: [6, 0.5], art: 'camera', state: 'changed', note: 'Size 5 → 5.5' },
      ],
      picked: 'Slime (2)',
      view: 'Seen from the front, X and Y',
    },
    sprites: {
      slime: { palette: slime.palette, rows: slime.after, old: slime.before, scale: 1.5 },
      player: {
        scale: 3,
        palette: { w: '#e2e8f0', b: '#1e3a8a', c: '#60a5fa', o: '#f97316', g: '#64748b' },
        rows: ['...wwww...', '..wwwwww..', '..wbbbbw..', '..wbccbw..', '..wwwwww..', '.wwwwwwww.', '.wwowwoww.', '.wwwwwwww.', '..ww..ww..', '..ww..ww..', '.ggg..ggg.'],
      },
      bat: {
        scale: 3,
        palette: { k: '#7c3aed', r: '#fde047' },
        rows: ['k..........k', 'kk...kk...kk', 'kkk.kkkk.kkk', 'kkkkkrrkkkkk', '.kkkkkkkkkk.', '..kk.kk.kk..'],
      },
    },
  };

  const materialDiff = {
    kind: 'objects',
    unity: 'Material',
    change: unityWork.unstaged[2],
    side: 'unstaged',
    sources: onDisk,
    facts: 'Material · Shader Graphs/SlimeGlow',
    hidden: 1,
    objects: [
      {
        name: 'SlimeGlow',
        state: 'changed',
        type: 'Material',
        shader: 'Shader Graphs/SlimeGlow',
        components: [
          {
            name: 'Surface Inputs',
            props: [
              ['Base Color', rgba('#4ade80'), rgba('#22c55e')],
              ['Emission Color', rgba('#bbf7d0', 1, 0), rgba('#bbf7d0', 1, 1.5)],
              ['Noise Map', none('Texture'), ref('noise-soft', 'Texture', 'image')],
              ['Pulse Speed', '1', '2.5'],
              ['Rim Width', '0.2', '0.35'],
            ],
          },
          { name: 'Keywords', props: [['_EMISSION', undefined, 'On']] },
          { name: 'Advanced Options', props: [['Enable GPU Instancing', off, on]] },
        ],
      },
    ],
  };

  const jumpDiff = {
    kind: 'curves',
    unity: 'Animation clip',
    change: unityWork.unstaged[3],
    side: 'unstaged',
    sources: onDisk,
    facts: 'Animation clip · 0:30 → 0:35',
    fps: 60,
    sprite: { palette: slime.palette, rows: slime.after, old: slime.before },
    length: { before: 30, after: 35 },
    at: 17,
    current: 0,
    tracks: [
      { group: 'Slime : Transform', name: 'Position.y', state: 'changed', range: [-0.1, 1.2], before: [[0, 0], [15, 0.8], [30, 0]], after: [[0, 0], [17, 1.1], [35, 0]] },
      { group: 'Slime : Transform', name: 'Scale.x', state: 'changed', range: [0.6, 1.45], before: [[0, 1], [4, 1.2], [15, 0.9], [28, 1.25], [30, 1]], after: [[0, 1], [4, 1.3], [17, 0.85], [32, 1.35], [35, 1]] },
      { group: 'Slime : Transform', name: 'Scale.y', state: 'changed', range: [0.55, 1.25], before: [[0, 1], [4, 0.8], [15, 1.1], [28, 0.75], [30, 1]], after: [[0, 1], [4, 0.7], [17, 1.15], [32, 0.65], [35, 1]] },
      { group: 'Slime : Sprite Renderer', name: 'Sprite', state: 'changed', sprites: { before: [[0, 1], [12, 1.12], [26, 0.78]], after: [[0, 1], [14, 1.12], [30, 0.72], [33, 0.6]] } },
      { group: 'Slime : Sprite Renderer', name: 'Color.a', state: 'added', range: [0, 1.15], after: [[0, 1], [30, 1], [33, 0.6], [35, 1]] },
      { group: 'Slime : Transform', name: 'Rotation.z', state: 'removed', range: [-12, 12], before: [[0, 0], [10, -8], [20, 8], [30, 0]] },
    ],
    events: { before: [[28, 'OnLand']], after: [[32, 'OnLand'], [33, 'PlaySquish']] },
    details: {
      columns: ['Length', 'Sample rate', 'Loop time', 'Curves', 'Events'],
      before: ['0:30 · 0.50 s', '60', 'Off', '5', '1'],
      after: ['0:35 · 0.58 s', '60', 'Off', '5', '2'],
      deltas: { 0: '+5 frames' },
    },
  };

  const animatorDiff = {
    kind: 'animator',
    unity: 'Animator controller',
    change: unityWork.unstaged[4],
    side: 'unstaged',
    sources: onDisk,
    facts: 'Animator controller · Base Layer',
    current: 2,
    states: [
      { id: 'entry', name: 'Entry', role: 'entry', at: [20, 150] },
      { id: 'any', name: 'Any State', role: 'any', at: [20, 380] },
      { id: 'idle', name: 'Idle', role: 'default', at: [176, 150] },
      { id: 'hop', name: 'Hop', at: [326, 64] },
      { id: 'fall', name: 'Fall', at: [470, 150] },
      { id: 'land', name: 'Land', at: [470, 290] },
      { id: 'squish', name: 'Squish', state: 'added', at: [326, 370] },
      { id: 'stunned', name: 'Stunned', state: 'removed', at: [176, 460] },
    ],
    transitions: [
      ['entry', 'idle'],
      ['idle', 'hop', 'changed'],
      ['hop', 'fall'],
      ['fall', 'land'],
      ['land', 'idle', 'removed'],
      ['land', 'squish', 'added'],
      ['squish', 'idle', 'added'],
      ['any', 'stunned', 'removed'],
      ['stunned', 'idle', 'removed'],
    ],
    parameters: [['Jump', 'Trigger'], ['Speed', 'Float'], ['IsGrounded', 'Bool'], ['IsCrowned', 'Bool', 'added']],
    changes: [
      ['added', 'State', 'Squish', 'SlimeSquish.anim'],
      ['removed', 'State', 'Stunned', 'SlimeStunned.anim'],
      ['changed', 'Transition', 'Idle → Hop', '3 settings'],
      ['added', 'Transition', 'Land → Squish', 'Exit time 0.9'],
      ['added', 'Transition', 'Squish → Idle', 'Exit time 1'],
      ['removed', 'Transition', 'Land → Idle', ''],
      ['removed', 'Transition', 'Any State → Stunned', 'Hurt'],
      ['added', 'Parameter', 'IsCrowned', 'Bool'],
    ],
    picked: {
      name: 'Idle → Hop',
      type: 'Transition',
      components: [
        {
          name: 'Transition',
          props: [
            ['Conditions', 'Jump', 'Jump, IsGrounded is true'],
            ['Has Exit Time', on, off],
            ['Transition Duration', '0.25', '0.1'],
          ],
        },
      ],
    },
  };

  const slimeMetaDiff = {
    ...slimeDiff,
    change: unityWork.unstaged[6],
    facts: '24 × 24 PNG · import settings changed',
    imports: {
      file: 'slime.png.meta',
      columns: ['Pixels Per Unit', 'Filter Mode', 'Compression', 'Max Size'],
      before: ['16', 'Bilinear', 'Normal Quality', '2048'],
      after: ['24', 'Point (no filter)', 'None', '256'],
      same: 21,
    },
  };

  const guidDiff = {
    kind: 'objects',
    unity: 'Meta',
    change: unityWork.unstaged[7],
    side: 'unstaged',
    sources: onDisk,
    facts: 'Import settings of slime-hit.wav',
    guid: {
      asset: 'slime-hit.wav',
      users: [
        ['Assets/Prefabs/Enemies/Slime.prefab', 'Slime › Slime Controller › Hit Sound'],
        ['Assets/Scenes/Level1.unity', 'Audio Manager › Sound Effects › Element 4'],
        ['Assets/Data/Enemies/SlimeStats.asset', 'Hurt Sound'],
      ],
    },
    objects: [
      {
        name: 'slime-hit.wav',
        state: 'changed',
        type: 'Audio Importer',
        components: [
          { name: 'Asset', props: [['GUID', { mono: '9a3c51e07d2b4f6c8e1a0b2c3d4e5f60' }, { mono: '51e0c4a1b9f84d2e97c3a6b5d4e3f201' }]] },
          {
            name: 'Default settings',
            props: [
              ['Load Type', 'Decompress On Load', 'Compressed In Memory'],
              ['Preload Audio Data', off, on],
              ['Quality', '70', '100'],
            ],
          },
        ],
      },
    ],
  };

  const brainDiff = {
    kind: 'scriptgraph',
    unity: 'Script graph',
    change: unityWork.unstaged[5],
    side: 'unstaged',
    sources: onDisk,
    facts: 'Script graph · Visual Scripting',
    graph: 'SlimeBrain',
    current: 4,
    nodes: [
      { id: 'update', title: 'On Update', kind: 'event', icon: 'zap', at: [16, 56], width: 112, rows: [[null, { flow: true }]] },
      { id: 'if', title: 'If', kind: 'flow', icon: 'git-branch', at: [150, 56], width: 118, rows: [[{ flow: true }, { label: 'True', flow: true }], [{ label: 'Condition', type: 'bool' }, { label: 'False', flow: true }]] },
      { id: 'grounded', title: 'Get Variable', kind: 'variable', icon: 'box', at: [16, 166], width: 118, rows: [[{ label: 'IsGrounded', field: true }, { type: 'bool' }]] },
      { id: 'cooldown', title: 'Cooldown', kind: 'flow', icon: 'clock', state: 'changed', at: [292, 56], width: 150, rows: [[{ flow: true }, { label: 'Ready', flow: true }], [{ label: 'Duration', type: 'float', value: '0.6', was: '0.8' }, { label: 'Not Ready', flow: true }]] },
      { id: 'force', title: 'Add Force', kind: 'flow', icon: 'arrow-up', at: [458, 196], width: 128, rows: [[{ flow: true }, { flow: true }], [{ label: 'Target', type: 'object', value: 'This' }, null], [{ label: 'Force', type: 'vector' }, null], [{ label: 'Mode', type: 'object', value: 'Impulse' }, null]] },
      { id: 'log', title: 'Debug: Log', kind: 'flow', icon: 'message-square', state: 'removed', at: [458, 352], width: 128, rows: [[{ flow: true }, { flow: true }], [{ label: 'Message', type: 'object', value: '"jump"' }, null]] },
      { id: 'vector', title: 'Create Vector 2', kind: 'value', icon: 'move-horizontal', state: 'changed', at: [284, 262], width: 150, rows: [[{ label: 'X', type: 'float', value: '0' }, { type: 'vector' }], [{ label: 'Y', type: 'float', was: '6' }, null]] },
      { id: 'multiply', title: 'Multiply', kind: 'value', icon: 'close', state: 'added', at: [146, 346], width: 116, rows: [[{ label: 'A', type: 'float', value: '7.5' }, { type: 'float' }], [{ label: 'B', type: 'float' }, null]] },
      { id: 'bonus', title: 'Get Variable', kind: 'variable', icon: 'box', state: 'added', at: [16, 440], width: 128, rows: [[{ label: 'CrownBonus', field: true }, { type: 'float' }]] },
    ],
    wires: [
      ['update', 0, 'if', 0],
      ['grounded', 0, 'if', 1],
      ['if', 0, 'cooldown', 0],
      ['cooldown', 0, 'force', 0],
      ['vector', 0, 'force', 2],
      ['force', 0, 'log', 0, 'removed'],
      ['multiply', 0, 'vector', 1, 'added'],
      ['bonus', 0, 'multiply', 1, 'added'],
    ],
    variables: [['Speed', 'Float 3'], ['IsGrounded', 'Boolean'], ['CrownBonus', 'Float 1.25', 'added']],
    changes: [
      ['added', 'Node', 'Multiply', 'A 7.5'],
      ['added', 'Node', 'Get Variable', 'CrownBonus'],
      ['removed', 'Node', 'Debug: Log', '"jump"'],
      ['changed', 'Node', 'Cooldown', 'Duration'],
      ['changed', 'Node', 'Create Vector 2', 'Y'],
      ['added', 'Connection', 'Multiply → Y', 'Float'],
      ['added', 'Connection', 'CrownBonus → B', 'Float'],
      ['removed', 'Connection', 'Add Force → Log', 'Flow'],
      ['added', 'Variable', 'CrownBonus', 'Float'],
    ],
    picked: {
      id: 'vector',
      name: 'Create Vector 2',
      type: 'Node',
      components: [{ name: 'Inputs', props: [['Y', '6', ref('Multiply', 'Result', 'spline')]] }],
    },
  };

  const unity = { work: unityWork, prefabDiff, prefabYaml, menuDiff, levelDiff, materialDiff, jumpDiff, animatorDiff, brainDiff, slimeMetaDiff, guidDiff };

  const stoppedAt ={ id: '3f2a91c', subject: 'Show conflicted files in the changes list' };

  const rebasedCommits = [
    { id: '9c4e1d2', subject: 'Read conflicted files from git status', author: 'Alex Martin', date: '2026-11-05 09:40', graph: [0, 2, 'O0'], head: true, refs: [] },
    { id: 'b71d0e4', subject: 'Paint conflicted files magenta', author: 'Alex Martin', date: '2026-11-05 09:58', graph: [1, 2, 'P0 O1'], refs: [['main', 'local']] },
    { ...stoppedAt, author: 'Alex Martin', date: '2026-11-05 09:51', graph: [1, 2, 'P0 I1 O1'], refs: [] },
    { id: 'e05a7b3', subject: 'Read conflicted files from git status', author: 'Alex Martin', date: '2026-11-05 09:40', graph: [1, 2, 'P0 I1 O1'], refs: [] },
    { id: '5d8c2fa', subject: 'Show the fetch time in the status bar', author: 'Sam Lee', date: '2026-11-05 08:51', graph: [0, 2, 'I0 O0 P1'], refs: [['origin/main', 'remote']] },
    { id: 'a3e90b1', subject: 'Fetch in the background every five minutes', author: 'Sam Lee', date: '2026-11-04 17:22', graph: [0, 2, 'I0 O0 P1'], refs: [] },
    { ...commits[0], graph: [0, 2, 'I0 I1 O0'], head: false, refs: [] },
    ...commits.slice(1),
  ].map((commit) => ({ ...commit, email: emails[commit.author] }));

  const rebasing = {
    ...history,
    head: 'rebasing main',
    total: history.total + 5,
    commits: rebasedCommits,
    refs: history.refs.map((label) =>
      label.name === 'main'
        ? { ...label, row: 1, head: false }
        : label.name === 'origin/main'
          ? { ...label, row: 4 }
          : { ...label, row: label.row + 6 },
    ),
  };

  const rebase = { kind: 'rebase', branch: 'main', onto: 'origin/main', step: 2, total: 3, commit: stoppedAt };
  const merge = { kind: 'merge', from: 'feature/blame', into: 'main' };
  const starfallMerge = { kind: 'merge', from: 'main', into: 'feature/slime-crown' };

  const statusFile = { kind: 'U', path: 'src/git/status.rs' };
  const listFile = { kind: 'U', path: 'src/repo/changes_list.rs' };
  const badgesFile = { kind: 'M', path: 'src/ui/badges.rs' };

  const rebaseChanges = { conflicted: [statusFile, listFile], staged: [badgesFile], unstaged: [] };
  const rebaseOneLeft = { conflicted: [listFile], staged: [badgesFile, { ...statusFile, kind: 'M' }], unstaged: [] };
  const rebaseResolved = { conflicted: [], staged: [badgesFile, { ...listFile, kind: 'M' }, { ...statusFile, kind: 'M' }], unstaged: [] };

  const statusConflicts = {
    kind: 'conflict',
    change: statusFile,
    side: 'conflicted',
    lines: 386,
    labels: { current: 'HEAD', incoming: `${stoppedAt.id} (${stoppedAt.subject})` },
    parts: [
      {
        hidden: 29,
        tail: [
          'impl ChangeKind {',
          "    pub(crate) fn letter(self) -> &'static str {",
          '        match self {',
          '            ChangeKind::Added => "A",',
          '            ChangeKind::Modified => "M",',
          '            ChangeKind::Deleted => "D",',
        ],
      },
      { conflict: { current: ['            ChangeKind::Renamed => "R",'], incoming: ['            ChangeKind::Conflicted => "U",'] } },
      {
        head: [
          '            ChangeKind::Untracked => "?",',
          '        }',
          '    }',
          '}',
          '',
          'impl FileChange {',
          '    pub(crate) fn new(path: &[u8], original: Option<&[u8]>, kind: ChangeKind) -> Self {',
          '        Self { path: decode(path).into(), original: original.map(decode), kind }',
        ],
        hidden: 166,
        tail: [
          'fn parse_status(output: &[u8]) -> WorkingChanges {',
          '    let mut changes = WorkingChanges::default();',
          '    let mut entries = output.split(|&byte| byte == 0);',
          '    while let Some(entry) = entries.next() {',
          '        let (index, worktree) = (entry[0], entry[1]);',
          "        let original_path = matches!(index, b'R' | b'C').then(|| entries.next());",
        ],
      },
      {
        conflict: {
          current: ['        let path = String::from_utf8_lossy(&entry[3..]).into_owned();'],
          incoming: [
            '        let path = &entry[3..];',
            "        if index == b'U' || worktree == b'U' {",
            '            changes.conflicted.push(FileChange::new(path, None, ChangeKind::Conflicted));',
            '            continue;',
            '        }',
          ],
        },
      },
      {
        head: [
          "        if index == b'?' {",
          '            changes.unstaged.push(FileChange::new(path, None, ChangeKind::Untracked));',
          '            continue;',
          '        }',
          '        if let Some(kind) = ChangeKind::from_status_code(index) {',
          '            changes.staged.push(FileChange::new(path, original_path, kind));',
          '        }',
          '    }',
        ],
        hidden: 142,
      },
    ],
  };

  const statusMerged = {
    ...statusConflicts,
    current: 1,
    reveal: 1,
    resolutions: [
      { take: ['current', 'incoming'] },
      {
        take: ['incoming'],
        caret: 2,
        edited: [
          ['edited', '        let path = String::from_utf8_lossy(&entry[3..]).into_owned();'],
          ['incoming', "        if index == b'U' || worktree == b'U' {"],
          ['edited', '            changes.conflicted.push(FileChange::new(&path, None, ChangeKind::Conflicted));'],
          ['incoming', '            continue;'],
          ['incoming', '        }'],
        ],
      },
    ],
  };

  const slimeConflict = {
    ...slimeDiff,
    change: { kind: 'U', path: slimeDiff.change.path },
    side: 'conflicted',
    sources: ['Current', 'Incoming'],
    details: {
      ...slimeDiff.details,
      before: ['24 × 24 px', 'RGBA, 8 bits', 'PNG', '913 B'],
      after: ['24 × 24 px', 'RGBA, 8 bits', 'PNG', '742 B'],
      deltas: { 3: '−171 B' },
    },
    image: { ...slime, before: slime.after, after: slime.before },
  };

  const starfallConflicts = {
    conflicted: [slimeConflict.change],
    staged: [{ kind: 'M', path: 'Assets/Scripts/Enemies/SlimeController.cs' }, { kind: 'A', path: 'Assets/Audio/Effects/squish.wav' }],
    unstaged: [],
  };

  const githubScopes = ['repo', 'read:org', 'write:public_key', 'workflow'];

  const accounts = [
    {
      id: 'github:github.com:583231',
      login: 'alex-martin',
      name: 'Alex Martin',
      email: 'alex@sternegit.dev',
      service: 'GitHub',
      server: 'github.com',
      method: 'device',
      scopes: githubScopes,
      key: { title: 'SterneGit on DESKTOP-7Q2K', type: 'ed25519', fingerprint: 'SHA256:q3V2pXbN0c8m1Lr7tWkY4fJ9sHdE6aUoZiG5yCvBn2Q', file: 'github-github.com-583231_ed25519' },
      owners: ['starfall-games'],
      using: ['E:\\Projects\\sternegit', 'E:\\Projects\\website'],
    },
    {
      id: 'github:github.com:9120447',
      login: 'alex-acme',
      name: 'Alex Martin',
      email: 'alex@acme.dev',
      service: 'GitHub',
      server: 'github.com',
      method: 'token',
      scopes: ['repo', 'read:org'],
      signedOut: true,
      owners: ['acme-web'],
      using: [],
    },
    {
      id: 'github:github.acme.dev:1042',
      login: 'amartin',
      name: 'Alex Martin',
      email: 'amartin@acme.dev',
      service: 'GitHub Enterprise Server',
      server: 'github.acme.dev',
      method: 'credential',
      scopes: ['repo', 'read:org'],
      owners: [],
      using: [],
    },
    {
      id: 'azure-devops:dev.azure.com:starfall.6c8f1a2b-4d3e-4f5a-9b8c-7d6e5f4a3b2c',
      login: 'alex@starfall.games',
      name: 'Alex Martin',
      email: 'alex@starfall.games',
      service: 'Azure DevOps',
      server: 'dev.azure.com/starfall',
      method: 'token',
      scopes: [],
      owners: [],
      using: [],
    },
  ];

  const serverKeys = [
    { title: 'SterneGit on DESKTOP-7Q2K', fingerprint: 'SHA256:q3V2pXbN0c8m1Lr7tWkY4fJ9sHdE6aUoZiG5yCvBn2Q', added: '2026-11-06', sternegit: true },
    { title: 'Laptop', fingerprint: 'SHA256:Hk81bR0vXyQe3sTnPwU5mZ7cLd2fGa9jK4oIi6NtVrE', added: '2025-03-14' },
    { title: 'Deploy key for the website', fingerprint: 'SHA256:W2nJx5eYp8RtQ0vLsK3dM7cBu1hZa6gFo9iT4wPyNqE', added: '2024-09-02', readOnly: true },
  ];

  const githubGrant = {
    reach: [
      { origin: 'https://api.github.com/', token: true },
      { origin: 'https://github.com/', token: false },
      { origin: '{server}/api/v3/', token: true, typed: true },
      { origin: '{server}/', token: false, typed: true },
    ],
    images: ['https://avatars.githubusercontent.com/'],
    remotes: ['github.com', 'ssh.github.com', '{server.host}'],
    signIn: [],
    features: ['Browse and clone repositories', 'Create and publish repositories', 'Add SSH keys'],
  };

  const gitlabGrant = {
    reach: [
      { origin: 'https://gitlab.com/api/v4/', token: true },
      { origin: 'https://gitlab.com/', token: false },
      { origin: '{server}/api/v4/', token: true, typed: true },
    ],
    images: ['https://secure.gravatar.com/avatar/'],
    remotes: ['gitlab.com', 'altssh.gitlab.com', '{server.host}'],
    signIn: ['https://gitlab.com/oauth/authorize_device', 'https://gitlab.com/oauth/token'],
    features: ['Browse and clone repositories', 'Create and publish repositories', 'Add SSH keys', 'Merge requests: list, review, merge'],
  };

  const connectors = [
    { id: 'github', name: 'GitHub', version: '0.1.0', origin: 'bundled', standing: 'ready', grant: githubGrant, accounts: 3 },
    { id: 'gitlab', name: 'GitLab', version: '1.2.0', origin: 'installed', standing: 'waiting', grant: gitlabGrant, accounts: 0 },
    { id: 'gitea', name: 'Gitea', version: '0.3.1', was: '0.3.0', origin: 'installed', standing: 'updated', grant: { reach: [{ origin: '{server}/api/v1/', token: true, typed: true }], images: [], remotes: ['{server.host}'], signIn: [], features: ['Browse and clone repositories'] }, accounts: 1 },
    {
      id: 'azure-devops',
      name: 'Azure DevOps',
      version: '0.1.0',
      origin: 'bundled',
      standing: 'ready',
      grant: {
        reach: [
          { origin: '{organization}/', token: true, typed: true },
          { origin: '{collection}/', token: true, typed: true },
        ],
        images: [],
        remotes: ['{organization.host}', 'ssh.dev.azure.com', 'vs-ssh.visualstudio.com', '{collection.host}'],
        signIn: ['https://login.microsoftonline.com/organizations/oauth2/v2.0/devicecode', 'https://login.microsoftonline.com/organizations/oauth2/v2.0/token'],
        features: ['Browse and clone repositories', 'Create and publish repositories', 'Pull requests: list, create, merge'],
      },
      accounts: 1,
    },
    { id: 'sourcehut', name: 'SourceHut', version: '0.9.0', origin: 'installed', standing: 'unusable', problem: 'Needs a newer SterneGit: it speaks protocol 2.0, and this SterneGit speaks 1.0.' },
    { id: 'bitbucket', name: 'Bitbucket', version: '0.2.0', origin: 'installed', standing: 'failed', problem: 'Stopped while starting: it ran out of time.', log: ['info  hello from bitbucket 0.2.0', 'debug building the route table', 'debug building the route table'] },
  ];

  const gitlabUpdate = { ...connectors[1], version: '1.3.0', was: '1.2.0', added: { reach: [{ origin: 'https://gitlab.com/api/graphql', token: true }], images: [], remotes: [], signIn: [], features: ['Merge requests: resolve threads'] } };

  const owners = [
    { login: 'alex-martin', kind: 'user', visibilities: ['public', 'private'], canInitialize: true },
    { login: 'sternegit-org', kind: 'organization', visibilities: ['public', 'private', 'internal'], canInitialize: true },
    { login: 'starfall-games', kind: 'organization', visibilities: ['public'], canInitialize: true, cannotCreate: true },
  ];

  const repositories = [
    { owner: 'sternegit-org', name: 'sternegit', description: 'A lightweight Git client for Linux, macOS and Windows', visibility: 'public', updated: '2026-11-06' },
    { owner: 'sternegit-org', name: 'connectors', description: 'Community connectors for SterneGit', visibility: 'public', updated: '2026-11-05' },
    { owner: 'alex-martin', name: 'starfall', description: 'A tiny space shooter made in Unity', visibility: 'private', updated: '2026-11-04' },
    { owner: 'sternegit-org', name: 'website', description: 'sternegit.dev', visibility: 'public', updated: '2026-11-02' },
    { owner: 'alex-martin', name: 'gpui-kit', description: 'UI components for GPUI', visibility: 'public', updated: '2026-10-28', fork: true },
    { owner: 'alex-martin', name: 'dotfiles', description: '', visibility: 'private', updated: '2026-10-19' },
    { owner: 'sternegit-org', name: 'design', description: 'Logos, icons and screenshots', visibility: 'internal', updated: '2026-09-30' },
    { owner: 'alex-martin', name: 'advent-2025', description: 'Advent of Code 2025 in Rust', visibility: 'public', updated: '2025-12-25', archived: true },
    { owner: 'alex-martin', name: 'notes', description: '', visibility: 'private', updated: '2025-11-12' },
  ];

  const hosting = { accounts, serverKeys, connectors, gitlabUpdate, owners, repositories, deviceCode: '8F2C-K7QD' };

  const pullList = {
    open: [
      { number: 14, title: 'Stash and pop from the header', author: 'alex-martin', head: 'feature/stash', base: 'main', state: 'open', review: 'changes_requested', checks: 'failure', created: '2026-11-02 10:30', updated: '2026-11-05 16:20' },
      { number: 12, title: 'Render side-by-side diffs with syntax colors', author: 'sam-lee', head: 'feature/diff-view', base: 'main', state: 'open', review: 'approved', checks: 'success', created: '2026-11-03 18:50', updated: '2026-11-04 08:15' },
      { number: 11, title: 'Blame view in the file comparison', author: 'sam-lee', head: 'feature/blame', base: 'main', state: 'open', draft: true, checks: 'running', created: '2026-11-01 22:40', updated: '2026-11-01 22:40' },
      { number: 9, title: 'Fix the build steps in the README', author: 'robin-chen', head: 'patch-1', headOwner: 'robin-chen', base: 'main', state: 'open', review: 'review_required', checks: 'pending', created: '2026-10-30 11:48', updated: '2026-10-30 12:02' },
    ],
    closed: [
      { number: 13, title: 'Keep one tab per repository', author: 'robin-chen', head: 'topic/tabs', base: 'main', state: 'merged', created: '2026-11-02 16:58', updated: '2026-11-03 17:05' },
      { number: 10, title: 'Try egui for the commit list', author: 'sam-lee', head: 'spike/egui', base: 'main', state: 'closed', created: '2026-10-28 14:02', updated: '2026-10-29 09:12' },
      { number: 8, title: 'Read history with libgit2', author: 'alex-martin', head: 'spike/libgit2', base: 'main', state: 'closed', created: '2026-10-26 10:15', updated: '2026-10-27 18:40' },
      { number: 7, title: 'Add a menu top bar', author: 'alex-martin', head: 'topic/menu', base: 'main', state: 'merged', created: '2026-11-02 09:30', updated: '2026-11-02 20:13' },
    ],
  };

  const pullBadges = {
    'feature/diff-view': { number: 12, state: 'open' },
    'feature/stash': { number: 14, state: 'open' },
    'feature/blame': { number: 11, state: 'open', draft: true },
  };

  const pulled = { ...history, pulls: pullBadges, refs: refs.map((label) => (label.kind === 'local' && pullBadges[label.name] ? { ...label, pull: pullBadges[label.name] } : label)) };

  const shownPull = pullList.open[1];

  const pullDetails = {
    body: "The file comparison now uses the kit's diff view in split mode, with syntax colors from syntect, which the kit already bundles: the exe grows by 40 KB.\n\nEach side keeps its own scroll position when you switch files.\n\nCloses #7.",
    headSha: '8ff3473',
    mergeBase: 'b47d0a8',
    commits: 3,
    mergeability: 'ready',
    methods: ['merge', 'squash', 'rebase'],
    method: 'squash',
    checks: [
      { name: 'build (windows-latest)', state: 'success', description: 'Successful in 4m 12s' },
      { name: 'build (ubuntu-latest)', state: 'success', description: 'Successful in 3m 05s' },
      { name: 'build (macos-latest)', state: 'success', description: 'Successful in 5m 40s' },
      { name: 'clippy', state: 'success', description: 'Successful in 1m 58s' },
      { name: 'size report', state: 'neutral', description: 'The exe grew by 40 KB' },
    ],
    conversation: [
      { author: 'alex-martin', date: '2026-11-03 19:02', body: 'Looks good. Could the split view keep the scroll position when switching files?' },
      { author: 'sam-lee', date: '2026-11-03 19:40', body: 'Done in 8ff3473: each side keeps its own scroll per file.' },
      { author: 'robin-chen', date: '2026-11-04 08:15', body: "Tried it on a 4K screen: the colors match the kit's theme, and the size report is fine." },
    ],
    files: commitDetails.files,
  };

  const failingPull = pullList.open[0];

  const failingDetails = {
    ...pullDetails,
    body: 'Pull, Push, Branch, Stash and Pop get a header above the graph. Pop brings back the newest stash.',
    headSha: '4540215',
    mergeBase: 'a708a7a',
    commits: 5,
    mergeability: 'failing',
    method: 'merge',
    checks: [
      { name: 'build (windows-latest)', state: 'success', description: 'Successful in 4m 02s' },
      { name: 'build (ubuntu-latest)', state: 'failure', description: 'Failing after 1m 12s: error[E0425]: cannot find value `stash_menu` in this scope' },
      { name: 'build (macos-latest)', state: 'running', description: 'Running for 3 minutes' },
      { name: 'clippy', state: 'pending', description: 'Waiting for a runner' },
    ],
    conversation: [{ author: 'robin-chen', date: '2026-11-05 16:20', body: 'Pop should be greyed out when there is no stash, as in the mockup.' }],
  };

  const forkPull = pullList.open[3];

  const forkDetails = {
    ...pullDetails,
    body: 'cargo build --release needs the wasm target for the connectors; the README now says so.',
    headSha: 'c41d7e0',
    mergeBase: '538453d',
    commits: 1,
    mergeability: 'blocked',
    blockedBy: 'At least one approving review is required by reviewers with write access.',
    checks: [{ name: 'build (windows-latest)', state: 'pending', description: 'Waiting for approval to run workflows from a fork' }],
    conversation: [],
    files: [{ path: 'README.md', kind: 'M', additions: 4, deletions: 1 }],
  };

  const pulls = { list: pullList, badges: pullBadges, pulled, shown: shownPull, details: pullDetails, failing: failingPull, failingDetails, fork: forkPull, forkDetails, mergedSha: '5d1c2e9' };

  return {
    history, tracked, stashed, collapsed, changes, partlyStaged, draft, commitDetails, commitFileDiff, fileHistory, fileDiff, stagedHunkDiff, partlyStagedDiff, selectedLines, tabs,
    pickedLine, pickedApart, pickedHead,
    starfall, assets, slimeDiff, crownIcon, introDiff, themeDiff, modelDiff,
    unity,
    rebasing, rebase, merge, starfallMerge, rebaseChanges, rebaseOneLeft, rebaseResolved, statusConflicts, statusMerged, slimeConflict, starfallConflicts,
    hosting,
    pulls,
    worktrees,
  };
})();
