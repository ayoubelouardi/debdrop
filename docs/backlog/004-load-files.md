# 004 — Load files: drag & drop, picker, `%f`

Status: done · Size: M · Depends on: 003

## Goal

Get a `.deb` into the app three ways — drop, dialog, `argv` — with the same
validation and the same result.

## Scope

- `GtkDropTarget::new(gdk::FileList::static_type(), DragAction::COPY)` on the
  clamp/stack container; `connect_accept` rejects anything that is not exactly
  one `.deb`; `connect_drop` loads it.
- `enter`/`leave` toggle the `.drop-hover` CSS class on the drop area.
- `GtkFileDialog` with a `application/vnd.debian.binary-package` + `*.deb`
  filter, opened from the empty-state button, the header "Open…" button, and
  `Ctrl+O`.
- `AdwApplication::connect_open` (or `connect_activate` argv inspection) so
  `debdrop foo.deb` and the desktop file's `%f` load that file at startup.
- Replace the stack child: Empty → Loaded on success; failures show an inline
  banner/toast and keep the previous state if one existed.
- Switch the state enum (`Empty | Loaded | Busy | Result`) with transitions in
  `ui/window.rs`; `busy` refuses to load a new file.

## Acceptance criteria

- [x] Dropping a valid `.deb` from Nautilus/Thunar shows the container with its
      filename and version (card is item 005; a placeholder row is fine here).
- [x] Dropping two files at once is rejected with a toast, not a partial load.
- [x] Dropping a `.txt`, a folder, or a symlink to a directory is rejected
      without spawning `dpkg-deb`.
- [x] File picker filters to `.deb` but a manually typed non-`.deb` path still
      reaches validation and is rejected cleanly.
- [x] `cargo run -- fixtures/hello-debdrop_1.0.0_all.deb` loads it on start.
- [x] Dropping a new file while one is loaded replaces it without a dialog.
- [x] No file is loaded automatically on launch when no argument is given.

## Notes / risks

- On Wayland, file managers offer `text/uri-list`; GDK converts it to
  `gdk::FileList`. Verify with a real file manager, not only a test harness.
- `FileList` can be empty on some drag sources; treat that as a rejection.
- Keep the last-used directory in memory for the dialog only (no persistence).
- `deb::load` is a small set of `dpkg-deb` calls and runs synchronously; there is
  no background hashing or lazy field filling.

## Out of scope

- Reading metadata into the card (005), installed state (006), long-press or
  context menus.

## Verification

`GtkDropTarget`, the `GtkFileDialog` picker (empty-state button, header Open
button, `Ctrl+O`), and `GApplication::HANDLES_OPEN` → `connect_open` →
`load_path` are all wired in `src/ui/window.rs` and `src/main.rs`. Launching a
second instance with a fixture filename loaded that file into the existing
window rather than creating a second window. Rejection of non-`.deb` input is
implemented in `deb::load` and surfaced as a toast with the validator's message.
The drag source was a file manager during manual verification, not a synthetic
test harness.
