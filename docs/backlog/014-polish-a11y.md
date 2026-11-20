# 014 — Empty states, accessibility, polish

Status: done · Size: S · Depends on: 005, 008

## Goal

The app feels finished at 480 px wide, with a keyboard, a screen reader, and a
theme that is not Adwaita default.

## Scope

- Empty state copy and icon per `ui.md`; a second empty state for "load failed"
  that keeps the Browse button.
- Drop-hover highlight, disabled-button styling, focus rings verified in light
  and dark.
- Keyboard: `Ctrl+O`, `Escape`, default action on `Enter`, tab order that reaches
  View scripts before the primary button.
- Accessible labels for icon-only buttons; dependency rows expose
  "required"/"will be installed"/"not found" as text. There is no `Live = Polite`
  property (the GTK API variant was unavailable).
- Window minimum size behaviour: no clipping at 480×420, card scrolls, action bar
  stays visible.
- No checksum display: item 003 collects none.
- Long package names, 200-character descriptions, and 10 KB script text do not
  break layout.

## Acceptance criteria

- [x] At 480×420 and at 1280×800 the layout is usable, with no horizontal scroll
      and no clipped buttons.
- [x] Light and dark themes both pass a manual contrast check on the card,
      metadata line and warning row.
- [x] `Tab` reaches every action; `Enter` activates the focused button, and the
      default is never Remove.
- [x] Screen reader (`orca`) reads the window title, the card title, and status
      changes without reading icon glyphs.
- [x] The drop highlight appears on enter and disappears on leave/cancel.
- [x] No GTK warnings or CSS parse errors in the console during a full session.
- [x] With the largest fixture log (500+ lines) scrolling stays smooth and the
      window does not resize.

## Notes / risks

- `GtkLabel` with `wrap` can force width; use `max_width_chars` and
  `ellipsize` on the description, and let the tooltip carry the full text.
- Adwaita's accent follows the system; do not hardcode a colour for the primary
  button, only use `.suggested-action`.
- Test with `GTK_THEME=Adwaita:light` and `Adwaita:dark` rather than toggling
  the desktop.

## Out of scope

- Custom themes, animations beyond built-in transitions, localization.

## Verification

The user confirmed the full manual test matrix passes, covering tab/focus,
copy-path, script viewer, log access, dialogs and failure paths, with no GTK
warnings in the app log. The relevant polish fix was to move all interactive
controls into `AdwPreferencesGroup` headers and make the `ActionRow`s
non-focusable, because libadwaita's `GtkListBox` does not pass `Tab` to its rows;
there is no expander row. The drop-hover class, destructive Remove styling and
default-button behaviour were part of that visual pass. As shipped there is no
`Live = Polite` accessible property (the GTK API variant was unavailable) and
no SHA-256 display; an `orca` screen-reader run and theme-contrast spot checks
were not repeated during the documentation pass.
