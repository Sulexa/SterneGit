# SterneGit mockup

A reference for what every SterneGit screen looks like, built from the same reusable
pieces as the Rust code. Open `index.html` in a browser: there is no build step and
nothing to install.

- **Screens** shows each state of the window at 1440 × 900, its default size.
  **Outline components** (or the `o` key) draws the outline of every component on
  the screen; hover one to see its name and source file, click it to jump to it.
  ← and → move between screens, and **Open alone** shows a screen with nothing
  around it, for screenshots. Click a section or a folder in the refs list or in a
  file tree to fold or unfold it.
- **Components** shows every reusable piece with its variants, the Rust type or
  function it stands for, the file it lives in, what it is built from, and the
  screens that use it. Those last two lists are worked out from the screens, so
  they stay right on their own.

## Files

| File | Holds |
| --- | --- |
| `app.css` | The theme tokens (gpui-kit's Default Dark) and the look of every component |
| `components.js` | One function per component, named after its Rust counterpart |
| `screens.js` | Each screen, composed from the components |
| `catalog.js` | The component pages: variants, Rust name, source file, description |
| `data.js` | The sample repository every screen shows |
| `viewer.js`, `viewer.css`, `index.html` | The page around the mockup |

## Keeping it up to date

Change the mockup in the same commit as the UI change it mirrors.

- **A component changes:** edit its function in `components.js` (and its styles in
  `app.css`). Every screen that uses it follows.
- **A new component:** add a function to `C` in `components.js` whose root element
  carries `data-c="Name"`, then an entry under the right group in `catalog.js`.
- **A new screen or state:** add an entry to `screens.js`.
- **A theme change:** edit the tokens at the top of `app.css`.

`cargo test` checks that every `source:` path in the mockup still names a file in
`src/`, so moving or renaming a file reminds you to update it.
