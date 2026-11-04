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
    { subject: 'Add a menu top bar', author: 'Alex Martin', date: '2026-11-02 20:13', graph: [0, 3, 'I0 O0 P1 P2'], refs: [['v0.2.0', 'tag']] },
    { subject: 'Open each repository in its own tab', author: 'Robin Chen', date: '2026-11-02 16:58', graph: [2, 3, 'P0 P1 I2 O0'] },
    { subject: 'Fetch remotes and pick up outside changes', author: 'Alex Martin', date: '2026-11-02 10:04', graph: [0, 2, 'I0 O0 P1'] },
    { subject: 'Parse git diff output into hunks', author: 'Sam Lee', date: '2026-11-01 22:36', graph: [1, 2, 'P0 I1 O0'] },
    { subject: 'Add read-only commit graph prototype', author: 'Alex Martin', date: '2026-11-01 14:09', graph: [0, 1, 'I0 O0'], refs: [['v0.1.0', 'tag']] },
    ...linear.map((subject, ix) => ({
      subject,
      author: authors[ix % authors.length],
      date: `2026-10-${String(31 - ix).padStart(2, '0')} ${String(9 + (ix * 5) % 12).padStart(2, '0')}:${String((ix * 17) % 60).padStart(2, '0')}`,
      graph: [0, 1, ix === linear.length - 1 ? 'I0' : 'I0 O0'],
    })),
  ].map((commit, ix) => ({ ...commit, id: hex(ix + 7), refs: commit.refs || [] }));

  const refs = [
    { name: 'feature/diff-view', kind: 'local', row: 2 },
    { name: 'main', kind: 'local', row: 0, head: true },
    { name: 'topic/tabs', kind: 'local', row: 5 },
    { name: 'origin/feature/diff-view', kind: 'remote', row: 2 },
    { name: 'origin/main', kind: 'remote', row: 0 },
    { name: 'v0.1.0', kind: 'tag', row: 10 },
    { name: 'v0.2.0', kind: 'tag', row: 6 },
  ];

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
        path: 'src/repo/patch_view.rs',
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
        path: 'Cargo.toml',
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

  const fileDiff = {
    change: { kind: 'M', path: 'src/git/patch.rs' },
    side: 'unstaged',
    additions: 9,
    deletions: 1,
    hunks: [
      {
        header: '@@ -14,10 +14,17 @@ pub(crate) enum FilePatch {',
        lines: [
          [' ', 14, 14, 'impl Repo {'],
          [' ', 15, 15, '    pub(crate) fn file_patch(&self, change: &FileChange, side: Side) -> Result<FilePatch> {'],
          ['-', 16, null, '        let args = diff_args(change, side);'],
          ['+', null, 16, '        if change.kind == ChangeKind::Conflicted {'],
          ['+', null, 17, '            return self.conflicted(change);'],
          ['+', null, 18, '        }'],
          ['+', null, 19, '        let args = diff_args(change, side)?;'],
          [' ', 17, 20, '        let output = self.git(&args)?;'],
          [' ', 18, 21, '        Ok(FilePatch::Diff(output))'],
          [' ', 19, 22, '    }'],
          [' ', 20, 23, ''],
          ['+', null, 24, '    fn conflicted(&self, change: &FileChange) -> Result<FilePatch> {'],
          ['+', null, 25, '        let contents = std::fs::read_to_string(self.workdir().join(&change.path))?;'],
          ['+', null, 26, '        Ok(FilePatch::Conflicted { path: change.path.clone(), contents })'],
          ['+', null, 27, '    }'],
          ['+', null, 28, ''],
          [' ', 21, 29, '    fn untracked(&self, path: &str) -> Result<String> {'],
          [' ', 22, 30, '        let null = if cfg!(windows) { "NUL" } else { "/dev/null" };'],
        ],
      },
    ],
  };

  const tabs = [
    { name: 'fastgit', state: 'ready' },
    { name: 'website', state: 'ready' },
    { name: 'linux', state: 'reading' },
  ];

  return { history, changes, commitDetails, fileDiff, tabs };
})();
