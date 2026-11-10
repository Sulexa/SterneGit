# Vendored crates

## gpui-component 0.9.0

`gpui-component/` is the crates.io release of gpui-component 0.9.0, the styled
component library behind `gpui_kit::component`, with changes to its `Diff` and to
the colors of its dark theme.
`Cargo.toml` points at it with `[patch.crates-io]`. The crate's own `tests/`
folder and `Cargo.lock` were left out, and so was the `[[test]]` target that
needs them. Its unit tests don't build here either: they include assets, themes
and examples from the upstream repository that the release doesn't ship.

| Change | Where | Why |
| --- | --- | --- |
| `Diff::render_hunk_suffix` | `src/diff/mod.rs` | Puts Stage, Unstage and Discard hunk buttons at the right end of each `@@` header |
| `DiffState::refresh_files` | `src/diff/state.rs` | Re-reads a file after a hunk moves without jumping back to its top |
| `Diff::on_line_secondary_press`, `Diff::on_hunk_secondary_press` | `src/diff/mod.rs` | Tell the right-click menu which line or hunk header was pressed. `on_line_hover` can't: moving down from one line to the next reports the new line, then `None` for the old one |
| `window.viewport_size()` instead of `window.bounds().size` | `src/menu/`, `src/input/popovers/`, `src/window_border.rs` | Menus and popovers fit themselves into the window's layout size. `bounds()` is the window on the screen, which stops matching the layout once the window is zoomed (see gpui-pre below) |
| Hunk row above its own context, with the fold merged into it | `src/diff/state.rs` (`project_file`, `DisplayRow::Hunk::fold`), `src/diff/mod.rs` | One row between two hunks folds the unchanged lines and holds the hunk buttons. With the whole file shown it sits three lines above the change instead of right after the previous hunk |
| `DiffHunkSeparator::Scope`, `DiffHunkSeparator::ScopeAlways` | `src/diff/mod.rs` | The row shows the enclosing scope Git reports, without the `@@` ranges. With the whole file shown, `Scope` leaves nothing but the buttons, while `ScopeAlways` keeps the scope, for one the text doesn't show, such as the Unity object a YAML line belongs to |
| One horizontal scroll per column | `src/diff/mod.rs` (`CodeScroll`) | Long lines scroll inside each column, both columns together, with the line numbers kept in place; each column has its scrollbar |
| Changed lines tinted from the code on, their numbers in the change color, missing lines hatched | `src/diff/mod.rs` | The gutter stays neutral beside `DiffChangeIndicator::Bars` |
| Coherent syntax names | `src/diff/code_names.rs`, `src/diff/presentation.rs` | On top of syntect, in code: capitalized names and built-in types are types, names called with `(` or `!` are functions, numbers, booleans and CONSTANTS are literals, `self` and `Self` are keywords, and string placeholders stay in their string |
| Default Dark syntax palette | `src/theme/default-theme.json` | Keywords violet, types sand, functions blue, strings cyan, literals pink, comments grey italic, so code colors stay clear of the red and green of changes |

`gpui-component.patch` holds exactly these changes against the release.

### Moving to a newer release

1. Copy the new release from `~/.cargo/registry/src/*/gpui-component-<version>/`
   over `gpui-component/`, then delete `tests/`, `Cargo.lock`, `Cargo.toml.orig`,
   `.cargo-ok`, `.cargo_vcs_info.json` and the `[[test]]` section of `Cargo.toml`.
2. Apply the changes again from the `gpui-component` folder:

   ```bash
   git apply -p1 ../gpui-component.patch
   ```

3. Once a release offers these additions upstream, delete this folder and the
   `[patch.crates-io]` section.

## gpui-pre 0.3.11

`gpui-pre/` is the crates.io release of gpui-pre 0.3.11, the GPUI core behind
`gpui_kit::*`, with a content zoom added to `Window`. `Cargo.toml` points at it
with `[patch.crates-io]`. The crate's `examples/`, `tests/`, `benches/`,
`Cargo.lock` and `Cargo.toml.orig` were left out, and so were the `[[example]]`
and `[[test]]` targets that need them.

| Change | Where | Why |
| --- | --- | --- |
| `Window::content_zoom`, `Window::set_content_zoom` | `src/window.rs`, `src/window/zoom.rs` | Zooms everything a window draws, like a browser's zoom: the window reports a scale factor `zoom` times larger and a viewport `zoom` times smaller, so text, pictures and hit testing all follow. SterneGit's Ctrl + wheel and Settings → Interface set it |
| Input from the platform moved into zoomed units | `src/window.rs` (`on_input`), `src/window/zoom.rs` | Mouse, wheel, touch and file-drop positions arrive in platform units. Only events from the platform are converted: GPUI dispatches events of its own that are already in window units |
| Lengths sent to the platform moved out of zoomed units | `src/window.rs`, `src/platform.rs` | `resize`, the IME candidate box, the window menu, client insets, corner radii, input regions and the macOS traffic lights take platform units; insets and the visual viewport come back from it |

`gpui-pre.patch` holds exactly these changes against the release.

### Moving to a newer release

1. Copy `src/`, `resources/`, `docs/`, `build.rs`, `README.md`, `LICENSE-APACHE`
   and `Cargo.toml` from `~/.cargo/registry/src/*/gpui-pre-<version>/` over
   `gpui-pre/`, then delete the `[[example]]` and `[[test]]` sections of
   `Cargo.toml`. Every `gpui-pre-*` crate must stay at the same version.
2. Apply the changes again from the `gpui-pre` folder:

   ```bash
   git apply -p1 ../gpui-pre.patch
   ```

3. Once GPUI offers a window zoom upstream, delete this folder and its line in
   `[patch.crates-io]`.
