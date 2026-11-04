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
  ].map((commit, ix) => ({ ...commit, id: hex(ix + 7), refs: commit.refs || [] }));

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
    name: 'fastgit',
    head: 'main',
    total: 1284,
    readMs: 38,
    truncated: false,
    commits,
    refs,
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

  const commitFile = commitDetails.files[0];

  const commitFileDiff = {
    change: { kind: commitFile.kind, path: commitFile.path },
    commit: commits[2].id,
    additions: commitFile.additions,
    deletions: commitFile.deletions,
    hunks: commitFile.hunks,
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

  const workingDiff = (side, hunks) => ({ change: patchFile, side, ...counts(hunks), hunks });

  const fileDiff = workingDiff('unstaged', [
    hunk(14, 14, 'pub(crate) enum FilePatch {', conflictedBranch),
    hunk(27, 30, 'impl Repo {', conflictedReader),
    hunk(52, 60, 'fn diff_args(change: &FileChange, side: Side) -> Result<Vec<String>> {', pathArguments),
  ]);

  const stagedHunkDiff = workingDiff('staged', [hunk(14, 14, 'pub(crate) enum FilePatch {', conflictedBranch)]);

  const partlyStagedDiff = workingDiff('unstaged', [
    hunk(30, 30, 'impl Repo {', conflictedReader),
    hunk(55, 60, 'fn diff_args(change: &FileChange, side: Side) -> Result<Vec<String>> {', pathArguments),
  ]);

  const partlyStaged = {
    unstaged: changes.unstaged,
    staged: [...changes.staged, patchFile],
  };

  const selectedLines = { hunk: 0, from: 3, to: 5 };

  const draft = {
    message: 'Show conflicted files in the changes list',
    description: 'git status marks them UU. The list gives them a U badge\nand the comparison shows the file with its markers.',
  };

  const tabs = [
    { name: 'fastgit', state: 'ready' },
    { name: 'website', state: 'ready' },
    { name: 'linux', state: 'reading' },
  ];

  return { history, collapsed, changes, partlyStaged, draft, commitDetails, commitFileDiff, fileDiff, stagedHunkDiff, partlyStagedDiff, selectedLines, tabs };
})();
