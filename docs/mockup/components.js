const IconPaths = {
  'git-branch': '<path d="M15 6a9 9 0 0 0-9 9V3"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/>',
  'git-merge': '<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M6 21V9a9 9 0 0 0 9 9"/>',
  cloud: '<path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/>',
  tag: '<path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"/><circle cx="7.5" cy="7.5" r=".5" fill="currentColor"/>',
  'columns-2': '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M12 3v18"/>',
  'rows-2': '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 12h18"/>',
  plus: '<path d="M5 12h14"/><path d="M12 5v14"/>',
  minus: '<path d="M5 12h14"/>',
  close: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
  'undo-2': '<path d="M9 14 4 9l5-5"/><path d="M4 9h10.5a5.5 5.5 0 0 1 5.5 5.5a5.5 5.5 0 0 1-5.5 5.5H11"/>',
  'triangle-alert': '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
  loader: '<path d="M12 2v4"/><path d="m16.2 7.8 2.9-2.9"/><path d="M18 12h4"/><path d="m16.2 16.2 2.9 2.9"/><path d="M12 18v4"/><path d="m4.9 19.1 2.9-2.9"/><path d="M2 12h4"/><path d="m4.9 4.9 2.9 2.9"/>',
  'chevron-down': '<path d="m6 9 6 6 6-6"/>',
  'chevron-right': '<path d="m9 18 6-6-6-6"/>',
  folder: '<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>',
  list: '<path d="M3 5h.01"/><path d="M3 12h.01"/><path d="M3 19h.01"/><path d="M8 5h13"/><path d="M8 12h13"/><path d="M8 19h13"/>',
  'list-tree': '<path d="M8 5h13"/><path d="M13 12h8"/><path d="M13 19h8"/><path d="M3 10a2 2 0 0 0 2 2h3"/><path d="M3 5v12a2 2 0 0 0 2 2h3"/>',
  'arrow-down-to-line': '<path d="M12 17V3"/><path d="m6 11 6 6 6-6"/><path d="M19 21H5"/>',
  'arrow-up-from-line': '<path d="m18 9-6-6-6 6"/><path d="M12 3v14"/><path d="M5 21h14"/>',
  'git-branch-plus': '<path d="M6 3v12"/><path d="M18 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"/><path d="M6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6z"/><path d="M15 6a9 9 0 0 0-9 9"/><path d="M18 15v6"/><path d="M21 18h-6"/>',
  archive: '<rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8"/><path d="M10 12h4"/>',
  'archive-restore': '<rect width="20" height="5" x="2" y="3" rx="1"/><path d="M4 8v11a2 2 0 0 0 2 2h2"/><path d="M20 8v11a2 2 0 0 1-2 2h-2"/><path d="m9 15 3-3 3 3"/><path d="M12 12v9"/>',
  'circle-check': '<circle cx="12" cy="12" r="10"/><path d="m16 9-5.5 5.5L8 12"/>',
  check: '<path d="M20 6 9 17l-5-5"/>',
  'circle-x': '<circle cx="12" cy="12" r="10"/><path d="m15 9-6 6"/><path d="m9 9 6 6"/>',
  'window-minimize': '<path d="M4.8 11.2h14.4" stroke-width="1.6"/>',
  'window-maximize': '<rect width="14.4" height="14.4" x="4.8" y="4.8" rx="1.6" stroke-width="1.6"/>',
  'window-close': '<path d="M19.2 4.8 4.8 19.2" stroke-width="1.6"/><path d="m4.8 4.8 14.4 14.4" stroke-width="1.6"/>',
  'maximize-2': '<path d="M15 3h6v6"/><path d="m21 3-7 7"/><path d="m3 21 7-7"/><path d="M9 21H3v-6"/>',
  'minimize-2': '<path d="m14 10 7-7"/><path d="M20 10h-6V4"/><path d="m3 21 7-7"/><path d="M4 14h6v6"/>',
  'arrow-up': '<path d="m5 12 7-7 7 7"/><path d="M12 19V5"/>',
  'arrow-down': '<path d="M12 5v14"/><path d="m19 12-7 7-7-7"/>',
  'chevron-left': '<path d="m15 18-6-6 6-6"/>',
  'chevrons-up-down': '<path d="m7 15 5 5 5-5"/><path d="m7 9 5-5 5 5"/>',
  ellipsis: '<circle cx="12" cy="12" r="1"/><circle cx="19" cy="12" r="1"/><circle cx="5" cy="12" r="1"/>',
  'square-split-horizontal': '<path d="M8 19H5c-1 0-2-1-2-2V7c0-1 1-2 2-2h3"/><path d="M16 5h3c1 0 2 1 2 2v10c0 1-1 2-2 2h-3"/><path d="M12 4v16"/>',
  blend: '<circle cx="9" cy="9" r="7"/><circle cx="15" cy="15" r="7"/>',
  diff: '<path d="M12 3v14"/><path d="M5 10h14"/><path d="M5 21h14"/>',
  'audio-lines': '<path d="M2 10v3"/><path d="M6 6v11"/><path d="M10 3v18"/><path d="M14 8v7"/><path d="M18 5v13"/><path d="M22 10v3"/>',
  'chart-column': '<path d="M3 3v16a2 2 0 0 0 2 2h16"/><path d="M18 17V9"/><path d="M13 17V5"/><path d="M8 17v-3"/>',
  play: '<path d="M6 4.5v15a1 1 0 0 0 1.5.86l12.5-7.5a1 1 0 0 0 0-1.72L7.5 3.64A1 1 0 0 0 6 4.5z" fill="currentColor"/>',
  pause: '<rect x="6" y="4" width="4" height="16" rx="1" fill="currentColor"/><rect x="14" y="4" width="4" height="16" rx="1" fill="currentColor"/>',
  'step-back': '<path d="M6 4v16"/><path d="M18 5 9 12l9 7z"/>',
  'step-forward': '<path d="M18 4v16"/><path d="m6 5 9 7-9 7z"/>',
  repeat: '<path d="m17 2 4 4-4 4"/><path d="M3 11v-1a4 4 0 0 1 4-4h14"/><path d="m7 22-4-4 4-4"/><path d="M21 13v1a4 4 0 0 1-4 4H3"/>',
  headphones: '<path d="M3 14h3a2 2 0 0 1 2 2v3a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-7a9 9 0 0 1 18 0v7a2 2 0 0 1-2 2h-1a2 2 0 0 1-2-2v-3a2 2 0 0 1 2-2h3"/>',
  'file-question': '<path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7z"/><path d="M9.1 9a3 3 0 0 1 5.82 1c0 2-3 3-3 3"/><path d="M12 17h.01"/>',
  'external-link': '<path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/>',
  'move-horizontal': '<path d="m18 8 4 4-4 4"/><path d="M2 12h20"/><path d="m6 8-4 4 4 4"/>',
};

const IconSizes = { xs: 12, sm: 14, md: 16, lg: 24 };

const LaneWidth = 12;

const GraphMargin = 6;

const laneX = (lane) => GraphMargin + LaneWidth * (lane + 0.5);

const LaneColors = ['var(--blue)', 'var(--magenta)', 'var(--green)', 'var(--yellow)', 'var(--cyan)', 'var(--red)'];

const ChangeColors = {
  A: 'var(--green)',
  '?': 'var(--green)',
  D: 'var(--red)',
  M: 'var(--yellow)',
  R: 'var(--blue)',
  C: 'var(--blue)',
  U: 'var(--magenta)',
};

const RefIcons = { local: 'git-branch', remote: 'cloud', tag: 'tag', stash: 'archive' };

const TreeIcons = {
  local: { top: 'folder', folder: 'folder', ref: 'git-branch' },
  remote: { top: 'cloud', folder: 'folder', ref: 'git-branch' },
  tag: { top: 'folder', folder: 'folder', ref: 'tag' },
};

const TreeIndent = 18;

const RefSections = [
  ['local', 'Branches'],
  ['remote', 'Remotes'],
  ['stash', 'Stashes'],
  ['tag', 'Tags'],
];

const SideLabels = {
  unstaged: {
    files: 'Unstaged files',
    state: 'unstaged',
    verb: 'Stage',
    all: 'Stage all',
    fileTooltip: 'Stage file',
    folderTooltip: 'Stage folder',
    openFileTooltip: 'Stage this file',
    icon: 'plus',
    discards: true,
  },
  staged: {
    files: 'Staged files',
    state: 'staged',
    verb: 'Unstage',
    all: 'Unstage all',
    fileTooltip: 'Unstage file',
    folderTooltip: 'Unstage folder',
    openFileTooltip: 'Unstage this file',
    icon: 'minus',
    discards: false,
  },
};

const lineCount = (n) => counted(n, 'line', 'lines');

const PartLabels = {
  hunk: (verb) => `${verb} hunk`,
  line: (verb) => `${verb} this line`,
  lines: (verb, n) => `${verb} ${lineCount(n)}`,
  hunkTooltip: { Stage: 'Stage this hunk', Unstage: 'Unstage this hunk', Discard: 'Throw away this hunk' },
  linesTooltip: { Stage: 'Stage the selected lines', Unstage: 'Unstage the selected lines', Discard: 'Throw away the selected lines' },
};

const Menus = {
  File: [['Open Repository…', 'Ctrl+O'], ['Close Tab', 'Ctrl+W'], '-', ['Quit', 'Ctrl+Q']],
  Edit: [['Copy Commit Hash', 'Ctrl+Shift+C']],
  View: [
    ['Refresh', 'Ctrl+R'],
    ['Fetch All Remotes', 'Ctrl+Shift+R'],
    ['Close File Diff', 'Esc'],
    '-',
    ['Newer Commit', 'Up'],
    ['Older Commit', 'Down'],
    '-',
    ['Next Tab', 'Ctrl+Tab'],
    ['Previous Tab', 'Ctrl+Shift+Tab'],
  ],
  About: [['About FastGit', '']],
};

const PullModes = [
  { key: 'merge', label: 'Fast-forward if possible', about: 'Merges when the branches have diverged' },
  { key: 'ff-only', label: 'Fast-forward only', about: 'Stops when the branches have diverged' },
  { key: 'rebase', label: 'Rebase', about: 'Replays your commits on top of the upstream ones' },
  '-',
  { key: 'fetch', label: 'Fetch only', about: 'Updates the remote branches, leaves yours alone' },
];

const pullMode = (key) => PullModes.find((mode) => mode.key === key);

const ToolbarGroups = [
  [
    { key: 'pull', label: 'Pull', icon: 'arrow-down-to-line', tooltip: 'Pull: fast-forward if possible' },
    { key: 'push', label: 'Push', icon: 'arrow-up-from-line', tooltip: 'Send your new commits to the upstream branch' },
  ],
  [{ key: 'branch', label: 'Branch', icon: 'git-branch-plus', tooltip: 'Create a branch where HEAD is and switch to it' }],
  [
    { key: 'stash', label: 'Stash', icon: 'archive', tooltip: 'Put every uncommitted change aside, untracked files included' },
    { key: 'pop', label: 'Pop', icon: 'archive-restore', tooltip: 'Bring back the latest stash and drop it' },
  ],
];

const PictureModes = [
  { key: 'split', label: 'Split', icon: 'columns-2', tooltip: 'Show both versions side by side' },
  { key: 'swipe', label: 'Swipe', icon: 'square-split-horizontal', tooltip: 'Drag a line between the two versions' },
  { key: 'onion', label: 'Onion skin', icon: 'blend', tooltip: 'Fade one version into the other' },
  { key: 'difference', label: 'Difference', icon: 'diff', tooltip: 'Show only the pixels that changed' },
];

const CompareModes = {
  text: [
    { key: 'split', label: 'Split', icon: 'columns-2', tooltip: 'Compare side by side' },
    { key: 'unified', label: 'Unified', icon: 'rows-2', tooltip: 'Show changes in one column' },
  ],
  image: PictureModes,
  video: PictureModes,
  audio: [
    { key: 'split', label: 'Split', icon: 'audio-lines', tooltip: 'Show both waveforms on one time scale' },
    { key: 'difference', label: 'Difference', icon: 'diff', tooltip: 'Show only what sounds different' },
  ],
};

const VersionLabels = { before: 'Before', after: 'After' };

const ViewOptions = [
  { key: 'words', label: 'Highlight changed words', about: 'Marks what changed inside a line' },
  { key: 'wrap', label: 'Wrap long lines', about: 'Breaks lines that are wider than the pane' },
  { key: 'whole', label: 'Show the whole file', about: 'Keeps unchanged lines instead of folding them' },
  { key: 'whitespace', label: 'Ignore whitespace', about: 'Hides changes to spaces and tabs; parts can’t be staged then' },
];

const PixelChanges = {
  '+': { color: 'var(--green)', label: 'added' },
  '~': { color: 'var(--yellow)', label: 'changed' },
  '-': { color: 'var(--red)', label: 'removed' },
};

const FoldLimit = 20;

const Texts = {
  welcomeTitle: 'Open a Git repository',
  welcomeDescription: 'FastGit reads history straight from the repository on disk. Each repository opens in its own tab.',
  openFailedTitle: "Couldn't open this repository",
  readingHistory: 'Reading history…',
  selectCommit: 'Select a commit to see its changes.',
  readingCommit: 'Reading commit…',
  commitFailed: "Couldn't read this commit",
  commitEmpty: "This commit doesn't change any files.",
  readingChanges: 'Reading changes…',
  fileFailed: "Couldn't compare this file",
  fileEmpty: 'No changes to show: only line endings or file mode differ.',
  noChanges: 'No uncommitted changes.',
  commitMessage: 'Commit message',
  commitDescription: 'Description',
  committing: 'Committing…',
  commitTooltip: 'Commit the staged files  Ctrl+Enter',
  nothingStaged: 'Stage files to commit them.',
  nothingUnstaged: 'Every change is staged.',
  listView: 'Show files as a list',
  treeView: 'Show files as a tree',
  aboutDescription: 'A lightweight Git client for Linux, macOS and Windows, built with GPUI and gitoxide.',
  discardTitle: (name) => `Discard the changes to ${name}?`,
  discardDescription: (path) => `The unstaged changes in ${path} are thrown away. Git keeps no copy of them, so this can't be undone.`,
  deleteTitle: (name) => `Delete ${name}?`,
  deleteDescription: (path) => `${path} isn't tracked by Git, so it can't be brought back once deleted.`,
  pathCopied: 'Path copied',
  branchTitle: 'Create a branch',
  branchDescription: (head) => `It starts at ${head}, where HEAD is, and is checked out right away. Uncommitted changes stay as they are.`,
  branchFromDescription: (start) => `It starts at ${start} and is checked out right away. Uncommitted changes come along unless they clash with it.`,
  checkOut: (name) => `Check out ${name}`,
  pullWithMode: (mode) => `Pull (${mode.toLowerCase()})`,
  pullFastForward: 'Pull (fast-forward only)',
  push: 'Push',
  merge: (from, into) => `Merge ${from} into ${into}`,
  rebase: (branch, onto) => `Rebase ${branch} onto ${onto}`,
  branchHere: 'Create branch here…',
  applyStash: (name) => `Apply ${name}`,
  popStash: (name) => `Pop ${name}`,
  deleteStash: (name) => `Delete ${name}…`,
  deleteStashTitle: (name) => `Delete ${name}?`,
  deleteStashDescription: (message) => `Git keeps no other copy of “${message}”, so its changes are lost.`,
  deleteBranch: (name) => `Delete ${name}…`,
  deleteRemoteBranch: (name, remote) => `Delete ${name} from ${remote}…`,
  deleteBranchTitle: (name) => `Delete ${name}?`,
  deleteMerged: (name) => `Every commit of ${name} is on another branch or tag too, so nothing is lost.`,
  deleteUnmerged: (name, n) => `${counted(n, 'commit is', 'commits are')} on no other branch or tag. Deleting ${name} loses ${n === 1 ? 'it' : 'them'}.`,
  deleteUncounted: (name) => `${name} points to a commit older than the newest ${thousands(Texts.historyLimit)} shown, so FastGit can't tell whether deleting it loses any commits.`,
  deleteRemoteTitle: (name, remote) => `Delete ${name} from ${remote}?`,
  deleteRemoteDescription: (name, remote) => `The branch is removed from ${remote} for everyone who uses it. A local ${name}, if you have one, stays.`,
  branchPlaceholder: 'Branch name',
  branchExists: (name) => `There is already a branch named ${name}.`,
  branchInvalid: "Git doesn't accept this name: no spaces, and none of ~ ^ : ? * [ \\ or ..",
  createBranch: 'Create branch',
  historyLimit: 10000,
  useWindow: 'Use the whole window',
  showPanes: 'Show the refs and the file list again',
  viewOptions: 'View options',
  previousChange: 'Previous change  Alt+Up',
  nextChange: 'Next change  Alt+Down',
  previousFile: 'Previous file  Alt+Left',
  nextFile: 'Next file  Alt+Right',
  lfs: 'Stored with Git LFS: FastGit compares the file, not its pointer',
  backdrop: 'What shows behind transparent pixels',
  showAbove: (n) => `Show ${n} lines above`,
  showBelow: (n) => `Show ${n} lines below`,
  expandLines: (n) => `Expand ${counted(n, 'unchanged line', 'unchanged lines')}`,
  cannotShow: (type) => `FastGit can’t draw ${type} files`,
  cannotShowDescription: 'Open each version in the app that made it, or hand both to the diff tool set in Git’s diff.tool.',
  fileHistory: 'File history',
  openFile: 'Open file',
  showInFolder: 'Show in folder',
  copyPath: 'Copy path',
  historyPosition: (at, n, name) => `Commit ${thousands(at)} of ${thousands(n)} changing ${name}`,
  renamedFrom: (path) => `Renamed from ${path}`,
  hear: (side) => `Hear ${side === 'before' ? 'the version before the change' : 'the changed version'}  Tab switches`,
};

const esc = (text) =>
  String(text).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

const thousands = (n) => n.toLocaleString('en-US');

const counted = (n, singular, plural) => `${thousands(n)} ${n === 1 ? singular : plural}`;

const join = (parts) => parts.filter(Boolean).join('');

const Bits64 = 0xffffffffffffffffn;

function avatarHash(email) {
  let h = 0xcbf29ce484222325n;
  for (const byte of new TextEncoder().encode(email.trim().toLowerCase())) h = ((h ^ BigInt(byte)) * 0x100000001b3n) & Bits64;
  h ^= h >> 33n;
  h = (h * 0xff51afd7ed558ccdn) & Bits64;
  h ^= h >> 33n;
  h = (h * 0xc4ceb9fe1a85ec53n) & Bits64;
  return h ^ (h >> 33n);
}

const AvatarGrid = 5;

const AvatarHalf = Math.ceil(AvatarGrid / 2);

const AvatarPatternShift = 9n;

function identiconCells(hash) {
  const cells = [];
  for (let y = 0; y < AvatarGrid; y++)
    for (let x = 0; x < AvatarHalf; x++)
      if ((hash >> (AvatarPatternShift + BigInt(y * AvatarHalf + x))) & 1n) {
        cells.push([x, y]);
        if (x !== AvatarGrid - 1 - x) cells.push([AvatarGrid - 1 - x, y]);
      }
  return cells;
}

function authorInitials(name, email) {
  const words = name.trim().split(/\s+/).filter(Boolean);
  if (words.length > 1) return (words[0][0] + words[words.length - 1][0]).toUpperCase();
  return (words[0] || email.split('@')[0]).slice(0, 2).toUpperCase();
}

function foldSyncedRefs(refs) {
  const upstreamOf = (name) => `origin/${name}`;
  const upstreams = new Set(refs.filter(([, kind]) => kind === 'local').map(([name]) => upstreamOf(name)));
  const remotes = new Set(refs.filter(([, kind]) => kind === 'remote').map(([name]) => name));
  return refs
    .filter(([name, kind]) => !(kind === 'remote' && upstreams.has(name)))
    .map(([name, kind, head]) => ({ name, kind, head, synced: kind === 'local' && remotes.has(upstreamOf(name)) }));
}

function icon(name, size = 'md', color) {
  const px = IconSizes[size];
  const style = color ? ` style="color:${color}"` : '';
  return `<svg class="icon" width="${px}" height="${px}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"${style}>${IconPaths[name]}</svg>`;
}

const RustTokens = /("(?:[^"\\]|\\.)*")|\b(fn|let|pub|impl|if|else|return|match|mut|use|struct|enum|crate|self|Self|true|false|move|for|in|const)\b|\b([A-Z][A-Za-z0-9_]*)\b|\b(\d+(?:\.\d+)*)\b|\b([a-z_][a-z0-9_]*)(?=\()/g;

function highlight(code, words = []) {
  const pieces = [];
  let last = 0;
  for (const match of code.matchAll(RustTokens)) {
    if (match.index > last) pieces.push([last, match.index, '']);
    const kind = match[1] ? 'string' : match[2] ? 'keyword' : match[3] ? 'type' : match[4] ? 'number' : 'function';
    pieces.push([match.index, match.index + match[0].length, `tok-${kind}`]);
    last = match.index + match[0].length;
  }
  if (last < code.length) pieces.push([last, code.length, '']);
  const cuts = words.flat();
  const marked = (from, to) => words.some(([start, end]) => from >= start && to <= end);
  let html = '';
  for (const [start, end, kind] of pieces) {
    const points = [...new Set(cuts.filter((point) => point > start && point < end))].sort((a, b) => a - b);
    let from = start;
    for (const to of [...points, end]) {
      const classes = [kind, marked(from, to) && 'inline'].filter(Boolean).join(' ');
      const text = esc(code.slice(from, to));
      html += classes ? `<span class="${classes}">${text}</span>` : text;
      from = to;
    }
  }
  return html;
}

const WordTokens = /\w+|\s+|[^\w\s]/g;

function changedWords(before, after) {
  const a = [...before.matchAll(WordTokens)];
  const b = [...after.matchAll(WordTokens)];
  const common = Array.from({ length: a.length + 1 }, () => new Array(b.length + 1).fill(0));
  for (let i = a.length - 1; i >= 0; i--) {
    for (let j = b.length - 1; j >= 0; j--) {
      common[i][j] = a[i][0] === b[j][0] ? common[i + 1][j + 1] + 1 : Math.max(common[i + 1][j], common[i][j + 1]);
    }
  }
  const keptA = new Set();
  const keptB = new Set();
  for (let i = 0, j = 0; i < a.length && j < b.length; ) {
    if (a[i][0] === b[j][0]) {
      keptA.add(i++);
      keptB.add(j++);
    } else if (common[i + 1][j] >= common[i][j + 1]) i++;
    else j++;
  }
  const words = (tokens, kept) => tokens.filter((token, ix) => token[0].trim() && (!kept || kept.has(ix))).length;
  if ((2 * words(a, keptA)) / (words(a) + words(b) || 1) < 0.5) return [[], []];
  const runs = (tokens, kept, text) => {
    const ranges = [];
    tokens.forEach((token, ix) => {
      if (kept.has(ix)) return;
      const start = token.index;
      const end = start + token[0].length;
      const previous = ranges[ranges.length - 1];
      if (previous && text.slice(previous[1], start).trim() === '') previous[1] = end;
      else ranges.push([start, end]);
    });
    return ranges.filter(([start, end]) => text.slice(start, end).trim() !== '');
  };
  return [runs(a, keptA, before), runs(b, keptB, after)];
}

function wordChanges(lines) {
  const words = new Map();
  for (const [deleted, added] of pairLines(lines)) {
    if (!deleted || !added || deleted === added || deleted[0] !== '-' || added[0] !== '+') continue;
    const [before, after] = changedWords(deleted[3], added[3]);
    words.set(deleted, before);
    words.set(added, after);
  }
  return words;
}

function changeGroups(hunks) {
  const groups = [];
  for (const h of hunks) {
    let group = null;
    let lastNew = null;
    for (const [sign, , newNo] of h.lines) {
      if (sign === ' ') {
        if (group) group.end = newNo - 1;
        group = null;
        lastNew = newNo;
        continue;
      }
      if (!group) {
        group = { start: newNo ?? (lastNew ?? 0) + 1, end: newNo ?? (lastNew ?? 0) + 1, added: 0, deleted: 0 };
        groups.push(group);
      }
      if (sign === '+') {
        group.added++;
        group.end = newNo;
        lastNew = newNo;
      } else group.deleted++;
    }
  }
  return groups;
}

function newSpan(h) {
  const numbers = h.lines.map((line) => line[2]).filter((n) => n !== null);
  return [Math.min(...numbers), Math.max(...numbers)];
}

function widenedHeader(header, above, below) {
  const match = header.match(/^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@(.*)$/);
  if (!match) return header;
  const [, oldStart, oldCount = '1', newStart, newCount = '1', scope] = match;
  const range = (start, count) => `${Number(start) - above},${Number(count) + above + below}`;
  return `@@ -${range(oldStart, oldCount)} +${range(newStart, newCount)} @@${scope}`;
}

function hiddenLines(hunks, total) {
  const spans = hunks.map(newSpan);
  const above = spans.map(([start], ix) => start - (ix ? spans[ix - 1][1] : 0) - 1);
  return { above, below: total - spans[spans.length - 1][1] };
}

function pixelArt(rows, palette, scale) {
  const rects = [];
  rows.forEach((row, y) => {
    for (let x = 0; x < row.length; ) {
      let end = x + 1;
      while (end < row.length && row[end] === row[x]) end++;
      if (row[x] !== '.') rects.push(`<rect x="${x}" y="${y}" width="${end - x}" height="1" fill="${palette[row[x]]}"/>`);
      x = end;
    }
  });
  const w = rows[0].length;
  const h = rows.length;
  return `<svg width="${w * scale}" height="${h * scale}" viewBox="0 0 ${w} ${h}" shape-rendering="crispEdges">${rects.join('')}</svg>`;
}

function pixelChanges(before, after) {
  return before.map((row, y) => [...row].map((b, x) => {
    const a = after[y][x];
    return a === b ? '.' : b === '.' ? '+' : a === '.' ? '-' : '~';
  }).join(''));
}

function clock(seconds, digits = 2) {
  const minutes = Math.floor(seconds / 60);
  const rest = seconds - minutes * 60;
  const shown = digits ? rest.toFixed(digits).padStart(digits + 3, '0') : String(Math.floor(rest)).padStart(2, '0');
  return `${minutes}:${shown}`;
}

function percentOf(value, total) {
  return `${((value / total) * 100).toFixed(3)}%`;
}

function noise(seed, count) {
  let state = seed;
  return Array.from({ length: count }, () => {
    state = (state + 0x6d2b79f5) | 0;
    let t = Math.imul(state ^ (state >>> 15), 1 | state);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  });
}

function loudness(sections, duration, total, bins, grain) {
  return Array.from({ length: bins }, (_, ix) => {
    const t = ((ix + 0.5) / bins) * total;
    if (t > duration) return null;
    const [from, to, a, b] = sections.find(([start, end]) => t >= start && t < end) || sections[sections.length - 1];
    const level = a + (b - a) * Math.min(1, (t - from) / (to - from));
    const beat = (t * 2.2) % 1 < 0.14 ? 1.18 : 1;
    return Math.min(0.97, level * (0.58 + 0.42 * grain(ix, t)) * beat);
  });
}

function waveform(levels, changed) {
  let plain = '';
  let marked = '';
  levels.forEach((level, ix) => {
    if (level === null) return;
    const h = Math.max(1, level * 96);
    const bar = `M${ix + 0.15} ${(50 - h / 2).toFixed(2)}h0.7v${h.toFixed(2)}h-0.7z`;
    if (changed(ix)) marked += bar;
    else plain += bar;
  });
  return `<svg viewBox="0 0 ${levels.length} 100" preserveAspectRatio="none"><path d="${plain}" style="fill:var(--blue)"/><path d="${marked}" style="fill:var(--yellow)"/></svg>`;
}

function starfallFrame(version) {
  const after = version === 'after';
  const stars = noise(3, 90);
  const dots = [];
  for (let ix = 0; ix < 30; ix++) {
    dots.push(`<circle cx="${(stars[ix * 3] * 640).toFixed(1)}" cy="${(stars[ix * 3 + 1] * 210).toFixed(1)}" r="${(0.6 + stars[ix * 3 + 2] * 1.3).toFixed(2)}" fill="#e2e8f0" opacity="${(0.4 + stars[ix * 3 + 2] * 0.6).toFixed(2)}"/>`);
  }
  const planet = after
    ? '<circle cx="486" cy="104" r="62" fill="url(#sf-teal)"/><ellipse cx="486" cy="104" rx="104" ry="20" fill="none" stroke="#99f6e4" stroke-width="4" transform="rotate(-14 486 104)" opacity="0.8"/>'
    : '<circle cx="150" cy="118" r="52" fill="url(#sf-violet)"/><ellipse cx="150" cy="118" rx="88" ry="17" fill="none" stroke="#c4b5fd" stroke-width="4" transform="rotate(-18 150 118)" opacity="0.8"/>';
  const streak = after ? '<path d="M70 52 L214 112" stroke="url(#sf-streak)" stroke-width="3" stroke-linecap="round"/><circle cx="214" cy="112" r="3.5" fill="#fef9c3"/>' : '';
  const title = after
    ? '<text x="320" y="196" text-anchor="middle" font-family="Segoe UI, sans-serif" font-size="66" font-weight="800" letter-spacing="10" fill="url(#sf-gold)" filter="url(#sf-glow)">STARFALL</text>'
    : '<text x="320" y="190" text-anchor="middle" font-family="Segoe UI, sans-serif" font-size="62" font-weight="800" letter-spacing="10" fill="#f8fafc">STARFALL</text>';
  const prompt = after ? 'PRESS START' : 'PRESS ANY KEY';
  return `<svg viewBox="0 0 640 360" preserveAspectRatio="xMidYMid slice">
    <defs>
      <linearGradient id="sf-sky" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#050816"/><stop offset="1" stop-color="#1e2a5e"/></linearGradient>
      <radialGradient id="sf-violet" cx="0.35" cy="0.35"><stop offset="0" stop-color="#c084fc"/><stop offset="1" stop-color="#5b21b6"/></radialGradient>
      <radialGradient id="sf-teal" cx="0.35" cy="0.35"><stop offset="0" stop-color="#5eead4"/><stop offset="1" stop-color="#0f766e"/></radialGradient>
      <linearGradient id="sf-gold" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#fde68a"/><stop offset="1" stop-color="#f59e0b"/></linearGradient>
      <linearGradient id="sf-streak" x1="0" y1="0" x2="1" y2="0"><stop offset="0" stop-color="#fef9c3" stop-opacity="0"/><stop offset="1" stop-color="#fef9c3"/></linearGradient>
      <filter id="sf-glow" x="-20%" y="-50%" width="140%" height="200%"><feGaussianBlur stdDeviation="5" result="blur"/><feMerge><feMergeNode in="blur"/><feMergeNode in="SourceGraphic"/></feMerge></filter>
    </defs>
    <rect width="640" height="360" fill="url(#sf-sky)"/>${dots.join('')}${planet}${streak}
    <path d="M0 300 L70 248 L132 282 L214 214 L300 272 L382 228 L452 266 L540 218 L640 258 L640 360 L0 360 Z" fill="#0b1124"/>
    <path d="M0 330 L96 296 L190 324 L284 290 L388 326 L476 300 L572 328 L640 306 L640 360 L0 360 Z" fill="#060a16"/>
    ${title}
    <text x="320" y="236" text-anchor="middle" font-family="Segoe UI, sans-serif" font-size="15" letter-spacing="5" fill="#cbd5e1">${prompt}</text>
  </svg>`;
}

function graphWidth(rows) {
  const lanes = Math.min(32, Math.max(1, ...rows.map((commit) => commit.graph[1])));
  return lanes * LaneWidth + 2 * GraphMargin;
}

function graphEdges(spec) {
  return spec.split(' ').filter(Boolean).map((token) => ({ kind: token[0], lane: Number(token.slice(1)) }));
}

function refTree(labels) {
  const root = [];
  for (const label of labels) {
    const parts = label.name.split('/');
    let level = root;
    parts.slice(0, -1).forEach((part, depth) => {
      let folder = level.find((node) => node.children && node.name === part);
      if (!folder) {
        folder = { name: part, path: parts.slice(0, depth + 1).join('/'), children: [] };
        level.push(folder);
      }
      level = folder.children;
    });
    level.push({ name: parts[parts.length - 1], label });
  }
  return root;
}

function fileTree(items) {
  const root = { children: [] };
  for (const item of items) {
    const parts = item.path.split('/');
    let folder = root;
    parts.slice(0, -1).forEach((part, depth) => {
      let next = folder.children.find((node) => node.children && node.name === part);
      if (!next) {
        next = { name: part, path: parts.slice(0, depth + 1).join('/'), children: [] };
        folder.children.push(next);
      }
      folder = next;
    });
    folder.children.push({ name: parts[parts.length - 1], item });
  }
  return compactFolders(root.children);
}

function compactFolders(nodes) {
  return nodes
    .map((node) => {
      if (!node.children) return node;
      let folder = node;
      while (folder.children.length === 1 && folder.children[0].children) {
        const only = folder.children[0];
        folder = { name: `${folder.name}/${only.name}`, path: only.path, children: only.children };
      }
      return { ...folder, children: compactFolders(folder.children) };
    })
    .sort((a, b) => !a.children - !b.children || a.name.localeCompare(b.name));
}

function fileTreeRows(nodes, render, depth = 0) {
  return nodes
    .map((node) => {
      if (!node.children) return render.file(node.item, depth);
      const open = !render.collapsed.has(node.path);
      return treeGroup(open, render.folder(node, depth, open), fileTreeRows(node.children, render, depth + 1));
    })
    .join('');
}

function twisty(open) {
  return `<span class="twisty">${open === null ? '' : icon(open ? 'chevron-down' : 'chevron-right', 'xs')}</span>`;
}

function treeGroup(open, header, children) {
  return `<div class="tree-group${open ? '' : ' collapsed'}">${header}<div class="tree-children">${children}</div></div>`;
}

function summarize(changes) {
  return [
    changes.unstaged.length && `${thousands(changes.unstaged.length)} unstaged`,
    changes.staged.length && `${thousands(changes.staged.length)} staged`,
  ]
    .filter(Boolean)
    .join(' · ');
}

function historySummary(history) {
  const stashes = history.stashes?.length ? ` · ${counted(history.stashes.length, 'stash', 'stashes')}` : '';
  const summary = `${thousands(history.total)} commits · ${thousands(history.refs.length)} refs${stashes} · read in ${history.readMs} ms`;
  return history.truncated ? `${summary} · newest ${thousands(Texts.historyLimit)} shown` : summary;
}

function splitPath(path) {
  const slash = path.lastIndexOf('/');
  return slash < 0 ? [path, null] : [path.slice(slash + 1), path.slice(0, slash)];
}

function pairLines(lines) {
  const rows = [];
  let deleted = [];
  let added = [];
  const flush = () => {
    const count = Math.max(deleted.length, added.length);
    for (let ix = 0; ix < count; ix++) rows.push([deleted[ix] || null, added[ix] || null]);
    deleted = [];
    added = [];
  };
  for (const line of lines) {
    if (line[0] === '-') deleted.push(line);
    else if (line[0] === '+') added.push(line);
    else {
      flush();
      rows.push([line, line]);
    }
  }
  flush();
  return rows;
}

function selectedEntries(hunks, selection) {
  if (!selection) return new Set();
  return new Set(hunks[selection.hunk].lines.slice(selection.from, selection.to + 1));
}

function selectionSize(selection) {
  return selection ? selection.to - selection.from + 1 : 0;
}

function lineMenuItems({ side, selected = 0, changed = true }) {
  const { verb, discards } = SideLabels[side];
  const pair = (label) => [[label(verb), ''], discards && [label('Discard'), '']].filter(Boolean);
  const hunk = pair(PartLabels.hunk);
  if (selected) return [...pair((action) => PartLabels.lines(action, selected)), '-', ...hunk];
  return changed ? [...hunk, '-', ...pair(PartLabels.line)] : hunk;
}

const menuGroups = (groups) => groups.map((group) => group.filter(Boolean)).filter((group) => group.length).flatMap((group, ix) => (ix ? ['-', ...group] : group));

const placeItems = (kind) => [kind !== 'D' && [Texts.openFile, ''], kind !== 'D' && [Texts.showInFolder, ''], [Texts.copyPath, '']];

function fileMenuItems({ change, side }) {
  const labels = SideLabels[side];
  const discard = labels.discards && change.kind !== 'U' && [change.kind === '?' ? 'Delete file…' : 'Discard changes…', ''];
  const committed = change.kind !== '?' && change.kind !== 'A' && [Texts.fileHistory, ''];
  return menuGroups([[[labels.fileTooltip, ''], discard], [committed], placeItems(change.kind)]);
}

function commitFileMenuItems({ file }) {
  return menuGroups([[[Texts.fileHistory, '']], placeItems(file.kind)]);
}

function remoteParts(name) {
  const [remote, ...rest] = name.split('/');
  return { remote, branch: rest.join('/') };
}

function branchMenuItems({ label, current, mode = 'merge' }) {
  const remote = label.kind === 'remote' && remoteParts(label.name);
  const checkedOut = Boolean(label.head);
  const item = (text) => [text, ''];
  const groups = [
    !checkedOut && [item(Texts.checkOut(remote ? remote.branch : label.name))],
    !remote && [item(checkedOut ? Texts.pullWithMode(pullMode(mode).label) : Texts.pullFastForward), item(Texts.push)],
    current && !checkedOut && [item(Texts.merge(label.name, current)), item(Texts.rebase(current, label.name))],
    [item(Texts.branchHere)],
    !checkedOut && [item(remote ? Texts.deleteRemoteBranch(remote.branch, remote.remote) : Texts.deleteBranch(label.name))],
  ].filter(Boolean);
  return groups.flatMap((group, ix) => (ix ? ['-', ...group] : group));
}

function stashMenuItems({ stash }) {
  const item = (text) => [text, ''];
  return [item(Texts.applyStash(stash.name)), item(Texts.popStash(stash.name)), '-', item(Texts.deleteStash(stash.name))];
}

const C = {
  Button({ label, icon: iconName, variant = 'default', size = 'xs', selected = false, disabled = false, loading = false, tooltip, dot }) {
    const classes = ['btn', variant, size, selected && 'selected', disabled && 'disabled', !label && 'icon-only']
      .filter(Boolean)
      .join(' ');
    const title = tooltip ? ` title="${esc(tooltip)}"` : '';
    const glyphSize = size === 'xs' ? 'xs' : 'sm';
    const glyph = loading
      ? icon('loader', glyphSize).replace('class="icon"', 'class="icon spinner"')
      : iconName
        ? icon(iconName, glyphSize)
        : '';
    const mark = dot ? `<span class="version-dot ${dot}"></span>` : '';
    return `<span data-c="Button" class="${classes}"${title}>${glyph}${mark}${label ? `<span>${esc(label)}</span>` : ''}</span>`;
  },

  Input({ value = '', placeholder = '', lines = 1, focused = false, disabled = false }) {
    const multiLine = lines > 1;
    const classes = ['input', multiLine && 'multi-line', focused && 'focused', disabled && 'disabled'].filter(Boolean).join(' ');
    const height = multiLine ? ` style="height:${lines * 20 + 17}px"` : '';
    const caret = focused ? '<span class="caret"></span>' : '';
    const text = value
      ? `<span class="${multiLine ? 'fill' : 'ellipsis'}">${esc(value)}${caret}</span>`
      : `${caret}<span class="placeholder">${esc(placeholder)}</span>`;
    return `<div data-c="Input" class="${classes}"${height}>${text}</div>`;
  },

  Spinner({ size = 'md' } = {}) {
    return `<span data-c="Spinner" class="h none">${icon('loader', size).replace('class="icon"', 'class="icon spinner"')}</span>`;
  },

  WindowControls() {
    return `<div data-c="WindowControls" class="window-controls"><span>${icon('window-minimize', 'sm')}</span><span>${icon('window-maximize', 'sm')}</span><span class="close">${icon('window-close', 'sm')}</span></div>`;
  },

  AppMenuBar({ open } = {}) {
    const items = Object.keys(Menus).map((name) => `<span class="${name === open ? 'open' : ''}">${name}</span>`);
    return `<div data-c="AppMenuBar" class="menu-bar">${items.join('')}</div>`;
  },

  TopBar({ platform = 'windows', repository, head, openMenu } = {}) {
    const mac = platform === 'mac';
    const lights = mac
      ? '<div class="traffic-lights"><span style="background:#ff5f57"></span><span style="background:#febc2e"></span><span style="background:#28c840"></span></div>'
      : '';
    const pill = head
      ? `<div class="h head-pill">${icon('git-branch', 'xs')}<div class="ellipsis">${esc(head)}</div></div>`
      : '';
    const repo = repository
      ? `<div class="h" style="gap:8px"><div class="sm medium ellipsis">${esc(repository)}</div>${pill}</div>`
      : '';
    return `<div data-c="TopBar" data-base="TitleBar" class="top-bar${mac ? ' mac' : ''}">${lights}<div class="content">${icon('git-merge', 'sm', 'var(--primary)')}${mac ? '' : C.AppMenuBar({ open: openMenu })}<div class="fill"></div>${repo}</div>${mac ? '' : C.WindowControls()}</div>`;
  },

  Tab({ name, state = 'ready', active = false }) {
    const prefix =
      state === 'reading'
        ? `<span class="prefix">${C.Spinner({ size: 'xs' })}</span>`
        : state === 'failed'
          ? `<span class="prefix">${icon('triangle-alert', 'xs', 'var(--red)')}</span>`
          : '';
    const close = C.Button({ icon: 'close', variant: 'ghost', tooltip: 'Close tab  Ctrl+W' });
    return `<div data-c="Tab" class="tab${active ? ' active' : ''}">${prefix}<span class="label ellipsis">${esc(name)}</span>${close}</div>`;
  },

  TabStrip({ tabs, active = 0 }) {
    const items = tabs.map((tab, ix) => C.Tab({ ...tab, active: ix === active })).join('');
    const open = C.Button({ icon: 'plus', variant: 'ghost', tooltip: 'Open a repository in a new tab  Ctrl+O' });
    return `<div data-c="TabStrip" data-base="TabBar" class="tab-strip"><div class="tabs">${items}</div><div class="suffix">${open}</div></div>`;
  },

  ListRow({ content, selected = false, extraClass = '', name = 'ListRow', base }) {
    const shownBase = base || (name === 'ListRow' ? '' : 'ListRow');
    const baseAttribute = shownBase ? ` data-base="${shownBase}"` : '';
    const classes = ['list-row', selected && 'selected', extraClass].filter(Boolean).join(' ');
    return `<div data-c="${name}"${baseAttribute} class="${classes}">${content}</div>`;
  },

  SectionHeader({ title, count, action, open }) {
    const foldable = open !== undefined;
    const shownCount = count === undefined ? '' : `<span>${thousands(count)}</span>`;
    return `<div data-c="SectionHeader" class="section-header${foldable ? ' foldable' : ''}">${foldable ? twisty(open) : ''}<span class="title ellipsis">${esc(title)}</span>${shownCount}${action || ''}</div>`;
  },

  RefBadge({ name, kind, head = false, synced = false, color = LaneColors[0] }) {
    const icons = icon(RefIcons[kind], 'xs') + (synced ? icon(RefIcons.remote, 'xs') : '');
    return `<div data-c="RefBadge" class="ref-badge${head ? ' head' : ''}" style="--c:${color}"><div class="icons">${icons}</div><div class="name ellipsis">${esc(name)}</div></div>`;
  },

  ChangeBadge({ kind }) {
    return `<div data-c="ChangeBadge" class="change-badge" style="--c:${ChangeColors[kind]}">${esc(kind)}</div>`;
  },

  AuthorAvatar({ name = '', email, size = 16, look = 'identicon' }) {
    const hash = avatarHash(email);
    const hue = Number(hash % 360n);
    const ink = `hsl(${hue} 60% 62%)`;
    const ground = `hsl(${hue} 30% 22%)`;
    const span = AvatarGrid + 1;
    const picture =
      look === 'initials'
        ? `<circle cx="${span / 2}" cy="${span / 2}" r="${span / 2}" fill="${ground}"/><text x="${span / 2}" y="${span / 2}" dy="0.36em" text-anchor="middle" font-size="2.6" font-weight="600" fill="${ink}">${esc(authorInitials(name, email))}</text>`
        : `<rect width="${span}" height="${span}" rx="1.2" fill="${ground}"/>` +
          identiconCells(hash).map(([x, y]) => `<rect x="${x + 0.5}" y="${y + 0.5}" width="1" height="1" fill="${ink}"/>`).join('');
    const title = name ? `${name} <${email}>` : email;
    return `<svg data-c="AuthorAvatar" class="author-avatar" width="${size}" height="${size}" viewBox="0 0 ${span} ${span}" shape-rendering="${look === 'initials' ? 'auto' : 'crispEdges'}"><title>${esc(title)}</title>${picture}</svg>`;
  },

  Additions({ count, extraClass = '' }) {
    return `<span data-c="Additions" class="additions${extraClass ? ` ${extraClass}` : ''}">+${thousands(count)}</span>`;
  },

  Deletions({ count, extraClass = '' }) {
    return `<span data-c="Deletions" class="deletions${extraClass ? ` ${extraClass}` : ''}">−${thousands(count)}</span>`;
  },

  GraphCell({ graph, merge = false, head = false, stash = false, width }) {
    const [node, lanes, spec] = graph;
    const w = width || lanes * LaneWidth + 2 * GraphMargin;
    const x = laneX;
    const color = (lane) => LaneColors[lane % LaneColors.length];
    const paths = graphEdges(spec).map(({ kind, lane }) => {
      const d =
        kind === 'P'
          ? `M${x(lane)} 0 L${x(lane)} 28`
          : kind === 'I'
            ? lane === node
              ? `M${x(lane)} 0 L${x(node)} 14`
              : `M${x(lane)} 0 C${x(lane)} 14 ${x(lane)} 14 ${x(node)} 14`
            : lane === node
              ? `M${x(node)} ${stash ? 18.5 : 14} L${x(node)} 28`
              : `M${x(node)} 14 C${x(lane)} 14 ${x(lane)} 14 ${x(lane)} 28`;
      return `<path d="${d}" fill="none" style="stroke:${color(lane)}" stroke-width="2"/>`;
    });
    const radius = merge ? 4.5 * 0.7 : 4.5;
    const dot = stash
      ? `<rect x="${x(node) - 4.5}" y="9.5" width="9" height="9" rx="1.5" fill="none" style="stroke:${color(node)}" stroke-width="1.5"/>`
      : `<circle cx="${x(node)}" cy="14" r="${radius}" style="fill:${color(node)}"/>`;
    const ring = head ? `<circle cx="${x(node)}" cy="14" r="7.5" fill="none" style="stroke:${color(node)}" stroke-width="1.5"/>` : '';
    return `<span data-c="GraphCell" class="graph-cell none" style="width:${w}px"><svg width="${w}" height="28" viewBox="0 0 ${w} 28">${paths.join('')}${dot}${ring}</svg></span>`;
  },

  UncommittedCell() {
    const w = LaneWidth + 2 * GraphMargin;
    return `<span data-c="UncommittedCell" class="graph-cell none" style="width:${w}px"><svg width="${w}" height="28" viewBox="0 0 ${w} 28"><circle cx="${laneX(0)}" cy="14" r="4.5" fill="none" style="stroke:var(--muted)" stroke-width="1.5"/></svg></span>`;
  },

  Loading({ message }) {
    return `<div data-c="Loading" class="loading">${C.Spinner()}<div class="sm">${esc(message)}</div></div>`;
  },

  Hint({ message }) {
    return `<div data-c="Hint" class="hint sm muted">${esc(message)}</div>`;
  },

  Failure({ title, reason }) {
    return `<div data-c="Failure" class="failure-text"><div class="medium">${esc(title)}</div><div class="sm muted">${esc(reason)}</div></div>`;
  },

  Notice({ title, description, action }) {
    return `<div data-c="Notice" class="notice">${icon('git-merge', 'lg', 'var(--muted)')}<div class="lg medium">${esc(title)}</div><div class="description sm muted">${esc(description)}</div>${action}</div>`;
  },

  RefRow({ label, name = label.name, depth = 0, selected = false }) {
    const content = `<div class="h fill tree-row" style="padding-left:${depth * TreeIndent}px">${twisty(null)}${icon(TreeIcons[label.kind].ref, 'sm', 'var(--muted)')}<div class="fill ellipsis sm${label.head ? ' semibold' : ''}">${esc(name)}</div></div>`;
    return C.ListRow({ content, selected, name: 'RefRow' });
  },

  RefFolderRow({ name, kind, depth = 0, open = true }) {
    const shape = depth === 0 ? TreeIcons[kind].top : TreeIcons[kind].folder;
    const content = `<div class="h fill tree-row" style="padding-left:${depth * TreeIndent}px">${twisty(open)}${icon(shape, 'sm', 'var(--muted)')}<div class="fill ellipsis sm">${esc(name)}</div></div>`;
    return C.ListRow({ content, name: 'RefFolderRow' });
  },

  StashRow({ stash, selected = false }) {
    const content = `<div class="h fill tree-row">${twisty(null)}${icon('archive', 'sm', 'var(--muted)')}<div class="fill ellipsis sm">${esc(stash.message)}</div></div>`;
    return C.ListRow({ content, selected, name: 'StashRow' });
  },

  RefsList({ refs, stashes = [], selectedRow, collapsed = [] }) {
    const closed = new Set(collapsed);
    const nodes = (kind, list, depth) =>
      list
        .map((node) => {
          if (!node.children) return C.RefRow({ label: node.label, name: node.name, depth, selected: node.label.row === selectedRow });
          const open = !closed.has(`${kind}:${node.path}`);
          return treeGroup(open, C.RefFolderRow({ name: node.name, kind, depth, open }), nodes(kind, node.children, depth + 1));
        })
        .join('');
    const stashSection = () => {
      if (!stashes.length) return '';
      const open = !closed.has('stash');
      const rows = stashes.map((stash) => C.StashRow({ stash, selected: stash.row === selectedRow })).join('');
      return treeGroup(open, C.SectionHeader({ title: 'Stashes', count: stashes.length, open }), rows);
    };
    const sections = RefSections.map(([kind, title]) => {
      if (kind === 'stash') return stashSection();
      const items = refs.filter((label) => label.kind === kind);
      if (!items.length) return '';
      const open = !closed.has(kind);
      return treeGroup(open, C.SectionHeader({ title, count: items.length, open }), nodes(kind, refTree(items), 0));
    });
    return `<div data-c="RefsList" class="refs">${sections.join('')}</div>`;
  },

  CommitRow({ commit, selected = false, width }) {
    const color = LaneColors[commit.graph[0] % LaneColors.length];
    const refs = foldSyncedRefs(commit.refs);
    const stashBadge = commit.stash ? C.RefBadge({ name: commit.stash, kind: 'stash', color }) : '';
    const badges = stashBadge + refs.slice(0, 3).map((label) => C.RefBadge({ ...label, color })).join('');
    const more = refs.length > 3 ? `<div class="none xs muted">+${refs.length - 3}</div>` : '';
    const subject = commit.subject
      ? `<div class="fill ellipsis sm">${esc(commit.subject)}</div>`
      : '<div class="fill ellipsis sm muted">(no message)</div>';
    const content =
      C.GraphCell({ graph: commit.graph, merge: commit.merge, head: commit.head, stash: Boolean(commit.stash), width }) +
      `<div class="labels">${badges}${more}${subject}</div>` +
      `<div class="author h xs muted">${C.AuthorAvatar({ name: commit.author, email: commit.email })}<span class="ellipsis">${esc(commit.author)}</span></div>` +
      `<div class="date xs muted">${esc(commit.date)}</div>` +
      `<div class="short-id xs muted mono">${esc(commit.id)}</div>`;
    return C.ListRow({ content, selected, extraClass: 'commit-row', name: 'CommitRow' });
  },

  UncommittedRow({ changes, selected = false }) {
    const content =
      C.UncommittedCell() +
      '<div class="fill ellipsis sm italic">Uncommitted changes</div>' +
      `<div class="none xs muted">${esc(summarize(changes))}</div>`;
    return C.ListRow({ content, selected, extraClass: 'uncommitted-row', name: 'UncommittedRow' });
  },

  HistoryList({ history, selectedRow, changes, wipSelected = false }) {
    const width = graphWidth(history.commits);
    const wip = changes && (changes.unstaged.length || changes.staged.length) ? C.UncommittedRow({ changes, selected: wipSelected }) : '';
    const rows = history.commits.map((commit, ix) => C.CommitRow({ commit, selected: ix === selectedRow, width })).join('');
    const thumb = Math.max(20, Math.round((28 / history.total) * 800));
    return `<div data-c="HistoryList" class="history v">${wip}<div class="fill" style="position:relative;overflow:hidden">${rows}<div class="scrollbar" style="top:2px;height:${thumb}px"></div></div></div>`;
  },

  DiffView({ files, mode = 'unified', header = true, actions, selection, pointed, menu = '', words = true }) {
    const fold = (count, arrow) => {
      if (count <= 0) return '';
      const partial = count > FoldLimit;
      const reveal = partial ? C.Button({ icon: arrow, variant: 'ghost', tooltip: arrow === 'arrow-up' ? Texts.showAbove(FoldLimit) : Texts.showBelow(FoldLimit) }) : '';
      const all = C.Button({ icon: partial ? 'chevrons-up-down' : 'chevron-down', label: counted(count, 'unchanged line', 'unchanged lines'), variant: 'ghost', tooltip: Texts.expandLines(count) });
      return `<div class="diff-line fold">${reveal}${all}</div>`;
    };
    const line = (entry, side, marks) => {
      if (!entry) return '<div class="diff-line blank"><span class="number"></span><span class="sign"></span><span class="text"></span></div>';
      const [sign, oldNo, newNo, text] = entry;
      const kind = sign === '+' ? 'added' : sign === '-' ? 'deleted' : '';
      const pointedSide = pointed && (pointed.side || (sign === '-' ? 'old' : 'new'));
      const anchored = entry === marks.pointed && (side === 'both' || side === pointedSide);
      const classes = ['diff-line', kind, marks.selected.has(entry) && 'selected'].filter(Boolean).join(' ');
      const numbers =
        side === 'old'
          ? `<span class="number">${oldNo ?? ''}</span>`
          : side === 'new'
            ? `<span class="number">${newNo ?? ''}</span>`
            : `<span class="number">${oldNo ?? ''}</span><span class="number">${newNo ?? ''}</span>`;
      return `<div class="${classes}">${numbers}<span class="sign">${sign === ' ' ? '' : sign === '-' ? '−' : '+'}</span><span class="text">${highlight(text, marks.words.get(entry))}</span>${anchored ? menu : ''}</div>`;
    };
    const hunk = (h, hx, marks, own) => {
      const selected = own && selection && selection.hunk === hx ? selectionSize(selection) : 0;
      const buttons = actions ? C.HunkActions({ side: actions.side, selected, writing: actions.writing }) : '';
      const above = marks.hidden ? marks.hidden.above[hx] : 0;
      const below = marks.hidden && hx === marks.last ? marks.hidden.below : 0;
      const title = marks.hidden ? widenedHeader(h.header, above, below) : h.header;
      const top = `<div class="diff-line hunk"><span class="text">${esc(title)}</span>${buttons}</div>${fold(above, 'arrow-up')}`;
      if (mode === 'split') {
        return top + pairLines(h.lines).map(([left, right]) => `<div class="diff-split-row">${line(left, 'old', marks)}${line(right, 'new', marks)}</div>`).join('');
      }
      return top + h.lines.map((entry) => line(entry, 'both', marks)).join('');
    };
    const file = (f, fx) => {
      const own = fx === 0;
      const marks = {
        selected: selectedEntries(f.hunks, own ? selection : null),
        pointed: own && pointed ? f.hunks[pointed.hunk].lines[pointed.line] : null,
        words: words ? new Map(f.hunks.flatMap((h) => [...wordChanges(h.lines)])) : new Map(),
        hidden: f.lines && f.hunks.length ? hiddenLines(f.hunks, f.lines) : null,
        last: f.hunks.length - 1,
      };
      const top = header
        ? `<div class="diff-file-header">${C.Button({ icon: 'chevron-down', variant: 'ghost' })}<span class="title ellipsis">${esc(f.path)}</span><span class="stats xs muted">+${thousands(f.additions)} −${thousands(f.deletions)}</span></div>`
        : '';
      const end = fold(marks.hidden ? marks.hidden.below : 0, 'arrow-down');
      return `<div class="diff-file">${top}${f.hunks.map((h, hx) => hunk(h, hx, marks, own)).join('')}${end}</div>`;
    };
    return `<div data-c="DiffView" data-base="Diff" class="diff">${files.map(file).join('')}</div>`;
  },

  HunkActions({ side, selected = 0, writing = false }) {
    const { verb, icon: verbIcon, discards } = SideLabels[side];
    const tips = selected ? PartLabels.linesTooltip : PartLabels.hunkTooltip;
    const label = (action) => (selected ? PartLabels.lines(action, selected) : PartLabels.hunk(action));
    const discard = discards ? C.Button({ label: label('Discard'), icon: 'undo-2', variant: 'ghost', disabled: writing, tooltip: tips.Discard }) : '';
    const move = C.Button({ label: label(verb), icon: verbIcon, variant: 'ghost', disabled: writing, tooltip: tips[verb] });
    return `<div data-c="HunkActions" class="hunk-actions">${discard}${move}</div>`;
  },

  LineMenu({ side, selected = 0, changed = true, left = 0, top = 0, hot }) {
    return C.Menu({ items: lineMenuItems({ side, selected, changed }), left, top, hot, name: 'LineMenu' });
  },

  FileMenu({ change, side, left = 0, top = 0, hot }) {
    return C.Menu({ items: fileMenuItems({ change, side }), left, top, hot, name: 'FileMenu' });
  },

  CommitFileMenu({ file, left = 0, top = 0, hot }) {
    return C.Menu({ items: commitFileMenuItems({ file }), left, top, hot, name: 'CommitFileMenu' });
  },

  FileViewToggle({ view = 'list' } = {}) {
    const list = C.Button({ icon: 'list', variant: 'ghost', selected: view === 'list', tooltip: Texts.listView });
    const tree = C.Button({ icon: 'list-tree', variant: 'ghost', selected: view === 'tree', tooltip: Texts.treeView });
    return `<div data-c="FileViewToggle" class="file-view-toggle">${list}${tree}</div>`;
  },

  FolderRow({ name, depth = 0, open = true, trailing = '' }) {
    const content = `<div class="h fill tree-row" style="padding-left:${depth * TreeIndent}px">${twisty(open)}${icon('folder', 'sm', 'var(--muted)')}<div class="fill ellipsis sm">${esc(name)}</div></div>${trailing}`;
    return C.ListRow({ content, extraClass: 'folder-row', name: 'FolderRow' });
  },

  CommitPanel({ commit, details, state = 'ready', reason = '', open, view = 'list', collapsed, pointed }) {
    if (!commit || state === 'none') return `<div data-c="CommitPanel" class="commit-panel">${C.Hint({ message: Texts.selectCommit })}</div>`;
    const ready = state === 'ready' || state === 'empty';
    const files = state === 'empty' ? [] : details.files;
    const additions = files.reduce((sum, f) => sum + f.additions, 0);
    const deletions = files.reduce((sum, f) => sum + f.deletions, 0);
    const parents = details.parents.join(', ');
    const relation = details.parents.length > 1 ? 'merges' : 'parent';
    const body = ready && details.body ? `<div class="message-body sm muted">${esc(details.body)}</div>` : '';
    const stats =
      ready && files.length
        ? `<div class="h xs" style="gap:12px"><div class="muted">${counted(files.length, 'file', 'files')}</div>${C.Additions({ count: additions })}${C.Deletions({ count: deletions })}<div class="fill"></div>${C.FileViewToggle({ view })}</div>`
        : '';
    const copy = C.Button({ label: 'Copy hash', variant: 'ghost', tooltip: 'Copy the full hash  Ctrl+Shift+C' });
    const header =
      `<div class="panel-header">` +
      `<div class="base medium">${esc(commit.subject || '(no message)')}</div>` +
      body +
      `<div class="h xs muted" style="gap:8px">${C.AuthorAvatar({ name: commit.author, email: commit.email, size: 20 })}<span>${esc(commit.author)}</span><span>·</span><span>${esc(commit.date)}</span></div>` +
      `<div class="h xs muted" style="gap:8px"><span class="mono">${esc(commit.id)}</span>${parents ? `<span>${relation} ${esc(parents)}</span>` : ''}<div class="fill"></div>${copy}</div>` +
      stats +
      `</div>`;
    const content =
      state === 'loading'
        ? C.Loading({ message: Texts.readingCommit })
        : state === 'failed'
          ? C.Failure({ title: Texts.commitFailed, reason })
          : state === 'empty'
            ? C.Hint({ message: Texts.commitEmpty })
            : C.CommitFiles({ files, open, view, collapsed, pointed });
    return `<div data-c="CommitPanel" class="commit-panel">${header}<div class="fill v">${content}</div></div>`;
  },

  FileRow({ kind, path, depth, selected = false, trailing = '', name = 'FileRow' }) {
    const [file, folder] = splitPath(path);
    const place =
      depth === undefined
        ? C.ChangeBadge({ kind }) +
          `<div class="path"><div class="none ellipsis sm" style="max-width:100%">${esc(file)}</div>${folder ? `<div class="ellipsis xs muted">${esc(folder)}</div>` : ''}</div>`
        : `<div class="h fill tree-row" style="padding-left:${depth * TreeIndent}px">${twisty(null)}${C.ChangeBadge({ kind })}<div class="fill ellipsis sm">${esc(file)}</div></div>`;
    return C.ListRow({ content: place + trailing, selected, extraClass: 'file-row', name, base: name === 'FileRow' ? 'ListRow' : 'FileRow' });
  },

  CommitFileRow({ file, depth, selected = false, menu = '' }) {
    const counts = file.binary
      ? '<div class="counts xs muted">binary</div>'
      : `<div class="counts xs">${C.Additions({ count: file.additions })}${C.Deletions({ count: file.deletions })}</div>`;
    return C.FileRow({ kind: file.kind, path: file.path, depth, selected, trailing: counts + menu, name: 'CommitFileRow' });
  },

  CommitFiles({ files, open, view = 'list', collapsed = [], pointed }) {
    const menu = (file) => (pointed && pointed.path === file.path ? C.CommitFileMenu({ file, left: pointed.left, top: pointed.top, hot: pointed.hot }) : '');
    const row = (file, depth) => C.CommitFileRow({ file, depth, selected: file.path === open, menu: menu(file) });
    const rows =
      view === 'tree'
        ? fileTreeRows(fileTree(files), {
            collapsed: new Set(collapsed),
            file: row,
            folder: (node, depth, open) => C.FolderRow({ name: node.name, depth, open }),
          })
        : files.map((file) => row(file)).join('');
    return `<div data-c="CommitFiles" class="commit-files">${rows}</div>`;
  },

  ChangeRow({ change, side, depth, selected = false, writing = false, menu = '' }) {
    const labels = SideLabels[side];
    const trailing = C.Button({ icon: labels.icon, variant: 'ghost', disabled: writing, tooltip: labels.fileTooltip }) + menu;
    return C.FileRow({ kind: change.kind, path: change.path, depth, selected, trailing, name: 'ChangeRow' });
  },

  CommitForm({ message = '', description = '', focus, staged = 0, committing = false, writing = false, failure }) {
    const busy = committing || writing;
    const ready = !busy && staged > 0 && message.trim() !== '';
    const label = committing ? Texts.committing : staged ? `Commit ${counted(staged, 'file', 'files')}` : 'Commit';
    return (
      `<div data-c="CommitForm" class="commit-form">` +
      C.Input({ value: message, placeholder: Texts.commitMessage, focused: focus === 'message', disabled: committing }) +
      C.Input({ value: description, placeholder: Texts.commitDescription, lines: 4, focused: focus === 'description', disabled: committing }) +
      C.Button({ label, variant: 'primary', size: 'md', disabled: !ready && !committing, loading: committing, tooltip: Texts.commitTooltip }) +
      (failure ? `<div class="failure xs">${esc(failure)}</div>` : '') +
      `</div>`
    );
  },

  ChangesList({ changes, open, pointed, draft = {}, committing = false, writing = false, failure, view = 'list', collapsed = [] }) {
    if (!changes) return `<div data-c="ChangesList" class="changes">${C.Loading({ message: Texts.readingChanges })}</div>`;
    const busy = writing || committing;
    const form = C.CommitForm({ ...draft, staged: changes.staged.length, committing, writing, failure });
    if (!changes.unstaged.length && !changes.staged.length) {
      return `<div data-c="ChangesList" class="changes">${form}<div class="fill v">${C.Hint({ message: Texts.noChanges })}</div></div>`;
    }
    const changed = new Set([...changes.staged, ...changes.unstaged].map((change) => change.path)).size;
    const bar = `<div class="changes-bar">${C.SectionHeader({ title: counted(changed, 'changed file', 'changed files'), action: C.FileViewToggle({ view }) })}</div>`;
    const foldedIn = (side) => new Set(collapsed.filter((key) => key.startsWith(`${side}:`)).map((key) => key.slice(side.length + 1)));
    const section = (side, empty) => {
      const files = changes[side];
      const labels = SideLabels[side];
      const action = C.Button({ label: labels.all, icon: labels.icon, variant: 'ghost', disabled: busy || !files.length });
      const head = C.SectionHeader({ title: `${labels.files} (${thousands(files.length)})`, action });
      const at = (place, change) => !!place && place.side === side && place.path === change.path;
      const row = (change, depth) =>
        C.ChangeRow({
          change,
          side,
          depth,
          writing: busy,
          selected: at(open, change),
          menu: at(pointed, change) ? C.FileMenu({ change, side, left: pointed.left, top: pointed.top, hot: pointed.hot }) : '',
        });
      const folder = (node, depth, open) =>
        C.FolderRow({ name: node.name, depth, open, trailing: C.Button({ icon: labels.icon, variant: 'ghost', disabled: busy, tooltip: labels.folderTooltip }) });
      const rows =
        view === 'tree'
          ? fileTreeRows(fileTree(files), { collapsed: foldedIn(side), file: row, folder })
          : files.map((change) => row(change)).join('');
      const body = files.length ? `<div class="fill">${rows}</div>` : C.Hint({ message: empty });
      return `<div class="change-section">${head}${body}</div>`;
    };
    return `<div data-c="ChangesList" class="changes">${form}${bar}${section('staged', Texts.nothingStaged)}${section('unstaged', Texts.nothingUnstaged)}</div>`;
  },

  FileBar({ diff, state = 'ready', writing = false, expanded = false, filePosition }) {
    const labels = diff.commit ? null : SideLabels[diff.side];
    const shownState = diff.commit ? `in ${diff.commit}` : labels.state;
    const moveButton = labels ? C.Button({ label: labels.verb, icon: labels.icon, disabled: writing, tooltip: labels.openFileTooltip }) : '';
    const path = diff.change.original ? `${diff.change.original} → ${diff.change.path}` : diff.change.path;
    const text = !diff.kind || diff.kind === 'text';
    const facts = !text
      ? `<div class="none xs muted">${esc(diff.facts)}</div>`
      : state === 'ready' || state === 'empty'
        ? C.Additions({ count: state === 'empty' ? 0 : diff.additions, extraClass: 'none xs' }) + C.Deletions({ count: state === 'empty' ? 0 : diff.deletions, extraClass: 'none xs' })
        : '';
    const lfs = diff.lfs ? `<span class="lfs-chip" title="${esc(Texts.lfs)}">LFS</span>` : '';
    const stepper =
      expanded && filePosition
        ? `<div class="file-stepper">${C.Button({ icon: 'chevron-left', variant: 'ghost', tooltip: Texts.previousFile })}<span class="xs muted">File ${filePosition[0]} of ${filePosition[1]}</span>${C.Button({ icon: 'chevron-right', variant: 'ghost', tooltip: Texts.nextFile })}</div>`
        : '';
    const expand = C.Button({ icon: expanded ? 'minimize-2' : 'maximize-2', variant: 'ghost', selected: expanded, tooltip: expanded ? Texts.showPanes : Texts.useWindow });
    return (
      `<div data-c="FileBar" class="diff-toolbar">` +
      C.ChangeBadge({ kind: diff.change.kind }) +
      `<div class="ellipsis sm mono">${esc(path)}</div>` +
      lfs +
      `<div class="none xs muted">${esc(shownState)}</div>` +
      facts +
      `<div class="fill"></div>` +
      stepper +
      moveButton +
      expand +
      C.Button({ icon: 'close', variant: 'ghost', tooltip: 'Back to the graph  Esc' }) +
      `</div>`
    );
  },

  ChangeStepper({ current, total }) {
    const text = total ? `Change ${thousands(current)} of ${thousands(total)}` : 'No changes';
    return `<div data-c="ChangeStepper" class="change-stepper"><span class="xs muted">${text}</span>${C.Button({ icon: 'arrow-up', variant: 'ghost', disabled: !total, tooltip: Texts.previousChange })}${C.Button({ icon: 'arrow-down', variant: 'ghost', disabled: !total, tooltip: Texts.nextChange })}</div>`;
  },

  ZoomControl({ percent, fit = true }) {
    const shown = percent === undefined ? '' : `<span class="xs muted percent">${thousands(percent)} %</span>`;
    return `<div data-c="ZoomControl" class="zoom-control">${C.Button({ icon: 'minus', variant: 'ghost', tooltip: 'Zoom out  Ctrl+−' })}${shown}${C.Button({ icon: 'plus', variant: 'ghost', tooltip: 'Zoom in  Ctrl+=' })}${C.Button({ label: 'Fit', variant: 'ghost', selected: fit, tooltip: 'Fit the pane  Ctrl+0' })}</div>`;
  },

  BackdropButton({ open = false } = {}) {
    const classes = ['btn', 'ghost', 'xs', 'icon-only', open && 'selected'].filter(Boolean).join(' ');
    return `<span data-c="BackdropButton" data-base="Button" class="${classes}" title="${esc(Texts.backdrop)}"><span class="swatch checker"></span></span>`;
  },

  ViewOptionsMenu({ checked = ['words', 'wrap'], hot, right = 8, top = 30 }) {
    const rows = ViewOptions.map(
      (item) =>
        `<div class="item${item.key === hot ? ' hot' : ''}"><span class="check">${checked.includes(item.key) ? icon('check', 'sm') : ''}</span><span class="v label"><span>${esc(item.label)}</span><span class="about">${esc(item.about)}</span></span></div>`,
    ).join('');
    return `<div data-c="ViewOptionsMenu" data-base="PopupMenu" class="menu choices sm" style="right:${right}px;top:${top}px"><div class="heading">Every text comparison</div>${rows}</div>`;
  },

  ViewBar({ kind = 'text', mode, changes, zoom, menuOpen = false, hot }) {
    const modes = CompareModes[kind]
      .map((item) => C.Button({ label: item.label, icon: item.icon, variant: 'ghost', selected: item.key === mode, tooltip: item.tooltip }))
      .join('');
    const stepper = changes ? C.ChangeStepper(changes) : '';
    const separator = '<div class="separator"></div>';
    const tools =
      kind === 'text'
        ? C.Button({ icon: 'ellipsis', variant: 'ghost', selected: menuOpen, tooltip: Texts.viewOptions })
        : C.ZoomControl(zoom || {}) + (kind === 'image' ? C.BackdropButton() : '');
    const menu = menuOpen ? C.ViewOptionsMenu({ hot }) : '';
    return `<div data-c="ViewBar" class="view-bar"><div class="modes">${modes}</div><div class="fill"></div>${stepper}${stepper ? separator : ''}${tools}${menu}</div>`;
  },

  ChangeRuler({ hunks, lines }) {
    const marks = changeGroups(hunks).map((group) => {
      const color = group.added && group.deleted ? 'var(--yellow)' : group.added ? 'var(--green)' : 'var(--red)';
      const size = Math.max(group.end - group.start + 1, 1);
      return `<span class="mark" style="top:${percentOf(group.start - 1, lines)};height:${percentOf(size, lines)};--c:${color}"></span>`;
    });
    const [first] = newSpan(hunks[0]);
    const [, last] = newSpan(hunks[hunks.length - 1]);
    const shown = `<span class="view" style="top:${percentOf(first - 1, lines)};height:${percentOf(last - first + 1, lines)}"></span>`;
    return `<div data-c="ChangeRuler" class="change-ruler" title="Where the changes are in the whole file">${marks.join('')}${shown}</div>`;
  },

  VersionTag({ side, from, hearing = false, align = 'left' }) {
    const ear = hearing ? `<span class="ear">${icon('headphones', 'xs')}</span>` : '';
    return `<div data-c="VersionTag" class="version-tag${align === 'right' ? ' right' : ''}"><span class="version-dot ${side}"></span><span>${VersionLabels[side]}</span>${from ? `<span class="source">${esc(from)}</span>` : ''}${ear}</div>`;
  },

  MediaDetails({ details, sources, sides = ['before', 'after'], open = true }) {
    const { columns, before, after, deltas = {} } = details;
    const values = { before, after };
    const head = `<tr><th class="who"><span class="h" style="gap:6px">${twisty(open)}Details</span></th>${columns.map((name) => `<th>${esc(name)}</th>`).join('')}</tr>`;
    const row = (side) => {
      const cells = values[side].map((value, ix) => {
        const changed = side === 'after' && sides.length === 2 && value !== before[ix];
        const delta = changed && deltas[ix] ? `<span class="delta">${esc(deltas[ix])}</span>` : '';
        return `<td class="${changed ? 'changed' : ''}">${esc(value)}${delta}</td>`;
      });
      return `<tr><td class="who"><span class="h" style="gap:6px"><span class="version-dot ${side}"></span>${VersionLabels[side]}<span class="source">${esc(sources[side === 'before' ? 0 : 1])}</span></span></td>${cells.join('')}</tr>`;
    };
    const body = open ? sides.map(row).join('') : '';
    return `<div data-c="MediaDetails" class="media-details"><table>${head}${body}</table></div>`;
  },

  ImageCompare({ diff, mode = 'split', at = 0.46, mix = 0.5, probe, sides = ['before', 'after'] }) {
    const { image, sources, current = 0 } = diff;
    const zoom = mode === 'split' && sides.length === 2 ? 12 : 18;
    const size = image.after.length * zoom;
    const art = (side) => pixelArt(image[side], image.palette, zoom);
    const picture = (inner) => `<div class="picture checker" style="width:${size}px;height:${size}px">${inner}</div>`;
    const tag = (side, align) => C.VersionTag({ side, from: sources[side === 'before' ? 0 : 1], align });
    let canvas;
    if (sides.length === 1) {
      canvas = `<div class="compare-canvas">${tag(sides[0])}${picture(art(sides[0]))}</div>`;
    } else if (mode === 'split') {
      canvas = `<div class="compare-canvas split">${sides.map((side) => `<div class="side">${tag(side)}${picture(art(side))}</div>`).join('')}</div>`;
    } else if (mode === 'swipe') {
      const handle = `<div class="swipe-handle" style="left:${at * 100}%"><span class="knob">${icon('move-horizontal', 'sm')}</span></div>`;
      canvas = `<div class="compare-canvas">${tag('before')}${tag('after', 'right')}${picture(`${art('before')}<div class="layer" style="clip-path:inset(0 0 0 ${at * 100}%)">${art('after')}</div>${handle}`)}</div>`;
    } else if (mode === 'onion') {
      const slider = `<div class="onion-slider"><span class="h" style="gap:6px"><span class="version-dot before"></span>Before</span><span class="slider"><span class="track"></span><span class="fill" style="width:${mix * 100}%"></span><span class="thumb" style="left:${mix * 100}%"></span></span><span class="h" style="gap:6px">After<span class="version-dot after"></span></span></div>`;
      canvas = `<div class="compare-canvas">${tag('before')}${tag('after', 'right')}${picture(`${art('before')}<div class="layer" style="opacity:${mix}">${art('after')}</div>`)}${slider}</div>`;
    } else {
      const changes = pixelChanges(image.before, image.after);
      const paint = Object.fromEntries(Object.entries(PixelChanges).map(([key, value]) => [key, value.color]));
      const marks = pixelArt(changes, paint, zoom).replace(/fill="(var\([^)]+\))"/g, 'style="fill:$1"');
      const boxes = image.areas
        .map((area, ix) => `<div class="change-box${ix === current ? ' current' : ''}" style="left:${area.x * zoom - 4}px;top:${area.y * zoom - 4}px;width:${area.w * zoom + 8}px;height:${area.h * zoom + 8}px"><span class="number">${ix + 1}</span></div>`)
        .join('');
      const counts = Object.keys(PixelChanges).map((key) => [key, changes.join('').split(key).length - 1]);
      const changedPixels = counts.reduce((sum, [, n]) => sum + n, 0);
      const share = Math.round((changedPixels / (image.after.length * image.after[0].length)) * 100);
      const legend = `<div class="compare-legend">${counts.map(([key, n]) => `<span class="h" style="gap:6px"><span class="key" style="--c:${PixelChanges[key].color}"></span>${thousands(n)} ${PixelChanges[key].label}</span>`).join('')}<span>${share} % of the picture</span></div>`;
      const pointer = probe
        ? `<div class="pixel-hot" style="left:${probe.x * zoom}px;top:${probe.y * zoom}px;width:${zoom}px;height:${zoom}px"></div><div class="pixel-probe" style="left:${(probe.x + 1) * zoom + 12}px;top:${probe.y * zoom - 10}px"><div class="medium">Pixel ${probe.x}, ${probe.y}</div>${sides.map((side) => {
            const value = image.palette[image[side][probe.y][probe.x]] || 'transparent';
            return `<div class="h" style="gap:8px"><span class="swatch" style="background:${value}"></span><span class="mono">${esc(value)}</span><span class="muted">${VersionLabels[side]}</span></div>`;
          }).join('')}</div>`
        : '';
      canvas = `<div class="compare-canvas">${picture(`<div class="layer dim">${art('after')}</div><div class="layer">${marks}</div>${boxes}${pointer}`)}${legend}</div>`;
    }
    return `<div data-c="ImageCompare" class="media-compare">${canvas}${C.MediaDetails({ details: diff.details, sources, sides })}</div>`;
  },

  MediaTimeline({ duration, at, changes, current = 0, step = 10 }) {
    const total = Math.max(duration.before, duration.after);
    const place = (t) => percentOf(t, total);
    const lane = (side) => {
      const length = duration[side];
      const segments = changes
        .filter(([from]) => from < length)
        .map(([from, to]) => {
          const ix = changes.findIndex((change) => change[0] === from);
          return `<span class="segment${ix === current ? ' current' : ''}" style="left:${place(from)};width:${place(Math.min(to, length) - from)}"></span>`;
        })
        .join('');
      const ended = length < total ? `<span class="ended" style="left:${place(length)};right:0"></span>` : '';
      return `<div class="lane"><span class="who"><span class="version-dot ${side}"></span>${VersionLabels[side]}<span class="xs mono muted">${clock(length)}</span></span><span class="length" style="width:${place(length)}"></span>${ended}${segments}</div>`;
    };
    const numbers = changes.map(([from, to], ix) => `<span class="${ix === current ? 'current' : ''}" style="left:${place((from + to) / 2)}">${ix + 1}</span>`).join('');
    const ticks = [];
    for (let t = 0; t <= total; t += step) ticks.push(`<span style="left:${place(t)}">${clock(t, 0)}</span>`);
    return `<div data-c="MediaTimeline" class="media-timeline"><div class="tracks"><div class="numbers">${numbers}</div>${lane('before')}${lane('after')}<div class="ticks">${ticks.join('')}</div><span class="playhead" style="left:${place(at)}"></span></div></div>`;
  },

  Transport({ at, duration, frames = false, hearing = 'after', loop = false, playing = false }) {
    const total = Math.max(duration.before, duration.after);
    const play = `<span data-c="Button" class="btn primary play" title="${playing ? 'Pause both' : 'Play both'}  Space">${icon(playing ? 'pause' : 'play', 'sm')}</span>`;
    const stepBack = frames ? C.Button({ icon: 'step-back', variant: 'ghost', tooltip: 'Previous frame  ,' }) : '';
    const stepForward = frames ? C.Button({ icon: 'step-forward', variant: 'ghost', tooltip: 'Next frame  .' }) : '';
    const repeat = C.Button({ label: 'Loop change', icon: 'repeat', variant: 'ghost', selected: loop, tooltip: 'Play the current change over and over  L' });
    const ears = ['before', 'after'].map((side) => C.Button({ label: VersionLabels[side], dot: side, variant: 'ghost', selected: hearing === side, tooltip: Texts.hear(side) })).join('');
    return `<div data-c="Transport" class="transport">${stepBack}${play}${stepForward}<span class="time"><b>${clock(at)}</b> / ${clock(total)}</span>${repeat}<div class="fill"></div><span class="hear"><span class="label">${icon('headphones', 'xs')}Hear</span>${ears}</span></div>`;
  },

  VideoCompare({ diff, mode = 'split' }) {
    const { sources } = diff;
    const sides = ['before', 'after']
      .map((side, ix) => `<div class="side">${C.VersionTag({ side, from: sources[ix], hearing: diff.hearing === side })}<div class="frame">${starfallFrame(side)}</div></div>`)
      .join('');
    return (
      `<div data-c="VideoCompare" class="media-compare">` +
      `<div class="compare-canvas split" data-mode="${mode}">${sides}</div>` +
      C.MediaTimeline({ duration: diff.duration, at: diff.at, changes: diff.changes, current: diff.current }) +
      C.Transport({ at: diff.at, duration: diff.duration, frames: true, hearing: diff.hearing }) +
      C.MediaDetails({ details: diff.details, sources }) +
      `</div>`
    );
  },

  AudioCompare({ diff, mode = 'split', bins = 352 }) {
    const { sources, duration, changes, current = 0, song } = diff;
    const total = Math.max(duration.before, duration.after);
    const place = (t) => percentOf(t, total);
    const inChange = (t) => changes.some(([from, to]) => t >= from && t < to);
    const timeOf = (ix) => ((ix + 0.5) / bins) * total;
    const shared = noise(7, bins);
    const fresh = noise(11, bins);
    const lane = (side, ix) => {
      const grain = (bin, t) => (side === 'after' && inChange(t) ? fresh[bin] : shared[bin]);
      const levels = loudness(song[side], duration[side], total, bins, grain);
      const ended = duration[side] < total ? `<span class="ended" style="left:${place(duration[side])}"></span>` : '';
      const quiet = diff.hearing !== side;
      return `<div class="audio-lane${quiet ? ' quiet' : ''}">${C.VersionTag({ side, from: sources[ix], hearing: !quiet })}${waveform(levels, (bin) => inChange(timeOf(bin)))}${ended}</div>`;
    };
    const ruler = [];
    for (let t = 0; t <= total; t += 10) {
      const major = t % 30 === 0;
      ruler.push(`<span class="tick${major ? ' major' : ''}" style="left:${place(t)}"></span>${major ? `<span class="label" style="left:${place(t)}">${clock(t, 0)}</span>` : ''}`);
    }
    const regions = changes
      .map(([from, to], ix) => `<div class="audio-region${ix === current ? ' current' : ''}" style="left:${place(from)};width:${place(to - from)}"><span class="number">${ix + 1}</span></div>`)
      .join('');
    return (
      `<div data-c="AudioCompare" class="media-compare">` +
      `<div class="audio-area" data-mode="${mode}"><div class="audio-inner"><div class="time-ruler">${ruler.join('')}</div>${lane('before', 0)}${lane('after', 1)}${regions}<span class="playhead" style="left:${place(diff.at)}"><span class="knob"></span></span></div></div>` +
      C.Transport({ at: diff.at, duration, hearing: diff.hearing, loop: true }) +
      C.MediaDetails({ details: diff.details, sources }) +
      `</div>`
    );
  },

  BinaryFacts({ diff }) {
    const buttons =
      C.Button({ label: 'Open Before', icon: 'external-link', size: 'md', tooltip: 'Open a copy of the version before the change with its default app' }) +
      C.Button({ label: 'Open After', icon: 'external-link', size: 'md', tooltip: 'Open the file on disk with its default app' }) +
      C.Button({ label: 'Compare in difftool', variant: 'primary', size: 'md', tooltip: 'git difftool, with the tool set in diff.tool' });
    return (
      `<div data-c="BinaryFacts" class="media-compare">` +
      `<div class="compare-canvas"><div class="notice">${icon('file-question', 'lg', 'var(--muted)')}<div class="base medium">${esc(Texts.cannotShow(diff.type))}</div><div class="description sm muted">${esc(Texts.cannotShowDescription)}</div><div class="h" style="gap:8px;margin-top:4px">${buttons}</div></div></div>` +
      C.MediaDetails({ details: diff.details, sources: diff.sources }) +
      `</div>`
    );
  },

  FileDiffPanel({ diff, mode = 'split', state = 'ready', writing = false, reason = '', selection, pointed, expanded = false, filePosition, menuOpen = false, hot, view = {} }) {
    const kind = diff.kind || 'text';
    const labels = diff.commit ? null : SideLabels[diff.side];
    const ready = state === 'ready';
    const total = kind === 'text' ? changeGroups(diff.hunks).length : (diff.changes || diff.image?.areas || []).length;
    const zoom = kind === 'image' ? { percent: (mode === 'split' && !view.sides ? 12 : 18) * 100 } : kind === 'video' ? { percent: 36 } : {};
    const viewBar =
      kind === 'binary'
        ? ''
        : C.ViewBar({ kind, mode, changes: ready ? { current: (diff.current || 0) + 1, total } : null, zoom, menuOpen, hot });
    const text = () => {
      const diffView = C.DiffView({
        files: [{ path: diff.change.path, additions: diff.additions, deletions: diff.deletions, hunks: diff.hunks, lines: diff.lines }],
        mode,
        header: false,
        actions: labels && diff.change.kind === 'M' ? { side: diff.side, writing } : undefined,
        selection,
        pointed,
        menu: pointed && labels ? C.LineMenu({ side: diff.side, selected: pointed.selected, changed: pointed.changed, left: pointed.left, top: pointed.top, hot: pointed.hot }) : '',
      });
      const ruler = diff.lines ? C.ChangeRuler({ hunks: diff.hunks, lines: diff.lines }) : '';
      return `<div class="diff-area">${diffView}${ruler}</div>`;
    };
    const bodies = {
      text,
      image: () => C.ImageCompare({ diff, mode, ...view }),
      video: () => C.VideoCompare({ diff, mode }),
      audio: () => C.AudioCompare({ diff, mode }),
      binary: () => C.BinaryFacts({ diff }),
    };
    const content =
      state === 'loading'
        ? C.Loading({ message: Texts.readingChanges })
        : state === 'failed'
          ? C.Failure({ title: Texts.fileFailed, reason })
          : state === 'empty'
            ? C.Hint({ message: Texts.fileEmpty })
            : bodies[kind]();
    return `<div data-c="FileDiffPanel" class="file-diff">${C.FileBar({ diff, state, writing, expanded, filePosition })}${viewBar}<div class="fill v">${content}</div></div>`;
  },

  FileHistoryRow({ entry, selected = false }) {
    const { commit } = entry;
    const subject = commit.subject
      ? `<div class="ellipsis sm">${esc(commit.subject)}</div>`
      : '<div class="ellipsis sm muted">(no message)</div>';
    const counts = entry.binary
      ? '<div class="counts xs muted">binary</div>'
      : `<div class="counts xs">${C.Additions({ count: entry.additions })}${C.Deletions({ count: entry.deletions })}</div>`;
    const detail = entry.original ? Texts.renamedFrom(entry.original) : `${commit.author} · ${commit.date.slice(0, 10)}`;
    const avatar = entry.original ? '' : C.AuthorAvatar({ name: commit.author, email: commit.email, size: 14 });
    const content =
      C.ChangeBadge({ kind: entry.kind }) +
      `<div class="lines">${subject}<div class="h detail">${avatar}<div class="fill ellipsis xs muted">${esc(detail)}</div>${counts}</div></div>`;
    return C.ListRow({ content, selected, extraClass: 'file-history-row', name: 'FileHistoryRow' });
  },

  FileHistoryList({ history, selected = 0, reading = false }) {
    const head = C.SectionHeader({ title: Texts.fileHistory, count: reading ? undefined : history.entries.length, action: reading ? C.Spinner({ size: 'xs' }) : '' });
    const rows = reading ? '' : history.entries.map((entry, ix) => C.FileHistoryRow({ entry, selected: ix === selected })).join('');
    return `<div data-c="FileHistoryList" class="file-history">${head}${rows}</div>`;
  },

  RepoStatusBar({ summary = '', position = '', fetching = false, failure }) {
    const fetch = fetching
      ? '<div>Fetching…</div>'
      : failure
        ? `<div class="failure ellipsis">Fetch failed: ${esc(failure)}</div>`
        : '';
    return `<div data-c="RepoStatusBar" data-base="StatusBar" class="status-bar"><div class="ellipsis">${esc(summary)}</div><div class="right">${fetch}<div>${esc(position)}</div></div></div>`;
  },

  ToolbarButton({ label, icon: iconName, disabled = false, loading = false, tooltip }) {
    const classes = ['btn', 'ghost', 'toolbar-button', disabled && 'disabled'].filter(Boolean).join(' ');
    const title = tooltip ? ` title="${esc(tooltip)}"` : '';
    const glyph = loading ? icon('loader', 'md').replace('class="icon"', 'class="icon spinner"') : icon(iconName, 'md');
    return `<span data-c="ToolbarButton" data-base="Button" class="${classes}"${title}>${glyph}<span>${esc(label)}</span></span>`;
  },

  PullButton({ mode = 'merge', disabled = false, loading = false, menuOpen = false, ready = true }) {
    const button = C.ToolbarButton({ ...ToolbarGroups[0][0], tooltip: `Pull: ${pullMode(mode).label.toLowerCase()}`, disabled, loading });
    const caretClasses = ['btn', 'ghost', 'caret', !ready && 'disabled'].filter(Boolean).join(' ');
    const caret = `<span data-c="Button" class="${caretClasses}" title="Choose how Pull works">${icon('chevron-down', 'xs')}</span>`;
    return `<span data-c="PullButton" class="toolbar-split${menuOpen ? ' open' : ''}">${button}${caret}</span>`;
  },

  PullModeMenu({ mode = 'merge', left = 0, top = 0, hot }) {
    const rows = PullModes.map((item) =>
      item === '-'
        ? '<div class="separator"></div>'
        : `<div class="item${item.key === hot ? ' hot' : ''}"><span class="check">${item.key === mode ? icon('check', 'sm') : ''}</span><span class="v label"><span>${esc(item.label)}</span><span class="about">${esc(item.about)}</span></span></div>`,
    ).join('');
    return `<div data-c="PullModeMenu" data-base="PopupMenu" class="menu pull-modes sm" style="left:${left}px;top:${top}px"><div class="heading">Pull mode</div>${rows}</div>`;
  },

  RepoToolbar({ ready = true, running, unavailable = ['pop'], mode = 'merge', menuOpen = false } = {}) {
    const disabled = (key) => !ready || (running ? running !== key : unavailable.includes(key));
    const render = (button) =>
      button.key === 'pull'
        ? C.PullButton({ mode, disabled: disabled('pull'), loading: running === 'pull', menuOpen, ready })
        : C.ToolbarButton({ ...button, disabled: disabled(button.key), loading: running === button.key });
    const groups = ToolbarGroups.map((group) => `<div class="group">${group.map(render).join('')}</div>`);
    return `<div data-c="RepoToolbar" class="repo-toolbar">${groups.join('<div class="separator"></div>')}</div>`;
  },

  RepoView({ body, left, middle, right, status, toolbar = C.RepoToolbar(), expanded = false }) {
    const side = (place, content) => (expanded ? '' : `<div class="pane ${place}">${content}</div>`);
    const panes = body || `<div class="panes">${side('left', left)}<div class="pane middle">${middle}</div>${side('right', right)}</div>`;
    return `<div data-c="RepoView" class="repo-view">${toolbar}<div class="fill v">${panes}</div>${status}</div>`;
  },

  BranchMenu({ label, current, mode, left = 0, top = 0, hot }) {
    return C.Menu({ items: branchMenuItems({ label, current, mode }), left, top, hot, name: 'BranchMenu' });
  },

  StashMenu({ stash, left = 0, top = 0, hot }) {
    return C.Menu({ items: stashMenuItems({ stash }), left, top, hot, name: 'StashMenu' });
  },

  DeleteStashDialog({ stash }) {
    return C.AlertDialog({ title: Texts.deleteStashTitle(stash.name), description: Texts.deleteStashDescription(stash.message), confirm: 'Delete', name: 'DeleteStashDialog' });
  },

  DeleteBranchDialog({ label, only = 0 }) {
    if (label.kind === 'remote') {
      const { remote, branch } = remoteParts(label.name);
      return C.AlertDialog({ title: Texts.deleteRemoteTitle(branch, remote), description: Texts.deleteRemoteDescription(branch, remote), confirm: 'Delete', name: 'DeleteBranchDialog' });
    }
    const description = only === null ? Texts.deleteUncounted(label.name) : only ? Texts.deleteUnmerged(label.name, only) : Texts.deleteMerged(label.name);
    return C.AlertDialog({ title: Texts.deleteBranchTitle(label.name), description, confirm: 'Delete', name: 'DeleteBranchDialog' });
  },

  BranchDialog({ head, from, value = '', problem }) {
    const field = C.Input({ value, placeholder: Texts.branchPlaceholder, focused: true });
    const hint = problem ? `<div class="xs problem">${esc(problem)}</div>` : '';
    const buttons =
      C.Button({ label: 'Cancel', size: 'md' }) +
      C.Button({ label: Texts.createBranch, variant: 'primary', size: 'md', disabled: !value || Boolean(problem) });
    const close = `<span class="close">${icon('close', 'sm')}</span>`;
    const description = from ? Texts.branchFromDescription(from) : Texts.branchDescription(head);
    return `<div class="overlay"><div data-c="BranchDialog" data-base="Dialog" class="dialog">${close}<div class="base semibold">${esc(Texts.branchTitle)}</div><div class="sm muted">${esc(description)}</div><div class="v field">${field}${hint}</div><div class="footer stretch">${buttons}</div></div></div>`;
  },

  Menu({ items, left = 0, top = 0, hot, name = 'Menu' }) {
    const rows = items
      .map((item) =>
        item === '-'
          ? '<div class="separator"></div>'
          : `<div class="item${item[0] === hot ? ' hot' : ''}"><span class="label">${esc(item[0])}</span><span class="keys">${esc(item[1])}</span></div>`,
      )
      .join('');
    const length = (value) => (typeof value === 'number' ? `${value}px` : value);
    return `<div data-c="${name}" data-base="PopupMenu" class="menu sm" style="left:${length(left)};top:${length(top)}">${rows}</div>`;
  },

  Notification({ message, title, kind }) {
    const glyphs = {
      success: icon('circle-check', 'md', 'var(--green)'),
      warning: icon('triangle-alert', 'md', 'var(--yellow)'),
      error: icon('circle-x', 'md', 'var(--red)'),
    };
    const glyph = glyphs[kind] ? `<span class="kind">${glyphs[kind]}</span>` : '';
    const heading = title ? `<div class="semibold">${esc(title)}</div>` : '';
    return `<div data-c="Notification" class="toast sm">${glyph}<div class="message">${heading}${esc(message)}</div>${C.Button({ icon: 'close', variant: 'ghost' })}</div>`;
  },

  AlertDialog({ title, description, confirm, name = 'AlertDialog' }) {
    const buttons = confirm
      ? C.Button({ label: 'Cancel', size: 'md' }) + C.Button({ label: confirm, variant: 'danger', size: 'md' })
      : C.Button({ label: 'OK', variant: 'primary', size: 'md' });
    const base = name === 'AlertDialog' ? '' : ' data-base="AlertDialog"';
    return `<div class="overlay"><div data-c="${name}"${base} class="dialog"><div class="base semibold">${esc(title)}</div><div class="sm muted">${esc(description)}</div><div class="footer">${buttons}</div></div></div>`;
  },

  DiscardDialog({ change }) {
    const [name] = splitPath(change.path);
    const untracked = change.kind === '?';
    return C.AlertDialog({
      title: untracked ? Texts.deleteTitle(name) : Texts.discardTitle(name),
      description: untracked ? Texts.deleteDescription(change.path) : Texts.discardDescription(change.path),
      confirm: untracked ? 'Delete' : 'Discard',
    });
  },

  FastGit({ topBar, tabStrip = '', body, overlays = '' }) {
    return `<div data-c="FastGit" class="fg fg-window">${topBar}${tabStrip}<div class="fill v">${body}</div>${overlays}</div>`;
  },
};
