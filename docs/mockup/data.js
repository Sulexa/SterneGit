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
  const emails = { 'Alex Martin': 'alex@fastgit.dev', 'Sam Lee': 'sam@fastgit.dev', 'Robin Chen': 'robin@fastgit.dev' };
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
    name: 'fastgit',
    head: 'main',
    total: 1284,
    readMs: 38,
    truncated: false,
    commits,
    refs,
  };

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
    lines: 404,
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

  const workingDiff = (side, hunks, lines) => ({ change: patchFile, side, ...counts(hunks), hunks, lines });

  const fileDiff = workingDiff('unstaged', [
    hunk(14, 14, 'pub(crate) enum FilePatch {', conflictedBranch),
    hunk(27, 30, 'impl Repo {', conflictedReader),
    hunk(52, 60, 'fn diff_args(change: &FileChange, side: Side) -> Result<Vec<String>> {', pathArguments),
  ], 183);

  const stagedHunkDiff = workingDiff('staged', [hunk(14, 14, 'pub(crate) enum FilePatch {', conflictedBranch)], 179);

  const partlyStagedDiff = workingDiff('unstaged', [
    hunk(30, 30, 'impl Repo {', conflictedReader),
    hunk(55, 60, 'fn diff_args(change: &FileChange, side: Side) -> Result<Vec<String>> {', pathArguments),
  ], 183);

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

  const stoppedAt = { id: '3f2a91c', subject: 'Show conflicted files in the changes list' };

  const rebasedCommits = [
    { id: '9c4e1d2', subject: 'Read conflicted files from git status', author: 'Alex Martin', date: '2026-11-05 09:40', graph: [0, 2, 'O0'], head: true, refs: [] },
    { id: 'b71d0e4', subject: 'Paint conflicted files magenta', author: 'Alex Martin', date: '2026-11-05 09:58', graph: [1, 2, 'P0 O1'], refs: [['main', 'local']] },
    { ...stoppedAt, author: 'Alex Martin', date: '2026-11-05 09:51', graph: [1, 2, 'P0 I1 O1'], refs: [] },
    { id: 'e05a7b3', subject: 'Read conflicted files from git status', author: 'Alex Martin', date: '2026-11-05 09:40', graph: [1, 2, 'P0 I1 O1'], refs: [] },
    { id: '5d8c2fa', subject: 'Show the fetch time in the status bar', author: 'Sam Lee', date: '2026-11-05 08:51', graph: [0, 2, 'I0 O0 P1'], refs: [['origin/main', 'remote']] },
    { id: 'a3e90b1', subject: 'Fetch in the background every five minutes', author: 'Sam Lee', date: '2026-11-04 17:22', graph: [0, 2, 'I0 O0 P1'], refs: [] },
    { ...commits[0], graph: [0, 2, 'I0 I1 O0'], head: false, refs: [] },
    ...commits.slice(1),
  ];

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

  return {
    history, stashed, collapsed, changes, partlyStaged, draft, commitDetails, commitFileDiff, fileHistory, fileDiff, stagedHunkDiff, partlyStagedDiff, selectedLines, tabs,
    starfall, assets, slimeDiff, crownIcon, introDiff, themeDiff, modelDiff,
    rebasing, rebase, merge, starfallMerge, rebaseChanges, rebaseOneLeft, rebaseResolved, statusConflicts, statusMerged, slimeConflict, starfallConflicts,
  };
})();
