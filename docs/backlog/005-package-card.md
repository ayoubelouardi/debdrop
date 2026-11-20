# 005 — Package card and script viewer

Status: done · Size: M · Depends on: 003, 004

## Goal

See everything that matters about a package — and what will run as root — before
deciding to install.

## Scope

- `AdwPreferencesPage` → `AdwPreferencesGroup` card per `ui.md`:
  - leading icon (`application-x-deb`, fallback `package-x-generic`),
    title = package name, subtitle = version,
    metadata line = arch · section · installed size.
  - maintainer row.
  - long description in a wrapping `GtkLabel` (`wrap`, `xalign 0`, max ~4 lines
    with ellipsize then full text in a tooltip).
  - Dependencies group of plain `ActionRow`s (populated by item 007; here render
    the declared `Depends` as rows with the constraint text).
  - "Runs N maintainer scripts as root" warning row with a **View** button, shown
    only when `scripts.any()`; debconf note when `has_debconf`.
- Script viewer dialog: `AdwDialog` with a read-only monospace `GtkTextView` and
  a tab/selector per script. Extract via `dpkg-deb -e` into a 0700 temp dir
  (via `tempfile`), read files, delete the dir when the dialog closes.
- Action bar: primary button (label from item 006) and, when installed,
  **Remove**; both disabled until the card is fully populated.
- Empty/loaded stack switching plus a subtle fade (`AdwSpringAnimation` not
  needed; a `GtkStack` transition is enough).

## Acceptance criteria

- [x] The fixture card shows the same values as `dpkg-deb -f` (spot-check each
      field, including a package with no `Section` or `Maintainer`).
- [x] A package with `postinst` shows the warning row; one without shows no row.
- [x] The viewer displays the script byte-for-byte (compare with
      `dpkg-deb -e … && cat`), scrolls, and cannot be edited.
- [x] Closing the dialog removes the temp dir (`ls /tmp` before/after).
- [x] A package with a very long description does not stretch the window or
      clip mid-word.
- [x] The card scrolls instead of growing past the window at 480×420.

## Notes / risks

- `dpkg-deb -e` writes files owned by the user; names are fixed (`postinst`,
  …). Do not glob the directory into the viewer without filtering to known
  script names — a crafted control archive can contain arbitrary member names.
- Size formatting: `Installed-Size` is KiB; show MiB when ≥ 1024 with one
  decimal. Do not imply download size.
- Every package-supplied value passes through `markup_escape_text` before being
  set as Pango markup; the group header's copy button copies the file path.

## Out of scope

- Dependency satisfaction colours (007), primary action semantics (006).
- Syntax highlighting of shell scripts.

## Verification

The shipped card renders the fixture metadata, dependency/plan rows and script
names, confirmed visually. The maintainer-scripts dialog shows `postinst` from
the extracted control directory. The footer buttons and the group-header
actions (copy path, Show/Hide, View…) were verified visually. As shipped, the
metadata is presented by `AdwPreferencesGroup`/`ActionRow`s with Pango markup
escaping; the shipped UI uses plain groups/rows rather than an expander row
(see `docs/ui.md`). Extraction uses a
private `TempDir` held by `DebInfo`, so it is removed when the `DebInfo` is
dropped rather than on dialog close.
