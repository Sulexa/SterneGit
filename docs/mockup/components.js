const IconPaths = {
  'git-branch': '<path d="M15 6a9 9 0 0 0-9 9V3"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/>',
  'git-merge': '<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M6 21V9a9 9 0 0 0 9 9"/>',
  cloud: '<path d="M17.5 19H9a7 7 0 1 1 6.71-9h1.79a4.5 4.5 0 1 1 0 9Z"/>',
  tag: '<path d="M12.586 2.586A2 2 0 0 0 11.172 2H4a2 2 0 0 0-2 2v7.172a2 2 0 0 0 .586 1.414l8.704 8.704a2.426 2.426 0 0 0 3.42 0l6.58-6.58a2.426 2.426 0 0 0 0-3.42z"/><circle cx="7.5" cy="7.5" r=".5" fill="currentColor"/>',
  'columns-2': '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M12 3v18"/>',
  'rows-2': '<rect width="18" height="18" x="3" y="3" rx="2"/><path d="M3 12h18"/>',
  plus: '<path d="M5 12h14"/><path d="M12 5v14"/>',
  minus: '<path d="M5 12h14"/>',
  monitor: '<rect width="20" height="14" x="2" y="3" rx="2"/><line x1="8" x2="16" y1="21" y2="21"/><line x1="12" x2="12" y1="17" y2="21"/>',
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
  settings: '<path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/><circle cx="12" cy="12" r="3"/>',
  user: '<path d="M19 21v-2a4 4 0 0 0-4-4H9a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/>',
  plug: '<path d="M12 22v-5"/><path d="M9 8V2"/><path d="M15 8V2"/><path d="M18 8v5a4 4 0 0 1-4 4h-4a4 4 0 0 1-4-4V8Z"/>',
  'key-round': '<path d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z"/><circle cx="16.5" cy="7.5" r=".5" fill="currentColor"/>',
  link: '<path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"/><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"/>',
  lock: '<rect width="18" height="11" x="3" y="11" rx="2" ry="2"/><path d="M7 11V7a5 5 0 0 1 10 0v4"/>',
  globe: '<circle cx="12" cy="12" r="10"/><path d="M12 2a14.5 14.5 0 0 0 0 20 14.5 14.5 0 0 0 0-20"/><path d="M2 12h20"/>',
  building: '<rect width="16" height="20" x="4" y="2" rx="2" ry="2"/><path d="M9 22v-4h6v4"/><path d="M8 6h.01"/><path d="M16 6h.01"/><path d="M12 6h.01"/><path d="M12 10h.01"/><path d="M12 14h.01"/><path d="M16 10h.01"/><path d="M16 14h.01"/><path d="M8 10h.01"/><path d="M8 14h.01"/>',
  search: '<circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/>',
  copy: '<rect width="14" height="14" x="8" y="8" rx="2" ry="2"/><path d="M4 16c-1.1 0-2-.9-2-2V4c0-1.1.9-2 2-2h10c1.1 0 2 .9 2 2"/>',
  'git-fork': '<circle cx="12" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><circle cx="18" cy="6" r="3"/><path d="M18 9v2c0 .6-.4 1-1 1H7c-.6 0-1-.4-1-1V9"/><path d="M12 12v3"/>',
  'folder-open': '<path d="m6 14 1.5-2.9A2 2 0 0 1 9.24 10H20a2 2 0 0 1 1.94 2.5l-1.54 6a2 2 0 0 1-1.95 1.5H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h3.9a2 2 0 0 1 1.69.9l.81 1.2a2 2 0 0 0 1.67.9H18a2 2 0 0 1 2 2v2"/>',
  'shield-check': '<path d="M20 13c0 5-3.5 7.5-7.66 8.95a1 1 0 0 1-.67-.01C7.5 20.5 4 18 4 13V6a1 1 0 0 1 1-1c2 0 4.5-1.2 6.24-2.72a1.17 1.17 0 0 1 1.52 0C14.51 3.81 17 5 19 5a1 1 0 0 1 1 1z"/><path d="m9 12 2 2 4-4"/>',
  clock: '<circle cx="12" cy="12" r="10"/><path d="M12 6v6l4 2"/>',
  'log-in': '<path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/><path d="m10 17 5-5-5-5"/><path d="M15 12H3"/>',
  'refresh-cw': '<path d="M3 12a9 9 0 0 1 9-9 9.75 9.75 0 0 1 6.74 2.74L21 8"/><path d="M21 3v5h-5"/><path d="M21 12a9 9 0 0 1-9 9 9.75 9.75 0 0 1-6.74-2.74L3 16"/><path d="M8 16H3v5"/>',
  download: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m7 10 5 5 5-5"/><path d="M12 15V3"/>',
  upload: '<path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><path d="m17 8-5-5-5 5"/><path d="M12 3v12"/>',
  'circle-alert': '<circle cx="12" cy="12" r="10"/><path d="M12 8v4"/><path d="M12 16h.01"/>',
  info: '<circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>',
  server: '<rect width="20" height="8" x="2" y="2" rx="2" ry="2"/><rect width="20" height="8" x="2" y="14" rx="2" ry="2"/><path d="M6 6h.01"/><path d="M6 18h.01"/>',
  image: '<rect width="18" height="18" x="3" y="3" rx="2" ry="2"/><circle cx="9" cy="9" r="2"/><path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21"/>',
  book: '<path d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20"/>',
  'trash-2': '<path d="M3 6h18"/><path d="M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6"/><path d="M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2"/><path d="M10 11v6"/><path d="M14 11v6"/>',
  'folder-plus': '<path d="M12 10v6"/><path d="M9 13h6"/><path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z"/>',
  'git-pull-request': '<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M13 6h3a2 2 0 0 1 2 2v7"/><path d="M6 9v12"/>',
  'git-pull-request-draft': '<circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/><path d="M18 6V5"/><path d="M18 11v-1"/><path d="M6 9v12"/>',
  'git-pull-request-closed': '<circle cx="6" cy="6" r="3"/><path d="M6 9v12"/><path d="m21 3-6 6"/><path d="m21 9-6-6"/><path d="M18 11.5V15"/><circle cx="18" cy="18" r="3"/>',
  'folder-git': '<path d="M4 20h16a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.93a2 2 0 0 1-1.66-.9l-.82-1.2A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13c0 1.1.9 2 2 2Z"/><circle cx="12" cy="13" r="2"/><path d="M14 13h3"/><path d="M7 13h3"/>',
  'list-filter': '<path d="M3 6h18"/><path d="M7 12h10"/><path d="M10 18h4"/>',
  'circle-minus': '<circle cx="12" cy="12" r="10"/><path d="M8 12h8"/>',
  eye: '<path d="M2.062 12.348a1 1 0 0 1 0-.696 10.75 10.75 0 0 1 19.876 0 1 1 0 0 1 0 .696 10.75 10.75 0 0 1-19.876 0"/><circle cx="12" cy="12" r="3"/>',
  'message-square': '<path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/>',
  'arrow-right': '<path d="M5 12h14"/><path d="m12 5 7 7-7 7"/>',
  box: '<path d="M21 8a2 2 0 0 0-1-1.73l-7-4a2 2 0 0 0-2 0l-7 4A2 2 0 0 0 3 8v8a2 2 0 0 0 1 1.73l7 4a2 2 0 0 0 2 0l7-4A2 2 0 0 0 21 16Z"/><path d="m3.3 7 8.7 5 8.7-5"/><path d="M12 22V12"/>',
  'file-code': '<path d="M10 12.5 8 15l2 2.5"/><path d="m14 12.5 2 2.5-2 2.5"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7z"/>',
  'circle-dot': '<circle cx="12" cy="12" r="10"/><circle cx="12" cy="12" r="1"/>',
  'file-box': '<path d="M14.5 22H18a2 2 0 0 0 2-2V7l-5-5H6a2 2 0 0 0-2 2v4"/><path d="M14 2v4a2 2 0 0 0 2 2h4"/><path d="M3 13.1a2 2 0 0 0-1 1.76v3.24a2 2 0 0 0 .97 1.78L6 21.7a2 2 0 0 0 2.03.01L11 19.9a2 2 0 0 0 1-1.76V14.9a2 2 0 0 0-.97-1.78L8 11.3a2 2 0 0 0-2.03-.01Z"/><path d="M7 17v5"/><path d="M11.7 14.2 7 17l-4.7-2.8"/>',
  map: '<path d="M14.106 5.553a2 2 0 0 0 1.788 0l3.659-1.83A1 1 0 0 1 21 4.619v12.764a1 1 0 0 1-.553.894l-4.553 2.277a2 2 0 0 1-1.788 0l-4.212-2.106a2 2 0 0 0-1.788 0l-3.659 1.83A1 1 0 0 1 3 19.381V6.618a1 1 0 0 1 .553-.894l4.553-2.277a2 2 0 0 1 1.788 0z"/><path d="M15 5.764v15"/><path d="M9 3.236v15"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2v2"/><path d="M12 20v2"/><path d="m4.93 4.93 1.41 1.41"/><path d="m17.66 17.66 1.41 1.41"/><path d="M2 12h2"/><path d="M20 12h2"/><path d="m6.34 17.66-1.41 1.41"/><path d="m19.07 4.93-1.41 1.41"/>',
  video: '<path d="m16 13 5.223 3.482a.5.5 0 0 0 .777-.416V7.87a.5.5 0 0 0-.752-.432L16 10.5"/><rect x="2" y="6" width="14" height="12" rx="2"/>',
  workflow: '<rect width="8" height="8" x="3" y="3" rx="2"/><path d="M7 11v4a2 2 0 0 0 2 2h4"/><rect width="8" height="8" x="13" y="13" rx="2"/>',
  spline: '<circle cx="19" cy="5" r="2"/><circle cx="5" cy="19" r="2"/><path d="M5 17A12 12 0 0 1 17 5"/>',
  braces: '<path d="M8 3H7a2 2 0 0 0-2 2v5a2 2 0 0 1-2 2 2 2 0 0 1 2 2v5c0 1.1.9 2 2 2h1"/><path d="M16 21h1a2 2 0 0 0 2-2v-5c0-1.1.9-2 2-2a2 2 0 0 1-2-2V5a2 2 0 0 0-2-2h-1"/>',
  zap: '<path d="M4 14a1 1 0 0 1-.78-1.63l9.9-10.2a.5.5 0 0 1 .86.46l-1.92 6.02A1 1 0 0 0 13 10h7a1 1 0 0 1 .78 1.63l-9.9 10.2a.5.5 0 0 1-.86-.46l1.92-6.02A1 1 0 0 0 11 14z"/>',
  'eye-off': '<path d="M10.733 5.076a10.744 10.744 0 0 1 11.205 6.575 1 1 0 0 1 0 .696 10.747 10.747 0 0 1-1.444 2.49"/><path d="M14.084 14.158a3 3 0 0 1-4.242-4.242"/><path d="M17.479 17.499a10.75 10.75 0 0 1-15.417-5.151 1 1 0 0 1 0-.696 10.75 10.75 0 0 1 4.446-5.143"/><path d="m2 2 20 20"/>',
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

const RefIcons = { local: 'git-branch', remote: 'cloud', tag: 'tag', stash: 'archive', worktree: 'folder-git' };

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
  conflicted: {
    files: 'Conflicts',
    state: 'conflicted',
    verb: 'Mark resolved',
    openFileTooltip: 'Save the result and stage the file',
    icon: 'check',
    discards: false,
  },
};

const ConflictTexts = {
  title: { current: 'Current', incoming: 'Incoming' },
  accept: { current: 'Accept current', incoming: 'Accept incoming' },
  acceptTooltip: { current: 'Put these lines in the result', incoming: 'Put these lines in the result, after the current ones when both are ticked' },
  result: 'Result',
  hint: 'Tick a side above, or edit here',
  left: (n) => `${counted(n, 'conflict', 'conflicts')} left`,
  startOver: 'Start over',
  startOverTooltip: 'Put the conflict markers back and forget every choice and edit',
  byHand: 'Edited by hand in the result: Start over to choose again',
  keep: { current: 'Keep current', incoming: 'Keep incoming' },
  keepTooltip: { current: 'Stage the current version of the file', incoming: 'Stage the incoming version of the file' },
};

const OperationTexts = {
  rebase: (op) => `Rebasing ${op.branch} onto ${op.onto}`,
  merge: (op) => `Merging ${op.from} into ${op.into}`,
  'cherry-pick': (op) => `Cherry-picking onto ${op.onto}`,
  revert: (op) => `Reverting on ${op.branch}`,
  abort: { rebase: 'Abort rebase', merge: 'Abort merge', 'cherry-pick': 'Abort cherry-pick', revert: 'Abort revert' },
  abortTooltip: {
    rebase: 'Put the branch back where it was before the rebase',
    merge: 'Put the branch back where it was before the merge',
    'cherry-pick': 'Put the branch back where it was before the cherry-pick',
    revert: 'Put the branch back where it was before the revert',
  },
  go: { rebase: 'Continue', merge: 'Commit merge', 'cherry-pick': 'Commit', revert: 'Commit revert' },
  goTooltip: {
    rebase: 'Commit the resolved changes and replay the next commits',
    merge: 'Commit the merge with its usual message',
    'cherry-pick': "Commit the resolved changes with the picked commit's message and author",
    revert: 'Commit the resolved changes with the usual Revert message',
  },
  waiting: (n) => `Resolve the ${counted(n, 'conflicted file', 'conflicted files')} first`,
  noConflicts: 'No conflicts left',
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
  File: [
    ['Open Repository…', 'Ctrl+O'],
    ['Clone Repository…', 'Ctrl+Shift+O'],
    ['New Repository…', 'Ctrl+N'],
    '-',
    ['Settings…', 'Ctrl+,'],
    '-',
    ['Close Tab', 'Ctrl+W'],
    '-',
    ['Quit', 'Ctrl+Q'],
  ],
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
    '-',
    ['Zoom In', 'Ctrl+='],
    ['Zoom Out', 'Ctrl+-'],
    ['Reset Zoom', 'Ctrl+0'],
  ],
  About: [['About SterneGit', ''], ['Check for Updates…', '']],
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
  conflict: [],
  image: PictureModes,
  video: PictureModes,
  audio: [
    { key: 'split', label: 'Split', icon: 'audio-lines', tooltip: 'Show both waveforms on one time scale' },
    { key: 'difference', label: 'Difference', icon: 'diff', tooltip: 'Show only what sounds different' },
  ],
  objects: [
    { key: 'inspector', label: 'Inspector', icon: 'list-tree', tooltip: 'Compare objects, components and properties' },
    { key: 'yaml', label: 'YAML', icon: 'braces', tooltip: 'Compare the text Unity saved' },
  ],
  scene: [
    { key: 'inspector', label: 'Inspector', icon: 'list-tree', tooltip: 'Compare objects, components and properties' },
    { key: 'scene', label: 'Scene', icon: 'map', tooltip: 'See what moved, appeared or went away' },
    { key: 'yaml', label: 'YAML', icon: 'braces', tooltip: 'Compare the text Unity saved' },
  ],
  curves: [
    { key: 'curves', label: 'Curves', icon: 'spline', tooltip: 'Draw both versions of every animated property' },
    { key: 'yaml', label: 'YAML', icon: 'braces', tooltip: 'Compare the text Unity saved' },
  ],
  animator: [
    { key: 'graph', label: 'Graph', icon: 'workflow', tooltip: 'Compare the state machine' },
    { key: 'yaml', label: 'YAML', icon: 'braces', tooltip: 'Compare the text Unity saved' },
  ],
  scriptgraph: [
    { key: 'graph', label: 'Graph', icon: 'workflow', tooltip: 'Compare the nodes and their connections' },
    { key: 'json', label: 'JSON', icon: 'braces', tooltip: 'Compare the graph data, one value per line' },
  ],
};

const UnityModes = { Prefab: 'scene', Scene: 'scene', 'Script graph': 'scriptgraph', Material: 'objects', Meta: 'objects', 'Animation clip': 'curves', 'Animator controller': 'animator' };

const UnityOptions = [
  { key: 'unchanged', label: 'Show unchanged properties', about: 'Lists every property of a changed component' },
  { key: 'noise', label: 'Hide what Unity changes on its own', about: 'Float rounding, reordered overrides, version bumps' },
  { key: 'ids', label: 'Show file IDs', about: 'Names each object and component by its fileID too' },
];

const UnityStates = { added: 'A', changed: 'M', removed: 'D' };

const GraphTypes = { float: '#60a5fa', bool: '#f472b6', vector: '#fb923c', object: '#a3a3a3' };

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

const LinesAboveAChange = 4;

const ViewRows = 30;

const DrawnRows = 60;

const Texts = {
  welcomeTitle: 'Open a Git repository',
  welcomeDescription: 'SterneGit reads history straight from the repository on disk. Each repository opens in its own tab.',
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
  discardFilesTitle: (n) => `Discard the changes to ${n} files?`,
  discardFilesDescription: (n) => `The unstaged changes in these ${n} files are thrown away. Git keeps no copy of them, so this can't be undone.`,
  discardMixedDescription: (tracked, untracked) => `The unstaged changes in ${counted(tracked, 'file', 'files')} are thrown away and ${counted(untracked, 'untracked file is', 'untracked files are')} deleted. Git keeps no copy of them, so this can't be undone.`,
  deleteFilesTitle: (n) => `Delete ${n} files?`,
  deleteFilesDescription: (n) => `These ${n} files aren't tracked by Git, so they can't be brought back once deleted.`,
  pathsCopied: (n) => `${n} paths copied`,
  branchTitle: 'Create a branch',
  branchDescription: (head) => `It starts at ${head}, where HEAD is, and is checked out right away. Uncommitted changes stay as they are.`,
  branchFromDescription: (start) => `It starts at ${start} and is checked out right away. Uncommitted changes come along unless they clash with it.`,
  checkOut: (name) => `Check out ${name}`,
  pullWithMode: (mode) => `Pull (${mode.toLowerCase()})`,
  pullFastForward: 'Pull (fast-forward only)',
  push: 'Push',
  behindTitle: (branch, upstream) => `${upstream} has commits that ${branch} doesn't have`,
  behindCounts: (branch, upstream, theirs, ours) =>
    `${upstream} has ${counted(theirs, 'new commit', 'new commits')}, and ${branch} has ${ours ? `${ours} that ${upstream} doesn't` : 'none of its own to push'}.`,
  behindPull: (branch) => `Pull and push brings them into ${branch}, with a merge commit if ${branch} can't fast-forward, then pushes.`,
  behindFastForward: (branch, upstream) => `Pull fast-forwards ${branch} to ${upstream}.`,
  behindCheckOut: (branch) => `Check out ${branch} to pull them in: a branch that isn't checked out can only fast-forward.`,
  behindForce: (branch, upstream, theirs) => `Force push replaces ${upstream} with ${branch}, dropping ${theirs === 1 ? 'the new commit' : `the ${theirs} new commits`} for everyone.`,
  behindRewrittenTitle: (branch, upstream) => `${upstream} has older versions of ${branch}'s commits`,
  behindRewritten: (branch, upstream, theirs) =>
    theirs === 1
      ? `The commit ${upstream} has and ${branch} doesn't makes the same changes as one of ${branch}'s: it was rewritten after it was pushed, with a new message or a rebase.`
      : `The ${theirs} commits ${upstream} has and ${branch} doesn't make the same changes as commits of ${branch}: they were rewritten after they were pushed, with a new message or a rebase.`,
  behindRewrittenForce: (branch, upstream, theirs) =>
    `Force push replaces ${theirs === 1 ? 'it' : 'them'} on ${upstream} with ${branch}'s versions, so no change is lost. Pulling would bring the old versions back beside the new ones.`,
  pullAndPush: 'Pull and push',
  pullOnly: 'Pull',
  forcePush: 'Force push',
  pulledAndPushed: (branch, upstream) => `Pulled ${upstream} into ${branch}, then pushed ${branch} to ${upstream}`,
  forcePushed: (branch, upstream) => `Force pushed ${branch} to ${upstream}`,
  movedAgain: (upstream) => `${upstream} moved again since SterneGit fetched it, so nothing was overwritten. Push again to see what it has now.`,
  merge: (from, into) => `Merge ${from} into ${into}`,
  rebase: (branch, onto) => `Rebase ${branch} onto ${onto}`,
  branchHere: 'Create branch here…',
  applyStash: (name) => `Apply ${name}`,
  popStash: (name) => `Pop ${name}`,
  deleteStash: (name) => `Delete ${name}…`,
  deleteStashTitle: (name) => `Delete ${name}?`,
  deleteStashDescription: (message) => `Git keeps no other copy of “${message}”, so its changes are lost.`,
  checkOutCommit: 'Check out this commit',
  tagHere: 'Create tag here…',
  revertCommit: 'Revert this commit',
  revertMerge: 'Revert this merge',
  cherryPick: (branch) => `Cherry-pick onto ${branch}`,
  resetHere: (branch) => `Reset ${branch} to here`,
  resetModes: { soft: 'Soft: keep changes staged', mixed: 'Mixed: keep changes unstaged', hard: 'Hard: discard changes…' },
  copyHash: 'Copy hash',
  copyMessage: 'Copy message',
  messageCopied: 'Commit message copied',
  editMessageTip: 'Click to edit the message',
  saveMessage: 'Save message',
  savingMessage: 'Saving…',
  saveMessageTooltip: 'Save the message  Ctrl+Enter',
  rewrites: (after, branches) =>
    after
      ? `Saving gives this commit and the ${counted(after, 'commit', 'commits')} after it new hashes, and moves ${listed(branches)}.`
      : `Saving gives this commit a new hash and moves ${listed(branches)} to it.`,
  pushedAlready: (remotes) => `${listed(remotes)} already ${remotes.length > 1 ? 'have' : 'has'} it: the next push needs Force push.`,
  tagsStay: (tags) => `${listed(tags)} ${tags.length > 1 ? 'stay' : 'stays'} on the old ${tags.length > 1 ? 'commits' : 'commit'}.`,
  messageSaved: (old, id) => `Message saved: ${old} is now ${id}`,
  checkedOutCommit: (id) => `Checked out ${id}: HEAD is detached`,
  resetDone: (branch, id) => `Reset ${branch} to ${id}`,
  reverted: (id) => `Reverted ${id}`,
  cherryPicked: (id, branch) => `Cherry-picked ${id} onto ${branch}`,
  resetHardTitle: (branch, id) => `Reset ${branch} to ${id} and discard changes?`,
  resetHardFiles: (n) =>
    n
      ? `Every tracked file goes back to that commit: the uncommitted changes in ${counted(n, 'file', 'files')} are thrown away, and Git keeps no copy of them. Untracked files stay.`
      : 'Every tracked file goes back to that commit. There are no uncommitted changes to lose.',
  resetHardLost: (n) => `${counted(n, 'commit is', 'commits are')} on no other branch or tag, so ${n === 1 ? 'it leaves' : 'they leave'} the graph.`,
  tagTitle: 'Create a tag',
  tagDescription: (start) => `It points to ${start}. With a message it is an annotated tag, without one a lightweight tag.`,
  tagPlaceholder: 'Tag name',
  tagMessagePlaceholder: 'Message (optional)',
  tagExists: (name) => `There is already a tag named ${name}.`,
  pushTag: (remote) => `Push it to ${remote}`,
  createTag: 'Create tag',
  tagCreated: (name, id) => `Created tag ${name} at ${id}`,
  tagPushed: (name, remote) => `Created tag ${name} and pushed it to ${remote}`,
  deleteBranch: (name) => `Delete ${name}…`,
  deleteRemoteBranch: (name, remote) => `Delete ${name} from ${remote}…`,
  deleteBranchTitle: (name) => `Delete ${name}?`,
  deleteMerged: (name) => `Every commit of ${name} is on another branch or tag too, so nothing is lost.`,
  deleteUnmerged: (name, n) => `${counted(n, 'commit is', 'commits are')} on no other branch or tag. Deleting ${name} loses ${n === 1 ? 'it' : 'them'}.`,
  deleteUncounted: (name) => `${name} points to a commit older than the newest ${thousands(Texts.historyLimit)} shown, so SterneGit can't tell whether deleting it loses any commits.`,
  deleteRemoteTitle: (name, remote) => `Delete ${name} from ${remote}?`,
  deleteRemoteDescription: (name, remote) => `The branch is removed from ${remote} for everyone who uses it. A local ${name}, if you have one, stays.`,
  pushTagTo: (name, remote) => `Push ${name} to ${remote}`,
  tagPushedTo: (name, remote) => `Pushed ${name} to ${remote}`,
  tagAlreadyThere: (name, remote) => `${remote} already has ${name}`,
  tagTakenThere: (name, remote) => `${remote} already has a different ${name}. Delete ${name} from ${remote} first, then push it again.`,
  tagFetchedBack: (name, remote) => `If ${remote} has ${name} too, the next fetch brings it back; Delete ${name} from ${remote}… removes it there.`,
  deleteRemoteTagDescription: (name, remote) => `The tag is removed from ${remote} for everyone who uses it. ${name} stays here, so a push would send it back.`,
  tagDeleted: (name) => `Deleted tag ${name}`,
  copyName: 'Copy name',
  nameCopied: 'Tag name copied',
  remoteTagDeleted: (name, remote) => `Deleted ${name} from ${remote}`,
  branchPlaceholder: 'Branch name',
  branchExists: (name) => `There is already a branch named ${name}.`,
  aheadBehind: (ahead, behind, upstream) => {
    const pull = `${counted(behind, 'commit', 'commits')} to pull from ${upstream}`;
    const push = `${counted(ahead, 'commit', 'commits')} to push to ${upstream}`;
    return ahead && behind ? `${pull}, ${thousands(ahead)} to push` : behind ? pull : push;
  },
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
  lfs: 'Stored with Git LFS: SterneGit compares the file, not its pointer',
  backdrop: 'What shows behind transparent pixels',
  showAbove: (n) => `Show ${n} lines above`,
  showBelow: (n) => `Show ${n} lines below`,
  expandLines: (n) => `Expand ${counted(n, 'unchanged line', 'unchanged lines')}`,
  cannotShow: (type) => `SterneGit can’t draw ${type} files`,
  cannotShowDescription: 'Open each version in the app that made it, or hand both to the diff tool set in Git’s diff.tool.',
  metaWith: (file) => `${file}.meta changed too: it is staged, unstaged and discarded with ${file}`,
  unityHidden: (n) => `${counted(n, 'change', 'changes')} Unity made on its own hidden`,
  guidTitle: (n, asset) => `The GUID changed: ${counted(n, 'file loses', 'files lose')} ${asset}`,
  guidAbout: 'Unity finds assets by GUID, not by path. These files still use the old GUID, so Unity shows the asset as Missing in them:',
  keepOldGuid: 'Keep the old GUID',
  moveToNewGuid: (n) => `Point the ${counted(n, 'file', 'files')} to the new GUID`,
  fileHistory: 'File history',
  openFile: 'Open file',
  showInFolder: 'Show in folder',
  copyPath: 'Copy path',
  moveFiles: (verb, n) => `${verb} ${n} files`,
  discardFiles: (n) => `Discard ${n} files…`,
  deleteFiles: (n) => `Delete ${n} files…`,
  openFiles: (n) => `Open ${n} files`,
  copyPaths: (n) => `Copy ${n} paths`,
  commitsPicked: (n) => `${thousands(n)} commits picked`,
  pickedTogether: (from, to) => `Their changes together, from ${from} to ${to}`,
  pickedApart: "These commits don't follow one another on a single line, so their changes can't be shown as one. Ctrl+click the commits of one branch, or Shift+click a run of them, to see their changes together.",
  cherryPickCommits: (n, branch) => `Cherry-pick ${n} commits onto ${branch}`,
  revertCommits: (n) => `Revert ${n} commits`,
  squashCommits: (n) => `Squash ${n} commits…`,
  copyHashes: (n) => `Copy ${n} hashes`,
  copyMessages: (n) => `Copy ${n} messages`,
  squashTitle: (n) => `Squash ${n} commits into one`,
  squashDescription: (newest, oldest) => `One commit takes their place, with the files of ${newest}, the author and date of ${oldest}, and this message.`,
  squashMoves: (after, branches) =>
    after
      ? `Squashing gives the ${counted(after, 'commit', 'commits')} after them new hashes, and moves ${listed(branches)}.`
      : `Squashing moves ${listed(branches)} to the new commit.`,
  squashPushed: (remotes) => `${listed(remotes)} already ${remotes.length > 1 ? 'have' : 'has'} them: the next push needs Force push.`,
  squashButton: (n) => `Squash ${n} commits`,
  squashTooltip: 'Squash the commits  Ctrl+Enter',
  squashed: (n, id) => `Squashed ${n} commits into ${id}`,
  cherryPickedCommits: (n, branch) => `Cherry-picked ${n} commits onto ${branch}`,
  historyPosition: (at, n, name) => `Commit ${thousands(at)} of ${thousands(n)} changing ${name}`,
  renamedFrom: (path) => `Renamed from ${path}`,
  hear: (side) => `Hear ${side === 'before' ? 'the version before the change' : 'the changed version'}  Tab switches`,
  cloneRepository: 'Clone repository…',
};

const MethodLabels = {
  device: 'Signed in with the browser',
  token: 'Signed in with a pasted token',
  credential: 'Token from Git Credential Manager',
};

const StandingLooks = {
  ready: { label: () => 'Ready', color: 'var(--green)' },
  updated: { label: (connector) => `Updated from ${connector.was}`, color: 'var(--blue)' },
  waiting: { label: () => 'Waiting for approval', color: 'var(--yellow)' },
  modified: { label: () => 'Files changed', color: 'var(--yellow)' },
  unusable: { label: () => "Can't run", color: 'var(--red)' },
  failed: { label: () => 'Stopped', color: 'var(--red)' },
};

const Visibilities = {
  public: { label: 'Public', icon: 'globe', about: 'Anyone can see it' },
  private: { label: 'Private', icon: 'lock', about: 'Only you and the people you invite' },
  internal: { label: 'Internal', icon: 'building', about: 'Everyone in the organization' },
};

const ZoomLevels = [50, 67, 75, 80, 90, 100, 110, 125, 150, 175, 200];

const HostTexts = {
  settings: 'Settings',
  accounts: 'Accounts',
  connectors: 'Connectors',
  interface: 'Interface',
  interfaceAbout: 'How SterneGit draws its window on this computer.',
  zoom: 'Zoom',
  zoomAbout: 'Makes everything in the window bigger or smaller. Hold Ctrl and turn the mouse wheel to zoom from anywhere; over a picture or a sound being compared, the wheel zooms that instead.',
  zoomOut: 'Zoom out  Ctrl+-',
  zoomIn: 'Zoom in  Ctrl+=',
  resetZoom: 'Reset',
  resetZoomTip: 'Back to 100 %  Ctrl+0',
  updates: 'Updates',
  updatesAbout: 'SterneGit looks for a new release on GitHub when it starts and every six hours.',
  installedAt: (folder) => `Installed for you in ${folder}`,
  portableAt: (folder) => `Portable, in ${folder}`,
  builtHere: 'Built on this computer from source',
  upToDate: (ago) => `Up to date · checked ${ago}`,
  checking: 'Looking for a new release…',
  downloading: (version) => `Downloading SterneGit ${version}…`,
  readyStatus: (version) => `SterneGit ${version} is ready. It installs when you restart, or when you quit.`,
  availableStatus: (version) => `SterneGit ${version} is out.`,
  readOnlyStatus: (version) => `SterneGit ${version} is out, but this folder can't be written to, so download it from its release page.`,
  checkFailed: (why) => `Couldn't check: ${why}`,
  builtStatus: 'This copy never updates itself. Install SterneGit from a release to get updates.',
  checkNow: 'Check for updates',
  whatsNew: "What's new",
  restartToUpdate: 'Restart to update',
  updateTo: (version) => `Update to ${version}`,
  update: 'Update',
  downloadingShort: 'Downloading…',
  releaseOut: (version) => `SterneGit ${version} is out`,
  openReleasePage: 'Open the release page',
  automatic: 'Update automatically',
  automaticAbout: 'Downloads each new release in the background and installs it when you restart or quit SterneGit. When off, SterneGit only tells you a release is out.',
  readyTip: (version) => `SterneGit ${version} is downloaded. Restart to install it now; quitting installs it too.`,
  upToDateToast: (version) => `SterneGit ${version} is up to date.`,
  foundTitle: (version) => `SterneGit ${version} is out`,
  foundDownloading: "It's downloading now. Restart to update appears at the top right when it's ready.",
  foundManual: 'Update downloads it, then Restart to update installs it.',
  checkFailedTitle: "Couldn't check for updates",
  builtToast: "This copy was built on this computer and doesn't update itself.",
  accountsAbout: "SterneGit signs in to hosting services to clone, create and publish repositories. It keeps their tokens in Windows Credential Manager under its own names, apart from the credentials Git uses in a terminal.",
  addAccount: 'Add account…',
  noAccounts: 'No accounts yet',
  noAccountsAbout: 'Add one to clone your repositories, publish new ones and add SSH keys. Git keeps using your own credentials meanwhile.',
  signedOut: (service) => `Signed out: ${service} refused the token`,
  signInAgain: 'Sign in again',
  removeAccount: 'Remove…',
  signIn: 'Sign-in',
  method: 'Method',
  scopes: 'Scopes',
  tokenKept: 'Token',
  tokenWhere: (id) => `In Windows Credential Manager as SterneGit/${id}`,
  sshKey: 'SSH key',
  sshKeyAbout: (service) => `The key SterneGit made for this account, then the other keys ${service} lists for it.`,
  sternegitKey: "SterneGit's key",
  readOnly: 'Read-only',
  noKey: (service) => `No SSH key yet. SterneGit can make one and add it to ${service}, so this account's clones can use SSH.`,
  createKey: 'Create SSH key…',
  copyPublicKey: 'Copy public key',
  added: (date) => `Added ${date}`,
  usedBy: (n) => (n ? `Used by ${counted(n, 'repository', 'repositories')}` : 'Not used by any repository yet'),
  remoteOwners: 'Remote owners',
  remoteOwnersAbout: "When another account also fits a remote, SterneGit uses this one for these owners' repositories.",
  noOwners: 'None yet: SterneGit asks when two accounts fit a remote.',
  forget: 'Forget',
  connectorsAbout: 'Connectors let SterneGit talk to hosting services. Each one runs in a sandbox: it reaches only the servers listed for it, and SterneGit adds the token to its requests, so it never sees one.',
  connectorsFolder: 'To install a connector, copy its folder into %APPDATA%\\SterneGit\\connectors and restart SterneGit.',
  openFolder: 'Open folder',
  bundled: 'Bundled',
  installed: 'Installed',
  review: 'Review…',
  showLog: 'Show log',
  reaches: (hosts, typed) => `Reaches ${[hosts.join(', '), typed && (hosts.length ? 'servers you add' : 'the servers you add')].filter(Boolean).join(' and ')}`,
  approveTitle: (name, version) => `Approve ${name} ${version}?`,
  moreTitle: (name, version, was) => `${name} ${version} asks for more than ${was} did`,
  installedIn: (id) => `Installed in %APPDATA%\\SterneGit\\connectors\\${id}`,
  approvalWarning: (name) => `Approve only a connector you trust. Once approved, it acts on your ${name} accounts as far as their tokens allow, through SterneGit. It can't read your files, your SSH keys or your other accounts.`,
  sameAsBefore: (was) => `Everything else stays as you approved it for ${was}.`,
  reach: 'Servers it can reach',
  reachAbout: 'Requests anywhere else fail before they leave SterneGit.',
  withToken: 'With your token',
  withoutToken: 'Without a token',
  serverSlot: 'server you add',
  images: 'Pictures from',
  remotes: 'Remotes it recognizes',
  remotesAbout: 'Repositories with these remotes use its accounts.',
  hostSlot: 'the server of each account you add',
  signInPages: 'Sign-in pages',
  features: 'What it offers',
  approve: 'Approve',
  notNow: 'Not now',
  newGrant: 'New',
  waitingTitle: (name, version) => `${name} ${version} is waiting for your approval`,
  waitingMessage: "It was copied into SterneGit's connectors folder. Nothing of it runs until you approve it.",
  addAccountTitle: 'Add an account',
  service: 'Service',
  serverAddress: 'Server address',
  methodDevice: 'Sign in with the browser',
  methodDeviceAbout: 'GitHub shows a short code to confirm. Recommended.',
  methodToken: 'Paste a token',
  methodTokenAbout: 'Make a token on GitHub with the scopes SterneGit needs.',
  methodCredential: "Use Git Credential Manager's token",
  methodCredentialAbout: (host) => `Reuse the token Git already has for ${host}.`,
  signInTo: (service) => `Sign in to ${service}`,
  deviceOpened: (page) => `SterneGit opened ${page} in your browser. Enter this code there:`,
  copy: 'Copy',
  waitingForYou: (service) => `Waiting for you to approve SterneGit on ${service}…`,
  openAgain: 'Open the page again',
  expired: 'The code expired after 15 minutes. Nothing was saved.',
  newCode: 'Get a new code',
  denied: (service) => `${service} says you declined. Nothing was saved.`,
  tryAgain: 'Try again',
  tokenTitle: (service) => `Sign in to ${service} with a token`,
  tokenAbout: (scopes) => `Create a classic token with the scopes ${scopes}, then paste it here. SterneGit checks it before saving it.`,
  createToken: 'Create a token on GitHub',
  tokenLabel: 'Token',
  tokenRefused: (service) => `${service} refused this token: it may have expired or been revoked.`,
  tokenLacks: (scope) => `This token lacks ${scope}, so SterneGit can't add SSH keys with it. Signing in works anyway.`,
  signInButton: 'Sign in',
  checking: 'Checking…',
  credentialTitle: "Use Git Credential Manager's token?",
  credentialAbout: (host) => `Git Credential Manager has a token for ${host}. SterneGit copies it into its own entry in Windows Credential Manager and signs in with it; Git Credential Manager keeps its copy. These tokens usually can't add SSH keys: sign in with the browser for that.`,
  useToken: 'Use this token',
  keyTitle: (login) => `Create an SSH key for ${login}`,
  keyAbout: (type, service) => `SterneGit makes an ${type} key without a passphrase in %LOCALAPPDATA%\\SterneGit\\ssh, adds its public half to ${service}, and uses it only in the repositories you clone or publish with this account.`,
  keyLabel: (service) => `Title on ${service}`,
  keyForbidden: (service, scope) => `${service} refused: this account's token lacks the ${scope} scope.`,
  createAndAdd: 'Create and add',
  removeTitle: (login) => `Remove ${login}?`,
  removeAbout: (service) => `SterneGit forgets the account and deletes its token from Windows Credential Manager. It deletes its copy of the SSH key too; ${service} keeps the public half until you delete it there.`,
  removeUsing: (n) => `${counted(n, 'repository uses', 'repositories use')} this account's SSH key:`,
  undoSshCommand: 'Remove core.sshCommand from them, so they use your own SSH setup again',
  remove: 'Remove',
  choiceText: (host, owner, n) => `${n === 2 ? 'Two' : thousands(n)} accounts fit ${host}/${owner}. Which one should SterneGit use for ${owner}'s repositories?`,
  use: 'Use',
  choiceLater: "Not now: Git keeps using your own credentials",
  cloneTitle: 'Clone a repository',
  url: 'URL',
  addAccountShort: 'Add account…',
  repositoryUrl: 'Repository URL',
  urlHint: 'Anything git clone accepts: https://, ssh:// or user@host:path. When an account fits the URL, SterneGit lends Git its token.',
  allOwners: 'All owners',
  searchIn: (login) => `Search ${login}'s repositories`,
  loadMore: 'Load more',
  cloneInto: 'Clone into',
  browse: 'Browse…',
  protocol: 'Protocol',
  sshNeedsKey: 'Create an SSH key for this account first, in Settings → Accounts',
  clone: 'Clone',
  cloning: (repo, folder) => `Cloning ${repo} into ${folder}…`,
  receiving: (percent, done, total) => `Receiving objects: ${percent}% (${thousands(done)} of ${thousands(total)})`,
  cancel: 'Cancel',
  newTitle: 'New repository',
  newAbout: 'SterneGit makes the folder, runs git init in it and opens it in a new tab.',
  name: 'Name',
  folder: 'Folder',
  creates: (path) => `Creates ${path}`,
  alsoOn: 'Also create it on',
  owner: 'Owner',
  visibility: 'Visibility',
  description: 'Description',
  optional: 'Optional',
  hostedHint: (service) => `SterneGit creates it empty on ${service} and adds it as origin. Your first push sends your branch there.`,
  create: 'Create',
  publishTitle: (name) => `Publish ${name}`,
  publishAbout: (name, branch) => `${name} has no remote yet. SterneGit creates it on your account, adds it as origin and pushes ${branch} to it.`,
  publishNoAccount: (name) => `${name} has no remote yet. Add an account to create it there; SterneGit then adds it as origin and pushes.`,
  account: 'Account',
  nameTaken: (owner, name) => `${owner} already has a repository named ${name}.`,
  publish: 'Publish',
  cannotCreate: (owner) => `You can't create repositories in ${owner}`,
};

const Failures = {
  signedOut: { title: 'GitHub refused the token of alex-acme', message: 'It expired or was revoked. Sign in again to keep using this account.', action: 'Sign in again' },
  forbidden: { title: "alex-martin's token can't do this", message: 'It lacks the write:public_key scope. Sign in with the browser to get a token that has it.', action: 'Sign in with the browser', tone: 'warning' },
  rateLimited: { title: "GitHub's rate limit for alex-martin is used up", message: 'SterneGit asks again after 14:32, when GitHub resets it.', tone: 'wait' },
  notFound: { title: 'Not found on GitHub', message: 'The repository was deleted or renamed, or this account can’t see it.' },
  net: { title: "Couldn't reach api.github.com", message: 'curl: (6) Could not resolve host: api.github.com', action: 'Retry' },
  stopped: { title: 'The GitHub connector stopped', message: 'It ran out of time. Its last log lines:', log: ['debug GET /user/repos?per_page=100&affiliation=owner,collaborator,organization_member', 'debug 100 repositories, next page', 'warn  sorting 2,400 repositories'], action: 'Copy log' },
  unavailable: { title: 'Not available', message: "This connector doesn't offer it.", tone: 'wait' },
};

const PullStates = {
  open: { label: 'Open', icon: 'git-pull-request', color: 'var(--green)' },
  draft: { label: 'Draft', icon: 'git-pull-request-draft', color: 'var(--muted)' },
  merged: { label: 'Merged', icon: 'git-merge', color: 'var(--magenta)' },
  closed: { label: 'Closed', icon: 'git-pull-request-closed', color: 'var(--red)' },
};

const pullLook = (pull) => PullStates[pull.state === 'open' && pull.draft ? 'draft' : pull.state];

const ReviewLooks = {
  approved: { label: 'Approved', icon: 'circle-check', color: 'var(--green)' },
  changes_requested: { label: 'Changes requested', icon: 'circle-alert', color: 'var(--red)' },
  review_required: { label: 'Review required', icon: 'eye', color: 'var(--yellow)' },
};

const CheckLooks = {
  failure: { icon: 'circle-x', color: 'var(--red)', word: 'failing' },
  action_required: { icon: 'circle-alert', color: 'var(--yellow)', word: 'waiting for you' },
  running: { icon: 'loader', color: 'var(--yellow)', word: 'running', spin: true },
  pending: { icon: 'clock', color: 'var(--yellow)', word: 'waiting' },
  success: { icon: 'circle-check', color: 'var(--green)', word: 'passed' },
  neutral: { icon: 'circle-minus', color: 'var(--muted)', word: 'neutral' },
  skipped: { icon: 'circle-minus', color: 'var(--muted)', word: 'skipped' },
  cancelled: { icon: 'circle-minus', color: 'var(--muted)', word: 'cancelled' },
};

const MergeMethodLabels = {
  merge: { label: 'Create a merge commit', button: 'Merge', about: 'Keeps every commit and adds a merge commit' },
  squash: { label: 'Squash and merge', button: 'Squash and merge', about: 'Makes one commit of all the changes' },
  rebase: { label: 'Rebase and merge', button: 'Rebase and merge', about: 'Replays each commit onto the base, without a merge commit' },
  semi_linear: { label: 'Semi-linear merge', button: 'Merge', about: 'Rebases, then adds a merge commit' },
  fast_forward: { label: 'Fast-forward', button: 'Fast-forward', about: 'Moves the base to the head, without a merge commit' },
};

const MergeLooks = {
  ready: { icon: 'circle-check', color: 'var(--green)', title: () => 'Ready to merge', about: (pull) => `No conflicts with ${pull.base}, and the checks the repository requires passed.`, can: true },
  failing: { icon: 'triangle-alert', color: 'var(--yellow)', title: () => 'Some checks failed', about: () => "The repository doesn't require them, so it can still be merged.", can: true },
  checking: { icon: 'loader', spin: true, color: 'var(--muted)', title: () => 'Checking whether it can merge…', about: () => 'The server is still working it out. SterneGit asks again after the next fetch.' },
  conflicts: { icon: 'circle-x', color: 'var(--red)', title: (pull) => `Conflicts with ${pull.base}`, about: (pull) => `Merge ${pull.base} into ${pull.head} or rebase it, resolve the conflicts, then push.` },
  behind: { icon: 'arrow-down', color: 'var(--yellow)', title: (pull) => `${pull.base} moved on`, about: (pull) => `The repository requires ${pull.head} to contain ${pull.base}: merge it in or rebase, then push.` },
  draft: { icon: 'git-pull-request-draft', color: 'var(--muted)', title: () => 'Still a draft', about: () => 'Mark it ready for review on the server to merge it.' },
  blocked: { icon: 'lock', color: 'var(--red)', title: () => 'Merging is blocked', about: (pull, details) => details.blockedBy },
  merged: { icon: 'git-merge', color: 'var(--magenta)', title: (pull) => `Merged into ${pull.base}`, about: (pull) => `The branch ${pull.head} isn't needed any more.` },
  closed: { icon: 'git-pull-request-closed', color: 'var(--red)', title: () => 'Closed without merging', about: () => 'Reopen it on the server to merge it.' },
};

const WorktreeTexts = {
  section: 'Worktrees',
  headerTooltip: 'Open the Worktrees panel to add and clean up worktrees',
  rowTooltip: (tree, where) => `${tree.path}\n${where}`,
  lockedBecause: (reason) => `Locked: ${reason}`,
  missingTooltip: (path) => `${path} is gone. Forget this worktree to free its branch.`,
  detached: (id) => `detached at ${id}`,
  elsewhere: (name) => `Checked out in the worktree ${name}`,
  thisTab: 'This tab',
  main: 'Main worktree',
  changed: (n) => counted(n, 'changed file', 'changed files'),
  locked: 'Locked',
  missing: 'Folder missing',
  merged: (into) => `Merged into ${into}`,
  openInTab: 'Open in a new tab',
  showInFolder: 'Show in folder',
  copyPath: 'Copy path',
  lock: 'Lock…',
  lockTitle: (name) => `Lock the worktree ${name}`,
  lockAbout: "A locked worktree is never forgotten by git worktree prune, even while its folder is on a drive that isn't plugged in, and can't be removed until it is unlocked.",
  lockPlaceholder: 'Reason (optional)',
  lockConfirm: 'Lock',
  unlock: 'Unlock',
  forget: 'Forget this worktree',
  remove: 'Remove worktree…',
  newWorktree: 'New worktree…',
  newFor: (branch) => `New worktree for ${branch}…`,
  newFrom: (branch) => `New worktree from ${branch}…`,
  openWorktree: (name) => `Open the worktree ${name}`,
  dialogTitle: 'New worktree',
  dialogAbout: 'Another folder with its own checkout of this repository. Commits, branches and stashes are shared with every worktree; the files, the staged changes and HEAD are its own.',
  existing: 'Existing branch',
  fresh: 'New branch',
  branch: 'Branch',
  oneCheckout: 'A branch can be checked out in one worktree at a time. A remote branch checks out its local branch, or gets one that tracks it.',
  branchName: 'Name',
  from: 'Starting from',
  folder: 'Folder',
  browse: 'Browse…',
  folderHint: (parent) => `Next to the repository, in ${parent}, named after the branch`,
  openAfter: 'Open it in a new tab',
  create: 'Create worktree',
  inUse: (branch, name) => `${branch} is already checked out in ${name}`,
  notEmpty: "This folder already exists and isn't empty",
  removeTitle: (name) => `Remove the worktree ${name}?`,
  removeFolder: (path) => `Deletes the folder ${path}.`,
  removeChanges: (n) => `Its ${counted(n, 'uncommitted change is', 'uncommitted changes are')} lost.`,
  removeKeeps: (branch) => `Its commits stay on ${branch}.`,
  deleteBranch: (branch) => `Also delete the branch ${branch}`,
  deleteBranchUnmerged: (branch, into) => `${branch} isn't merged into ${into}, so it is kept`,
  removeConfirm: 'Remove',
  manageTitle: (repository) => `Worktrees of ${repository}`,
  manageAbout: 'Every folder that checks out this repository. Removing one deletes its folder; its commits stay on its branch.',
  selectMerged: 'Select merged',
  forgetMissing: 'Forget missing folders',
  removeSelected: (n) => (n ? `Remove ${counted(n, 'worktree', 'worktrees')}` : 'Remove'),
  lostWarning: (n) => `${counted(n, 'worktree has', 'worktrees have')} uncommitted changes, which are lost.`,
  close: 'Close',
  uncommitted: 'Uncommitted changes',
  openTabTooltip: 'Open this worktree in a new tab',
  showTooltip: 'Show the folder in the file manager',
};

const PullTexts = {
  section: 'Pull requests',
  filters: { open: 'Open', closed: 'Closed', all: 'All' },
  filterAbout: { open: 'Waiting to be merged', closed: 'Merged or closed, newest first', all: 'Every pull request, newest first' },
  filterTooltip: 'Choose which pull requests to list',
  filterHeading: 'List',
  reading: 'Reading pull requests…',
  none: { open: 'No open pull requests', closed: 'No closed pull requests', all: 'No pull requests yet' },
  noAccount: (service, server) => `Add a ${service} account to see the pull requests of ${server}`,
  signedOut: (login) => `${login} is signed out`,
  rowTooltip: (pull) => `#${pull.number} by ${pull.author}: ${pull.head} into ${pull.base}`,
  wantsToMerge: (author, head, base, merged) => `${author} ${merged ? 'merged' : 'wants to merge'} ${head} into ${base}`,
  opened: (date) => `opened ${date}`,
  checkOut: 'Check out',
  checkOutTooltip: (head) => `Check out ${head} and track it`,
  checkOutFork: (number, head) => `Check out pr/${number}-${head}, made from the fork's branch`,
  checkedOut: 'Checked out',
  openOn: (service) => `Open on ${service}`,
  noDescription: 'No description.',
  checks: 'Checks',
  checksSummary: (checks) => {
    const counts = new Map();
    for (const check of checks) counts.set(check.state, (counts.get(check.state) || 0) + 1);
    return Object.keys(CheckLooks).filter((state) => counts.has(state)).map((state) => `${thousands(counts.get(state))} ${CheckLooks[state].word}`).join(', ');
  },
  noChecks: 'No checks ran on this commit.',
  conversation: 'Conversation',
  noComments: 'No comments yet.',
  commentPlaceholder: 'Write a comment',
  comment: 'Comment',
  plainText: 'Sent as you type it. SterneGit shows comments as plain text.',
  merge: 'Merge…',
  changesIn: (n) => `Changes in #${n}`,
  sinceBase: (n, base) => `${counted(n, 'commit', 'commits')} since ${base}, where it branched off`,
  fetching: (n, remote) => `Fetching #${n} from ${remote}…`,
  fetchFailed: "Couldn't fetch this pull request",
  position: (n, count, filter) => `Pull request #${n} · ${thousands(count)} ${filter.toLowerCase()}`,
  showItem: (n) => `Show pull request #${n}`,
  createItem: 'Create pull request…',
  createTitle: 'Create a pull request',
  createAbout: (service) => `Asks ${service} to merge a branch into another. Reviewers and labels are set on ${service}.`,
  from: 'From',
  into: 'Into',
  title: 'Title',
  description: 'Description',
  draft: 'Create as draft',
  draftAbout: "Can't be merged until it is marked ready for review",
  pushFirst: (branch, n, remote) => `${branch} has ${counted(n, 'commit', 'commits')} that ${remote} doesn't. SterneGit pushes ${n === 1 ? 'it' : 'them'} first.`,
  publishFirst: (branch, remote) => `${branch} isn't on ${remote} yet. SterneGit pushes it first and makes ${remote}/${branch} its upstream.`,
  compared: (n, files) => `${counted(n, 'commit', 'commits')} and ${counted(files, 'file', 'files')} changed since it branched off`,
  create: 'Create pull request',
  alreadyOpen: (n) => `A pull request from this branch is already open: #${n}.`,
  mergeTitle: (n) => `Merge pull request #${n}`,
  mergeAbout: (head, base, sha) => `${head} is merged into ${base} on GitHub, only if its head is still ${sha}.`,
  method: 'Method',
  commitTitle: 'Commit title',
  commitMessage: 'Commit message',
  rebaseNote: (n, base) => `Each of the ${counted(n, 'commit', 'commits')} is replayed onto ${base}; GitHub keeps their messages.`,
  merged: (n, base) => `Merged #${n} into ${base}`,
  mergedMessage: (head) => `${head} isn't needed any more.`,
  deleteBranchTooltip: 'Delete it from the remote, and here unless it is checked out or holds other commits',
  deleteBranch: 'Delete branch',
  movedOn: (head) => `${head} moved since SterneGit read it, so nothing was merged. Look at the new commits first.`,
  checkedOutHead: (branch, upstream) => `Checked out ${branch}, tracking ${upstream}`,
  checkedOutFork: (branch, owner) => `Checked out ${branch} from ${owner}'s fork, without an upstream`,
  choosing: 'Pick an account above to see pull requests',
  failed: (title, message) => `${title}: ${message}`,
  created: (n) => `Created pull request #${n}`,
  commented: 'Comment posted',
};

const esc = (text) =>
  String(text).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');

const thousands = (n) => n.toLocaleString('en-US');

const counted = (n, singular, plural) => `${thousands(n)} ${n === 1 ? singular : plural}`;

const join = (parts) => parts.filter(Boolean).join('');

const listed = (names) => (names.length > 1 ? `${names.slice(0, -1).join(', ')} and ${names[names.length - 1]}` : names[0] || '');

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

const RustTokens = /(\/\/.*$)|("(?:[^"\\]|\\.)*")|\b(fn|let|pub|impl|if|else|return|match|mut|use|struct|enum|crate|self|Self|move|for|in|const|as|where|loop|while|break|continue|ref|dyn|trait|type|mod|static|super|async|await|unsafe)\b|\b([A-Z][A-Z0-9_]+|true|false|\d[\d_]*(?:\.\d*)?)\b|\b([A-Z][A-Za-z0-9_]*|bool|char|str|f32|f64|usize|isize|u8|u16|u32|u64|i8|i16|i32|i64)\b|\b([a-z_][a-z0-9_]*!?)(?=\()/g;

function highlight(code, words = []) {
  const pieces = [];
  let last = 0;
  for (const match of code.matchAll(RustTokens)) {
    if (match.index > last) pieces.push([last, match.index, '']);
    const kind = match[1] ? 'comment' : match[2] ? 'string' : match[3] ? 'keyword' : match[4] ? 'number' : match[5] ? 'type' : 'function';
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

function textSides(diff) {
  if (diff.commit) return [diff.parent || '', diff.commit];
  if (diff.side === 'staged') return ['HEAD', 'Staged'];
  return [diff.stagedToo ? 'Staged' : 'HEAD', 'Working copy'];
}

function changeGroups(hunks) {
  const groups = [];
  for (const h of hunks) {
    let group = null;
    let lastNew = newSpan(h)[0] - 1;
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

function wholeFileTop(hunks) {
  return Math.max(1, newSpan(hunks[0])[0] - LinesAboveAChange);
}

function oldShift(h, at) {
  const both = h.lines.filter((line) => line[1] !== null && line[2] !== null);
  const line = at === 'start' ? both[0] : both[both.length - 1];
  return line ? line[1] - line[2] : 0;
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

function unityValue(value, other) {
  if (value === undefined) return '';
  const differs = (a, b) => b !== undefined && JSON.stringify(a) !== JSON.stringify(b);
  if (typeof value !== 'object') return `<span class="val${differs(value, other) && typeof other !== 'object' ? ' inline' : ''}">${esc(value)}</span>`;
  if (value.vec) {
    const names = ['X', 'Y', 'Z', 'W'];
    const axes = value.vec.map((n, ix) => `<span class="axis${other?.vec && other.vec[ix] !== n ? ' inline' : ''}"><b>${names[ix]}</b>${esc(n)}</span>`);
    return `<span class="vec">${axes.join('')}</span>`;
  }
  if (value.color) {
    const hex = `<span class="mono${other?.color && other.color !== value.color ? ' inline' : ''}">${esc(value.color.toUpperCase())}</span>`;
    const hdr = value.hdr === undefined ? '' : `<span class="${other && other.hdr !== value.hdr ? 'inline' : 'muted'}">Intensity ${esc(value.hdr)}</span>`;
    return `<span class="color-value"><span class="color-swatch" style="background:${value.color}"></span>${hex}${hdr}</span>`;
  }
  if (value.ref) return `<span class="ref-chip">${icon(value.glyph, 'xs')}<span>${esc(value.ref)}</span><span class="muted">(${esc(value.type)})</span></span>`;
  if (value.none) return `<span class="muted">None (${esc(value.none)})</span>`;
  if ('bool' in value) return `<span class="bool-value"><span class="tick${value.bool ? ' on' : ''}">${value.bool ? icon('check', 'xs') : ''}</span>${value.bool ? 'On' : 'Off'}</span>`;
  if (value.texture) return `<span class="texture-value"><span class="thumb checker">${pixelArt(value.art, value.palette, 4)}</span><span>${esc(value.texture)}</span></span>`;
  if (value.mono) return `<span class="val mono${differs(value, other) ? ' inline' : ''}">${esc(value.mono)}</span>`;
  return esc(String(value));
}

function unityChangeCount(objects) {
  return objects.reduce((sum, object) => sum + (object.state === 'changed' ? object.components.reduce((n, component) => n + component.props.length, 0) : 1), 0);
}

function artBounds(rows) {
  const filled = rows.flatMap((row, y) => [...row].map((cell, x) => (cell === '.' ? null : [x, y])).filter(Boolean));
  const xs = filled.map(([x]) => x);
  const ys = filled.map(([, y]) => y);
  return [Math.min(...xs), Math.min(...ys), Math.max(...xs), Math.max(...ys)];
}

function easedCurve(keys, x, y) {
  return keys
    .map(([frame, value], ix) => {
      if (!ix) return `M${x(frame)} ${y(value)}`;
      const [lastFrame, lastValue] = keys[ix - 1];
      const third = (x(frame) - x(lastFrame)) / 3;
      return `C${x(lastFrame) + third} ${y(lastValue)} ${x(frame) - third} ${y(value)} ${x(frame)} ${y(value)}`;
    })
    .join(' ');
}

function twisty(open) {
  return `<span class="twisty">${open === null ? '' : icon(open ? 'chevron-down' : 'chevron-right', 'xs')}</span>`;
}

function treeGroup(open, header, children) {
  return `<div class="tree-group${open ? '' : ' collapsed'}">${header}<div class="tree-children">${children}</div></div>`;
}

function summarize(changes) {
  return [
    changes.conflicted?.length && `${thousands(changes.conflicted.length)} conflicted`,
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
  const move = labels.fileTooltip && [labels.fileTooltip, ''];
  return menuGroups([[move, discard], [committed], placeItems(change.kind)]);
}

function pickedMenuItems({ files, side }) {
  const labels = SideLabels[side];
  const n = files.length;
  const untracked = files.every((change) => change.kind === '?');
  const discard = labels.discards && [untracked ? Texts.deleteFiles(n) : Texts.discardFiles(n), ''];
  const open = files.every((change) => change.kind !== 'D') && [Texts.openFiles(n), ''];
  return menuGroups([[[Texts.moveFiles(labels.verb, n), ''], discard], [open, [Texts.copyPaths(n), '']]]);
}

function discardFilesQuestion(changes) {
  const n = changes.length;
  const untracked = changes.filter((change) => change.kind === '?').length;
  if (untracked === n) return { title: Texts.deleteFilesTitle(n), description: Texts.deleteFilesDescription(n), confirm: 'Delete' };
  const description = untracked ? Texts.discardMixedDescription(n - untracked, untracked) : Texts.discardFilesDescription(n);
  return { title: Texts.discardFilesTitle(n), description, confirm: 'Discard' };
}

function commitFileMenuItems({ file }) {
  return menuGroups([[[Texts.fileHistory, '']], placeItems(file.kind)]);
}

function remoteParts(name) {
  const [remote, ...rest] = name.split('/');
  return { remote, branch: rest.join('/') };
}

function branchMenuItems({ label, current, mode = 'merge', pull }) {
  const remote = label.kind === 'remote' && remoteParts(label.name);
  const checkedOut = Boolean(label.head);
  const elsewhere = label.worktree;
  const taken = checkedOut || elsewhere || (remote && remote.branch === current);
  const item = (text) => [text, ''];
  const groups = [
    !checkedOut && [item(elsewhere ? WorktreeTexts.openWorktree(elsewhere) : Texts.checkOut(remote ? remote.branch : label.name))],
    !remote && [item(checkedOut ? Texts.pullWithMode(pullMode(mode).label) : Texts.pullFastForward), item(Texts.push)],
    pull && [item(pull === 'create' ? PullTexts.createItem : PullTexts.showItem(pull.number))],
    current && !checkedOut && [item(Texts.merge(label.name, current)), item(Texts.rebase(current, label.name))],
    [item(Texts.branchHere), item(taken ? WorktreeTexts.newFrom(label.name) : WorktreeTexts.newFor(remote ? remote.branch : label.name))],
    !checkedOut && !elsewhere && [item(remote ? Texts.deleteRemoteBranch(remote.branch, remote.remote) : Texts.deleteBranch(label.name))],
  ].filter(Boolean);
  return groups.flatMap((group, ix) => (ix ? ['-', ...group] : group));
}

function tagMenuItems({ label, remote }) {
  const item = (text) => [text, ''];
  return menuGroups([
    [item(Texts.checkOut(label.name))],
    [remote && item(Texts.pushTagTo(label.name, remote))],
    [item(Texts.branchHere)],
    [item(Texts.copyName)],
    [item(Texts.deleteBranch(label.name)), remote && item(Texts.deleteRemoteBranch(label.name, remote))],
  ]);
}

function changedCount(tree) {
  const changes = tree.changes || { unstaged: [], staged: [] };
  return new Set([...(changes.conflicted || []), ...changes.staged, ...changes.unstaged].map((change) => change.path)).size;
}

function worktreeMenuItems({ tree, here = false }) {
  const item = (text) => [text, ''];
  if (tree.missing) return menuGroups([[item(WorktreeTexts.forget)], [item(WorktreeTexts.copyPath)]]);
  return menuGroups([
    [!here && item(WorktreeTexts.openInTab)],
    [item(WorktreeTexts.showInFolder), item(WorktreeTexts.copyPath)],
    [!tree.main && item(tree.locked ? WorktreeTexts.unlock : WorktreeTexts.lock)],
    [!tree.main && !here && !tree.locked && item(WorktreeTexts.remove)],
  ]);
}

function stashMenuItems({ stash }) {
  const item = (text) => [text, ''];
  return [item(Texts.applyStash(stash.name)), item(Texts.popStash(stash.name)), '-', item(Texts.deleteStash(stash.name))];
}

const ResetModes = ['soft', 'mixed', 'hard'];

function commitMenuItems({ commit, current, reachable = true, reset }) {
  const item = (text) => [text, ''];
  const branch = current || 'HEAD';
  const modes = { items: ResetModes.map((mode) => item(Texts.resetModes[mode])), open: Boolean(reset), hot: reset && Texts.resetModes[reset] };
  const undo = reachable ? item(commit.merge ? Texts.revertMerge : Texts.revertCommit) : !commit.merge && item(Texts.cherryPick(branch));
  return menuGroups([
    [!commit.head && item(Texts.checkOutCommit)],
    [item(Texts.branchHere), item(Texts.tagHere)],
    [undo, !commit.head && [Texts.resetHere(branch), '', modes]],
    [item(Texts.copyHash), item(Texts.copyMessage)],
  ]);
}

function pickedCommitsMenuItems({ commits, current, held = 'none', line = false }) {
  const item = (text) => [text, ''];
  const n = commits.length;
  const merges = commits.some((commit) => commit.merge);
  const undo = held === 'all' ? item(Texts.revertCommits(n)) : held === 'none' && !merges && item(Texts.cherryPickCommits(n, current || 'HEAD'));
  return menuGroups([[undo], [line && !merges && item(Texts.squashCommits(n))], [item(Texts.copyHashes(n)), item(Texts.copyMessages(n))]]);
}

const typedOrigin = (origin) => origin.startsWith('{');

const originHost = (origin) => (typedOrigin(origin) ? null : new URL(origin).host);

function grantSummary(grant) {
  const hosts = [...new Set(grant.reach.map((entry) => originHost(entry.origin)).filter(Boolean))];
  return HostTexts.reaches(hosts, grant.reach.some((entry) => entry.typed));
}

function slotted(text, slot, label) {
  return esc(text).replace(esc(slot), `<span class="slot">${esc(label)}</span>`);
}

function checkIcon(state, size = 'sm') {
  const look = CheckLooks[state];
  const glyph = icon(look.icon, size, look.color);
  return look.spin ? glyph.replace('class="icon"', 'class="icon spinner"') : glyph;
}

function checkRollup(checks) {
  return Object.keys(CheckLooks).find((state) => checks.some((check) => check.state === state));
}

function personAvatar(login, size) {
  return C.AuthorAvatar({ name: login, email: login, size });
}

function pullChip(pull) {
  const look = pullLook(pull);
  return `<span class="pull-chip" style="--c:${look.color}" title="${esc(`Pull request #${pull.number}${pull.draft ? ', a draft' : ''}`)}">${icon(look.icon, 'xs')}${thousands(pull.number)}</span>`;
}

function accountAvatar(account, size) {
  return C.AuthorAvatar({ name: account.name || account.login, email: account.email, size });
}

function formField({ label, control, hint, problem, warning, extra = '' }) {
  const note = problem
    ? `<div class="problem">${esc(problem)}</div>`
    : warning
      ? `<div class="warn">${esc(warning)}</div>`
      : hint
        ? `<div class="note">${esc(hint)}</div>`
        : '';
  const head = label ? `<div class="h label"><span class="fill sm medium">${esc(label)}</span>${extra}</div>` : '';
  return `<div class="v form-field">${head}${control}${note}</div>`;
}

function withButton(control, button) {
  return `<div class="h" style="gap:8px"><div class="fill v">${control}</div>${button}</div>`;
}

function dialogShell({ name, title, description, body = '', buttons, size = 'wide', stretch = false }) {
  const close = `<span class="close">${icon('close', 'sm')}</span>`;
  const text = description ? `<div class="sm muted">${esc(description)}</div>` : '';
  return `<div class="overlay"><div data-c="${name}" data-base="Dialog" class="dialog ${size}">${close}<div class="base semibold">${esc(title)}</div>${text}${body}<div class="footer${stretch ? ' stretch' : ''}">${buttons}</div></div></div>`;
}

function protocolToggle(hasKey) {
  const ssh = C.Button({ label: 'SSH', variant: 'ghost', size: 'sm', selected: false, disabled: !hasKey, tooltip: hasKey ? 'Clone over SSH with the account\'s SterneGit key' : HostTexts.sshNeedsKey });
  return `<div class="h toggle-group">${C.Button({ label: 'HTTPS', variant: 'ghost', size: 'sm', selected: true })}${ssh}</div>`;
}

function visibilityChoice(visibilities, chosen) {
  const radios = visibilities.map((key) => C.Radio({ label: Visibilities[key].label, about: Visibilities[key].about, checked: key === chosen }));
  return `<div class="h radios">${radios.join('')}</div>`;
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

  Checkbox({ label, checked = false, disabled = false, tooltip }) {
    const title = tooltip ? ` title="${esc(tooltip)}"` : '';
    return `<span data-c="Checkbox" class="checkbox${checked ? ' checked' : ''}${disabled ? ' disabled' : ''}"${title}><span class="box">${checked ? icon('check', 'xs') : ''}</span><span>${esc(label)}</span></span>`;
  },

  Input({ value = '', placeholder = '', lines = 1, focused = false, disabled = false, prefix, mono = false }) {
    const multiLine = lines > 1;
    const classes = ['input', multiLine && 'multi-line', focused && 'focused', disabled && 'disabled', prefix && 'with-prefix'].filter(Boolean).join(' ');
    const height = multiLine ? ` style="height:${lines * 20 + 17}px"` : '';
    const caret = focused ? '<span class="caret"></span>' : '';
    const text = value
      ? `<span class="${multiLine ? 'fill' : 'ellipsis'}${mono ? ' mono' : ''}">${esc(value)}${caret}</span>`
      : `${caret}<span class="placeholder">${esc(placeholder)}</span>`;
    const lead = prefix ? `<span class="prefix">${icon(prefix, 'sm')}</span>` : '';
    return `<div data-c="Input" class="${classes}"${height}>${lead}${text}</div>`;
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

  AppLogo({ size = 16 } = {}) {
    return `<img data-c="AppLogo" class="app-logo" src="logo.svg" width="${size}" height="${size}" alt="SterneGit">`;
  },

  TopBar({ platform = 'windows', repository, head, openMenu, worktree, update } = {}) {
    const mac = platform === 'mac';
    const lights = mac
      ? '<div class="traffic-lights"><span style="background:#ff5f57"></span><span style="background:#febc2e"></span><span style="background:#28c840"></span></div>'
      : '';
    const pill = head
      ? `<div class="h head-pill">${icon('git-branch', 'xs')}<div class="ellipsis">${esc(head)}</div></div>`
      : '';
    const tree = worktree ? `<div class="h head-pill" title="${esc(WorktreeTexts.section)}">${icon('folder-git', 'xs')}<div class="ellipsis">${esc(worktree)}</div></div>` : '';
    const repo = repository
      ? `<div class="h" style="gap:8px"><div class="sm medium ellipsis">${esc(repository)}</div>${tree}${pill}</div>`
      : '';
    return `<div data-c="TopBar" data-base="TitleBar" class="top-bar${mac ? ' mac' : ''}">${lights}<div class="content">${C.AppLogo()}${mac ? '' : C.AppMenuBar({ open: openMenu })}<div class="fill"></div>${repo}${update ? C.UpdateButton(update) : ''}</div>${mac ? '' : C.WindowControls()}</div>`;
  },

  Tab({ name, state = 'ready', active = false, icon: glyph }) {
    const prefix =
      state === 'reading'
        ? `<span class="prefix">${C.Spinner({ size: 'xs' })}</span>`
        : state === 'failed'
          ? `<span class="prefix">${icon('triangle-alert', 'xs', 'var(--red)')}</span>`
          : glyph
            ? `<span class="prefix">${icon(glyph, 'xs', 'var(--muted)')}</span>`
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

  SectionHeader({ title, count, action, open, opens, hot = false }) {
    const foldable = open !== undefined;
    const shownCount = count === undefined ? '' : `<span>${thousands(count)}</span>`;
    const panel = opens ? `<span class="opens-icon">${icon('maximize-2', 'xs')}</span>` : '';
    const classes = ['section-header', foldable && 'foldable', opens && 'opens', hot && 'hot'].filter(Boolean).join(' ');
    const title_ = opens ? ` title="${esc(opens)}"` : '';
    return `<div data-c="SectionHeader" class="${classes}"${title_}>${foldable ? twisty(open) : ''}<span class="title ellipsis">${esc(title)}</span>${shownCount}${panel}${action || ''}</div>`;
  },

  RefBadge({ name, kind, head = false, synced = false, color = LaneColors[0], pull, worktree }) {
    const icons = icon(RefIcons[kind], 'xs') + (synced ? icon(RefIcons.remote, 'xs') : '') + (worktree ? icon(RefIcons.worktree, 'xs') : '');
    const title = worktree ? ` title="${esc(WorktreeTexts.elsewhere(worktree))}"` : '';
    return `<div data-c="RefBadge" class="ref-badge${head ? ' head' : ''}" style="--c:${color}"${title}><div class="icons">${icons}</div><div class="name ellipsis">${esc(name)}</div>${pull ? pullChip(pull) : ''}</div>`;
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

  GraphCell({ graph, merge = false, head = false, stash = false, wip = false, width }) {
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
              ? `M${x(node)} ${stash || wip ? 18.5 : 14} L${x(node)} 28`
              : `M${x(node)} 14 C${x(lane)} 14 ${x(lane)} 14 ${x(lane)} 28`;
      const dashed = wip && kind === 'O' ? ' stroke-dasharray="3 3"' : '';
      return `<path d="${d}" fill="none" style="stroke:${color(lane)}" stroke-width="2"${dashed}/>`;
    });
    const radius = merge ? 4.5 * 0.7 : 4.5;
    const dot = wip
      ? `<circle cx="${x(node)}" cy="14" r="4.5" fill="none" style="stroke:${color(node)}" stroke-width="1.5" stroke-dasharray="2.4 2"/>`
      : stash
      ? `<rect x="${x(node) - 4.5}" y="9.5" width="9" height="9" rx="1.5" fill="none" style="stroke:${color(node)}" stroke-width="1.5"/>`
      : `<circle cx="${x(node)}" cy="14" r="${radius}" style="fill:${color(node)}"/>`;
    const ring = head ? `<circle cx="${x(node)}" cy="14" r="7.5" fill="none" style="stroke:${color(node)}" stroke-width="1.5"/>` : '';
    return `<span data-c="GraphCell" class="graph-cell none" style="width:${w}px"><svg width="${w}" height="28" viewBox="0 0 ${w} 28">${paths.join('')}${dot}${ring}</svg></span>`;
  },

  UncommittedCell() {
    const w = LaneWidth + 2 * GraphMargin;
    return `<span data-c="UncommittedCell" class="graph-cell none" style="width:${w}px"><svg width="${w}" height="28" viewBox="0 0 ${w} 28"><circle cx="${laneX(0)}" cy="14" r="4.5" fill="none" style="stroke:var(--muted)" stroke-width="1.5"/></svg></span>`;
  },

  Loading({ message, detail, action = '' }) {
    const more = detail ? `<div class="xs muted mono">${esc(detail)}</div>` : '';
    return `<div data-c="Loading" class="loading">${C.Spinner()}<div class="sm">${esc(message)}</div>${more}${action}</div>`;
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

  AheadBehind({ ahead = 0, behind = 0, upstream }) {
    const side = (count, arrow) => (count ? `<span class="h">${icon(arrow, 'xs')}${thousands(count)}</span>` : '');
    return `<span data-c="AheadBehind" class="ahead-behind h xs muted" title="${esc(Texts.aheadBehind(ahead, behind, upstream))}">${side(behind, 'arrow-down')}${side(ahead, 'arrow-up')}</span>`;
  },

  RefRow({ label, name = label.name, depth = 0, selected = false }) {
    const apart = label.ahead || label.behind ? C.AheadBehind(label) : '';
    const pull = label.pull ? pullChip(label.pull) : '';
    const elsewhere = label.worktree ? `<span class="none h" title="${esc(WorktreeTexts.elsewhere(label.worktree))}">${icon('folder-git', 'xs', 'var(--muted)')}</span>` : '';
    const content = `<div class="h fill tree-row" style="padding-left:${depth * TreeIndent}px">${twisty(null)}${icon(TreeIcons[label.kind].ref, 'sm', 'var(--muted)')}<div class="fill ellipsis sm${label.head ? ' semibold' : ''}">${esc(name)}</div>${pull}${apart}${elsewhere}</div>`;
    return C.ListRow({ content, selected, name: 'RefRow' });
  },

  RefFolderRow({ name, kind, depth = 0, open = true }) {
    const shape = depth === 0 ? TreeIcons[kind].top : TreeIcons[kind].folder;
    const content = `<div class="h fill tree-row" style="padding-left:${depth * TreeIndent}px">${twisty(open)}${icon(shape, 'sm', 'var(--muted)')}<div class="fill ellipsis sm">${esc(name)}</div></div>`;
    return C.ListRow({ content, name: 'RefFolderRow' });
  },

  WorktreeRow({ tree, here = false, selected = false }) {
    const where = tree.branch || WorktreeTexts.detached(tree.id);
    const n = changedCount(tree);
    const status = tree.missing
      ? `<span class="none h" title="${esc(WorktreeTexts.missingTooltip(tree.path))}">${icon('triangle-alert', 'xs', 'var(--yellow)')}</span>`
      : join([
          tree.locked && `<span class="none h" title="${esc(WorktreeTexts.lockedBecause(tree.locked))}">${icon('lock', 'xs', 'var(--muted)')}</span>`,
          n && `<span class="none h xs worktree-dirty" title="${esc(WorktreeTexts.changed(n))}"><span class="dot"></span>${thousands(n)}</span>`,
        ]);
    const name = `<span class="worktree-name ellipsis sm${here ? ' semibold' : ''}${tree.missing ? ' muted' : ''}">${esc(tree.name)}</span>`;
    const content = `<div class="h fill tree-row" title="${esc(WorktreeTexts.rowTooltip(tree, where))}">${twisty(null)}${icon('folder-git', 'sm', 'var(--muted)')}${name}<span class="fill ellipsis xs muted">${esc(where)}</span>${status}</div>`;
    return C.ListRow({ content, selected, name: 'WorktreeRow' });
  },

  StashRow({ stash, selected = false }) {
    const content = `<div class="h fill tree-row">${twisty(null)}${icon('archive', 'sm', 'var(--muted)')}<div class="fill ellipsis sm">${esc(stash.message)}</div></div>`;
    return C.ListRow({ content, selected, name: 'StashRow' });
  },

  RefsList({ refs, stashes = [], selectedRow, picked = [], collapsed = [], pulls, worktrees }) {
    const closed = new Set(collapsed);
    const nodes = (kind, list, depth) =>
      list
        .map((node) => {
          if (!node.children) return C.RefRow({ label: node.label, name: node.name, depth, selected: node.label.row === selectedRow || picked.includes(node.label.row) });
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
    const worktreeSection = () => {
      if (!worktrees) return '';
      const open = !closed.has('worktree');
      const rows = worktrees.list.map((tree) => C.WorktreeRow({ tree, here: tree.name === worktrees.here, selected: tree.name === worktrees.selected })).join('');
      const header = C.SectionHeader({ title: WorktreeTexts.section, count: worktrees.list.length, open, opens: WorktreeTexts.headerTooltip, hot: worktrees.hot });
      return treeGroup(open, header, rows);
    };
    sections.splice(1, 0, worktreeSection());
    const pullSection = pulls ? C.PullsSection({ ...pulls, open: !closed.has('pulls') }) : '';
    return `<div data-c="RefsList" class="refs">${pullSection}${sections.join('')}</div>`;
  },

  CommitRow({ commit, selected = false, width, pulls = {}, elsewhere = {} }) {
    const color = LaneColors[commit.graph[0] % LaneColors.length];
    const refs = foldSyncedRefs(commit.refs).map((label) => (label.kind === 'local' && elsewhere[label.name] ? { ...label, worktree: elsewhere[label.name] } : label));
    const stashBadge = commit.stash ? C.RefBadge({ name: commit.stash, kind: 'stash', color }) : '';
    const treeBadge = commit.worktree ? C.RefBadge({ name: commit.worktree, kind: 'worktree', color }) : '';
    const badges = stashBadge + treeBadge + refs.slice(0, 3).map((label) => C.RefBadge({ ...label, color, pull: label.kind === 'local' ? pulls[label.name] : undefined })).join('');
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

  WorktreeChangesRow({ tree, graph, width, selected = false }) {
    const color = LaneColors[graph[0] % LaneColors.length];
    const content =
      C.GraphCell({ graph, width, wip: true }) +
      `<div class="labels">${C.RefBadge({ name: tree.name, kind: 'worktree', color })}<div class="fill ellipsis sm italic">${esc(WorktreeTexts.uncommitted)}</div></div>` +
      `<div class="xs muted ellipsis worktree-summary">${esc(summarize(tree.changes))}</div>`;
    return C.ListRow({ content, selected, extraClass: 'commit-row', name: 'WorktreeChangesRow' });
  },

  HistoryList({ history, selectedRow, picked = [], changes, wipSelected = false }) {
    const width = graphWidth(history.commits);
    const wip = changes && (changes.conflicted?.length || changes.unstaged.length || changes.staged.length) ? C.UncommittedRow({ changes, selected: wipSelected }) : '';
    const rows = history.commits
      .map((commit, ix) =>
        commit.wip
          ? C.WorktreeChangesRow({ tree: commit.wip, graph: commit.graph, width, selected: ix === selectedRow })
          : C.CommitRow({ commit, selected: ix === selectedRow || picked.includes(ix), width, pulls: history.pulls, elsewhere: history.elsewhere }),
      )
      .join('');
    const thumb = Math.max(20, Math.round((28 / history.total) * 800));
    return `<div data-c="HistoryList" class="history v">${wip}<div class="fill" style="position:relative;overflow:hidden">${rows}<div class="scrollbar" style="top:2px;height:${thumb}px"></div></div></div>`;
  },

  DiffView({ files, mode = 'unified', header = true, actions, selection, pointed, menu = '', words = true, look = {} }) {
    const bars = look.marks === 'bars';
    const foldButtons = (count, ...arrows) => {
      const partial = count > FoldLimit;
      const reveal = partial ? arrows.map((arrow) => C.Button({ icon: arrow, variant: 'ghost', tooltip: arrow === 'arrow-up' ? Texts.showAbove(FoldLimit) : Texts.showBelow(FoldLimit) })).join('') : '';
      const all = C.Button({ icon: partial ? 'chevrons-up-down' : 'chevron-down', label: counted(count, 'unchanged line', 'unchanged lines'), variant: 'ghost', tooltip: Texts.expandLines(count) });
      return reveal + all;
    };
    const fold = (count, arrow) => (count <= 0 ? '' : `<div class="diff-line fold">${foldButtons(count, arrow)}</div>`);
    const marker = (sign) => (bars ? '<span class="bar"></span>' : `<span class="sign">${sign === ' ' || !sign ? '' : sign === '-' ? '−' : '+'}</span>`);
    const line = (entry, side, marks) => {
      if (!entry) return `<div class="diff-line blank"><span class="number"></span>${marker('')}<span class="text"></span></div>`;
      const [sign, oldNo, newNo, text] = entry;
      const kind = sign === '+' ? 'added' : sign === '-' ? 'deleted' : 'context';
      const pointedSide = pointed && (pointed.side || (sign === '-' ? 'old' : 'new'));
      const anchored = entry === marks.pointed && (side === 'both' || side === pointedSide);
      const classes = ['diff-line', kind, marks.selected.has(entry) && 'selected'].filter(Boolean).join(' ');
      const numbers =
        side === 'old'
          ? `<span class="number">${oldNo ?? ''}</span>`
          : side === 'new'
            ? `<span class="number">${newNo ?? ''}</span>`
            : `<span class="number">${oldNo ?? ''}</span><span class="number">${newNo ?? ''}</span>`;
      const names = (look.hints || []).filter(([needle]) => text.includes(needle)).map(([, label]) => label);
      const hints = names.length ? `<span class="ref-hint">  # ${esc(names.join(', '))}</span>` : '';
      return `<div class="${classes}">${numbers}${marker(sign)}<span class="text">${highlight(text, marks.words.get(entry))}${hints}</span>${anchored ? menu : ''}</div>`;
    };
    const hunk = (h, hx, marks, own) => {
      const selected = own && selection && selection.hunk === hx ? selectionSize(selection) : 0;
      const buttons = actions ? C.HunkActions({ side: actions.side, selected, writing: actions.writing }) : '';
      const above = marks.hidden ? marks.hidden.above[hx] : 0;
      const below = marks.hidden && hx === marks.last ? marks.hidden.below : 0;
      const title = marks.hidden ? widenedHeader(h.header, above, below) : h.header;
      const scope = h.header.replace(/^@@[^@]*@@\s*/, '');
      const top = marks.whole
        ? look.gaps === 'scope-always'
          ? `<div class="diff-line gap"><span class="scope">${esc(scope)}</span>${buttons}</div>`
          : buttons && `<div class="diff-line gap"><span class="fill"></span>${buttons}</div>`
        : look.gaps === 'scope' || look.gaps === 'scope-always'
          ? `<div class="diff-line gap">${above > 0 ? foldButtons(above, ...(hx ? ['arrow-down', 'arrow-up'] : ['arrow-up'])) : ''}<span class="scope">${esc(scope)}</span>${buttons}</div>`
          : `<div class="diff-line hunk"><span class="text">${esc(title)}</span>${buttons}</div>${fold(above, 'arrow-up')}`;
      if (mode === 'split') {
        return top + pairLines(h.lines).map(([left, right]) => `<div class="diff-split-row">${line(left, 'old', marks)}${line(right, 'new', marks)}</div>`).join('');
      }
      return top + h.lines.map((entry) => line(entry, 'both', marks)).join('');
    };
    const sidesRow = () => {
      if (!look.sides) return '';
      const [before, after] = look.sides.map((from, ix) => C.VersionTag({ side: ix ? 'after' : 'before', from }));
      return mode === 'split' ? `<div class="diff-sides split"><div>${before}</div><div>${after}</div></div>` : `<div class="diff-sides">${before}${after}</div>`;
    };
    const scrollbars = () => {
      if (!look.scroll) return '';
      const thumb = '<span class="thumb"></span>';
      return mode === 'split' ? `<div class="hscroll split"><div>${thumb}</div><div>${thumb}</div></div>` : `<div class="hscroll"><div>${thumb}</div></div>`;
    };
    const plain = (entry, marks) => (mode === 'split' ? `<div class="diff-split-row">${line(entry, 'old', marks)}${line(entry, 'new', marks)}</div>` : line(entry, 'both', marks));
    const wholeFile = (f, marks, own) => {
      const from = wholeFileTop(f.hunks);
      const to = from + DrawnRows;
      let next = from;
      let rows = '';
      f.hunks.forEach((h, hx) => {
        const [start, end] = newSpan(h);
        const shift = hx ? oldShift(f.hunks[hx - 1], 'end') : oldShift(h, 'start');
        for (; next < start && next <= to; next++) rows += plain([' ', next + shift, next, f.unchanged(next) ?? ''], marks);
        if (start <= to) rows += hunk(h, hx, marks, own);
        next = Math.max(next, end + 1);
      });
      const last = f.hunks[f.hunks.length - 1];
      for (; next <= to && f.unchanged(next) !== undefined; next++) rows += plain([' ', next + oldShift(last, 'end'), next, f.unchanged(next)], marks);
      return rows;
    };
    const file = (f, fx) => {
      const own = fx === 0;
      const whole = Boolean(look.whole && f.unchanged && f.hunks.length);
      const marks = {
        selected: selectedEntries(f.hunks, own ? selection : null),
        pointed: own && pointed ? f.hunks[pointed.hunk].lines[pointed.line] : null,
        words: words ? new Map(f.hunks.flatMap((h) => [...wordChanges(h.lines)])) : new Map(),
        whole,
        hidden: !whole && f.lines && f.hunks.length ? hiddenLines(f.hunks, f.lines) : null,
        last: f.hunks.length - 1,
      };
      const top = header
        ? `<div class="diff-file-header">${C.Button({ icon: 'chevron-down', variant: 'ghost' })}<span class="title ellipsis">${esc(f.path)}</span><span class="stats xs muted">+${thousands(f.additions)} −${thousands(f.deletions)}</span></div>`
        : '';
      const end = fold(marks.hidden ? marks.hidden.below : 0, 'arrow-down');
      const body = whole ? wholeFile(f, marks, own) : f.hunks.map((h, hx) => hunk(h, hx, marks, own)).join('') + end;
      return `<div class="diff-file">${top}${sidesRow()}${body}</div>`;
    };
    const classes = ['diff', bars && 'bars', look.scroll && 'nowrap', ...(look.palette || []).map((name) => `palette-${name}`)].filter(Boolean).join(' ');
    return `<div data-c="DiffView" data-base="Diff" class="${classes}">${files.map(file).join('')}${scrollbars()}</div>`;
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

  FileMenu({ change, side, picked = [], left = 0, top = 0, hot }) {
    const items = picked.length > 1 ? pickedMenuItems({ files: picked, side }) : fileMenuItems({ change, side });
    return C.Menu({ items, left, top, hot, name: 'FileMenu' });
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

  CommitPanel({ commit, details, state = 'ready', reason = '', open, view = 'list', collapsed, pointed, pointing = false, edit }) {
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
    const tip = pointing ? `<div class="tip xs">${esc(Texts.editMessageTip)}</div>` : '';
    const message = edit
      ? C.MessageEditor({ subject: commit.subject, description: details.body, ...edit })
      : `<div class="commit-message${pointing ? ' hot' : ''}"><div class="base medium">${esc(commit.subject || '(no message)')}</div>${body}${tip}</div>`;
    const header =
      `<div class="panel-header">` +
      message +
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

  PickedCommitsPanel({ commits, together, open, view = 'list' }) {
    const authors = [...new Set(commits.map((commit) => commit.author))];
    const days = commits.map((commit) => commit.date.slice(0, 10)).sort();
    const span = days[0] === days[days.length - 1] ? days[0] : `${days[0]} – ${days[days.length - 1]}`;
    const row = (commit) =>
      `<div class="picked-commit h">${C.AuthorAvatar({ name: commit.author, email: commit.email })}<div class="fill ellipsis sm">${esc(commit.subject || '(no message)')}</div><span class="none xs muted mono">${esc(commit.id)}</span></div>`;
    const files = together ? together.files : [];
    const additions = files.reduce((sum, f) => sum + f.additions, 0);
    const deletions = files.reduce((sum, f) => sum + f.deletions, 0);
    const changes = together
      ? `<div class="xs muted">${esc(Texts.pickedTogether(together.from, commits[0].id))}</div>` +
        `<div class="h xs" style="gap:12px"><div class="muted">${counted(files.length, 'file', 'files')}</div>${C.Additions({ count: additions })}${C.Deletions({ count: deletions })}<div class="fill"></div>${C.FileViewToggle({ view })}</div>`
      : '';
    const header =
      `<div class="panel-header">` +
      `<div class="base medium">${esc(Texts.commitsPicked(commits.length))}</div>` +
      `<div class="h xs muted" style="gap:8px"><span class="ellipsis">${esc(listed(authors))}</span><span>·</span><span class="none">${esc(span)}</span></div>` +
      `<div class="picked-commits v">${commits.map(row).join('')}</div>` +
      changes +
      `</div>`;
    const body = together ? C.CommitFiles({ files, open, view }) : C.Hint({ message: Texts.pickedApart });
    return `<div data-c="PickedCommitsPanel" class="commit-panel">${header}<div class="fill v">${body}</div></div>`;
  },

  MessageEditor({ subject = '', description = '', focus, after = 0, branches = [], remotes = [], tags = [], saving = false, changed = true, failure }) {
    const lines = [
      `<div class="xs muted">${esc(Texts.rewrites(after, branches))}</div>`,
      remotes.length ? `<div class="h xs edit-warn">${icon('triangle-alert', 'xs')}<span>${esc(Texts.pushedAlready(remotes))}</span></div>` : '',
      tags.length ? `<div class="xs muted">${esc(Texts.tagsStay(tags))}</div>` : '',
    ];
    const save = C.Button({ label: saving ? Texts.savingMessage : Texts.saveMessage, variant: 'primary', size: 'sm', loading: saving, disabled: !changed || subject.trim() === '', tooltip: Texts.saveMessageTooltip });
    return (
      `<div data-c="MessageEditor" class="message-editor">` +
      C.Input({ value: subject, placeholder: Texts.commitMessage, focused: focus === 'subject', disabled: saving }) +
      C.Input({ value: description, placeholder: Texts.commitDescription, lines: 4, focused: focus === 'description', disabled: saving }) +
      join(lines) +
      `<div class="h edit-actions"><div class="fill"></div>${C.Button({ label: 'Cancel', size: 'sm', disabled: saving })}${save}</div>` +
      (failure ? `<div class="failure xs">${esc(failure)}</div>` : '') +
      `</div>`
    );
  },

  FileRow({ kind, path, depth, selected = false, trailing = '', name = 'FileRow', tag = '' }) {
    const [file, folder] = splitPath(path);
    const place =
      depth === undefined
        ? C.ChangeBadge({ kind }) +
          `<div class="path"><div class="none ellipsis sm" style="max-width:100%">${esc(file)}</div>${tag}${folder ? `<div class="ellipsis xs muted">${esc(folder)}</div>` : ''}</div>`
        : `<div class="h fill tree-row" style="padding-left:${depth * TreeIndent}px">${twisty(null)}${C.ChangeBadge({ kind })}<div class="${tag ? 'none' : 'fill'} ellipsis sm">${esc(file)}</div>${tag}</div>`;
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
    const move = labels.fileTooltip ? C.Button({ icon: labels.icon, variant: 'ghost', disabled: writing, tooltip: labels.fileTooltip }) : '';
    const trailing = move + menu;
    const file = splitPath(change.path)[0];
    const meta = change.meta ? `<span class="meta-chip" title="${esc(Texts.metaWith(file))}">meta</span>` : '';
    const warning = change.warning ? `<span class="row-warn" title="${esc(change.warning)}">${icon('triangle-alert', 'xs')}</span>` : '';
    return C.FileRow({ kind: change.kind, path: change.path, depth, selected, trailing, name: 'ChangeRow', tag: meta + warning });
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

  ChangesList({ changes, open, picked, pointed, draft = {}, committing = false, writing = false, failure, view = 'list', collapsed = [], operation, worktree }) {
    if (!changes) return `<div data-c="ChangesList" class="changes">${C.Loading({ message: Texts.readingChanges })}</div>`;
    const busy = writing || committing;
    const form = (worktree ? C.WorktreeBar({ tree: worktree }) : '') + (operation ? '' : C.CommitForm({ ...draft, staged: changes.staged.length, committing, writing, failure }));
    const conflicted = changes.conflicted || [];
    if (!conflicted.length && !changes.unstaged.length && !changes.staged.length) {
      return `<div data-c="ChangesList" class="changes">${form}<div class="fill v">${C.Hint({ message: Texts.noChanges })}</div></div>`;
    }
    const changed = new Set([...conflicted, ...changes.staged, ...changes.unstaged].map((change) => change.path)).size;
    const bar = `<div class="changes-bar">${C.SectionHeader({ title: counted(changed, 'changed file', 'changed files'), action: C.FileViewToggle({ view }) })}</div>`;
    const foldedIn = (side) => new Set(collapsed.filter((key) => key.startsWith(`${side}:`)).map((key) => key.slice(side.length + 1)));
    const section = (side, empty) => {
      const files = changes[side];
      const labels = SideLabels[side];
      const action = labels.all ? C.Button({ label: labels.all, icon: labels.icon, variant: 'ghost', disabled: busy || !files.length }) : '';
      const head = C.SectionHeader({ title: `${labels.files} (${thousands(files.length)})`, action });
      const at = (place, change) => !!place && place.side === side && place.path === change.path;
      const pickedHere = picked && picked.side === side ? files.filter((change) => picked.paths.includes(change.path)) : [];
      const isPicked = (change) => (picked ? pickedHere.includes(change) : at(open, change));
      const row = (change, depth) =>
        C.ChangeRow({
          change,
          side,
          depth,
          writing: busy,
          selected: isPicked(change),
          menu: at(pointed, change) ? C.FileMenu({ change, side, picked: isPicked(change) ? pickedHere : [], left: pointed.left, top: pointed.top, hot: pointed.hot }) : '',
        });
      const folder = (node, depth, open) =>
        C.FolderRow({ name: node.name, depth, open, trailing: C.Button({ icon: labels.icon, variant: 'ghost', disabled: busy, tooltip: labels.folderTooltip }) });
      const rows =
        view === 'tree'
          ? fileTreeRows(fileTree(files), { collapsed: foldedIn(side), file: row, folder })
          : files.map((change) => row(change)).join('');
      const body = files.length ? `<div class="fill">${rows}</div>` : C.Hint({ message: empty });
      return `<div class="change-section${side === 'conflicted' ? ' conflicts' : ''}">${head}${body}</div>`;
    };
    const conflicts = conflicted.length ? section('conflicted') : '';
    return `<div data-c="ChangesList" class="changes">${form}${bar}${conflicts}${section('staged', Texts.nothingStaged)}${section('unstaged', Texts.nothingUnstaged)}</div>`;
  },

  FileBar({ diff, state = 'ready', writing = false, expanded = false, filePosition, tools = '' }) {
    const labels = diff.commit ? null : SideLabels[diff.side];
    const shownState = diff.picked ? `in ${counted(diff.picked, 'commit', 'commits')}` : diff.commit ? `in ${diff.commit}` : labels.state;
    const conflicted = diff.side === 'conflicted';
    const total = conflicted && diff.parts ? diff.parts.filter((part) => part.conflict).length : 0;
    const chosen = conflicted && diff.resolutions ? diff.resolutions.filter(Boolean).length : 0;
    const keep = (side) => C.Button({ label: ConflictTexts.keep[side], disabled: writing, tooltip: ConflictTexts.keepTooltip[side] });
    const moveButton = !labels
      ? ''
      : conflicted && !diff.parts
        ? keep('current') + keep('incoming')
        : conflicted
          ? C.Button({ label: labels.verb, icon: labels.icon, variant: chosen === total ? 'primary' : 'default', disabled: writing || chosen < total, tooltip: chosen < total ? 'Resolve every conflict in the result first' : labels.openFileTooltip })
          : C.Button({ label: labels.verb, icon: labels.icon, disabled: writing, tooltip: labels.openFileTooltip });
    const path = diff.change.original ? `${diff.change.original} → ${diff.change.path}` : diff.change.path;
    const text = !diff.kind || diff.kind === 'text';
    const facts = diff.parts
      ? `<div class="none xs muted">${chosen} of ${counted(total, 'conflict', 'conflicts')} resolved</div>`
      : !text
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
      tools +
      (tools ? '<div class="separator"></div>' : '') +
      stepper +
      moveButton +
      expand +
      C.Button({ icon: 'close', variant: 'ghost', tooltip: 'Back to the graph  Esc' }) +
      `</div>`
    );
  },

  MergeEditor({ merge }) {
    const resolutions = merge.resolutions || [];
    const lead = 3;
    const shownRows = 15;
    const row = (no, text, classes = '', { plain = false, caret = false, tint = false } = {}) => {
      const content = (plain ? esc(text) : highlight(text)) + (caret ? '<span class="caret"></span>' : '');
      return `<div class="diff-line${classes ? ` ${classes}` : ''}"><span class="number">${no}</span><span class="sign"></span><span class="text">${tint ? `<span class="tint">${content}</span>` : content}</span></div>`;
    };
    const blank = '<div class="diff-line blank"><span class="number"></span><span class="sign"></span><span class="text"></span></div>';
    const fold = (n) =>
      n > 0
        ? `<div class="diff-line fold">${C.Button({ icon: n > FoldLimit ? 'chevrons-up-down' : 'chevron-down', label: counted(n, 'unchanged line', 'unchanged lines'), variant: 'ghost', tooltip: Texts.expandLines(n) })}</div>`
        : '';
    const pick = (index) => {
      const chosen = resolutions[index];
      const byHand = !!(chosen && chosen.edited);
      const boxes = ['current', 'incoming']
        .map((side) =>
          C.Checkbox({
            label: ConflictTexts.accept[side],
            checked: !!chosen && chosen.take.includes(side),
            disabled: byHand,
            tooltip: byHand ? ConflictTexts.byHand : ConflictTexts.acceptTooltip[side],
          }),
        )
        .join('');
      return { html: `<div class="diff-line conflict-head"><span class="fill"></span>${boxes}</div>`, head: index };
    };
    const at = { current: 1, incoming: 1, result: 1 };
    const versions = [];
    const result = [];
    let index = -1;
    let headed = -1;
    const last = merge.parts.length - 1;
    merge.parts.forEach((part, partIndex) => {
      if (part.conflict) {
        index += 1;
        if (headed < index) {
          versions.push(pick(index));
          headed = index;
        }
        const { current, incoming } = part.conflict;
        for (let ix = 0; ix < Math.max(current.length, incoming.length); ix++) {
          const left = ix < current.length ? row(at.current++, current[ix], 'conflicted current') : blank;
          const right = ix < incoming.length ? row(at.incoming++, incoming[ix], 'conflicted incoming') : blank;
          versions.push({ html: `<div class="diff-split-row">${left}${right}</div>` });
        }
        const chosen = resolutions[index];
        const first = result.length;
        if (!chosen) {
          result.push({ html: row(at.result++, `<<<<<<< ${merge.labels.current}`, 'marker', { plain: true, tint: true }) });
          current.forEach((text) => result.push({ html: row(at.result++, text, 'conflicted current', { tint: true }) }));
          result.push({ html: row(at.result++, '=======', 'marker', { plain: true, tint: true }) });
          incoming.forEach((text) => result.push({ html: row(at.result++, text, 'conflicted incoming', { tint: true }) }));
          result.push({ html: row(at.result++, `>>>>>>> ${merge.labels.incoming}`, 'marker', { plain: true, tint: true }) });
        } else {
          const lines = chosen.edited || chosen.take.flatMap((side) => part.conflict[side].map((text) => [side, text]));
          lines.forEach(([origin, text], ix) =>
            result.push({ html: row(at.result++, text, `conflicted ${chosen.edited ? 'edited' : origin}`, { caret: chosen.caret === ix, tint: true }) }),
          );
        }
        result[first].starts = index;
        return;
      }
      const head = part.head || [];
      const tail = part.tail || [];
      const hidden = part.hidden || 0;
      const total = head.length + hidden + tail.length;
      const textAt = (k) => (k < head.length ? head[k] : k >= total - tail.length ? tail[k - (total - tail.length)] : null);
      const line = (k) => {
        const text = textAt(k);
        return { html: `<div class="diff-split-row">${row(at.current + k, text)}${row(at.incoming + k, text)}</div>` };
      };
      const range = (from, to) => Array.from({ length: Math.max(0, to - from) }, (_, k) => from + k);
      if (partIndex === 0) {
        versions.push(pick(0));
        headed = 0;
        versions.push({ html: fold(total - lead) });
        range(total - lead, total).forEach((k) => versions.push(line(k)));
      } else if (partIndex === last) {
        range(0, Math.min(lead, total)).forEach((k) => versions.push(line(k)));
        versions.push({ html: fold(total - lead) });
      } else {
        range(0, lead).forEach((k) => versions.push(line(k)));
        versions.push({ html: fold(total - 2 * lead) });
        versions.push(pick(index + 1));
        headed = index + 1;
        range(total - lead, total).forEach((k) => versions.push(line(k)));
      }
      for (let k = 0; k < total; k++) {
        const text = textAt(k);
        result.push(text === null ? { hidden: true } : { html: row(at.result + k, text) });
      }
      at.current += total;
      at.incoming += total;
      at.result += total;
    });
    const reveal = merge.reveal || 0;
    const topAt = Math.max(0, versions.findIndex((entry) => entry.head === reveal));
    const shownVersions = versions.slice(topAt).map((entry) => entry.html).join('');
    const resultAt = Math.max(0, result.findIndex((entry) => entry.starts === reveal) - 2);
    const shownResult = [];
    for (const entry of result.slice(resultAt)) {
      if (entry.hidden || shownResult.length >= shownRows) break;
      shownResult.push(entry.html);
    }
    const total = index + 1;
    const left = total - resolutions.filter(Boolean).length;
    const versionHead = (side) =>
      `<div class="merge-head"><span class="version-dot ${side}"></span><span class="medium">${ConflictTexts.title[side]}</span><span class="ellipsis muted">${esc(merge.labels[side])}</span></div>`;
    const state = left
      ? `<span class="conflicts-left">${esc(ConflictTexts.left(left))}</span><span class="muted ellipsis">${esc(ConflictTexts.hint)}</span>`
      : `<span class="resolved">${esc(OperationTexts.noConflicts)}</span>`;
    const resultHead =
      `<div class="merge-head result"><span class="medium">${ConflictTexts.result}</span>${state}<span class="fill"></span>` +
      C.Button({ label: ConflictTexts.startOver, icon: 'undo-2', variant: 'ghost', disabled: !resolutions.length, tooltip: ConflictTexts.startOverTooltip }) +
      `</div>`;
    return (
      `<div data-c="MergeEditor" class="merge-editor">` +
      `<div class="versions"><div class="merge-heads">${versionHead('current')}${versionHead('incoming')}</div><div class="diff">${shownVersions}</div></div>` +
      `<div class="result">${resultHead}<div class="diff editor">${shownResult.join('')}</div></div>` +
      `</div>`
    );
  },

  ChangeStepper({ current, total, noun = 'Change' }) {
    const text = total ? `${noun} ${thousands(current)} of ${thousands(total)}` : 'No changes';
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

  ViewOptionsMenu({ checked = ['words', 'whole'], hot, right = 8, top = 30, items = ViewOptions, heading = 'Every text comparison' }) {
    const rows = items.map(
      (item) =>
        `<div class="item${item.key === hot ? ' hot' : ''}"><span class="check">${checked.includes(item.key) ? icon('check', 'sm') : ''}</span><span class="v label"><span>${esc(item.label)}</span><span class="about">${esc(item.about)}</span></span></div>`,
    ).join('');
    return `<div data-c="ViewOptionsMenu" data-base="PopupMenu" class="menu choices sm" style="right:${right}px;top:${top}px"><div class="heading">${esc(heading)}</div>${rows}</div>`;
  },

  VersionPicker({ shown = 'both' } = {}) {
    const choice = (key, label, dot, tooltip) => C.Button({ label, dot, variant: 'ghost', selected: shown === key, tooltip });
    return (
      `<span data-c="VersionPicker" class="version-picker"><span class="xs muted">Show</span>` +
      choice('before', 'Before', 'before', 'Draw the graph as it was before the change') +
      choice('both', 'Both', undefined, 'Draw both versions over each other') +
      choice('after', 'After', 'after', 'Draw the graph as it is after the change') +
      `</span>`
    );
  },

  ViewBar({ kind = 'text', modes: modeSet = kind, mode, changes, zoom, menuOpen = false, hot, hidden = 0, shown }) {
    const modes = CompareModes[modeSet]
      .map((item) => C.Button({ label: item.label, icon: item.icon, variant: 'ghost', selected: item.key === mode, tooltip: item.tooltip }))
      .join('');
    const stepper = changes ? C.ChangeStepper(changes) : '';
    const separator = '<div class="separator"></div>';
    const unity = kind === 'objects' && mode !== 'scene';
    const tools =
      kind === 'text' || kind === 'conflict' || unity
        ? C.Button({ icon: 'ellipsis', variant: 'ghost', selected: menuOpen, tooltip: Texts.viewOptions })
        : C.ZoomControl(zoom || {}) + (kind === 'image' ? C.BackdropButton() : '');
    const menu = !menuOpen ? '' : unity ? C.ViewOptionsMenu({ hot, items: UnityOptions, heading: 'Every Unity comparison', checked: ['noise'] }) : C.ViewOptionsMenu({ hot });
    const note = hidden ? `<span class="hidden-note xs muted" title="Hide what Unity changes on its own, in ⋯">${icon('eye-off', 'xs')}${esc(Texts.unityHidden(hidden))}</span>` : '';
    const picker = shown ? `${separator}${C.VersionPicker({ shown })}` : '';
    return `<div data-c="ViewBar" class="view-bar"><div class="modes">${modes}</div>${picker}<div class="fill"></div>${note}${note ? separator : ''}${stepper}${stepper ? separator : ''}${tools}${menu}</div>`;
  },

  ChangeRuler({ hunks, lines, shown: [top, bottom] = [] }) {
    const marks = changeGroups(hunks).map((group) => {
      const color = group.added && group.deleted ? 'var(--yellow)' : group.added ? 'var(--green)' : 'var(--red)';
      const size = Math.max(group.end - group.start + 1, 1);
      return `<span class="mark" style="top:${percentOf(group.start - 1, lines)};height:${percentOf(size, lines)};--c:${color}"></span>`;
    });
    const first = top ?? newSpan(hunks[0])[0];
    const last = bottom ?? newSpan(hunks[hunks.length - 1])[1];
    const shown = `<span class="view" style="top:${percentOf(first - 1, lines)};height:${percentOf(last - first + 1, lines)}"></span>`;
    return `<div data-c="ChangeRuler" class="change-ruler" title="Where the changes are in the whole file">${marks.join('')}${shown}</div>`;
  },

  VersionTag({ side, from, hearing = false, align = 'left', label = VersionLabels[side] }) {
    const ear = hearing ? `<span class="ear">${icon('headphones', 'xs')}</span>` : '';
    return `<div data-c="VersionTag" class="version-tag${align === 'right' ? ' right' : ''}"><span class="version-dot ${side}"></span><span>${esc(label)}</span>${from ? `<span class="source">${esc(from)}</span>` : ''}${ear}</div>`;
  },

  MediaDetails({ details, sources, sides = ['before', 'after'], open = true, labels = VersionLabels, title = 'Details', note = '' }) {
    const { columns, before, after, deltas = {} } = details;
    const values = { before, after };
    const extra = note ? `<span class="xs muted" style="font-weight:400">${esc(note)}</span>` : '';
    const head = `<tr><th class="who"><span class="h" style="gap:6px">${twisty(open)}${esc(title)}${extra}</span></th>${columns.map((name) => `<th>${esc(name)}</th>`).join('')}</tr>`;
    const row = (side) => {
      const cells = values[side].map((value, ix) => {
        const changed = side === 'after' && sides.length === 2 && value !== before[ix];
        const delta = changed && deltas[ix] ? `<span class="delta">${esc(deltas[ix])}</span>` : '';
        return `<td class="${changed ? 'changed' : ''}">${esc(value)}${delta}</td>`;
      });
      return `<tr><td class="who"><span class="h" style="gap:6px"><span class="version-dot ${side}"></span>${esc(labels[side])}<span class="source">${esc(sources[side === 'before' ? 0 : 1])}</span></span></td>${cells.join('')}</tr>`;
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
    const labels = diff.labels || VersionLabels;
    const tag = (side, align) => C.VersionTag({ side, from: sources[side === 'before' ? 0 : 1], align, label: labels[side] });
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
    const imports = diff.imports ? C.MediaDetails({ details: diff.imports, sources, sides, labels, title: 'Import settings', note: `${diff.imports.file} · ${diff.imports.same} unchanged` }) : '';
    return `<div data-c="ImageCompare" class="media-compare">${canvas}${C.MediaDetails({ details: diff.details, sources, sides, labels })}${imports}</div>`;
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

  ResizeHandle({ state = 'idle' } = {}) {
    return `<div data-c="ResizeHandle" class="resize-handle ${state}"><span class="pill"></span></div>`;
  },

  HierarchyTree({ tree, selected }) {
    const rows = tree.map((node) => {
      const spacer = `<span class="none" style="width:${node.depth * 16}px"></span>`;
      const fold = twisty(node.open === undefined ? null : node.open);
      const glyph = icon('box', 'sm', node.prefab ? 'var(--blue)' : 'var(--muted)');
      const badge = node.state ? C.ChangeBadge({ kind: UnityStates[node.state] }) : '';
      const content = `${spacer}${fold}${glyph}<span class="fill ellipsis sm name">${esc(node.name)}</span>${badge}`;
      return C.ListRow({ content, selected: node.name === selected, extraClass: `hierarchy-row ${node.state || 'same'}`, name: 'HierarchyRow', base: 'ListRow' });
    });
    const tally = ['added', 'removed', 'changed']
      .map((state) => [state, tree.filter((node) => node.state === state).length])
      .filter(([, n]) => n)
      .map(([state, n]) => `${n} ${state}`)
      .join(' · ');
    const head = `<div class="pane-head"><span class="sm medium fill">Hierarchy</span>${C.Button({ label: 'Changed only', icon: 'list-filter', variant: 'ghost', tooltip: 'Hide the objects that did not change' })}</div>`;
    return `<div data-c="HierarchyTree" class="hierarchy">${head}<div class="fill">${rows.join('')}</div><div class="pane-foot xs muted">${tally}</div></div>`;
  },

  InspectorCard({ component, compact = false }) {
    const { name, script, overrides, props } = component;
    const cell = (side, value, other) =>
      value === undefined ? '<div class="cell missing"></div>' : `<div class="cell ${side}">${unityValue(value, other)}</div>`;
    const row = ([label, before, after, children = []], depth = 0) => {
      const state = before === undefined ? 'added' : after === undefined ? 'removed' : 'changed';
      const own = `<div class="prop-row ${state}"><div class="prop" style="padding-left:${(compact ? 12 : 28) + depth * 16}px">${esc(label)}</div>${cell('before', before, after)}${cell('after', after, before)}</div>`;
      return own + children.map((child) => row(child, depth + 1)).join('');
    };
    const glyph = script ? icon('file-code', 'sm', 'var(--muted)') : '';
    const kind = script ? `<span class="xs muted">(Script)</span>` : '';
    const override = overrides ? `<span class="override-chip" title="Set on this instance, not in the prefab">Override</span>` : '';
    const head = `<div class="card-head${overrides ? ' overridden' : ''}">${glyph}<span class="sm medium">${esc(name)}</span>${kind}${override}</div>`;
    return `<div data-c="InspectorCard" class="inspector-card${compact ? ' compact' : ''}">${head}${props.map((prop) => row(prop)).join('')}</div>`;
  },

  ObjectSection({ object }) {
    const { name, parent, state, prefab, components, folded = false, type, shader } = object;
    const glyph = icon(type ? (type === 'Material' ? 'circle-dot' : 'file-box') : 'box', 'sm', prefab ? 'var(--blue)' : 'var(--muted)');
    const where = parent ? `<span class="xs muted">in ${esc(parent)}</span>` : '';
    const what = prefab ? (prefab.startsWith('Root') ? prefab : `Instance of ${prefab}`) : type ? [type, shader].filter(Boolean).join(' · ') : '';
    const changed = components.filter((component) => component.props.length).length;
    const summary = folded
      ? `${counted(components.length, 'component', 'components')}: ${components.map((component) => component.name).join(', ')}`
      : type
        ? ''
        : state === 'changed'
        ? counted(changed, 'component changed', 'components changed')
          : counted(components.length, 'component', 'components');
    const head =
      `<div class="object-head">${twisty(!folded)}${C.ChangeBadge({ kind: UnityStates[state] })}${glyph}<span class="sm medium name">${esc(name)}</span>${where}` +
      `${what ? `<span class="xs muted">${esc(what)}</span>` : ''}<span class="fill"></span><span class="xs muted">${esc(summary)}</span></div>`;
    const cards = folded ? '' : components.map((component) => C.InspectorCard({ component })).join('');
    return `<div data-c="ObjectSection" class="object-section ${state}">${head}${cards}</div>`;
  },

  InspectorSides({ sources, handle }) {
    const edge = handle === undefined ? '' : `<div class="prop-edge">${C.ResizeHandle({ state: handle })}</div>`;
    return `<div data-c="InspectorSides" class="inspector-sides"><div class="prop xs muted">Property</div><div>${C.VersionTag({ side: 'before', from: sources[0] })}</div><div>${C.VersionTag({ side: 'after', from: sources[1] })}</div>${edge}</div>`;
  },

  GuidWarning({ guid }) {
    const glyphOf = (file) => (file.endsWith('.unity') ? 'map' : file.endsWith('.prefab') ? 'box' : 'file-box');
    const users = guid.users
      .map(([path, where]) => {
        const [file, folder] = splitPath(path);
        return `<div class="guid-user" title="${esc(path)}">${icon(glyphOf(file), 'xs', 'var(--muted)')}<span class="sm">${esc(file)}</span><span class="xs muted">${esc(folder)}</span><span class="xs where ellipsis">${esc(where)}</span></div>`;
      })
      .join('');
    const actions =
      C.Button({ label: Texts.keepOldGuid, variant: 'primary', size: 'sm', tooltip: 'Write the old GUID back into the .meta file' }) +
      C.Button({ label: Texts.moveToNewGuid(guid.users.length), size: 'sm', tooltip: 'Replace the old GUID with the new one in those files' });
    return (
      `<div data-c="GuidWarning" class="guid-warning">` +
      `<div class="title">${icon('triangle-alert', 'sm')}<span class="sm medium">${esc(Texts.guidTitle(guid.users.length, guid.asset))}</span></div>` +
      `<div class="xs muted">${esc(Texts.guidAbout)}</div><div class="users">${users}</div><div class="actions">${actions}</div></div>`
    );
  },

  ObjectCompare({ diff, hierarchy = {}, property = {} }) {
    const { width = 232, handle } = hierarchy;
    const { width: propWidth = 176, handle: propHandle = 'idle' } = property;
    const tree = diff.tree
      ? `<div class="hierarchy-pane" style="width:${width}px">${C.HierarchyTree({ tree: diff.tree, selected: diff.selected })}</div>${C.ResizeHandle({ state: handle })}`
      : '';
    const warning = diff.guid ? C.GuidWarning({ guid: diff.guid }) : '';
    const sections = diff.objects.map((object) => C.ObjectSection({ object })).join('');
    return `<div data-c="ObjectCompare" class="object-compare">${tree}<div class="inspector" style="--prop-w:${propWidth}px">${warning}${C.InspectorSides({ sources: diff.sources, handle: propHandle })}<div class="inspector-scroll">${sections}</div></div></div>`;
  },

  SceneLayout({ diff, width = 940, height = 500 }) {
    const { layout, sprites, objects } = diff;
    const { unit, origin, camera, ground, gizmos = [], grid = 1 } = layout;
    const px = ([x, y]) => [(x - origin[0]) * unit, (origin[1] - y) * unit];
    const major = grid < 1 ? 1 : 5;
    const kindOf = (value) => (Math.abs(value) < 1e-6 ? 'axis' : Math.abs(value / major - Math.round(value / major)) < 1e-6 ? 'major' : 'minor');
    const lines = [];
    for (let x = Math.ceil(origin[0] / grid) * grid; x <= origin[0] + width / unit; x += grid) {
      const [left] = px([x, 0]);
      lines.push(`<line x1="${left}" y1="0" x2="${left}" y2="${height}" class="${kindOf(x)}"/>`);
    }
    for (let y = Math.floor(origin[1] / grid) * grid; y >= origin[1] - height / unit; y -= grid) {
      const [, top] = px([0, y]);
      lines.push(`<line x1="0" y1="${top}" x2="${width}" y2="${top}" class="${kindOf(y)}"/>`);
    }
    const floor = (() => {
      if (!ground) return '';
      const [left, top] = px([ground.from, ground.top]);
      const [right, bottom] = px([ground.to, ground.bottom]);
      const tiles = [];
      for (let x = ground.from; x < ground.to; x++) tiles.push(`<line x1="${px([x, 0])[0]}" y1="${top}" x2="${px([x, 0])[0]}" y2="${top + unit}"/>`);
      return `<rect x="${left}" y="${top}" width="${right - left}" height="${bottom - top}" class="ground"/><g class="tiles">${tiles.join('')}</g><line x1="${left}" y1="${top}" x2="${right}" y2="${top}" class="ground-edge"/>`;
    })();
    const frame = (size, side) => {
      const [left, top] = px([camera.at[0] - size * camera.aspect, camera.at[1] + size]);
      return `<rect x="${left}" y="${top}" width="${size * camera.aspect * 2 * unit}" height="${size * 2 * unit}" class="camera ${side}"/>`;
    };
    const frames = camera ? frame(camera.before, 'before') + frame(camera.after, 'after') : '';
    const drawn = (item, old) => {
      const [cx, cy] = px(old ? item.from : item.at);
      if (item.art === 'light') {
        return { svg: `<g transform="translate(${cx - 15} ${cy - 15})"${old ? ' class="ghost"' : ''}><svg width="30" height="30" viewBox="0 0 24 24" fill="none" stroke="#fde68a" stroke-width="2" stroke-linecap="round">${IconPaths.sun}</svg></g>`, box: [cx - 15, cy - 15, 30, 30] };
      }
      const sprite = sprites[item.art];
      const rows = old && sprite.old ? sprite.old : sprite.rows;
      const { scale } = sprite;
      const left = cx - (rows[0].length * scale) / 2;
      const top = cy - (rows.length * scale) / 2;
      const [x0, y0, x1, y1] = artBounds(rows);
      return {
        svg: `<g transform="translate(${left} ${top})"${old ? ' class="ghost"' : ''}>${pixelArt(rows, sprite.palette, scale)}</g>`,
        box: [left + x0 * scale, top + y0 * scale, (x1 - x0 + 1) * scale, (y1 - y0 + 1) * scale],
      };
    };
    const ring = ([x, y, w, h], kind, pad = 5) => `<rect x="${x - pad}" y="${y - pad}" width="${w + pad * 2}" height="${h + pad * 2}" rx="${pad - 1}" class="ring ${kind}"/>`;
    const center = ([x, y, w, h]) => [x + w / 2, y + h / 2];
    const art = [];
    const marks = [];
    const labels = [];
    const tag = (item) => `${item.state ? C.ChangeBadge({ kind: UnityStates[item.state] }) : ''}<span>${esc(item.name)}</span>${item.note ? `<span class="muted">${esc(item.note)}</span>` : ''}`;
    const label = (item, [x, y, w, h]) => {
      const side = item.label || 'below';
      const place = side === 'right' ? `left:${x + w + 14}px;top:${y + h / 2 - 11}px` : side === 'left' ? `left:${x - 14}px;top:${y + h / 2 - 11}px` : `left:${x + w / 2}px;top:${y + h + 10}px`;
      labels.push(`<div class="scene-label ${side}${item.state ? '' : ' quiet'}" style="${place}">${tag(item)}</div>`);
    };
    layout.items.forEach((item) => {
      if (item.art === 'camera') {
        const [left, top] = px([camera.at[0] - camera.after * camera.aspect, camera.at[1] + camera.after]);
        labels.push(`<div class="scene-label corner" style="left:${left + 8}px;top:${top + 8}px">${C.ChangeBadge({ kind: 'M' })}${icon('video', 'xs')}<span>${esc(item.name)}</span><span class="muted">${esc(item.note)}</span></div>`);
        return;
      }
      const then = item.from && drawn(item, true);
      const now = item.at && drawn(item, false);
      if (then) {
        art.push(then.svg);
        marks.push(ring(then.box, 'before'));
        const moved = now && (item.from[0] !== item.at[0] || item.from[1] !== item.at[1]);
        if (moved) {
          const [x1, y1] = center(then.box);
          const [x2, y2] = center(now.box);
          const length = Math.hypot(x2 - x1, y2 - y1);
          const [ux, uy] = [(x2 - x1) / length, (y2 - y1) / length];
          const start = Math.max(then.box[2], then.box[3]) / 2 + 8;
          const end = Math.max(now.box[2], now.box[3]) / 2 + 12;
          marks.push(`<line x1="${x1 + ux * start}" y1="${y1 + uy * start}" x2="${x2 - ux * end}" y2="${y2 - uy * end}" class="moved" marker-end="url(#scene-arrow)"/>`);
        }
        if (!now) {
          const [cx, cy] = center(then.box);
          marks.push(`<path d="M${cx - 6} ${cy - 6} L${cx + 6} ${cy + 6} M${cx + 6} ${cy - 6} L${cx - 6} ${cy + 6}" class="cross"/>`);
          label(item, then.box);
        }
      }
      if (now) {
        art.push(now.svg);
        if (item.state) marks.push(ring(now.box, item.state));
        if (item.name === layout.picked) marks.push(ring(now.box, 'picked', 9));
        if (!item.silent) label(item, now.box);
      }
    });
    gizmos.forEach((gizmo) => {
      const [cx, cy] = px(gizmo.at);
      const r = gizmo.after * unit;
      marks.push(`<circle cx="${cx}" cy="${cy}" r="${gizmo.before * unit}" class="gizmo before"/><circle cx="${cx}" cy="${cy}" r="${r}" class="gizmo after"/>`);
      labels.push(`<div class="scene-label right" style="left:${cx + r + 12}px;top:${cy - 11}px">${C.ChangeBadge({ kind: 'M' })}<span>${esc(gizmo.name)}</span><span class="muted">${esc(gizmo.note)}</span></div>`);
    });
    const defs = `<defs><marker id="scene-arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0 0 L10 5 L0 10 z" fill="var(--yellow)"/></marker></defs>`;
    const svg = `<svg width="${width}" height="${height}" class="scene-svg">${defs}<g class="grid">${lines.join('')}</g>${floor}${frames}${art.join('')}${marks.join('')}</svg>`;
    const legend = `<div class="scene-legend">${C.VersionTag({ side: 'before', from: 'HEAD · dashed' })}${C.VersionTag({ side: 'after', from: 'Working copy' })}<span class="xs muted">${esc(layout.view)}</span></div>`;
    const picked = objects.find((object) => object.name === layout.picked);
    const pick = picked ? `<div class="scene-pick">${C.InspectorSides({ sources: diff.sources })}${C.ObjectSection({ object: picked })}</div>` : '';
    return `<div data-c="SceneLayout" class="media-compare"><div class="compare-canvas scene-canvas"><div class="scene-stage" style="width:${width}px;height:${height}px">${svg}${labels.join('')}</div>${legend}</div>${pick}</div>`;
  },

  CanvasLayout({ diff, width = 940, height = 500 }) {
    const { layout, objects } = diff;
    const { canvas } = layout;
    const [canvasWidth, canvasHeight] = canvas.size;
    const scale = Math.min((width - 80) / canvasWidth, (height - 70) / canvasHeight);
    const left = (width - canvasWidth * scale) / 2;
    const top = 40;
    const px = ([x, y]) => [left + x * scale, top + (canvasHeight - y) * scale];
    const box = ([x, y, w, h]) => {
      const [bx, by] = px([x, y + h]);
      return [bx, by, w * scale, h * scale];
    };
    const lines = [];
    for (let x = -400; x <= canvasWidth + 400; x += 100) {
      const [at] = px([x, 0]);
      if (at >= 0 && at <= width) lines.push(`<line x1="${at}" y1="0" x2="${at}" y2="${height}" class="${x === 0 ? 'axis' : x % 500 ? 'minor' : 'major'}"/>`);
    }
    for (let y = -400; y <= canvasHeight + 400; y += 100) {
      const [, at] = px([0, y]);
      if (at >= 0 && at <= height) lines.push(`<line x1="0" y1="${at}" x2="${width}" y2="${at}" class="${y === 0 ? 'axis' : y % 500 ? 'minor' : 'major'}"/>`);
    }
    const graphic = (item, rect, old) => {
      const [x, y, w, h] = box(rect);
      const fill = item.fill ? `<rect x="${x}" y="${y}" width="${w}" height="${h}" rx="${(item.radius || 0) * scale}" fill="${item.fill}"/>` : '';
      const text = item.text
        ? (() => {
            const size = (old && item.text.oldSize ? item.text.oldSize : item.text.size) * scale;
            const align = item.text.align || 'middle';
            const tx = align === 'start' ? x + 4 : align === 'end' ? x + w - 4 : x + w / 2;
            return `<text x="${tx}" y="${y + h / 2}" font-size="${size}" fill="${item.text.color}" text-anchor="${align}" dominant-baseline="central" font-weight="${item.text.bold ? 700 : 500}">${esc(old && item.text.old ? item.text.old : item.text.value)}</text>`;
          })()
        : '';
      return `<g${old ? ' class="ghost"' : ''}>${fill}${text}</g>`;
    };
    const ring = ([x, y, w, h], kind, pad = 4) => `<rect x="${x - pad}" y="${y - pad}" width="${w + pad * 2}" height="${h + pad * 2}" rx="${pad - 1}" class="ring ${kind}"/>`;
    const art = [];
    const marks = [];
    const labels = [];
    const tag = (item) => `${C.ChangeBadge({ kind: UnityStates[item.state] })}<span>${esc(item.name)}</span>${item.note ? `<span class="muted">${esc(item.note)}</span>` : ''}`;
    const label = (item, [x, y, w, h]) => {
      const side = item.label || 'below';
      const place =
        side === 'right' ? `left:${x + w + 12}px;top:${y + h / 2 - 11}px` : side === 'left' ? `left:${x - 12}px;top:${y + h / 2 - 11}px` : side === 'above' ? `left:${x + w / 2}px;top:${y - 32}px` : `left:${x + w / 2}px;top:${y + h + 10}px`;
      labels.push(`<div class="scene-label ${side === 'above' ? 'below' : side}" style="${place}">${tag(item)}</div>`);
    };
    layout.items.forEach((item) => {
      const changed = item.state && item.state !== 'added';
      if (item.from && changed && item.from.join() !== (item.rect || []).join()) {
        art.push(graphic(item, item.from, true));
        marks.push(ring(box(item.from), 'before'));
      }
      if (item.rect) art.push(graphic(item, item.rect, false));
      if (!item.state) return;
      if (item.rect) {
        marks.push(ring(box(item.rect), item.state));
        if (item.name === layout.picked) marks.push(ring(box(item.rect), 'picked', 8));
        label(item, box(item.rect));
      } else {
        const [x, y, w, h] = box(item.from);
        const [cx, cy] = [x + w / 2, y + h / 2];
        marks.push(`<path d="M${cx - 6} ${cy - 6} L${cx + 6} ${cy + 6} M${cx + 6} ${cy - 6} L${cx - 6} ${cy + 6}" class="cross"/>`);
        label(item, box(item.from));
      }
    });
    const [frameX, frameY] = px([0, canvasHeight]);
    const frame = `<rect x="${frameX}" y="${frameY}" width="${canvasWidth * scale}" height="${canvasHeight * scale}" class="canvas-frame"/>`;
    const name = `<div class="canvas-name xs" style="left:${frameX}px;top:${frameY - 20}px">${icon('box', 'xs')}<span>${esc(canvas.name)}</span><span class="muted">${esc(canvas.note)}</span></div>`;
    const svg = `<svg width="${width}" height="${height}" class="scene-svg"><g class="grid">${lines.join('')}</g>${art.join('')}${frame}${marks.join('')}</svg>`;
    const legend = `<div class="scene-legend">${C.VersionTag({ side: 'before', from: 'HEAD · dashed' })}${C.VersionTag({ side: 'after', from: 'Working copy' })}<span class="xs muted">${esc(layout.view)}</span></div>`;
    const picked = objects.find((object) => object.name === layout.picked);
    const pick = picked ? `<div class="scene-pick">${C.InspectorSides({ sources: diff.sources })}${C.ObjectSection({ object: picked })}</div>` : '';
    return `<div data-c="CanvasLayout" class="media-compare"><div class="compare-canvas scene-canvas"><div class="scene-stage" style="width:${width}px;height:${height}px">${svg}${name}${labels.join('')}</div>${legend}</div>${pick}</div>`;
  },

  CurveCompare({ diff, plot = 704 }) {
    const { tracks, events, length, at, sources } = diff;
    const frames = 40;
    const x = (frame) => (frame / frames) * plot;
    const time = (frame) => `0:${String(frame).padStart(2, '0')}`;
    const ticks = [];
    for (let frame = 0; frame <= frames; frame += 5) ticks.push(`<span class="tick${frame % 10 ? '' : ' major'}" style="left:${x(frame)}px">${frame % 10 ? '' : time(frame)}</span>`);
    const ends = ['before', 'after'].map((side) => `<span class="end ${side}" style="left:${x(length[side])}px" title="${VersionLabels[side]} ends at ${time(length[side])}"></span>`).join('');
    const ruler = `<div class="curve-row ruler"><div class="lane-name xs muted">${esc(diff.fps)} samples a second</div><div class="lane-plot" style="width:${plot}px">${ticks.join('')}${ends}</div></div>`;
    const diamond = (cx, cy, kind) => `<rect x="${cx - 4}" y="${cy - 4}" width="8" height="8" transform="rotate(45 ${cx} ${cy})" class="key ${kind}"/>`;
    const name = (track) => `<div class="lane-name">${C.ChangeBadge({ kind: UnityStates[track.state] })}<div class="v"><span class="xs muted ellipsis">${esc(track.group)}</span><span class="sm${track.state === 'removed' ? ' struck' : ''}">${esc(track.name)}</span></div></div>`;
    const curveLane = (track) => {
      const height = track.name === 'Position.y' ? 96 : 68;
      const [lo, hi] = track.range;
      const y = (value) => 8 + (1 - (value - lo) / (hi - lo)) * (height - 16);
      const sameKeys = new Set((track.before || []).filter(([f, val]) => (track.after || []).some(([g, w]) => g === f && w === val)).map(([f, val]) => `${f}:${val}`));
      const side = (kind) => {
        const keys = track[kind];
        if (!keys) return '';
        return `<path d="${easedCurve(keys, x, y)}" class="curve ${kind}"/>` + keys.map(([f, val]) => diamond(x(f), y(val), sameKeys.has(`${f}:${val}`) ? 'same' : kind)).join('');
      };
      const scale = `<text x="4" y="12" class="scale">${esc(hi)}</text><text x="4" y="${height - 4}" class="scale">${esc(lo)}</text>`;
      return `<div class="curve-row ${track.state}">${name(track)}<div class="lane-plot" style="width:${plot}px"><svg width="${plot}" height="${height}">${scale}${side('before')}${side('after')}</svg></div></div>`;
    };
    const spriteLane = (track) => {
      const art = (kind, stretch) => `<span class="sprite-key"><span class="sprite" style="transform:scaleY(${stretch})">${pixelArt(kind === 'before' ? diff.sprite.old : diff.sprite.rows, diff.sprite.palette, 1)}</span></span>`;
      const row = (kind) => `<div class="sprite-track ${kind}">${track.sprites[kind].map(([f, stretch]) => `<span class="sprite-at" style="left:${x(f)}px">${art(kind, stretch)}</span>`).join('')}</div>`;
      return `<div class="curve-row ${track.state}">${name(track)}<div class="lane-plot sprites" style="width:${plot}px">${row('before')}${row('after')}</div></div>`;
    };
    const lanes = tracks.map((track) => (track.sprites ? spriteLane(track) : curveLane(track))).join('');
    const pins = (kind) => events[kind].map(([f, label], ix) => `<span class="event-pin ${kind}" style="left:${x(f)}px;--row:${ix}"><span class="flag"></span><span class="xs">${esc(label)}</span></span>`).join('');
    const eventLane = `<div class="curve-row changed"><div class="lane-name">${C.ChangeBadge({ kind: 'M' })}<div class="v"><span class="xs muted">Animation events</span><span class="sm">Events</span></div></div><div class="lane-plot events" style="width:${plot}px"><div class="event-track before">${pins('before')}</div><div class="event-track after">${pins('after')}</div></div></div>`;
    const sides = `<div class="curve-sides">${C.VersionTag({ side: 'before', from: `${sources[0]} · dashed` })}${C.VersionTag({ side: 'after', from: sources[1] })}<span class="xs muted">A diamond is a key: white when both versions have it</span></div>`;
    return (
      `<div data-c="CurveCompare" class="media-compare">` +
      `<div class="curve-area">${sides}<div class="curve-lanes">${ruler}${lanes}${eventLane}</div></div>` +
      C.MediaDetails({ details: diff.details, sources }) +
      `</div>`
    );
  },

  GraphParameters({ title, rows, corner = 'left' }) {
    const lines = rows
      .map(([name, type, state]) => `<div class="param${state ? ` ${state}` : ''}">${state ? C.ChangeBadge({ kind: UnityStates[state] }) : '<span class="badge-space"></span>'}<span class="fill">${esc(name)}</span><span class="muted">${esc(type)}</span></div>`)
      .join('');
    return `<div data-c="GraphParameters" class="graph-params ${corner}"><div class="xs medium">${esc(title)}</div>${lines}</div>`;
  },

  GraphChanges({ diff, hidden = null }) {
    const rows = diff.changes
      .map(([state, kind, name, detail], ix) =>
        C.ListRow({
          content: `${C.ChangeBadge({ kind: UnityStates[state] })}<span class="xs muted kind">${esc(kind)}</span><span class="sm fill ellipsis">${esc(name)}</span><span class="xs muted">${esc(detail)}</span>`,
          selected: ix === diff.current,
          extraClass: `graph-change${state === hidden ? ' elsewhere' : ''}`,
          name: 'GraphChangeRow',
          base: 'ListRow',
        }),
      )
      .join('');
    const card = diff.picked.components.map((component) => C.InspectorCard({ component, compact: true })).join('');
    return (
      `<div data-c="GraphChanges" class="graph-side"><div class="pane-head"><span class="sm medium fill">Changes</span><span class="xs muted">${diff.changes.length}</span></div><div class="graph-changes">${rows}</div>` +
      `<div class="pane-head"><span class="sm medium fill">${esc(diff.picked.name)}</span><span class="xs muted">${esc(diff.picked.type)}</span></div>${C.InspectorSides({ sources: diff.sources })}${card}</div>`
    );
  },

  AnimatorCompare({ diff, width = 600, shown = 'both' }) {
    const nodeWidth = 112;
    const nodeHeight = 34;
    const hidden = shown === 'before' ? 'added' : shown === 'after' ? 'removed' : null;
    const states = Object.fromEntries(diff.states.map((state) => [state.id, state]));
    const center = (state) => [state.at[0] + nodeWidth / 2, state.at[1] + nodeHeight / 2];
    const picked = diff.picked.name;
    const nameOf = (id) => states[id].name;
    const arrows = diff.transitions
      .filter(([from, to, state]) => state !== hidden && states[from].state !== hidden && states[to].state !== hidden)
      .map(([from, to, state = '']) => {
        const [x1, y1] = center(states[from]);
        const [x2, y2] = center(states[to]);
        const angle = (Math.atan2(y2 - y1, x2 - x1) * 180) / Math.PI;
        const [mx, my] = [(x1 + x2) / 2, (y1 + y2) / 2];
        const hot = `${nameOf(from)} → ${nameOf(to)}` === picked ? ' picked' : '';
        return `<g class="transition ${state}${hot}"><line x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}"/><path d="M-6 -5 L6 0 L-6 5 z" transform="translate(${mx} ${my}) rotate(${angle})"/></g>`;
      })
      .join('');
    const nodes = diff.states
      .filter((state) => state.state !== hidden)
      .map((state) => {
        const badge = state.state ? C.ChangeBadge({ kind: UnityStates[state.state] }) : '';
        return `<div class="state-node ${state.role || 'plain'} ${state.state || ''}" style="left:${state.at[0]}px;top:${state.at[1]}px;width:${nodeWidth}px;height:${nodeHeight}px">${badge}<span class="sm">${esc(state.name)}</span></div>`;
      })
      .join('');
    const canvas =
      `<div class="graph-canvas${hidden ? ' solo' : ''}" style="width:${width}px">` +
      `<svg class="graph-lines" width="${width}" height="560">${arrows}</svg>${nodes}` +
      `<div class="graph-layer">${C.Select({ value: 'Base Layer', lead: icon('workflow', 'sm', 'var(--muted)') })}</div>` +
      C.GraphParameters({ title: 'Parameters', rows: diff.parameters.filter(([, , state]) => state !== hidden) }) +
      `</div>`;
    return `<div data-c="AnimatorCompare" class="animator-compare">${canvas}${C.GraphChanges({ diff, hidden })}</div>`;
  },

  ScriptGraphCompare({ diff, width = 600, shown = 'both' }) {
    const header = 26;
    const row = 20;
    const hidden = shown === 'before' ? 'added' : shown === 'after' ? 'removed' : null;
    const nodes = Object.fromEntries(diff.nodes.map((node) => [node.id, node]));
    const portY = (node, ix) => node.at[1] + header + ix * row + row / 2;
    const shownWires = diff.wires.filter(([from, , to, , state]) => state !== hidden && nodes[from].state !== hidden && nodes[to].state !== hidden);
    const wires = shownWires
      .map(([from, out, to, input, state = '']) => {
        const a = nodes[from];
        const b = nodes[to];
        const port = a.rows[out][1];
        const [x1, y1, x2, y2] = [a.at[0] + a.width, portY(a, out), b.at[0], portY(b, input)];
        const bend = x2 > x1 ? Math.max(30, (x2 - x1) / 2) : 26;
        const color = port.flow ? '#e5e5e5' : GraphTypes[port.type];
        return `<path d="M${x1} ${y1} C${x1 + bend} ${y1} ${x2 - bend} ${y2} ${x2} ${y2}" class="wire ${port.flow ? 'flow' : 'value'} ${state}" style="--c:${color}"/>`;
      })
      .join('');
    const fed = new Set(shownWires.filter((wire) => hidden || wire[4] !== 'removed').map(([, , to, input]) => `${to}:${input}`));
    const port = (spec, side, node, ix) => {
      if (!spec) return '<span class="slot"></span>';
      const glyph = spec.field ? '' : spec.flow ? `<span class="port flow ${side}"></span>` : `<span class="port value ${side}" style="--c:${GraphTypes[spec.type]}"></span>`;
      const changed = spec.was !== undefined;
      const value = shown === 'before' && changed ? spec.was : spec.value;
      const literal =
        side === 'in' && value !== undefined && !fed.has(`${node.id}:${ix}`)
          ? `<span class="literal${changed ? ' changed' : ''}"${changed && shown === 'both' ? ` title="Before: ${esc(spec.was)}"` : ''}>${esc(value)}</span>`
          : '';
      const name = spec.label ? `<span class="${spec.field ? 'field' : 'muted'}">${esc(spec.label)}</span>` : '';
      return `<span class="slot ${side}">${side === 'in' ? glyph + name + literal : name + glyph}</span>`;
    };
    const units = diff.nodes
      .filter((node) => node.state !== hidden)
      .map((node) => {
        const badge = node.state ? C.ChangeBadge({ kind: UnityStates[node.state] }) : '';
        const rows = node.rows.map(([input, output], ix) => `<div class="unit-row">${port(input, 'in', node, ix)}${port(output, 'out', node, ix)}</div>`).join('');
        const classes = ['unit', node.kind, node.state, node.id === diff.picked.id && 'picked'].filter(Boolean).join(' ');
        return `<div class="${classes}" style="left:${node.at[0]}px;top:${node.at[1]}px;width:${node.width}px"><div class="unit-head">${icon(node.icon, 'xs')}<span class="xs medium fill ellipsis">${esc(node.title)}</span>${badge}</div>${rows}</div>`;
      })
      .join('');
    const canvas =
      `<div class="graph-canvas script${hidden ? ' solo' : ''}" style="width:${width}px">` +
      `<svg class="graph-lines" width="${width}" height="640">${wires}</svg>${units}` +
      `<div class="graph-layer">${C.Select({ value: diff.graph, lead: icon('workflow', 'sm', 'var(--muted)') })}</div>` +
      C.GraphParameters({ title: 'Graph variables', rows: diff.variables.filter(([, , state]) => state !== hidden), corner: 'right' }) +
      `</div>`;
    return `<div data-c="ScriptGraphCompare" class="animator-compare">${canvas}${C.GraphChanges({ diff, hidden })}</div>`;
  },

  FileDiffPanel({ diff, mode = 'split', state = 'ready', writing = false, reason = '', selection, pointed, expanded = false, filePosition, menuOpen = false, hot, view = {}, look = {} }) {
    const kind = diff.kind || 'text';
    const modes = diff.unity ? UnityModes[diff.unity] : kind;
    const labels = diff.commit ? null : SideLabels[diff.side];
    const ready = state === 'ready';
    const total =
      kind === 'text'
        ? changeGroups(diff.hunks).length
        : kind === 'conflict'
          ? diff.parts.filter((part) => part.conflict).length
          : kind === 'objects'
            ? unityChangeCount(diff.objects)
            : kind === 'curves'
              ? diff.tracks.length + 1
              : (diff.changes || diff.image?.areas || []).length;
    const noun = kind === 'conflict' ? 'Conflict' : 'Change';
    const zoom = kind === 'image' ? { percent: (mode === 'split' && !view.sides ? 12 : 18) * 100 } : kind === 'video' ? { percent: 36 } : ['curves', 'animator', 'scriptgraph'].includes(kind) || mode === 'scene' ? { percent: 100 } : {};
    const hidden = kind === 'objects' ? diff.hidden || 0 : 0;
    const viewBar =
      kind === 'binary'
        ? ''
        : C.ViewBar({ kind, modes, mode, changes: ready ? { current: (diff.current || 0) + 1, total, noun } : null, zoom, menuOpen, hot, hidden, shown: ['animator', 'scriptgraph'].includes(kind) ? view.shown || 'both' : undefined });
    const conflict = () => C.MergeEditor({ merge: diff });
    const text = () => {
      const diffView = C.DiffView({
        files: [{ path: diff.change.path, additions: diff.additions, deletions: diff.deletions, hunks: diff.hunks, lines: diff.lines, unchanged: diff.unchanged }],
        mode: mode === 'yaml' ? 'split' : mode,
        header: false,
        actions: labels && diff.change.kind === 'M' ? { side: diff.side, writing } : undefined,
        selection,
        pointed,
        menu: pointed && labels ? C.LineMenu({ side: diff.side, selected: pointed.selected, changed: pointed.changed, left: pointed.left, top: pointed.top, hot: pointed.hot }) : '',
        look: { gaps: diff.unity ? 'scope-always' : 'scope', marks: 'bars', sides: textSides(diff), scroll: true, whole: true, hints: diff.hints, ...look },
      });
      const whole = look.whole !== false && diff.unchanged && diff.hunks.length;
      const top = whole ? wholeFileTop(diff.hunks) : undefined;
      const ruler = diff.lines ? C.ChangeRuler({ hunks: diff.hunks, lines: diff.lines, shown: whole ? [top, top + ViewRows - 1] : [] }) : '';
      return `<div class="diff-area">${diffView}${ruler}</div>`;
    };
    const bodies = {
      text,
      conflict,
      image: () => C.ImageCompare({ diff, mode, ...view }),
      video: () => C.VideoCompare({ diff, mode }),
      audio: () => C.AudioCompare({ diff, mode }),
      binary: () => C.BinaryFacts({ diff }),
      objects: () => (mode !== 'scene' ? C.ObjectCompare({ diff, hierarchy: view.hierarchy, property: view.property }) : diff.layout.canvas ? C.CanvasLayout({ diff }) : C.SceneLayout({ diff })),
      curves: () => C.CurveCompare({ diff }),
      animator: () => C.AnimatorCompare({ diff, shown: view.shown }),
      scriptgraph: () => C.ScriptGraphCompare({ diff, shown: view.shown }),
    };
    const content =
      state === 'loading'
        ? C.Loading({ message: Texts.readingChanges })
        : state === 'failed'
          ? C.Failure({ title: Texts.fileFailed, reason })
          : state === 'empty'
            ? C.Hint({ message: Texts.fileEmpty })
            : bodies[kind]();
    const bars = kind === 'text' && !diff.unity ? C.FileBar({ diff, state, writing, expanded, filePosition, tools: viewBar }) : C.FileBar({ diff, state, writing, expanded, filePosition }) + viewBar;
    return `<div data-c="FileDiffPanel" class="file-diff">${bars}<div class="fill v">${content}</div></div>`;
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

  RepoView({ body, left, middle, right, status, toolbar = C.RepoToolbar(), expanded = false, operation = '' }) {
    const side = (place, content) => (expanded || content === false ? '' : `<div class="pane ${place}">${content}</div>`);
    const panes = body || `<div class="panes">${side('left', left)}<div class="pane middle">${middle}</div>${side('right', right)}</div>`;
    return `<div data-c="RepoView" class="repo-view">${toolbar}${operation}<div class="fill v">${panes}</div>${status}</div>`;
  },

  OperationBar({ operation, conflicts = 0, running }) {
    const { kind } = operation;
    const stuck = conflicts > 0;
    const glyph = stuck ? icon('triangle-alert', 'md', 'var(--yellow)') : icon('circle-check', 'md', 'var(--green)');
    const stopped = operation.commit ? `<span class="mono muted">${esc(operation.commit.id)}</span><span class="ellipsis">${esc(operation.commit.subject)}</span>` : '';
    const step = operation.total ? `<span class="muted">Commit ${operation.step} of ${operation.total}</span>${stopped}` : stopped;
    const state = stuck
      ? `<span class="conflict-count">${esc(counted(conflicts, 'conflicted file', 'conflicted files'))}</span>`
      : `<span class="resolved">${esc(OperationTexts.noConflicts)}</span>`;
    const abort = C.Button({ label: OperationTexts.abort[kind], icon: 'undo-2', size: 'sm', variant: 'ghost', disabled: !!running, loading: running === 'abort', tooltip: OperationTexts.abortTooltip[kind] });
    const go = C.Button({
      label: OperationTexts.go[kind],
      size: 'sm',
      variant: 'primary',
      disabled: stuck || !!running,
      loading: running === 'continue',
      tooltip: stuck ? OperationTexts.waiting(conflicts) : OperationTexts.goTooltip[kind],
    });
    return (
      `<div data-c="OperationBar" class="operation-bar${stuck ? ' stuck' : ''}">` +
      glyph +
      `<span class="semibold none">${esc(OperationTexts[kind](operation))}</span>` +
      `<div class="h step sm">${step}</div>` +
      state +
      abort +
      go +
      `</div>`
    );
  },

  BranchMenu({ label, current, mode, left = 0, top = 0, hot, pull }) {
    return C.Menu({ items: branchMenuItems({ label, current, mode, pull }), left, top, hot, name: 'BranchMenu' });
  },

  WorktreeMenu({ tree, here = false, left = 0, top = 0, hot }) {
    return C.Menu({ items: worktreeMenuItems({ tree, here }), left, top, hot, name: 'WorktreeMenu' });
  },

  LockWorktreeDialog({ tree, reason = '' }) {
    const field = `<div class="v field">${C.Input({ value: reason, placeholder: WorktreeTexts.lockPlaceholder, focused: true })}</div>`;
    const buttons = C.Button({ label: 'Cancel', size: 'md' }) + C.Button({ label: WorktreeTexts.lockConfirm, variant: 'primary', size: 'md' });
    const close = `<span class="close">${icon('close', 'sm')}</span>`;
    return `<div class="overlay"><div data-c="LockWorktreeDialog" data-base="Dialog" class="dialog wide">${close}<div class="base semibold">${esc(WorktreeTexts.lockTitle(tree.name))}</div><div class="sm muted">${esc(WorktreeTexts.lockAbout)}</div>${field}<div class="footer stretch">${buttons}</div></div></div>`;
  },

  WorktreeDialog({ branch, fresh = false, name = '', from = 'main', folder, parent, openTab = false, problem, folderProblem }) {
    const lead = (glyph) => `<span class="prefix">${icon(glyph, 'sm', 'var(--muted)')}</span>`;
    const radios = `<div class="h radios">${C.Radio({ label: WorktreeTexts.existing, checked: !fresh })}${C.Radio({ label: WorktreeTexts.fresh, checked: fresh })}</div>`;
    const branchFields = fresh
      ? `<div class="h" style="gap:12px;align-items:flex-start"><div class="fill v">${formField({ label: WorktreeTexts.branchName, control: C.Input({ value: name, placeholder: Texts.branchPlaceholder, focused: true }), problem })}</div><div class="v none" style="width:200px">${formField({ label: WorktreeTexts.from, control: C.Select({ value: from, lead: lead('git-branch') }) })}</div></div>`
      : formField({ label: WorktreeTexts.branch, control: C.Select({ value: branch, lead: lead('git-branch') }), problem, hint: WorktreeTexts.oneCheckout });
    const folderField = formField({ label: WorktreeTexts.folder, control: withButton(C.Input({ value: folder }), C.Button({ label: WorktreeTexts.browse, size: 'md' })), problem: folderProblem, hint: WorktreeTexts.folderHint(parent) });
    const after = `<div class="h">${C.Checkbox({ label: WorktreeTexts.openAfter, checked: openTab })}</div>`;
    const body = `<div class="form">${radios}${branchFields}${folderField}${after}</div>`;
    const ready = !problem && !folderProblem && (!fresh || name);
    const buttons = C.Button({ label: 'Cancel', size: 'md' }) + C.Button({ label: WorktreeTexts.create, icon: 'folder-plus', variant: 'primary', size: 'md', disabled: !ready });
    return dialogShell({ name: 'WorktreeDialog', title: WorktreeTexts.dialogTitle, description: WorktreeTexts.dialogAbout, body, buttons, size: 'wider' });
  },

  RemoveWorktreeDialog({ tree, merged = false, into = 'main' }) {
    const n = changedCount(tree);
    const description = [WorktreeTexts.removeFolder(tree.path), n && WorktreeTexts.removeChanges(n), tree.branch && WorktreeTexts.removeKeeps(tree.branch)].filter(Boolean).join(' ');
    const note = merged ? '' : `<div class="xs muted" style="padding-left:20px">${esc(WorktreeTexts.deleteBranchUnmerged(tree.branch, into))}</div>`;
    const box = tree.branch ? `<div class="v" style="gap:4px;margin-top:12px">${C.Checkbox({ label: WorktreeTexts.deleteBranch(tree.branch), checked: merged, disabled: !merged })}${note}</div>` : '';
    const buttons = C.Button({ label: 'Cancel', size: 'md' }) + C.Button({ label: WorktreeTexts.removeConfirm, variant: 'danger', size: 'md' });
    return `<div class="overlay"><div data-c="RemoveWorktreeDialog" data-base="AlertDialog" class="dialog wide"><div class="base semibold">${esc(WorktreeTexts.removeTitle(tree.name))}</div><div class="sm muted">${esc(description)}</div>${box}<div class="footer">${buttons}</div></div></div>`;
  },

  WorktreeBar({ tree }) {
    const where = tree.branch || WorktreeTexts.detached(tree.id);
    const buttons = C.Button({ icon: 'folder-open', variant: 'ghost', tooltip: WorktreeTexts.showTooltip }) + C.Button({ icon: 'external-link', variant: 'ghost', tooltip: WorktreeTexts.openTabTooltip });
    return `<div data-c="WorktreeBar" class="worktree-bar">${icon('folder-git', 'md', 'var(--muted)')}<div class="v fill" style="min-width:0"><div class="h" style="gap:8px"><span class="sm medium">${esc(tree.name)}</span><span class="h xs muted" style="gap:4px">${tree.branch ? icon('git-branch', 'xs') : ''}${esc(where)}</span></div><div class="xs muted ellipsis">${esc(tree.path)}</div></div>${buttons}</div>`;
  },

  WorktreeListRow({ tree, picked = false, here = false }) {
    const n = changedCount(tree);
    const fixed = tree.main || here || Boolean(tree.locked);
    const where = tree.branch ? `${icon('git-branch', 'xs')}<span class="ellipsis">${esc(tree.branch)}</span>` : `<span class="ellipsis">${esc(WorktreeTexts.detached(tree.id))}</span>`;
    const chips = join([
      tree.main && C.Chip({ label: WorktreeTexts.main }),
      here && C.Chip({ label: WorktreeTexts.thisTab, color: 'var(--blue)' }),
      tree.missing && C.Chip({ label: WorktreeTexts.missing, icon: 'triangle-alert', color: 'var(--red)' }),
      n && C.Chip({ label: WorktreeTexts.changed(n), color: 'var(--yellow)' }),
      tree.merged && C.Chip({ label: WorktreeTexts.merged('main'), icon: 'git-merge', color: 'var(--magenta)' }),
      tree.locked && C.Chip({ label: WorktreeTexts.locked, icon: 'lock' }),
    ]);
    const open = here || tree.missing ? '<span class="none" style="width:20px"></span>' : C.Button({ icon: 'external-link', variant: 'ghost', tooltip: WorktreeTexts.openTabTooltip });
    const content =
      C.Checkbox({ label: '', checked: picked, disabled: fixed }) +
      icon('folder-git', 'md', 'var(--muted)') +
      `<div class="v fill" style="min-width:0"><div class="h" style="gap:8px"><span class="sm medium">${esc(tree.name)}</span><span class="h xs muted" style="gap:4px;min-width:0">${where}</span></div><div class="xs muted ellipsis">${esc(tree.path)}</div></div>` +
      `<div class="h chips">${chips}</div>` +
      open;
    return C.ListRow({ content, selected: picked, extraClass: 'worktree-list-row', name: 'WorktreeListRow' });
  },

  WorktreesDialog({ repository, list, here, picked = [] }) {
    const rows = list.map((tree) => C.WorktreeListRow({ tree, here: tree.name === here, picked: picked.includes(tree.name) })).join('');
    const lost = list.filter((tree) => picked.includes(tree.name) && changedCount(tree)).length;
    const warn = lost ? `<div class="h xs worktree-warn">${icon('triangle-alert', 'xs')}<span>${esc(WorktreeTexts.lostWarning(lost))}</span></div>` : '';
    const tools = `<div class="h worktree-tools"><span class="fill xs muted">${esc(counted(list.length, 'worktree', 'worktrees'))}</span>${C.Button({ label: WorktreeTexts.selectMerged, size: 'sm', variant: 'ghost' })}${C.Button({ label: WorktreeTexts.forgetMissing, size: 'sm', variant: 'ghost' })}</div>`;
    const buttons =
      `<div class="h fill">${C.Button({ label: WorktreeTexts.newWorktree, icon: 'folder-plus', size: 'md' })}</div>` +
      C.Button({ label: WorktreeTexts.close, size: 'md' }) +
      C.Button({ label: WorktreeTexts.removeSelected(picked.length), icon: 'trash-2', variant: 'danger', size: 'md', disabled: !picked.length });
    const body = `${tools}<div class="v worktree-list">${rows}</div>${warn}`;
    return dialogShell({ name: 'WorktreesDialog', title: WorktreeTexts.manageTitle(repository), description: WorktreeTexts.manageAbout, body, buttons, size: 'worktrees' });
  },

  TagMenu({ label, remote, left = 0, top = 0, hot }) {
    return C.Menu({ items: tagMenuItems({ label, remote }), left, top, hot, name: 'TagMenu' });
  },

  DeleteTagDialog({ label, remote, only = 0, there = false }) {
    if (there) return C.AlertDialog({ title: Texts.deleteRemoteTitle(label.name, remote), description: Texts.deleteRemoteTagDescription(label.name, remote), confirm: 'Delete', name: 'DeleteTagDialog' });
    const lost = only === null ? Texts.deleteUncounted(label.name) : only ? Texts.deleteUnmerged(label.name, only) : Texts.deleteMerged(label.name);
    const description = [lost, remote && Texts.tagFetchedBack(label.name, remote)].filter(Boolean).join(' ');
    return C.AlertDialog({ title: Texts.deleteBranchTitle(label.name), description, confirm: 'Delete', name: 'DeleteTagDialog' });
  },

  StashMenu({ stash, left = 0, top = 0, hot }) {
    return C.Menu({ items: stashMenuItems({ stash }), left, top, hot, name: 'StashMenu' });
  },

  DeleteStashDialog({ stash }) {
    return C.AlertDialog({ title: Texts.deleteStashTitle(stash.name), description: Texts.deleteStashDescription(stash.message), confirm: 'Delete', name: 'DeleteStashDialog' });
  },

  CommitMenu({ commit, current, reachable = true, reset, picked = [], held, line, left = 0, top = 0, hot }) {
    const items = picked.length > 1 ? pickedCommitsMenuItems({ commits: picked, current, held, line }) : commitMenuItems({ commit, current, reachable, reset });
    return C.Menu({ items, left, top, hot, name: 'CommitMenu' });
  },

  SquashDialog({ commits, subject = '', description = '', focus = 'subject', after = 0, branches = [], remotes = [], tags = [] }) {
    const n = commits.length;
    const [newest, oldest] = [commits[0], commits[n - 1]];
    const lines = [
      `<div class="xs muted">${esc(Texts.squashMoves(after, branches))}</div>`,
      remotes.length ? `<div class="h xs edit-warn">${icon('triangle-alert', 'xs')}<span>${esc(Texts.squashPushed(remotes))}</span></div>` : '',
      tags.length ? `<div class="xs muted">${esc(Texts.tagsStay(tags))}</div>` : '',
    ];
    const buttons =
      C.Button({ label: 'Cancel', size: 'md' }) +
      C.Button({ label: Texts.squashButton(n), variant: 'primary', size: 'md', disabled: subject.trim() === '', tooltip: Texts.squashTooltip });
    const close = `<span class="close">${icon('close', 'sm')}</span>`;
    const fields =
      `<div class="v field">${C.Input({ value: subject, placeholder: Texts.commitMessage, focused: focus === 'subject' })}</div>` +
      `<div class="v field">${C.Input({ value: description, placeholder: Texts.commitDescription, lines: 4, focused: focus === 'description' })}</div>`;
    return `<div class="overlay"><div data-c="SquashDialog" data-base="Dialog" class="dialog squash-dialog">${close}<div class="base semibold">${esc(Texts.squashTitle(n))}</div><div class="sm muted">${esc(Texts.squashDescription(newest.id, oldest.id))}</div>${fields}<div class="v squash-notes">${join(lines)}</div><div class="footer stretch">${buttons}</div></div></div>`;
  },

  ResetHardDialog({ branch, commit, files, lost = 0 }) {
    const description = [Texts.resetHardFiles(files), lost && Texts.resetHardLost(lost)].filter(Boolean).join(' ');
    return C.AlertDialog({ title: Texts.resetHardTitle(branch, commit.id), description, confirm: 'Reset', name: 'ResetHardDialog' });
  },

  TagDialog({ commit, value = '', message = '', problem, remote, push = false }) {
    const name = C.Input({ value, placeholder: Texts.tagPlaceholder, focused: !message });
    const hint = problem ? `<div class="xs problem">${esc(problem)}</div>` : '';
    const body = C.Input({ value: message, placeholder: Texts.tagMessagePlaceholder, lines: 3, focused: Boolean(message) });
    const pushBox = remote ? `<div class="h">${C.Checkbox({ label: Texts.pushTag(remote), checked: push })}</div>` : '';
    const buttons =
      C.Button({ label: 'Cancel', size: 'md' }) +
      C.Button({ label: Texts.createTag, variant: 'primary', size: 'md', disabled: !value || Boolean(problem) });
    const close = `<span class="close">${icon('close', 'sm')}</span>`;
    const description = Texts.tagDescription(`${commit.id}, “${commit.subject}”`);
    return `<div class="overlay"><div data-c="TagDialog" data-base="Dialog" class="dialog">${close}<div class="base semibold">${esc(Texts.tagTitle)}</div><div class="sm muted">${esc(description)}</div><div class="v field">${name}${hint}</div><div class="v field">${body}</div>${pushBox}<div class="footer stretch">${buttons}</div></div></div>`;
  },

  DeleteBranchDialog({ label, only = 0 }) {
    if (label.kind === 'remote') {
      const { remote, branch } = remoteParts(label.name);
      return C.AlertDialog({ title: Texts.deleteRemoteTitle(branch, remote), description: Texts.deleteRemoteDescription(branch, remote), confirm: 'Delete', name: 'DeleteBranchDialog' });
    }
    const description = only === null ? Texts.deleteUncounted(label.name) : only ? Texts.deleteUnmerged(label.name, only) : Texts.deleteMerged(label.name);
    return C.AlertDialog({ title: Texts.deleteBranchTitle(label.name), description, confirm: 'Delete', name: 'DeleteBranchDialog' });
  },

  BehindDialog({ branch, upstream, theirs, ours, checkedOut = true, rewritten = false }) {
    if (rewritten) {
      const description = [Texts.behindRewritten(branch, upstream, theirs), Texts.behindRewrittenForce(branch, upstream, theirs)].join(' ');
      const buttons = C.Button({ label: 'Cancel', size: 'md' }) + C.Button({ label: Texts.forcePush, variant: 'primary', size: 'md' });
      return `<div class="overlay"><div data-c="BehindDialog" data-base="AlertDialog" class="dialog"><div class="base semibold">${esc(Texts.behindRewrittenTitle(branch, upstream))}</div><div class="sm muted">${esc(description)}</div><div class="footer stretch">${buttons}</div></div></div>`;
    }
    const pull = ours === 0 ? Texts.behindFastForward(branch, upstream) : checkedOut ? Texts.behindPull(branch) : Texts.behindCheckOut(branch);
    const description = [Texts.behindCounts(branch, upstream, theirs, ours), pull, Texts.behindForce(branch, upstream, theirs)].join(' ');
    const pullButton = ours === 0 || checkedOut ? C.Button({ label: ours === 0 ? Texts.pullOnly : Texts.pullAndPush, variant: 'primary', size: 'md' }) : '';
    const buttons = C.Button({ label: 'Cancel', size: 'md' }) + C.Button({ label: Texts.forcePush, variant: 'danger', size: 'md' }) + pullButton;
    return `<div class="overlay"><div data-c="BehindDialog" data-base="AlertDialog" class="dialog"><div class="base semibold">${esc(Texts.behindTitle(branch, upstream))}</div><div class="sm muted">${esc(description)}</div><div class="footer stretch">${buttons}</div></div></div>`;
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
    const row = ([label, keys, sub]) => {
      const end = sub ? icon('chevron-right', 'sm', 'var(--muted)') : `<span class="keys">${esc(keys)}</span>`;
      const open = sub && sub.open ? C.Menu({ items: sub.items, left: 'calc(100% + 8px)', top: -5, hot: sub.hot }) : '';
      return `<div class="item${label === hot ? ' hot' : ''}${sub ? ' parent' : ''}"><span class="label">${esc(label)}</span>${end}${open}</div>`;
    };
    const rows = items.map((item) => (item === '-' ? '<div class="separator"></div>' : row(item))).join('');
    const length = (value) => (typeof value === 'number' ? `${value}px` : value);
    return `<div data-c="${name}" data-base="PopupMenu" class="menu sm" style="left:${length(left)};top:${length(top)}">${rows}</div>`;
  },

  Notification({ message, title, kind, action = '' }) {
    const glyphs = {
      success: icon('circle-check', 'md', 'var(--green)'),
      warning: icon('triangle-alert', 'md', 'var(--yellow)'),
      error: icon('circle-x', 'md', 'var(--red)'),
      info: icon('info', 'md', 'var(--blue)'),
    };
    const glyph = glyphs[kind] ? `<span class="kind">${glyphs[kind]}</span>` : '';
    const heading = title ? `<div class="semibold">${esc(title)}</div>` : '';
    const actions = action ? `<div class="h actions">${action}</div>` : '';
    return `<div data-c="Notification" class="toast sm">${glyph}<div class="message">${heading}${esc(message)}${actions}</div>${C.Button({ icon: 'close', variant: 'ghost' })}</div>`;
  },

  AlertDialog({ title, description, confirm, name = 'AlertDialog' }) {
    const buttons = confirm
      ? C.Button({ label: 'Cancel', size: 'md' }) + C.Button({ label: confirm, variant: 'danger', size: 'md' })
      : C.Button({ label: 'OK', variant: 'primary', size: 'md' });
    const base = name === 'AlertDialog' ? '' : ' data-base="AlertDialog"';
    return `<div class="overlay"><div data-c="${name}"${base} class="dialog"><div class="base semibold">${esc(title)}</div><div class="sm muted">${esc(description)}</div><div class="footer">${buttons}</div></div></div>`;
  },

  DiscardDialog({ change, changes = [] }) {
    if (changes.length > 1) return C.AlertDialog(discardFilesQuestion(changes));
    const [name] = splitPath(change.path);
    const untracked = change.kind === '?';
    return C.AlertDialog({
      title: untracked ? Texts.deleteTitle(name) : Texts.discardTitle(name),
      description: untracked ? Texts.deleteDescription(change.path) : Texts.discardDescription(change.path),
      confirm: untracked ? 'Delete' : 'Discard',
    });
  },

  Chip({ label, color, icon: glyph }) {
    const tint = color ? ` tinted" style="--c:${color}` : '';
    return `<span data-c="Chip" data-base="Tag" class="chip${tint}">${glyph ? icon(glyph, 'xs') : ''}${esc(label)}</span>`;
  },

  Select({ value, placeholder = '', lead = '', open = false, disabled = false }) {
    const shown = value ? `<span class="fill ellipsis">${esc(value)}</span>` : `<span class="fill placeholder">${esc(placeholder)}</span>`;
    const classes = ['input', 'select', open && 'focused', disabled && 'disabled'].filter(Boolean).join(' ');
    return `<div data-c="Select" class="${classes}">${lead}${shown}${icon('chevron-down', 'sm', 'var(--muted)')}</div>`;
  },

  Radio({ label, checked = false, about }) {
    const title = about ? ` title="${esc(about)}"` : '';
    return `<span data-c="Radio" class="radio${checked ? ' checked' : ''}"${title}><span class="dot"></span><span>${esc(label)}</span></span>`;
  },

  GroupBox({ title, about, rows, action = '' }) {
    const head = title
      ? `<div class="h group-head"><div class="v fill"><div class="sm semibold">${esc(title)}</div>${about ? `<div class="xs muted">${esc(about)}</div>` : ''}</div>${action}</div>`
      : '';
    return `<div data-c="GroupBox" class="v group">${head}<div class="group-box">${rows.join('')}</div></div>`;
  },

  PageHeader({ title, about, subtitle = '', actions = '', crumb, here = title, media = '' }) {
    const crumbs = crumb ? `<div class="h crumbs xs"><span class="link">${esc(crumb)}</span>${icon('chevron-right', 'xs')}<span>${esc(here)}</span></div>` : '';
    const text = about ? `<div class="sm muted about">${esc(about)}</div>` : '';
    return `<div data-c="PageHeader" class="page-head">${crumbs}<div class="h top">${media}<div class="v fill" style="gap:2px"><div class="lg semibold">${esc(title)}</div>${subtitle}${text}</div>${actions ? `<div class="h actions">${actions}</div>` : ''}</div></div>`;
  },

  SettingsNav({ page = 'accounts', accounts = 0, waiting = 0, zoom = 100, update }) {
    const row = (key, label, glyph, extra) =>
      C.ListRow({ content: `<div class="h fill" style="gap:8px">${icon(glyph, 'sm', 'var(--muted)')}<span class="fill sm">${esc(label)}</span>${extra}</div>`, selected: key === page });
    const count = accounts ? `<span class="xs muted">${thousands(accounts)}</span>` : '';
    const badge = waiting ? `<span class="count-badge" title="${esc(counted(waiting, 'connector waits', 'connectors wait'))} for your approval">${thousands(waiting)}</span>` : '';
    const zoomed = zoom === 100 ? '' : `<span class="xs muted">${thousands(zoom)} %</span>`;
    const ready = update ? `<span class="xs update-ready" title="${esc(HostTexts.readyTip(update))}">${esc(update)}</span>` : '';
    return `<div data-c="SettingsNav" class="settings-nav">${C.SectionHeader({ title: HostTexts.settings })}${row('accounts', HostTexts.accounts, 'user', count)}${row('connectors', HostTexts.connectors, 'plug', badge)}${row('interface', HostTexts.interface, 'monitor', zoomed)}${row('updates', HostTexts.updates, 'download', ready)}</div>`;
  },

  SettingsTab({ page, content, nav = {} }) {
    return `<div data-c="SettingsTab" class="settings">${C.SettingsNav({ page, ...nav })}<div class="settings-page"><div class="page-body">${content}</div></div></div>`;
  },

  Switch({ checked = false, disabled = false, tooltip } = {}) {
    const title = tooltip ? ` title="${esc(tooltip)}"` : '';
    return `<span data-c="Switch" class="switch${checked ? ' on' : ''}${disabled ? ' disabled' : ''}"${title}><span class="knob"></span></span>`;
  },

  UpdateButton({ state = 'ready', version = '0.2.0' } = {}) {
    const news = () => C.Button({ label: HostTexts.whatsNew, variant: 'ghost' });
    const shown = {
      ready: () => C.Button({ label: HostTexts.restartToUpdate, icon: 'refresh-cw', variant: 'update', tooltip: HostTexts.readyTip(version) }) + news(),
      available: () => C.Button({ label: HostTexts.updateTo(version), icon: 'download', variant: 'update', tooltip: HostTexts.foundManual }) + news(),
      downloading: () => C.Button({ label: HostTexts.downloadingShort, variant: 'update', loading: true, disabled: true, tooltip: HostTexts.downloading(version) }),
      'read-only': () => C.Button({ label: HostTexts.releaseOut(version), icon: 'external-link', variant: 'ghost', tooltip: HostTexts.openReleasePage }),
    };
    return `<div data-c="UpdateButton" class="h update-button">${shown[state]()}</div>`;
  },

  UpdatesPage({ installation = 'installed', folder = 'C:\\Users\\alex\\AppData\\Local\\Programs\\SterneGit', running = '0.1.0', status = 'up-to-date', version = '0.2.0', ago = '4 minutes ago', why = '', automatic = true } = {}) {
    const built = installation === 'built';
    const shown = built ? 'built' : status;
    const where = built ? HostTexts.builtHere : installation === 'portable' ? HostTexts.portableAt(folder) : HostTexts.installedAt(folder);
    const spinning = icon('loader', 'xs').replace('class="icon"', 'class="icon spinner"');
    const [glyph, line] = {
      'up-to-date': [icon('circle-check', 'xs', 'var(--green)'), HostTexts.upToDate(ago)],
      checking: [spinning, HostTexts.checking],
      downloading: [spinning, HostTexts.downloading(version)],
      ready: [icon('circle-check', 'xs', 'var(--green)'), HostTexts.readyStatus(version)],
      available: [icon('info', 'xs', 'var(--blue)'), HostTexts.availableStatus(version)],
      'read-only': [icon('triangle-alert', 'xs', 'var(--yellow)'), HostTexts.readOnlyStatus(version)],
      failed: [icon('circle-alert', 'xs', 'var(--red)'), HostTexts.checkFailed(why)],
      built: [icon('info', 'xs', 'var(--muted)'), HostTexts.builtStatus],
    }[shown];
    const check = (disabled) => C.Button({ label: HostTexts.checkNow, icon: 'refresh-cw', size: 'sm', disabled });
    const news = C.Button({ label: HostTexts.whatsNew, size: 'sm', variant: 'ghost' });
    const actions = {
      'up-to-date': () => check(false),
      failed: () => check(false),
      checking: () => check(true),
      downloading: () => check(true),
      built: () => check(true),
      ready: () => news + C.Button({ label: HostTexts.restartToUpdate, icon: 'refresh-cw', size: 'sm', variant: 'update' }),
      available: () => news + C.Button({ label: HostTexts.update, icon: 'download', size: 'sm', variant: 'update' }),
      'read-only': () => C.Button({ label: HostTexts.openReleasePage, icon: 'external-link', size: 'sm' }),
    }[shown]();
    const versionRow = `<div class="row"><div class="v fill" style="gap:2px"><span class="sm medium">SterneGit ${esc(running)}</span><span class="xs muted">${esc(where)}</span><span class="h xs update-status">${glyph}<span>${esc(line)}</span></span></div><div class="h none" style="gap:8px">${actions}</div></div>`;
    const automaticRow = `<div class="row"><div class="v fill" style="gap:2px"><span class="sm medium">${esc(HostTexts.automatic)}</span><span class="xs muted">${esc(HostTexts.automaticAbout)}</span></div>${C.Switch({ checked: automatic && !built, disabled: built })}</div>`;
    return `<div data-c="UpdatesPage" class="v page">${C.PageHeader({ title: HostTexts.updates, about: HostTexts.updatesAbout })}${C.GroupBox({ rows: [versionRow, automaticRow] })}</div>`;
  },

  ZoomStepper({ percent = 100 }) {
    const out = C.Button({ icon: 'minus', size: 'sm', tooltip: HostTexts.zoomOut, disabled: percent <= ZoomLevels[0] });
    const into = C.Button({ icon: 'plus', size: 'sm', tooltip: HostTexts.zoomIn, disabled: percent >= ZoomLevels[ZoomLevels.length - 1] });
    return `<div data-c="ZoomStepper" class="zoom-stepper">${out}<span class="sm percent">${thousands(percent)} %</span>${into}</div>`;
  },

  InterfacePage({ zoom = 100 }) {
    const reset = C.Button({ label: HostTexts.resetZoom, size: 'sm', variant: 'ghost', tooltip: HostTexts.resetZoomTip, disabled: zoom === 100 });
    const row = `<div class="row"><div class="v fill" style="gap:2px"><span class="sm medium">${esc(HostTexts.zoom)}</span><span class="xs muted">${esc(HostTexts.zoomAbout)}</span></div>${C.ZoomStepper({ percent: zoom })}${reset}</div>`;
    return `<div data-c="InterfacePage" class="v page">${C.PageHeader({ title: HostTexts.interface, about: HostTexts.interfaceAbout })}${C.GroupBox({ rows: [row] })}</div>`;
  },

  AccountRow({ account, hot = false }) {
    const details = account.signedOut
      ? `<span class="h problem-text">${icon('triangle-alert', 'xs')}${esc(HostTexts.signedOut(account.service))}</span>`
      : `<span class="ellipsis">${esc([account.service, account.server, MethodLabels[account.method]].join(' · '))}</span>`;
    const key = account.key ? `<span class="h xs muted none" style="gap:4px" title="${esc(account.key.title)}">${icon('key-round', 'xs')}SSH</span>` : '';
    const end = account.signedOut ? C.Button({ label: HostTexts.signInAgain, icon: 'log-in', size: 'sm' }) : icon('chevron-right', 'sm', 'var(--muted)');
    const name = account.name ? `<span class="sm medium">${esc(account.name)}</span><span class="sm muted">${esc(account.login)}</span>` : `<span class="sm medium">${esc(account.login)}</span>`;
    return `<div data-c="AccountRow" class="row${hot ? ' hot' : ''}">${accountAvatar(account, 32)}<div class="v fill"><div class="h" style="gap:6px">${name}</div><div class="h xs muted">${details}</div></div>${key}${end}</div>`;
  },

  AccountsPage({ accounts }) {
    const add = C.Button({ label: HostTexts.addAccount, icon: 'plus', size: 'md', variant: 'primary' });
    const list = accounts.length
      ? C.GroupBox({ rows: accounts.map((account, ix) => C.AccountRow({ account, hot: ix === 0 })) })
      : `<div class="v empty-page">${icon('user', 'lg', 'var(--muted)')}<div class="medium">${esc(HostTexts.noAccounts)}</div><div class="sm muted">${esc(HostTexts.noAccountsAbout)}</div></div>`;
    return `<div data-c="AccountsPage" class="v page">${C.PageHeader({ title: HostTexts.accounts, about: HostTexts.accountsAbout, actions: add })}${list}</div>`;
  },

  SshKeyRow({ key, local = false }) {
    const chips = [key.sternegit && C.Chip({ label: HostTexts.sternegitKey, color: 'var(--blue)' }), key.readOnly && C.Chip({ label: HostTexts.readOnly })].filter(Boolean).join('');
    const side = local ? C.Button({ label: HostTexts.copyPublicKey, icon: 'copy', size: 'sm', variant: 'ghost' }) : `<span class="xs muted none">${esc(HostTexts.added(key.added))}</span>`;
    const second = local ? `${esc(key.type)} · ${esc(key.fingerprint)} · <span class="text">${esc(HostTexts.usedBy(key.using))}</span>` : esc(key.fingerprint);
    return `<div data-c="SshKeyRow" class="row">${icon('key-round', 'md', 'var(--muted)')}<div class="v fill"><div class="h" style="gap:6px"><span class="sm medium ellipsis">${esc(key.title)}</span>${chips}</div><div class="xs muted mono ellipsis">${second}</div></div>${side}</div>`;
  },

  AccountPage({ account, keys = [] }) {
    const subtitle = `<div class="sm muted">${esc([account.login, account.service, account.server].join(' · '))}</div>`;
    const actions = C.Button({ label: HostTexts.signInAgain, icon: 'log-in', size: 'md' }) + C.Button({ label: HostTexts.removeAccount, icon: 'trash-2', size: 'md', variant: 'danger' });
    const header = C.PageHeader({ title: account.name || account.login, crumb: HostTexts.accounts, subtitle, actions, media: accountAvatar(account, 48) });
    const def = (term, value) => `<div class="row def"><span class="term">${esc(term)}</span><div class="h fill" style="gap:6px;flex-wrap:wrap">${value}</div></div>`;
    const signIn = C.GroupBox({
      title: HostTexts.signIn,
      rows: [
        def(HostTexts.method, `<span class="sm">${esc(MethodLabels[account.method])}</span>`),
        def(HostTexts.scopes, account.scopes.map((scope) => C.Chip({ label: scope })).join('')),
        def(HostTexts.tokenKept, `<span class="sm muted">${esc(HostTexts.tokenWhere(account.id))}</span>`),
      ],
    });
    const local = account.key
      ? C.SshKeyRow({ key: { ...account.key, sternegit: true, using: account.using.length }, local: true })
      : `<div class="row">${icon('key-round', 'md', 'var(--muted)')}<span class="sm muted fill">${esc(HostTexts.noKey(account.service))}</span>${C.Button({ label: HostTexts.createKey, size: 'sm' })}</div>`;
    const others = keys.filter((key) => !key.sternegit).map((key) => C.SshKeyRow({ key }));
    const ssh = C.GroupBox({ title: HostTexts.sshKey, about: HostTexts.sshKeyAbout(account.service), rows: [local, ...others] });
    const owners = account.owners.length
      ? account.owners.map((owner) => `<div class="row">${icon('building', 'md', 'var(--muted)')}<span class="sm fill">${esc(`${account.server}/${owner}`)}</span>${C.Button({ label: HostTexts.forget, size: 'sm', variant: 'ghost' })}</div>`)
      : [`<div class="row"><span class="sm muted">${esc(HostTexts.noOwners)}</span></div>`];
    const remembered = C.GroupBox({ title: HostTexts.remoteOwners, about: HostTexts.remoteOwnersAbout, rows: owners });
    return `<div data-c="AccountPage" class="v page">${header}${signIn}${ssh}${remembered}</div>`;
  },

  ConnectorRow({ connector, hot = false }) {
    const look = StandingLooks[connector.standing];
    const chips = C.Chip({ label: connector.origin === 'bundled' ? HostTexts.bundled : HostTexts.installed }) + C.Chip({ label: look.label(connector), color: look.color });
    const facts = connector.problem || [grantSummary(connector.grant), connector.accounts && counted(connector.accounts, 'account', 'accounts')].filter(Boolean).join(' · ');
    const end =
      connector.standing === 'waiting' || connector.standing === 'modified'
        ? C.Button({ label: HostTexts.review, variant: 'primary', size: 'sm' })
        : connector.standing === 'failed'
          ? C.Button({ label: HostTexts.showLog, size: 'sm' })
          : icon('chevron-right', 'sm', 'var(--muted)');
    const problem = connector.problem ? ' problem-text' : '';
    return `<div data-c="ConnectorRow" class="row${hot ? ' hot' : ''}"><span class="mark">${icon('plug', 'md')}</span><div class="v fill"><div class="h" style="gap:6px"><span class="sm medium">${esc(connector.name)}</span><span class="sm muted">${esc(connector.version)}</span>${chips}</div><div class="xs muted ellipsis${problem}">${esc(facts)}</div></div>${end}</div>`;
  },

  ConnectorsPage({ connectors }) {
    const open = C.Button({ label: HostTexts.openFolder, icon: 'folder-open', size: 'md' });
    const header = C.PageHeader({ title: HostTexts.connectors, about: HostTexts.connectorsAbout, actions: open });
    const note = `<div class="h xs muted" style="gap:6px">${icon('info', 'xs')}${esc(HostTexts.connectorsFolder)}</div>`;
    const rows = connectors.map((connector) => C.ConnectorRow({ connector, hot: connector.standing === 'waiting' }));
    return `<div data-c="ConnectorsPage" class="v page">${header}${C.GroupBox({ rows })}${note}</div>`;
  },

  GrantList({ grant, fresh = false }) {
    const isNew = fresh ? C.Chip({ label: HostTexts.newGrant, color: 'var(--yellow)' }) : '';
    const row = (glyph, html, side = '') => `<div class="row slim">${icon(glyph, 'sm', 'var(--muted)')}<span class="fill sm">${html}</span>${isNew}${side}</div>`;
    const groups = [
      grant.reach.length &&
        C.GroupBox({
          title: HostTexts.reach,
          about: HostTexts.reachAbout,
          rows: grant.reach.map((entry) => row('globe', `<span class="mono">${slotted(entry.origin, '{server}', HostTexts.serverSlot)}</span>`, `<span class="xs muted none">${esc(entry.token ? HostTexts.withToken : HostTexts.withoutToken)}</span>`)),
        }),
      grant.images.length && C.GroupBox({ title: HostTexts.images, rows: grant.images.map((origin) => row('image', `<span class="mono">${esc(origin)}</span>`, `<span class="xs muted none">${esc(HostTexts.withoutToken)}</span>`)) }),
      grant.remotes.length && C.GroupBox({ title: HostTexts.remotes, about: HostTexts.remotesAbout, rows: grant.remotes.map((host) => row('server', host === '{server.host}' ? `<span class="slot">${esc(HostTexts.hostSlot)}</span>` : `<span class="mono">${esc(host)}</span>`)) }),
      grant.signIn.length && C.GroupBox({ title: HostTexts.signInPages, rows: grant.signIn.map((url) => row('log-in', `<span class="mono">${esc(url)}</span>`)) }),
      grant.features.length && C.GroupBox({ title: HostTexts.features, rows: grant.features.map((feature) => row('check', esc(feature))) }),
    ];
    return `<div data-c="GrantList" class="v grant">${groups.filter(Boolean).join('')}</div>`;
  },

  ApprovalPage({ connector, update = false }) {
    const look = StandingLooks[connector.standing];
    const title = update ? HostTexts.moreTitle(connector.name, connector.version, connector.was) : HostTexts.approveTitle(connector.name, connector.version);
    const chips = `<div class="h" style="gap:6px;margin-top:4px">${C.Chip({ label: HostTexts.installed })}${C.Chip({ label: look.label(connector), color: look.color })}<span class="xs muted">${esc(HostTexts.installedIn(connector.id))}</span></div>`;
    const header = C.PageHeader({ title, crumb: HostTexts.connectors, here: connector.name, subtitle: chips, media: `<span class="mark large">${icon('plug', 'lg')}</span>` });
    const warning = `<div class="callout">${icon('shield-check', 'md', 'var(--yellow)')}<div class="sm">${esc(HostTexts.approvalWarning(connector.name))}</div></div>`;
    const grant = C.GrantList({ grant: update ? connector.added : connector.grant, fresh: update });
    const rest = update ? `<div class="sm muted">${esc(HostTexts.sameAsBefore(connector.was))}</div>` : '';
    const actions = `<div class="h page-actions">${C.Button({ label: HostTexts.notNow, size: 'md' })}${C.Button({ label: HostTexts.approve, icon: 'shield-check', variant: 'primary', size: 'md' })}</div>`;
    return `<div data-c="ApprovalPage" class="v page">${header}${warning}${grant}${rest}${actions}</div>`;
  },

  SignInDialog({ step = 'choose', service = 'GitHub', host = 'github.com', enterprise = false, problem, warning, hot }) {
    const name = 'SignInDialog';
    const cancel = C.Button({ label: HostTexts.cancel, size: 'md' });
    if (step === 'choose') {
      const option = (key, glyph, label, about) =>
        `<div class="method-option${key === hot ? ' hot' : ''}">${icon(glyph, 'md', 'var(--muted)')}<div class="v fill"><span class="sm medium">${esc(label)}</span><span class="xs muted">${esc(about)}</span></div>${icon('chevron-right', 'sm', 'var(--muted)')}</div>`;
      const services = formField({ label: HostTexts.service, control: C.Select({ value: enterprise ? 'GitHub Enterprise Server' : 'GitHub — github.com', lead: `<span class="prefix">${icon('plug', 'sm')}</span>` }) });
      const server = enterprise ? formField({ label: HostTexts.serverAddress, control: C.Input({ value: 'https://github.acme.dev', focused: true }), hint: 'The address you open in the browser' }) : '';
      const methods = [
        !enterprise && option('device', 'log-in', HostTexts.methodDevice, HostTexts.methodDeviceAbout),
        option('token', 'key-round', HostTexts.methodToken, HostTexts.methodTokenAbout),
        option('credential', 'shield-check', HostTexts.methodCredential, HostTexts.methodCredentialAbout(enterprise ? 'github.acme.dev' : host)),
      ].filter(Boolean);
      return dialogShell({ name, title: HostTexts.addAccountTitle, body: `<div class="form">${services}${server}<div class="v" style="gap:8px">${methods.join('')}</div></div>`, buttons: cancel });
    }
    if (step === 'device' || step === 'expired' || step === 'denied') {
      const stale = step !== 'device';
      const code = `<div class="device-code${stale ? ' stale' : ''}"><span class="code">${esc(Sample.hosting.deviceCode)}</span>${stale ? '' : C.Button({ label: HostTexts.copy, icon: 'copy', size: 'sm' })}</div>`;
      const status =
        step === 'device'
          ? `<div class="dialog-status">${C.Spinner({ size: 'sm' })}<span class="muted">${esc(HostTexts.waitingForYou(service))}</span></div>`
          : step === 'expired'
            ? `<div class="dialog-status">${icon('clock', 'md', 'var(--yellow)')}<span>${esc(HostTexts.expired)}</span></div>`
            : `<div class="dialog-status">${icon('circle-x', 'md', 'var(--red)')}<span>${esc(HostTexts.denied(service))}</span></div>`;
      const buttons =
        step === 'device'
          ? C.Button({ label: HostTexts.openAgain, icon: 'external-link', size: 'md', variant: 'ghost' }) + cancel
          : cancel + C.Button({ label: step === 'expired' ? HostTexts.newCode : HostTexts.tryAgain, variant: 'primary', size: 'md' });
      return dialogShell({ name, title: HostTexts.signInTo(service), description: HostTexts.deviceOpened(`${host}/login/device`), body: code + status, buttons });
    }
    if (step === 'token') {
      const create = C.Button({ label: HostTexts.createToken, icon: 'external-link', size: 'sm', variant: 'ghost' });
      const field = formField({ label: HostTexts.tokenLabel, control: C.Input({ value: 'ghp_••••••••••••••••••••••••••••••••••••', focused: true, mono: true }), problem, warning, extra: create });
      const buttons = cancel + C.Button({ label: HostTexts.signInButton, variant: 'primary', size: 'md', disabled: Boolean(problem) });
      return dialogShell({ name, title: HostTexts.tokenTitle(service), description: HostTexts.tokenAbout('repo, read:org, write:public_key and workflow'), body: `<div class="form">${field}</div>`, buttons });
    }
    const failure = problem ? `<div class="dialog-status">${icon('circle-x', 'md', 'var(--red)')}<span>${esc(problem)}</span></div>` : '';
    const buttons = cancel + C.Button({ label: HostTexts.useToken, variant: 'primary', size: 'md', disabled: Boolean(problem) });
    return dialogShell({ name, title: HostTexts.credentialTitle, description: HostTexts.credentialAbout(host), body: failure, buttons });
  },

  SshKeyDialog({ account, forbidden = false }) {
    const field = formField({ label: HostTexts.keyLabel(account.service), control: C.Input({ value: 'SterneGit on DESKTOP-7Q2K', focused: true }), problem: forbidden ? HostTexts.keyForbidden(account.service, 'write:public_key') : null });
    const buttons = forbidden
      ? C.Button({ label: HostTexts.cancel, size: 'md' }) + C.Button({ label: HostTexts.methodDevice, icon: 'log-in', variant: 'primary', size: 'md' })
      : C.Button({ label: HostTexts.cancel, size: 'md' }) + C.Button({ label: HostTexts.createAndAdd, icon: 'key-round', variant: 'primary', size: 'md' });
    return dialogShell({ name: 'SshKeyDialog', title: HostTexts.keyTitle(account.login), description: HostTexts.keyAbout('ed25519', account.service), body: `<div class="form">${field}</div>`, buttons });
  },

  RemoveAccountDialog({ account }) {
    const using = account.using.length
      ? `<div class="v using"><div class="sm">${esc(HostTexts.removeUsing(account.using.length))}</div>${account.using.map((path) => `<div class="xs mono muted">${esc(path)}</div>`).join('')}${C.Checkbox({ label: HostTexts.undoSshCommand, checked: true })}</div>`
      : '';
    const buttons = C.Button({ label: HostTexts.cancel, size: 'md' }) + C.Button({ label: HostTexts.remove, variant: 'danger', size: 'md' });
    return dialogShell({ name: 'RemoveAccountDialog', title: HostTexts.removeTitle(account.login), description: HostTexts.removeAbout(account.service), body: using, buttons });
  },

  AccountMenu({ accounts, chosen, hot, left = 0, top = 0 }) {
    const rows = accounts
      .map(
        (account) =>
          `<div class="item${account.login === hot ? ' hot' : ''}"><span class="check">${account.login === chosen ? icon('check', 'sm') : ''}</span>${accountAvatar(account, 20)}<span class="v label"><span>${esc(account.login)}</span><span class="about">${esc(`${account.service} · ${account.server}`)}</span></span></div>`,
      )
      .join('');
    return `<div data-c="AccountMenu" data-base="PopupMenu" class="menu choices accounts sm" style="left:${left}px;top:${top}px">${rows}</div>`;
  },

  AccountChoiceBar({ host, owner, accounts, chosen, open = false }) {
    const account = accounts.find((candidate) => candidate.login === chosen);
    const select = `<div class="none" style="width:210px">${C.Select({ value: account.login, lead: `<span class="prefix">${accountAvatar(account, 18)}</span>`, open })}</div>`;
    const later = C.Button({ icon: 'close', variant: 'ghost', tooltip: HostTexts.choiceLater });
    return `<div data-c="AccountChoiceBar" class="account-bar">${icon('user', 'md', 'var(--blue)')}<span class="sm fill ellipsis">${esc(HostTexts.choiceText(host, owner, accounts.length))}</span>${select}${C.Button({ label: HostTexts.use, variant: 'primary', size: 'sm' })}${later}</div>`;
  },

  RepositoryRow({ repository, selected = false }) {
    const look = Visibilities[repository.visibility];
    const glyph = repository.fork ? 'git-fork' : repository.archived ? 'archive' : look.icon;
    const chips = [
      repository.visibility !== 'public' && C.Chip({ label: look.label }),
      repository.fork && C.Chip({ label: 'Fork' }),
      repository.archived && C.Chip({ label: 'Archived', color: 'var(--yellow)' }),
    ]
      .filter(Boolean)
      .join('');
    const about = repository.description ? `<div class="xs muted ellipsis">${esc(repository.description)}</div>` : '';
    const content = `${icon(glyph, 'sm', 'var(--muted)')}<div class="v fill"><div class="h" style="gap:6px"><span class="sm ellipsis"><span class="muted">${esc(repository.owner)}/</span><span class="medium">${esc(repository.name)}</span></span>${chips}</div>${about}</div><span class="xs muted none">${esc(repository.updated)}</span>`;
    return C.ListRow({ content, selected, extraClass: 'repo-row', name: 'RepositoryRow' });
  },

  ConnectorFailure({ title, message, action, log, tone = 'error' }) {
    const glyphs = { error: icon('circle-x', 'md', 'var(--red)'), warning: icon('triangle-alert', 'md', 'var(--yellow)'), wait: icon('clock', 'md', 'var(--muted)') };
    const lines = log ? `<div class="log mono xs">${log.map(esc).join('<br>')}</div>` : '';
    const button = action ? `<div class="h" style="margin-top:8px">${C.Button({ label: action, size: 'sm' })}</div>` : '';
    return `<div data-c="ConnectorFailure" class="connector-failure">${glyphs[tone]}<div class="v fill" style="gap:2px"><div class="sm medium">${esc(title)}</div><div class="sm muted">${esc(message)}</div>${lines}${button}</div></div>`;
  },

  CloneDialog({ from = 'url', accounts, repositories = [], selected = 0, url = '', folder = 'E:\\Projects', failure, hot }) {
    const account = accounts.find((candidate) => candidate.login === from);
    const sourceRow = (key, lead, label, extra = '') =>
      C.ListRow({ content: `<div class="h fill" style="gap:8px">${lead}<span class="fill sm ellipsis">${esc(label)}</span>${extra}</div>`, selected: key === from, extraClass: key === hot ? 'hot' : '' });
    const sources = [
      sourceRow('url', icon('link', 'sm', 'var(--muted)'), HostTexts.url),
      C.SectionHeader({ title: HostTexts.accounts }),
      ...accounts.map((each) =>
        sourceRow(each.login, accountAvatar(each, 18), each.login, each.signedOut ? icon('triangle-alert', 'xs', 'var(--yellow)') : `<span class="xs muted">${esc(each.server === 'github.com' ? 'GitHub' : each.server)}</span>`),
      ),
      sourceRow('add', icon('plus', 'sm', 'var(--muted)'), HostTexts.addAccountShort),
    ].join('');
    const target = (name) => formField({ label: HostTexts.cloneInto, control: withButton(C.Input({ value: name ? `${folder}\\${name}` : folder }), C.Button({ label: HostTexts.browse, size: 'md' })) });
    let main;
    if (!account) {
      main = formField({ label: HostTexts.repositoryUrl, control: C.Input({ value: url, placeholder: 'https://github.com/owner/name.git', focused: true }), hint: HostTexts.urlHint }) + target(url ? url.replace(/\.git$/, '').split('/').pop() : '');
    } else if (failure) {
      main = C.ConnectorFailure(failure);
    } else {
      const filters = `<div class="h" style="gap:8px"><div class="none" style="width:180px">${C.Select({ value: HostTexts.allOwners })}</div><div class="fill v">${C.Input({ placeholder: HostTexts.searchIn(account.login), prefix: 'search' })}</div></div>`;
      const rows = repositories.map((repository, ix) => C.RepositoryRow({ repository, selected: ix === selected })).join('');
      const more = `<div class="more">${C.Button({ label: HostTexts.loadMore, size: 'sm', variant: 'ghost' })}</div>`;
      const chosen = repositories[selected];
      main = `${filters}<div class="repo-list"><div class="v fill" style="overflow:hidden">${rows}</div>${more}</div><div class="h" style="gap:12px;align-items:flex-end"><div class="fill v">${target(chosen.name)}</div>${formField({ label: HostTexts.protocol, control: protocolToggle(Boolean(account.key)) })}</div>`;
    }
    const ready = account ? !failure : Boolean(url);
    const buttons = C.Button({ label: HostTexts.cancel, size: 'md' }) + C.Button({ label: HostTexts.clone, icon: 'download', variant: 'primary', size: 'md', disabled: !ready });
    const close = `<span class="close">${icon('close', 'sm')}</span>`;
    return `<div class="overlay"><div data-c="CloneDialog" data-base="Dialog" class="dialog clone">${close}<div class="base semibold">${esc(HostTexts.cloneTitle)}</div><div class="h clone-body"><div class="v clone-sources">${sources}</div><div class="v fill clone-main">${main}</div></div><div class="footer">${buttons}</div></div></div>`;
  },

  NewRepositoryDialog({ hosted = false, account, owners = [] }) {
    const name = formField({ label: HostTexts.name, control: C.Input({ value: 'starfall', focused: true }) });
    const folder = formField({ label: HostTexts.folder, control: withButton(C.Input({ value: 'E:\\Projects' }), C.Button({ label: HostTexts.browse, size: 'md' })), hint: HostTexts.creates('E:\\Projects\\starfall') });
    const toggle = `<div class="h" style="gap:10px">${C.Checkbox({ label: HostTexts.alsoOn, checked: hosted })}<div class="fill v">${C.Select({ value: `${account.login} · ${account.service}`, lead: `<span class="prefix">${accountAvatar(account, 18)}</span>`, disabled: !hosted })}</div></div>`;
    const more = hosted
      ? `<div class="v hosted" style="gap:14px">${formField({ label: HostTexts.owner, control: C.Select({ value: owners[0].login }) })}${formField({ label: HostTexts.visibility, control: visibilityChoice(owners[0].visibilities, 'private') })}${formField({ label: HostTexts.description, control: C.Input({ placeholder: HostTexts.optional }), hint: HostTexts.hostedHint(account.service) })}</div>`
      : '';
    const buttons = C.Button({ label: HostTexts.cancel, size: 'md' }) + C.Button({ label: HostTexts.create, icon: 'folder-plus', variant: 'primary', size: 'md' });
    return dialogShell({ name: 'NewRepositoryDialog', title: HostTexts.newTitle, description: HostTexts.newAbout, body: `<div class="form">${name}${folder}${toggle}${more}</div>`, buttons });
  },

  PublishDialog({ repository, branch = 'main', account, owner, problem, noAccount = false }) {
    if (noAccount) {
      const buttons = C.Button({ label: HostTexts.cancel, size: 'md' }) + C.Button({ label: HostTexts.addAccount, icon: 'plus', variant: 'primary', size: 'md' });
      return dialogShell({ name: 'PublishDialog', title: HostTexts.publishTitle(repository), description: HostTexts.publishNoAccount(repository), buttons });
    }
    const which = formField({ label: HostTexts.account, control: C.Select({ value: `${account.login} · ${account.service}`, lead: `<span class="prefix">${accountAvatar(account, 18)}</span>` }) });
    const where = `<div class="h" style="gap:8px;align-items:flex-start"><div class="v none" style="width:180px">${formField({ label: HostTexts.owner, control: C.Select({ value: owner.login }) })}</div><span class="slash">/</span><div class="v fill">${formField({ label: HostTexts.name, control: C.Input({ value: repository, focused: Boolean(problem) }), problem })}</div></div>`;
    const about = formField({ label: HostTexts.description, control: C.Input({ placeholder: HostTexts.optional }) });
    const shape = `<div class="h" style="gap:24px;align-items:flex-start">${formField({ label: HostTexts.visibility, control: visibilityChoice(owner.visibilities, 'private') })}${formField({ label: HostTexts.protocol, control: protocolToggle(Boolean(account.key)) })}</div>`;
    const buttons = C.Button({ label: HostTexts.cancel, size: 'md' }) + C.Button({ label: HostTexts.publish, icon: 'upload', variant: 'primary', size: 'md', disabled: Boolean(problem) });
    return dialogShell({ name: 'PublishDialog', title: HostTexts.publishTitle(repository), description: HostTexts.publishAbout(repository, branch), body: `<div class="form">${which}${where}${about}${shape}</div>`, buttons });
  },

  PullRow({ pull, selected = false }) {
    const look = pullLook(pull);
    const checks = pull.checks ? `<span class="none h" title="${esc(`Checks ${CheckLooks[pull.checks].word}`)}">${checkIcon(pull.checks, 'xs')}</span>` : '';
    const content = `<div class="h fill tree-row" title="${esc(PullTexts.rowTooltip(pull))}">${twisty(null)}${icon(look.icon, 'sm', look.color)}<div class="fill ellipsis sm">${esc(pull.title)}</div>${checks}<span class="none xs muted pull-number">#${pull.number}</span></div>`;
    return C.ListRow({ content, selected, name: 'PullRow' });
  },

  PullsSection({ pulls = [], filter = 'open', state = 'ready', selected, open = true, more = false, service = 'GitHub', server = 'github.com', login }) {
    const filterButton = C.Button({ icon: 'list-filter', label: filter === 'open' ? '' : PullTexts.filters[filter], variant: 'ghost', tooltip: PullTexts.filterTooltip });
    const header = C.SectionHeader({ title: PullTexts.section, count: state === 'ready' ? pulls.length : undefined, open, action: state === 'ready' ? filterButton : '' });
    const note = (glyph, text, action = '') => C.ListRow({ content: `<div class="h fill section-note xs muted" title="${esc(text)}">${glyph}<span class="fill ellipsis">${esc(text)}</span>${action}</div>`, name: 'ListRow' });
    const loadMore = more ? C.ListRow({ content: `<div class="h fill section-more xs muted">${esc(HostTexts.loadMore)}</div>` }) : '';
    const states = {
      loading: () => note(C.Spinner({ size: 'xs' }), PullTexts.reading),
      none: () => note(icon('user', 'xs'), PullTexts.noAccount(service, server), C.Button({ label: HostTexts.addAccountShort, variant: 'ghost' })),
      signedOut: () => note(icon('triangle-alert', 'xs'), PullTexts.signedOut(login), C.Button({ label: HostTexts.signInAgain, variant: 'ghost' })),
      choosing: () => note(icon('user', 'xs'), PullTexts.choosing),
      failed: () => note(icon('circle-x', 'xs'), PullTexts.failed(Failures.net.title, Failures.net.message), C.Button({ label: 'Retry', variant: 'ghost' })),
      ready: () => (pulls.length ? pulls.map((pull) => C.PullRow({ pull, selected: pull.number === selected })).join('') + loadMore : note('', PullTexts.none[filter])),
    };
    return `<div data-c="PullsSection" class="pulls-section">${treeGroup(open, header, states[state]())}</div>`;
  },

  PullFilterMenu({ filter = 'open', left = 0, top = 0, hot }) {
    const rows = Object.keys(PullTexts.filters)
      .map(
        (key) =>
          `<div class="item${key === hot ? ' hot' : ''}"><span class="check">${key === filter ? icon('check', 'sm') : ''}</span><span class="label">${esc(PullTexts.filters[key])}</span></div>`,
      )
      .join('');
    return `<div data-c="PullFilterMenu" data-base="PopupMenu" class="menu checks sm" style="left:${left}px;top:${top}px;min-width:120px">${rows}</div>`;
  },

  ChecksBox({ checks, state = 'ready' }) {
    if (state === 'loading') return `<div data-c="ChecksBox" class="pull-box"><div class="box-head">${C.Spinner({ size: 'sm' })}<span class="sm muted">${esc(PullTexts.checks)}</span></div></div>`;
    if (!checks.length) return `<div data-c="ChecksBox" class="pull-box"><div class="box-head">${icon('circle-minus', 'sm', 'var(--muted)')}<span class="sm muted">${esc(PullTexts.noChecks)}</span></div></div>`;
    const rows = checks
      .map(
        (check) =>
          `<div class="check-row">${checkIcon(check.state, 'sm')}<span class="sm none name">${esc(check.name)}</span><span class="xs muted fill ellipsis">${esc(check.description || '')}</span>${C.Button({ icon: 'external-link', variant: 'ghost', tooltip: 'Open on GitHub' })}</div>`,
      )
      .join('');
    return `<div data-c="ChecksBox" class="pull-box"><div class="box-head">${checkIcon(checkRollup(checks), 'sm')}<span class="sm medium">${esc(PullTexts.checks)}</span><span class="xs muted fill">${esc(PullTexts.checksSummary(checks))}</span></div>${rows}</div>`;
  },

  MergeBox({ pull, details, look: key = details.mergeability, running = false }) {
    const look = MergeLooks[key];
    const glyph = look.spin ? icon(look.icon, 'md', look.color).replace('class="icon"', 'class="icon spinner"') : icon(look.icon, 'md', look.color);
    const actions =
      key === 'merged'
        ? C.Button({ label: PullTexts.deleteBranch, icon: 'trash-2', size: 'sm', tooltip: PullTexts.deleteBranchTooltip })
        : key === 'closed'
          ? ''
          : C.Button({ label: PullTexts.merge, icon: 'git-merge', size: 'sm', variant: look.can ? 'primary' : 'default', disabled: !look.can || running, loading: running });
    return `<div data-c="MergeBox" class="pull-box merge-box"><div class="box-head tall">${glyph}<div class="v fill"><span class="sm medium">${esc(look.title(pull, details))}</span><span class="xs muted">${esc(look.about(pull, details))}</span></div>${actions}</div></div>`;
  },

  CommentRow({ comment }) {
    return `<div data-c="CommentRow" class="comment">${personAvatar(comment.author, 24)}<div class="v fill" style="gap:2px"><div class="h" style="gap:8px"><span class="sm medium">${esc(comment.author)}</span><span class="xs muted">${esc(comment.date)}</span></div><div class="sm comment-body">${esc(comment.body)}</div></div></div>`;
  },

  CommentBox({ value = '', posting = false, problem }) {
    const note = problem ? `<span class="xs problem fill">${esc(problem)}</span>` : `<span class="xs muted fill">${esc(PullTexts.plainText)}</span>`;
    return `<div data-c="CommentBox" class="comment-box">${C.Input({ value, placeholder: PullTexts.commentPlaceholder, lines: 3 })}<div class="h" style="gap:8px">${note}${C.Button({ label: PullTexts.comment, icon: 'message-square', size: 'sm', variant: 'primary', disabled: !value || posting, loading: posting })}</div></div>`;
  },

  PullOverview({ pull, details, state = 'ready', checksState = 'ready', merge, checkedOut = false, comment = {} }) {
    const look = pullLook(pull);
    const review = pull.review && pull.state === 'open' ? C.Chip({ label: ReviewLooks[pull.review].label, color: ReviewLooks[pull.review].color, icon: ReviewLooks[pull.review].icon }) : '';
    const branch = (name) => C.Chip({ label: pull.headOwner && name === pull.head ? `${pull.headOwner}:${name}` : name, icon: 'git-branch' });
    const head = `<span class="none">${branch(pull.head)}</span>`;
    const meta =
      `<div class="h pull-meta xs muted">${C.Chip({ label: look.label, color: look.color, icon: look.icon })}${review}` +
      `<span class="none">${esc(pull.author)}</span><span class="none">${pull.state === 'merged' ? 'merged' : 'wants to merge'}</span>${head}<span class="none">into</span><span class="none">${branch(pull.base)}</span></div>`;
    const checkOut = checkedOut
      ? C.Chip({ label: PullTexts.checkedOut, icon: 'check' })
      : C.Button({ label: PullTexts.checkOut, icon: 'git-branch', size: 'sm', tooltip: pull.headOwner ? PullTexts.checkOutFork(pull.number, pull.head) : PullTexts.checkOutTooltip(pull.head) });
    const actions = `<div class="h none" style="gap:6px">${pull.state === 'open' ? checkOut : ''}${C.Button({ icon: 'external-link', size: 'sm', variant: 'ghost', tooltip: PullTexts.openOn('GitHub') })}</div>`;
    const top = `<div class="pull-head"><div class="h" style="gap:12px;align-items:flex-start"><div class="v fill" style="gap:6px"><div class="lg semibold">${esc(pull.title)} <span class="muted">#${pull.number}</span></div>${meta}</div>${actions}</div></div>`;
    if (state === 'loading') return `<div data-c="PullOverview" class="pull-overview">${top}<div class="fill v">${C.Loading({ message: `Reading #${pull.number}…` })}</div></div>`;
    const body = `<div class="comment">${personAvatar(pull.author, 24)}<div class="v fill" style="gap:2px"><div class="h" style="gap:8px"><span class="sm medium">${esc(pull.author)}</span><span class="xs muted">${esc(PullTexts.opened(pull.created))}</span></div><div class="pull-body sm${details.body ? '' : ' muted'}">${esc(details.body || PullTexts.noDescription)}</div></div></div>`;
    const conversation = details.conversation.length ? details.conversation.map((each) => C.CommentRow({ comment: each })).join('') : `<div class="sm muted">${esc(PullTexts.noComments)}</div>`;
    const mergeBox = C.MergeBox({ pull, details, look: merge || (pull.state === 'open' ? (pull.draft ? 'draft' : details.mergeability) : pull.state) });
    const talk = `<div class="v" style="gap:12px"><div class="h sm semibold" style="gap:6px">${esc(PullTexts.conversation)}<span class="xs muted">${thousands(details.conversation.length)}</span></div>${conversation}${pull.state === 'open' ? C.CommentBox(comment) : ''}</div>`;
    return `<div data-c="PullOverview" class="pull-overview">${top}<div class="pull-scroll">${body}${C.ChecksBox({ checks: details.checks, state: checksState })}${mergeBox}${talk}</div></div>`;
  },

  PullFiles({ pull, details, state = 'ready', open, view = 'list', reason = '' }) {
    const files = details.files;
    const additions = files.reduce((sum, f) => sum + (f.additions || 0), 0);
    const deletions = files.reduce((sum, f) => sum + (f.deletions || 0), 0);
    const head = pull.headOwner ? `${pull.headOwner}:${pull.head}` : pull.head;
    const ready = state === 'ready';
    const facts = ready ? `<div class="h xs muted" style="gap:8px"><span class="mono">${esc(details.headSha)}</span><span class="ellipsis">${esc(PullTexts.sinceBase(details.commits, details.mergeBase))}</span></div>` : '';
    const stats = ready ? `<div class="h xs" style="gap:12px"><div class="muted">${counted(files.length, 'file', 'files')}</div>${C.Additions({ count: additions })}${C.Deletions({ count: deletions })}<div class="fill"></div>${C.FileViewToggle({ view })}</div>` : '';
    const header = `<div class="panel-header"><div class="base medium">${esc(PullTexts.changesIn(pull.number))}</div><div class="h xs muted" style="gap:6px">${C.Chip({ label: head, icon: 'git-branch' })}${icon('arrow-right', 'xs')}${C.Chip({ label: pull.base, icon: 'git-branch' })}</div>${facts}${stats}</div>`;
    const content =
      state === 'loading'
        ? C.Loading({ message: PullTexts.fetching(pull.number, 'origin') })
        : state === 'failed'
          ? C.Failure({ title: PullTexts.fetchFailed, reason })
          : C.CommitFiles({ files, open, view });
    return `<div data-c="PullFiles" class="commit-panel">${header}<div class="fill v">${content}</div></div>`;
  },

  CreatePullDialog({ head, base = 'main', title = '', body = '', draft = false, ahead = 0, unpublished = false, commits = 3, files = 5, problem, creating = false }) {
    const from = formField({ label: PullTexts.from, control: C.Select({ value: head, lead: `<span class="prefix">${icon('git-branch', 'sm')}</span>` }) });
    const into = formField({ label: PullTexts.into, control: C.Select({ value: base, lead: `<span class="prefix">${icon('git-branch', 'sm')}</span>` }) });
    const ends = `<div class="h" style="gap:8px;align-items:flex-end"><div class="v fill">${from}</div><span class="arrow">${icon('arrow-right', 'sm', 'var(--muted)')}</span><div class="v fill">${into}</div></div>`;
    const push = unpublished ? PullTexts.publishFirst(head, 'origin') : ahead ? PullTexts.pushFirst(head, ahead, 'origin') : '';
    const pushNote = push ? `<div class="h push-note xs">${icon('arrow-up-from-line', 'xs', 'var(--blue)')}<span>${esc(push)}</span></div>` : '';
    const name = formField({ label: PullTexts.title, control: C.Input({ value: title, focused: true }), problem });
    const about = formField({ label: PullTexts.description, control: C.Input({ value: body, placeholder: HostTexts.optional, lines: 5 }), hint: PullTexts.compared(commits, files) });
    const draftBox = `<div class="h" style="gap:8px">${C.Checkbox({ label: PullTexts.draft, checked: draft })}<span class="xs muted">${esc(PullTexts.draftAbout)}</span></div>`;
    const buttons = C.Button({ label: HostTexts.cancel, size: 'md' }) + C.Button({ label: PullTexts.create, icon: 'git-pull-request', variant: 'primary', size: 'md', disabled: !title || creating, loading: creating });
    return dialogShell({ name: 'CreatePullDialog', title: PullTexts.createTitle, description: PullTexts.createAbout('GitHub'), body: `<div class="form">${ends}${pushNote}${name}${about}${draftBox}</div>`, buttons, size: 'wider' });
  },

  MergePullDialog({ pull, details, method = details.method, problem }) {
    const choices = details.methods
      .map((key) => `<div class="method-choice${key === method ? ' chosen' : ''}">${C.Radio({ label: MergeMethodLabels[key].label, checked: key === method })}<span class="xs muted">${esc(MergeMethodLabels[key].about)}</span></div>`)
      .join('');
    const methodField = formField({ label: PullTexts.method, control: `<div class="v method-choices">${choices}</div>` });
    const squashTitle = `${pull.title} (#${pull.number})`;
    const mergeTitle = `Merge pull request #${pull.number} from sternegit-org/${pull.head}`;
    const squashMessage = ['* Render side-by-side diffs with syntax colors', '* Keep each side\'s scroll per file', '* Measure the exe after adding syntect'].join('\n');
    const words =
      method === 'rebase'
        ? `<div class="sm muted">${esc(PullTexts.rebaseNote(details.commits, pull.base))}</div>`
        : formField({ label: PullTexts.commitTitle, control: C.Input({ value: method === 'squash' ? squashTitle : mergeTitle }) }) +
          formField({ label: PullTexts.commitMessage, control: C.Input({ value: method === 'squash' ? squashMessage : pull.title, lines: 4 }) });
    const failure = problem ? `<div class="dialog-status">${icon('circle-x', 'md', 'var(--red)')}<span>${esc(problem)}</span></div>` : '';
    const buttons = C.Button({ label: HostTexts.cancel, size: 'md' }) + C.Button({ label: MergeMethodLabels[method].button, icon: 'git-merge', variant: 'primary', size: 'md', disabled: Boolean(problem) });
    return dialogShell({ name: 'MergePullDialog', title: PullTexts.mergeTitle(pull.number), description: PullTexts.mergeAbout(pull.head, pull.base, details.headSha), body: `<div class="form">${methodField}${words}</div>${failure}`, buttons, size: 'wider' });
  },

  SterneGit({ topBar, tabStrip = '', body, overlays = '', zoom = 100 }) {
    const zoomed = zoom === 100 ? '' : ` style="zoom:${zoom / 100};width:${144000 / zoom}px;height:${90000 / zoom}px"`;
    return `<div data-c="SterneGit" class="fg fg-window"${zoomed}>${topBar}${tabStrip}<div class="fill v">${body}</div>${overlays}</div>`;
  },
};
