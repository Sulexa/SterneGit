const Viewer = (() => {
  const WINDOW = { w: 1440, h: 900 };
  const ThemeTokens = [
    'background', 'foreground', 'border', 'muted', 'sidebar', 'title-bar', 'tab-bar', 'tab-active',
    'tab-foreground', 'status-bar', 'list-active', 'list-hover', 'secondary', 'secondary-hover',
    'primary', 'primary-foreground', 'popover', 'accent', 'overlay', 'scrollbar-thumb', 'danger', 'input', 'input-background', 'ring', 'selection',
  ];
  const BaseColors = ['blue', 'magenta', 'green', 'yellow', 'cyan', 'red'];
  const KitBases = {
    TitleBar: 'gpui_kit::TitleBar',
    TabBar: 'gpui_kit::tab::TabBar',
    StatusBar: 'gpui_kit::status_bar::StatusBar',
    Diff: 'gpui_kit::diff::Diff',
    PopupMenu: 'gpui_kit::menu::PopupMenu',
  };

  const byName = new Map();
  for (const group of Catalog) for (const item of group.items) byName.set(item.name, { group, item });

  const namesIn = (html) => {
    const names = new Set();
    for (const match of html.matchAll(/data-(?:c|base)="([A-Za-z]+)"/g)) names.add(match[1]);
    return names;
  };

  const usage = new Map();
  for (const screen of Screens) {
    screen.html = screen.render();
    screen.components = [...namesIn(screen.html)].filter((name) => byName.has(name));
    for (const name of screen.components) {
      if (!usage.has(name)) usage.set(name, []);
      usage.get(name).push(screen);
    }
  }

  const builtFrom = (item) => {
    if (!item.variants) return [];
    const names = new Set();
    for (const [, render] of item.variants) for (const name of namesIn(render())) names.add(name);
    names.delete(item.name);
    return [...names].filter((name) => byName.has(name));
  };

  const componentLink = (name) => {
    const entry = byName.get(name);
    return entry ? `<a class="mk-chip" href="#/components/${entry.group.key}/${name}">${name}</a>` : `<span class="mk-chip kit">${name}</span>`;
  };

  const screenLink = (screen) => `<a class="mk-chip screen" href="#/screen/${screen.id}">${screen.title}</a>`;

  const nav = (route) => {
    const screens = Screens.map(
      (screen) => `<a class="${route === `screen/${screen.id}` ? 'active' : ''}" href="#/screen/${screen.id}">${screen.title}</a>`,
    ).join('');
    const groups = Catalog.map((group) => {
      const items = group.items
        .map((item) => `<a class="sub${route === `components/${group.key}/${item.name}` ? ' active' : ''}" href="#/components/${group.key}/${item.name}">${item.name}</a>`)
        .join('');
      return `<a class="${route.startsWith(`components/${group.key}`) ? 'active' : ''}" href="#/components/${group.key}">${group.title}</a>${items}`;
    }).join('');
    return `<a class="mk-brand" href="#/">SterneGit mockup</a><a class="${route === '' ? 'active' : ''}" href="#/">Overview</a><h2>Screens</h2>${screens}<h2>Components</h2>${groups}`;
  };

  const scaled = (html, scale, w = WINDOW.w, h = WINDOW.h) =>
    `<div class="mk-scaler" style="width:${w * scale}px;height:${h * scale}px"><div style="width:${w}px;height:${h}px;transform:scale(${scale});transform-origin:0 0">${html}</div></div>`;

  const overview = () => {
    const cards = Screens.map(
      (screen) => `<a class="mk-card" href="#/screen/${screen.id}">${scaled(screen.html, 0.22)}<div class="mk-card-title">${screen.title}</div></a>`,
    ).join('');
    const groups = Catalog.map(
      (group) => `<a class="mk-group-card" href="#/components/${group.key}"><div class="mk-card-title">${group.title}</div><p>${group.about}</p><div class="mk-count">${group.items.length} entries</div></a>`,
    ).join('');
    return `<header class="mk-head"><h1>SterneGit mockup</h1><p>Every screen of the app, built from the same reusable components as the Rust code. Change a component in <code>components.js</code> and every screen that uses it follows.</p></header><h2 class="mk-section">Screens</h2><div class="mk-cards">${cards}</div><h2 class="mk-section">Components</h2><div class="mk-groups">${groups}</div>`;
  };

  const screenPage = (id) => {
    const ix = Screens.findIndex((screen) => screen.id === id);
    const screen = Screens[ix];
    if (!screen) return overview();
    const prev = Screens[ix - 1];
    const next = Screens[ix + 1];
    return `<header class="mk-head">
      <div class="mk-pager">${prev ? `<a href="#/screen/${prev.id}">← ${prev.title}</a>` : '<span></span>'}${next ? `<a href="#/screen/${next.id}">${next.title} →</a>` : '<span></span>'}</div>
      <h1>${screen.title}</h1>
      <p>${screen.about}</p>
      <div class="mk-meta">Laid out in <code>${screen.source}</code> · <a href="#/raw/${screen.id}">Open alone at 1440 × 900</a></div>
      <div class="mk-chips"><span class="mk-label">Built from</span>${screen.components.map(componentLink).join('')}</div>
      <div class="mk-tools">
        <label><input type="checkbox" id="mk-outline"${state.outline ? ' checked' : ''}> Outline components</label>
        <span class="mk-zoom"><button data-zoom="fit" class="${state.zoom === 'fit' ? 'on' : ''}">Fit</button><button data-zoom="1" class="${state.zoom === '1' ? 'on' : ''}">100 %</button></span>
      </div>
    </header>
    <div class="mk-stage${state.outline ? ' mk-outline' : ''}" id="mk-stage">${scaled(screen.html, 1)}</div>`;
  };

  const special = (kind) => {
    const root = getComputedStyle(document.documentElement);
    const swatch = (name, label = `--${name}`) =>
      `<div class="mk-swatch"><div class="mk-swatch-color" style="background:var(--${name})"></div><div><div class="mk-swatch-name">${label}</div><code>${root.getPropertyValue(`--${name}`).trim()}</code></div></div>`;
    if (kind === 'colors') return `<div class="mk-swatches">${ThemeTokens.map((name) => swatch(name)).join('')}</div><h4>Base colors</h4><div class="mk-swatches">${BaseColors.map((name) => swatch(name)).join('')}</div>`;
    if (kind === 'lanes') {
      const graph = [0, 6, 'P1 P2 P3 P4 P5 O0'];
      return `<div class="mk-swatches">${BaseColors.map((name, ix) => swatch(name, `lane ${ix} · ${name}`)).join('')}</div><div class="mk-specimen fg" style="display:inline-block;margin-top:12px">${C.GraphCell({ graph, head: true })}</div>`;
    }
    if (kind === 'type') {
      const rows = [
        ['lg_text', 'lg', '18 / 28', 'Notice titles'],
        ['base_text', 'base', '16 / 24', 'Panel titles (commit subject, Uncommitted changes)'],
        ['sm_text', 'sm', '14 / 20', 'Row text, descriptions'],
        ['xs_text', 'xs', '12 / 16', 'Meta: authors, dates, counts, section headers, status bar'],
      ];
      return `<table class="mk-table fg">${rows.map(([gpui, cls, px, use]) => `<tr><td><code>${gpui}</code></td><td>${px}</td><td class="${cls}">The quick brown fox</td><td class="muted xs">${use}</td></tr>`).join('')}<tr><td><code>mono_font_family</code></td><td>12–13</td><td class="mono sm">3f9c2e1 src/git/patch.rs</td><td class="muted xs">Short ids, paths, diffs</td></tr><tr><td><code>ROW_HEIGHT</code></td><td>28 px</td><td>${C.ListRow({ content: '<span class="sm">Every list row and section header</span>' })}</td><td class="muted xs">src/ui/rows.rs</td></tr></table>`;
    }
    if (kind === 'icons') {
      return `<div class="mk-icons fg">${Object.keys(IconPaths).map((name) => `<div class="mk-icon">${icon(name, 'lg')}<code>${name}</code></div>`).join('')}</div>`;
    }
    return '';
  };

  const variant = ([label, render, size = {}]) => {
    const html = render();
    const box = size.scale
      ? scaled(html, size.scale, size.w, size.h)
      : `<div class="mk-fit" style="${size.w ? `width:${size.w}px;` : ''}${size.h ? `height:${size.h}px;` : ''}${size.zoom ? `zoom:${size.zoom};` : ''}">${html}</div>`;
    return `<figure class="mk-variant"><figcaption>${label}</figcaption><div class="mk-specimen fg">${box}</div></figure>`;
  };

  const componentSection = (item) => {
    const used = usage.get(item.name) || [];
    const from = builtFrom(item);
    const base = item.variants ? [...new Set([...item.variants.flatMap(([, render]) => [...render().matchAll(new RegExp(`data-c="${item.name}" data-base="([A-Za-z]+)"`, 'g'))].map((m) => m[1]))])] : [];
    const meta = [
      item.code && `<code>${item.code}</code>`,
      `<code class="mk-path">${item.source}</code>`,
      ...base.map((name) => `built on <code>${KitBases[name] || name}</code>`),
    ].filter(Boolean).join(' · ');
    return `<section class="mk-component" id="c-${item.name}">
      <h3>${item.name}</h3>
      <div class="mk-meta">${meta}</div>
      <p>${item.about}</p>
      ${from.length ? `<div class="mk-chips"><span class="mk-label">Built from</span>${from.map(componentLink).join('')}</div>` : ''}
      ${used.length ? `<div class="mk-chips"><span class="mk-label">Used in</span>${used.map(screenLink).join('')}</div>` : ''}
      <div class="mk-variants">${item.special ? special(item.special) : item.variants.map(variant).join('')}</div>
    </section>`;
  };

  const groupPage = (key) => {
    const group = Catalog.find((candidate) => candidate.key === key);
    if (!group) return overview();
    return `<header class="mk-head"><h1>${group.title}</h1><p>${group.about}</p></header>${group.items.map(componentSection).join('')}`;
  };

  const state = { outline: false, zoom: 'fit' };

  const fit = () => {
    const stage = document.getElementById('mk-stage');
    if (!stage) return;
    const scaler = stage.firstElementChild;
    const inner = scaler.firstElementChild;
    const scale = state.zoom === 'fit' ? Math.min(1, (stage.clientWidth - 2) / WINDOW.w) : 1;
    scaler.style.width = `${WINDOW.w * scale}px`;
    scaler.style.height = `${WINDOW.h * scale}px`;
    inner.style.transform = `scale(${scale})`;
  };

  const tag = document.createElement('div');
  tag.className = 'mk-tag';

  const describe = (element) => {
    const name = element.dataset.c;
    const entry = byName.get(name);
    const base = element.dataset.base;
    const source = entry ? entry.item.source : '';
    return `<b>${name}</b>${base ? ` <span>on ${base}</span>` : ''}${source ? `<div>${source}</div>` : ''}`;
  };

  const wireStage = () => {
    const stage = document.getElementById('mk-stage');
    if (!stage) return;
    let hot = null;
    stage.addEventListener('mousemove', (event) => {
      if (!state.outline) return;
      const element = event.target.closest('[data-c]');
      if (element !== hot) {
        if (hot) hot.classList.remove('mk-hot');
        hot = element;
        if (hot) {
          hot.classList.add('mk-hot');
          tag.innerHTML = describe(hot);
        }
      }
      tag.style.display = hot ? 'block' : 'none';
      const flipX = event.clientX + 14 + tag.offsetWidth > window.innerWidth;
      const flipY = event.clientY + 14 + tag.offsetHeight > window.innerHeight;
      tag.style.left = `${flipX ? event.clientX - 14 - tag.offsetWidth : event.clientX + 14}px`;
      tag.style.top = `${flipY ? event.clientY - 14 - tag.offsetHeight : event.clientY + 14}px`;
    });
    stage.addEventListener('mouseleave', () => {
      if (hot) hot.classList.remove('mk-hot');
      hot = null;
      tag.style.display = 'none';
    });
    stage.addEventListener('click', (event) => {
      if (!state.outline) return;
      const element = event.target.closest('[data-c]');
      const entry = element && byName.get(element.dataset.c);
      if (entry) location.hash = `#/components/${entry.group.key}/${entry.item.name}`;
    });
    document.getElementById('mk-outline').addEventListener('change', (event) => {
      state.outline = event.target.checked;
      stage.classList.toggle('mk-outline', state.outline);
      if (!state.outline) tag.style.display = 'none';
    });
    for (const button of document.querySelectorAll('[data-zoom]')) {
      button.addEventListener('click', () => {
        state.zoom = button.dataset.zoom;
        for (const other of document.querySelectorAll('[data-zoom]')) other.classList.toggle('on', other === button);
        fit();
      });
    }
    fit();
  };

  const render = () => {
    const route = decodeURIComponent(location.hash.replace(/^#\/?/, ''));
    const [kind, id, name] = route.split('/');
    document.getElementById('mk-nav').innerHTML = nav(route);
    const main = document.getElementById('mk-main');
    tag.style.display = 'none';
    const raw = kind === 'raw' && Screens.find((screen) => screen.id === id);
    document.body.classList.toggle('mk-raw', !!raw);
    main.innerHTML = raw ? raw.html : kind === 'screen' ? screenPage(id) : kind === 'components' ? groupPage(id) : overview();
    const section = name && document.getElementById(`c-${name}`);
    if (section) section.scrollIntoView();
    else main.scrollTo(0, 0);
    const active = document.querySelector('#mk-nav a.active.sub') || document.querySelector('#mk-nav a.active');
    if (active) active.scrollIntoView({ block: 'nearest' });
    wireStage();
  };

  const step = (delta) => {
    const match = location.hash.match(/^#\/screen\/(.+)$/);
    if (!match) return;
    const ix = Screens.findIndex((screen) => screen.id === match[1]);
    const next = Screens[ix + delta];
    if (next) location.hash = `#/screen/${next.id}`;
  };

  const fold = (event) => {
    if (event.target.closest('.mk-outline')) return;
    const header = event.target.closest('.tree-group > :first-child');
    if (!header) return;
    const open = !header.parentElement.classList.toggle('collapsed');
    const chevron = header.querySelector('.twisty');
    if (chevron) chevron.innerHTML = icon(open ? 'chevron-down' : 'chevron-right', 'xs');
  };

  const start = () => {
    document.body.appendChild(tag);
    document.getElementById('mk-main').addEventListener('click', fold);
    window.addEventListener('hashchange', render);
    window.addEventListener('resize', fit);
    window.addEventListener('keydown', (event) => {
      if (event.target.closest('input, textarea')) return;
      if (event.key === 'ArrowRight') step(1);
      if (event.key === 'ArrowLeft') step(-1);
      if (event.key === 'o') {
        const box = document.getElementById('mk-outline');
        if (box) box.click();
      }
    });
    render();
  };

  return { start };
})();

Viewer.start();
