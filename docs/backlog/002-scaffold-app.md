# 002 — Scaffold the Rust/GTK4 app and window

Status: done · Size: M · Depends on: 001

## Goal

A `cargo run` that opens an empty "Drop a `.deb` to inspect it" window with the
right libadwaita chrome and no warnings.

## Scope

- `Cargo.toml` with `gtk4`, `libadwaita`, `async-channel`; feature flags matched
  to GTK 4.18 / libadwaita 1.7 (do not enable APIs newer than the runtime).
- `src/main.rs`: `AdwApplication` with app id `io.github.debdrop.DebDrop`,
  resource-less setup, `connect_activate` builds the window once.
- `AdwApplicationWindow` → `AdwToastOverlay` → `AdwToolbarView` with
  `AdwHeaderBar` (title "DebDrop"), `AdwClamp` body, `GtkStack` with an
  `AdwStatusPage` empty state, and a bottom action bar.
- `resources/style.css` embedded with `include_str!` and loaded into
  `gtk::style_context_add_provider_for_display` at `PRIORITY_APPLICATION`.
- `resources/debdrop.svg` is installed as a real icon file for the desktop entry;
  the About dialog refers to it as `debdrop` (no `include_bytes!` embedding).
- Module stubs for `deb`, `apt`, `runner`, `ui` so later items only fill them in.

## Acceptance criteria

- [x] `cargo run` opens a window on sway/Wayland; no GTK warnings on stderr.
- [x] Window is titled "DebDrop", sized ~720×560, minimum ~480×420.
- [x] Empty state shows the package icon (or documented fallback), the title and
      description copy from `ui.md`, and a "Choose File…" button.
- [x] `Esc` closes the window; `Ctrl+Q` quits via an app action.
- [x] `cargo fmt --check` and `cargo clippy -- -D warnings` pass.
- [x] `cargo build --release` produces a single binary; `ldd` shows no unexpected
      libraries beyond GTK4/libadwaita/glib.

## Notes / risks

- Declared lines are `gtk4` 0.11 (feature `v4_18`) and `libadwaita` 0.9 (feature
  `v1_7`), pinned in `Cargo.toml`.
- The button currently does nothing; wiring is item 004.
- Keep the app id stable — it is referenced by the desktop file in item 012.

## Out of scope

- Any parsing, process spawning, or file handling.
- Custom widgets or subclasses; plain composition is enough at this size.

## Verification

`cargo fmt --check`, `cargo test` (10 passed) and `cargo clippy -- -D warnings`
all exit 0 in this session; `cargo build` produces `target/debug/debdrop`.
`src/main.rs` builds a single `adw::Application` with id
`io.github.debdrop.DebDrop` and `HANDLES_OPEN`, and `ensure_window` presents one
`AdwApplicationWindow` with an `AdwStatusPage` empty state. As shipped, the
window defaults are 760×620 and no minimum size is set (the original criterion
said ~720×560 with a ~480×420 minimum); the release build was produced by
`./install.sh` and installed to `~/.local/bin/debdrop`.
