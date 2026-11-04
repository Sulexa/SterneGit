const IconPaths = {
  'git-branch': '<line x1="6" x2="6" y1="3" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 0 1-9 9"/>',
  'git-merge': '<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M6 21V9a9 9 0 0 0 9 9"/>',
  cloud: '<path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/>',
  tag: '<path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"/><circle cx="7.5" cy="7.5" r=".5" fill="currentColor"/>',
  'columns-2': '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M12 3v18"/>',
  'rows-2': '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 12h18"/>',
  plus: '<path d="M5 12h14"/><path d="M12 5v14"/>',
  minus: '<path d="M5 12h14"/>',
  close: '<path d="M18 6 6 18"/><path d="m6 6 12 12"/>',
  'triangle-alert': '<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/>',
  loader: '<path d="M21 12a9 9 0 1 1-6.219-8.56"/>',
  'chevron-down': '<path d="m6 9 6 6 6-6"/>',
  'chevron-right': '<path d="m9 18 6-6-6-6"/>',
  folder: '<path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>',
  'window-minimize': '<path d="M6 12h12"/>',
  'window-maximize': '<rect width="12" height="12" x="6" y="6" rx="1"/>',
  'window-close': '<path d="M17 7 7 17"/><path d="m7 7 10 10"/>',
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

const RefIcons = { local: 'git-branch', remote: 'cloud', tag: 'tag' };

const TreeIcons = {
  local: { top: 'folder', folder: 'folder', ref: 'git-branch' },
  remote: { top: 'cloud', folder: 'folder', ref: 'git-branch' },
  tag: { top: 'folder', folder: 'folder', ref: 'tag' },
};

const TreeIndent = 18;

const RefSections = [
  ['local', 'Branches'],
  ['remote', 'Remotes'],
  ['tag', 'Tags'],
];

const SideLabels = {
  unstaged: {
    files: 'Unstaged files',
    state: 'unstaged',
    verb: 'Stage',
    all: 'Stage all',
    fileTooltip: 'Stage file',
    openFileTooltip: 'Stage this file',
    icon: 'plus',
  },
  staged: {
    files: 'Staged files',
    state: 'staged',
    verb: 'Unstage',
    all: 'Unstage all',
    fileTooltip: 'Unstage file',
    openFileTooltip: 'Unstage this file',
    icon: 'minus',
  },
};

const Menus = {
  File: [['Open Repository…', 'Ctrl+O'], ['Close Tab', 'Ctrl+W'], '-', ['Quit', 'Ctrl+Q']],
  Edit: [['Copy Commit Hash', 'Ctrl+Shift+C']],
  View: [
    ['Refresh', 'Ctrl+R'],
    ['Fetch All Remotes', 'Ctrl+Shift+R'],
    ['Close File Diff', 'Esc'],
    '-',
    ['Newer Commit', '↑'],
    ['Older Commit', '↓'],
    '-',
    ['Next Tab', 'Ctrl+Tab'],
    ['Previous Tab', 'Ctrl+Shift+Tab'],
  ],
  About: [['About FastGit', '']],
};

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
  aboutDescription: 'A lightweight Git client for Linux, macOS and Windows, built with GPUI and gitoxide.',
  historyLimit: 10000,
};

const esc = (text) =>
  String(text).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

const thousands = (n) => n.toLocaleString('en-US');

const counted = (n, singular, plural) => `${thousands(n)} ${n === 1 ? singular : plural}`;

const join = (parts) => parts.filter(Boolean).join('');

function icon(name, size = 'md', color) {
  const px = IconSizes[size];
  const style = color ? ` style="color:${color}"` : '';
  return `<svg class="icon" width="${px}" height="${px}" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"${style}>${IconPaths[name]}</svg>`;
}

const RustTokens = /("(?:[^"\\]|\\.)*")|\b(fn|let|pub|impl|if|else|return|match|mut|use|struct|enum|crate|self|Self|true|false|move|for|in|const)\b|\b([A-Z][A-Za-z0-9_]*)\b|\b(\d+(?:\.\d+)*)\b|\b([a-z_][a-z0-9_]*)(?=\()/g;

function highlight(code) {
  let html = '';
  let last = 0;
  for (const match of code.matchAll(RustTokens)) {
    html += esc(code.slice(last, match.index));
    const kind = match[1] ? 'string' : match[2] ? 'keyword' : match[3] ? 'type' : match[4] ? 'number' : 'function';
    html += `<span class="tok-${kind}">${esc(match[0])}</span>`;
    last = match.index + match[0].length;
  }
  return html + esc(code.slice(last));
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
  const summary = `${thousands(history.total)} commits · ${thousands(history.refs.length)} refs · read in ${history.readMs} ms`;
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

const C = {
  Button({ label, icon: iconName, variant = 'default', size = 'xs', selected = false, disabled = false, tooltip }) {
    const classes = ['btn', variant, size, selected && 'selected', disabled && 'disabled', !label && 'icon-only']
      .filter(Boolean)
      .join(' ');
    const title = tooltip ? ` title="${esc(tooltip)}"` : '';
    return `<span data-c="Button" class="${classes}"${title}>${iconName ? icon(iconName, size === 'xs' ? 'xs' : 'sm') : ''}${label ? `<span>${esc(label)}</span>` : ''}</span>`;
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

  ListRow({ content, selected = false, extraClass = '', name = 'ListRow' }) {
    const base = name === 'ListRow' ? '' : ' data-base="ListRow"';
    return `<div data-c="${name}"${base} class="list-row${selected ? ' selected' : ''}${extraClass ? ` ${extraClass}` : ''}">${content}</div>`;
  },

  SectionHeader({ title, count, action, open }) {
    const foldable = open !== undefined;
    const shownCount = count === undefined ? '' : `<span>${thousands(count)}</span>`;
    return `<div data-c="SectionHeader" class="section-header${foldable ? ' foldable' : ''}">${foldable ? twisty(open) : ''}<span class="title ellipsis">${esc(title)}</span>${shownCount}${action || ''}</div>`;
  },

  RefBadge({ name, kind, head = false, color = LaneColors[0] }) {
    return `<div data-c="RefBadge" class="ref-badge${head ? ' head' : ''}" style="--c:${color}">${icon(RefIcons[kind], 'xs')}<div class="name ellipsis">${esc(name)}</div></div>`;
  },

  ChangeBadge({ kind }) {
    return `<div data-c="ChangeBadge" class="change-badge" style="--c:${ChangeColors[kind]}">${esc(kind)}</div>`;
  },

  Additions({ count, extraClass = '' }) {
    return `<span data-c="Additions" class="additions${extraClass ? ` ${extraClass}` : ''}">+${thousands(count)}</span>`;
  },

  Deletions({ count, extraClass = '' }) {
    return `<span data-c="Deletions" class="deletions${extraClass ? ` ${extraClass}` : ''}">−${thousands(count)}</span>`;
  },

  GraphCell({ graph, merge = false, head = false, width }) {
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
              ? `M${x(node)} 14 L${x(node)} 28`
              : `M${x(node)} 14 C${x(lane)} 14 ${x(lane)} 14 ${x(lane)} 28`;
      return `<path d="${d}" fill="none" style="stroke:${color(lane)}" stroke-width="2"/>`;
    });
    const radius = merge ? 4.5 * 0.7 : 4.5;
    const dot = `<circle cx="${x(node)}" cy="14" r="${radius}" style="fill:${color(node)}"/>`;
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

  RefsList({ refs, selectedRow, collapsed = [] }) {
    const closed = new Set(collapsed);
    const nodes = (kind, list, depth) =>
      list
        .map((node) => {
          if (!node.children) return C.RefRow({ label: node.label, name: node.name, depth, selected: node.label.row === selectedRow });
          const open = !closed.has(`${kind}:${node.path}`);
          return treeGroup(open, C.RefFolderRow({ name: node.name, kind, depth, open }), nodes(kind, node.children, depth + 1));
        })
        .join('');
    const sections = RefSections.map(([kind, title]) => {
      const items = refs.filter((label) => label.kind === kind);
      if (!items.length) return '';
      const open = !closed.has(kind);
      return treeGroup(open, C.SectionHeader({ title, count: items.length, open }), nodes(kind, refTree(items), 0));
    });
    return `<div data-c="RefsList" class="refs">${sections.join('')}</div>`;
  },

  CommitRow({ commit, selected = false, width }) {
    const color = LaneColors[commit.graph[0] % LaneColors.length];
    const badges = commit.refs.slice(0, 3).map(([name, kind, head]) => C.RefBadge({ name, kind, head, color })).join('');
    const more = commit.refs.length > 3 ? `<div class="none xs muted">+${commit.refs.length - 3}</div>` : '';
    const subject = commit.subject
      ? `<div class="fill ellipsis sm">${esc(commit.subject)}</div>`
      : '<div class="fill ellipsis sm muted">(no message)</div>';
    const content =
      C.GraphCell({ graph: commit.graph, merge: commit.merge, head: commit.head, width }) +
      `<div class="labels">${badges}${more}${subject}</div>` +
      `<div class="author ellipsis xs muted">${esc(commit.author)}</div>` +
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

  DiffView({ files, mode = 'unified', header = true }) {
    const line = (entry, side) => {
      if (!entry) return '<div class="diff-line blank"><span class="number"></span><span class="sign"></span><span class="text"></span></div>';
      const [sign, oldNo, newNo, text] = entry;
      const kind = sign === '+' ? ' added' : sign === '-' ? ' deleted' : '';
      const numbers =
        side === 'old'
          ? `<span class="number">${oldNo ?? ''}</span>`
          : side === 'new'
            ? `<span class="number">${newNo ?? ''}</span>`
            : `<span class="number">${oldNo ?? ''}</span><span class="number">${newNo ?? ''}</span>`;
      return `<div class="diff-line${kind}">${numbers}<span class="sign">${sign === ' ' ? '' : sign === '-' ? '−' : '+'}</span><span class="text">${highlight(text)}</span></div>`;
    };
    const hunk = (h) => {
      const top = `<div class="diff-line hunk"><span class="text">${esc(h.header)}</span></div>`;
      if (mode === 'split') {
        return top + pairLines(h.lines).map(([left, right]) => `<div class="diff-split-row">${line(left, 'old')}${line(right, 'new')}</div>`).join('');
      }
      return top + h.lines.map((entry) => line(entry, 'both')).join('');
    };
    const file = (f) => {
      const top = header
        ? `<div class="diff-file-header">${icon('chevron-down', 'xs', 'var(--muted)')}<span class="ellipsis mono">${esc(f.path)}</span>${C.Additions({ count: f.additions })}${C.Deletions({ count: f.deletions })}</div>`
        : '';
      return `<div class="diff-file">${top}${f.hunks.map(hunk).join('')}</div>`;
    };
    return `<div data-c="DiffView" data-base="Diff" class="diff">${files.map(file).join('')}</div>`;
  },

  CommitPanel({ commit, details, state = 'ready', reason = '' }) {
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
        ? `<div class="h xs" style="gap:12px"><div class="muted">${counted(files.length, 'file', 'files')}</div>${C.Additions({ count: additions })}${C.Deletions({ count: deletions })}</div>`
        : '';
    const copy = C.Button({ label: 'Copy hash', variant: 'ghost', tooltip: 'Copy the full hash  Ctrl+Shift+C' });
    const header =
      `<div class="panel-header">` +
      `<div class="base medium">${esc(commit.subject || '(no message)')}</div>` +
      body +
      `<div class="h xs muted" style="gap:8px"><span>${esc(commit.author)}</span><span>·</span><span>${esc(commit.date)}</span></div>` +
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
            : C.DiffView({ files, mode: 'unified' });
    return `<div data-c="CommitPanel" class="commit-panel">${header}<div class="fill v">${content}</div></div>`;
  },

  ChangeRow({ change, side, selected = false, writing = false }) {
    const [name, folder] = splitPath(change.path);
    const labels = SideLabels[side];
    const content =
      C.ChangeBadge({ kind: change.kind }) +
      `<div class="path"><div class="none ellipsis sm" style="max-width:100%">${esc(name)}</div>${folder ? `<div class="ellipsis xs muted">${esc(folder)}</div>` : ''}</div>` +
      C.Button({ icon: labels.icon, variant: 'ghost', disabled: writing, tooltip: labels.fileTooltip });
    return C.ListRow({ content, selected, extraClass: 'change-row', name: 'ChangeRow' });
  },

  ChangesList({ changes, open, writing = false, failure }) {
    if (!changes) return `<div data-c="ChangesList" class="changes">${C.Loading({ message: Texts.readingChanges })}</div>`;
    const header =
      `<div class="panel-header">` +
      `<div class="base medium">Uncommitted changes</div>` +
      `<div class="xs muted">${thousands(changes.unstaged.length)} unstaged · ${thousands(changes.staged.length)} staged</div>` +
      (failure ? `<div class="xs" style="color:var(--red)">${esc(failure)}</div>` : '') +
      `</div>`;
    if (!changes.unstaged.length && !changes.staged.length) {
      return `<div data-c="ChangesList" class="changes">${header}${C.Hint({ message: Texts.noChanges })}</div>`;
    }
    const section = (side) => {
      const files = changes[side];
      const labels = SideLabels[side];
      const action = C.Button({ label: labels.all, icon: labels.icon, variant: 'ghost', disabled: writing || !files.length });
      const head = C.SectionHeader({ title: `${labels.files} (${thousands(files.length)})`, action });
      const rows = files
        .map((change) => C.ChangeRow({ change, side, writing, selected: !!open && open.side === side && open.path === change.path }))
        .join('');
      return head + rows;
    };
    return `<div data-c="ChangesList" class="changes">${header}<div class="fill">${section('unstaged')}${section('staged')}</div></div>`;
  },

  FileDiffPanel({ diff, mode = 'split', state = 'ready', writing = false, reason = '' }) {
    const labels = SideLabels[diff.side];
    const path = diff.change.original ? `${diff.change.original} → ${diff.change.path}` : diff.change.path;
    const stats = state === 'ready' || state === 'empty'
      ? C.Additions({ count: state === 'empty' ? 0 : diff.additions, extraClass: 'none xs' }) + C.Deletions({ count: state === 'empty' ? 0 : diff.deletions, extraClass: 'none xs' })
      : '';
    const toolbar =
      `<div class="diff-toolbar">` +
      C.ChangeBadge({ kind: diff.change.kind }) +
      `<div class="ellipsis sm mono">${esc(path)}</div>` +
      `<div class="none xs muted">${labels.state}</div>` +
      stats +
      `<div class="fill"></div>` +
      C.Button({ label: 'Split', icon: 'columns-2', variant: 'ghost', selected: mode === 'split', tooltip: 'Compare side by side' }) +
      C.Button({ label: 'Unified', icon: 'rows-2', variant: 'ghost', selected: mode === 'unified', tooltip: 'Show changes in one column' }) +
      C.Button({ label: labels.verb, icon: labels.icon, disabled: writing, tooltip: labels.openFileTooltip }) +
      C.Button({ icon: 'close', variant: 'ghost', tooltip: 'Back to the graph  Esc' }) +
      `</div>`;
    const content =
      state === 'loading'
        ? C.Loading({ message: Texts.readingChanges })
        : state === 'failed'
          ? C.Failure({ title: Texts.fileFailed, reason })
          : state === 'empty'
            ? C.Hint({ message: Texts.fileEmpty })
            : C.DiffView({ files: [{ path: diff.change.path, additions: diff.additions, deletions: diff.deletions, hunks: diff.hunks }], mode, header: false });
    return `<div data-c="FileDiffPanel" class="file-diff">${toolbar}<div class="fill v">${content}</div></div>`;
  },

  RepoStatusBar({ summary = '', position = '', fetching = false, failure }) {
    const fetch = fetching
      ? '<div>Fetching…</div>'
      : failure
        ? `<div class="failure ellipsis">Fetch failed: ${esc(failure)}</div>`
        : '';
    return `<div data-c="RepoStatusBar" data-base="StatusBar" class="status-bar"><div class="ellipsis">${esc(summary)}</div><div class="right">${fetch}<div>${esc(position)}</div></div></div>`;
  },

  RepoView({ body, left, middle, right, status }) {
    const panes = body || `<div class="panes"><div class="pane left">${left}</div><div class="pane middle">${middle}</div><div class="pane right">${right}</div></div>`;
    return `<div data-c="RepoView" class="repo-view"><div class="fill v">${panes}</div>${status}</div>`;
  },

  Menu({ items, left = 0, top = 0, hot }) {
    const rows = items
      .map((item) =>
        item === '-'
          ? '<div class="separator"></div>'
          : `<div class="item${item[0] === hot ? ' hot' : ''}"><span class="label">${esc(item[0])}</span><span class="keys">${esc(item[1])}</span></div>`,
      )
      .join('');
    return `<div data-c="Menu" data-base="PopupMenu" class="menu sm" style="left:${left}px;top:${top}px">${rows}</div>`;
  },

  Notification({ message }) {
    return `<div data-c="Notification" class="toast sm"><div class="message">${esc(message)}</div>${C.Button({ icon: 'close', variant: 'ghost' })}</div>`;
  },

  AlertDialog({ title, description }) {
    return `<div class="overlay"><div data-c="AlertDialog" class="dialog"><div class="lg medium">${esc(title)}</div><div class="sm muted">${esc(description)}</div><div class="footer">${C.Button({ label: 'OK', variant: 'primary', size: 'md' })}</div></div></div>`;
  },

  FastGit({ topBar, tabStrip = '', body, overlays = '' }) {
    return `<div data-c="FastGit" class="fg fg-window">${topBar}${tabStrip}<div class="fill v">${body}</div>${overlays}</div>`;
  },
};
