# Vendored crates

## gpui-component 0.9.0

`gpui-component/` is the crates.io release of gpui-component 0.9.0, the styled
component library behind `gpui_kit::component`, with a few additions to its `Diff`.
`Cargo.toml` points at it with `[patch.crates-io]`. The crate's own `tests/`
folder and `Cargo.lock` were left out, and so was the `[[test]]` target that
needs them.

| Change | Where | Why |
| --- | --- | --- |
| `Diff::render_hunk_suffix` | `src/diff/mod.rs` | Puts Stage, Unstage and Discard hunk buttons at the right end of each `@@` header |
| `DiffState::refresh_files` | `src/diff/state.rs` | Re-reads a file after a hunk moves without jumping back to its top |
| `Diff::on_line_secondary_press`, `Diff::on_hunk_secondary_press` | `src/diff/mod.rs` | Tell the right-click menu which line or hunk header was pressed. `on_line_hover` can't: moving down from one line to the next reports the new line, then `None` for the old one |

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
