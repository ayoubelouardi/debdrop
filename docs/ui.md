# UI

One `AdwApplicationWindow`, system light/dark and accent, no custom palette.
The content area is a `GtkStack` with three pages: **empty**, **card**, and
**busy**. There are no tabs and no expander rows.

## Window states

```
 Empty ──drop / pick / %f──▶ Card ──primary action──▶ Busy ──exit──▶ Card + toast/banner
   ▲                          │                        │                    │
   └──── new file replaces ───┴────────────────────────┴────────────────────┘
                              (Busy blocks close and file loading)
```

| State | Stack page | Meaning |
| --- | --- | --- |
| `Empty` | `empty` | `AdwStatusPage`: package icon, "Drop a .deb to inspect it", "Nothing is installed until you click Install.", pill "Choose File…" button |
| `Card` | `card` | Valid `.deb` loaded; metadata, plan and script names shown; bottom action bar visible |
| `Busy` | `busy` | Child process running (install/remove/refresh); spinner + phase label + live log |

On success the app switches back to the card, re-queries installed state and the
plan, and shows a toast such as "Installed foo 1.2.3". On failure it returns to
the card and reveals a banner with a one-line hint plus a **Details** button (or
**Refresh lists** when the plan says the indexes are stale). Auth cancellation
(exit 126/127) is a toast, "Installation cancelled", not an error banner.

## Window layout

```
┌────────────────────────────────────────────────────────────┐
│ DebDrop                       [open icon] [menu ▾]  − □ × │  AdwHeaderBar
├────────────────────────────────────────────────────────────┤
│ (banner: one-line hint + [Details] / [Refresh lists])       │  AdwBanner
├────────────────────────────────────────────────────────────┤
│  AdwClamp (max 780) → GtkStack                             │
│  Card page: AdwPreferencesPage                             │
│   ┌────────────────────────────────────────────────────┐   │
│   │ Package                                 [copy ⧉]   │   │
│   │   foo  ·  all  ·  utils  ·  1.2 MiB installed     │   │
│   │   File path            /abs/path/foo_1.0.deb      │   │
│   │   Maintainer           Name <email>               │   │
│   │   Description          long text…                 │   │
│   │   Status               Installed 1.0.0            │   │
│   │   Configuration        Asks configuration…        │   │  (only if debconf)
│   ├────────────────────────────────────────────────────┤   │
│   │ Dependencies                [Show]/[Hide]          │   │
│   │   (group description = plan headline)              │   │
│   │   libc6 (>= 2.34)   required                      │   │
│   │   libfoo3           will be installed              │   │
│   ├────────────────────────────────────────────────────┤   │
│   │ Maintainer scripts                [View…]          │   │  (hidden if none)
│   │   ⚠ postinst                                      │   │
│   │     Runs with root privileges during installation  │   │
│   └────────────────────────────────────────────────────┘   │
├────────────────────────────────────────────────────────────┤
│ [Choose File…]                       [Remove]  [Install]   │  bottom bar
└────────────────────────────────────────────────────────────┘
```

The header bar's flat icon button opens the file picker (tooltip "Open a .deb
file (Ctrl+O)"). The menu contains **Open…**, **Refresh package lists**, **Show
last log**, and **About DebDrop**. Standard minimize/maximize/close controls are
provided by `AdwHeaderBar`.

The bottom bar is `AdwToolbarView`'s bottom bar (`add_bottom_bar`). It contains
**Choose File…**, a **Remove** button that is only visible when the package is
installed (`.destructive-action`, never the default), and the primary button
(`.suggested-action`, `set_receives_default(true)`, and also the window's default
widget). Its label is one of **Install**, **Reinstall**, **Upgrade to {version}**,
or **Downgrade to {version}**; it stays disabled until both the installed-state
check and the dependency simulation have returned successfully. Because it is
the default widget, `Enter` activates it.

The three groups are plain `AdwPreferencesGroup`s. **Rows inside the list box
are not tab stops** — libadwaita's `GtkListBox` does not pass `Tab` to its rows,
and each row is explicitly made non-focusable. Consequently every interactive
control lives in a group header:

- **Package** group header: a copy-path icon button. Clicking copies the
  absolute `.deb` path to the clipboard and changes its tooltip to "Copied".
- **Dependencies** group header: a **Show**/**Hide** button, visible only when
  there are rows to hide. The group's description is the plan headline
  ("All dependencies are available", "N package(s) will be installed with it",
  "Cannot resolve: …", "Package lists may be out of date", etc.). Rows use text
  such as "required", "or …", "will be installed", "will be upgraded",
  "will be downgraded", "will be removed", "not found", plus icons; warnings
  carry the `.debdrop-warning` class. There is no `AdwExpanderRow`.
- **Maintainer scripts** group header: a **View…** button. The group is hidden
  entirely when the package has no maintainer scripts; otherwise its row lists
  the script names (e.g. `postinst`) with the subtitle "Runs with root privileges
  during installation" and a warning icon.

## Busy view

The busy page replaces the card in the stack. It always shows, with no Details
expander:

```
        ⟳   Waiting for authentication… / Resolving dependencies… / …
                    (GtkSpinner + phase label)

        ┌──────────────────────────────────────────────┐
        │ $ /usr/bin/pkexec /usr/bin/env …             │
        │ Reading package lists...                     │
        │ Unpacking foo (1.0.0) ...                    │
        │ Setting up foo (1.0.0) ...                   │
        └──────────────────────────────────────────────┘  GtkTextView, live
```

The label starts at "Waiting for authentication…" and advances through
"Resolving dependencies…", "Downloading packages…", "Unpacking…", and
"Configuring…" as matching lines arrive. The log view is a read-only,
monospace `GtkTextView` that auto-scrolls while the user is at the bottom and
keeps the last 5 000 lines. While busy, all action buttons are disabled and
closing the window is guarded.

## Dialogs

- **Remove confirmation** (`AdwAlertDialog`): "Remove {package}?" with the
  installed version, a note that dependents may be removed too and that
  dependencies are kept. **Cancel** is the default response; **Remove** is
  destructive.
- **Scripts viewer** (`AdwDialog`): a dropdown listing the script names
  (`postinst`, etc.) above a read-only monospace text view, with a note that
  the scripts run as root. Contents come from the extracted control directory.
- **Last operation** (`AdwDialog`): the raw log of the last operation, or the
  simulation detail when no transaction has run; read-only and monospace.
- **About** (`AdwAboutDialog`): name, icon, version, MIT license, and the
  reminder that maintainer scripts run as root and nothing is installed without
  a click and a password prompt.
- **Close guard** (`AdwAlertDialog`): when closing during an operation,
  "Package operation in progress" warns about `sudo dpkg --configure -a` and
  offers **Keep running** (default) or **Quit anyway** (destructive). "Quit
  anyway" SIGTERMs the child process group.

## Copy

Short and factual; no "Oops", no exclamation marks. Notable strings:

| Slot | Text |
| --- | --- |
| Empty title | Drop a `.deb` to inspect it |
| Empty body | Nothing is installed until you click Install. |
| Primary (new) | Install |
| Primary (same version) | Reinstall |
| Primary (older installed) | Upgrade to {version} |
| Primary (newer installed) | Downgrade to {version} |
| Remove button | Remove |
| Scripts row subtitle | Runs with root privileges during installation |
| Debconf note | Asks configuration questions; defaults will be used |
| Deps satisfied | All dependencies are available |
| Deps installing | {n} package(s) will be installed with it |
| Deps unresolved | Cannot resolve: {names} |
| Stale lists | Package lists may be out of date |
| Busy (pre-auth) | Waiting for authentication… |
| Auth cancelled | Installation cancelled |
| Locked | Another package manager is busy |
| Success toast | {Installed,Reinstalled,Upgraded,Downgraded} {name} {version} |
| Remove success | Removed {name} |
| Refresh success | Package lists refreshed |

Toasts are also used for an invalid file (the validator's message), for "Drop
a single .deb file", and for "Wait for the current operation to finish". There
is no "Copy log" button and no "Retry" button anywhere; the raw log is reached
through **Show last log** or the banner's **Details** button.

## Keyboard and focus

| Input | Action |
| --- | --- |
| `Ctrl+O` | Open the file picker (`app.open`) |
| `Ctrl+Q` | Quit (`app.quit`) |
| `Tab` | Reaches the copy-path button, Dependencies Show/Hide, Maintainer scripts View…, the header Open button and menu, then Choose File…, Remove (if visible), and the primary button. Card rows are skipped. |
| `Enter` | Activates the focused button; the primary button is the window default, so with nothing else focused `Enter` starts the planned action |
| `Escape` | Closes dialogs; the close-during-operation guard defaults to "Keep running" |

Icon-only buttons (copy path, Open) have tooltips and accessible labels. The log
view is labelled "Installation log". Dependency outcomes are written as words
in row subtitles, not conveyed by icon colour alone. There is no
`Live = Polite` accessible property on the phase label: the GTK API variant
needed for it was not available, so the announcement is purely visual.

## CSS surface

`resources/style.css` is embedded with `include_str!` and loaded at application
priority. The only custom classes are:

```css
.drop-hover    { background: alpha(currentColor, 0.06); border-radius: 12px; }
.debdrop-log   { font-family: monospace; font-size: 0.86em; }
.debdrop-meta  { opacity: 0.65; }
.debdrop-warning { color: @warning_color; }
.debdrop-path  { opacity: 0.55; font-size: 0.9em; }
```

`.drop-hover` is toggled on the stack by `GtkDropTarget::enter`/`leave`. There
is no `.warning-row` class. Everything else is stock libadwaita.

## Drag and drop

A `GtkDropTarget` for `gdk::FileList` is attached to the stack. Entering
highlights the stack with `.drop-hover`; dropping exactly one file loads it;
dropping more than one shows a toast and does nothing. On Wayland file managers
offer `text/uri-list`, which GDK converts to `gdk::FileList`.

## Non-goals for the UI

No fake progress percentage, no auto-close on success, no batch queue UI, no
hidden advanced options, no dark-pattern "Install and close" flow.
