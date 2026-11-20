# Architecture

## Shape

One process, one window, no daemon, no helper service, no async runtime crate.
GTK owns the main thread. Root work is anything that writes to dpkg/apt state:
install/reinstall/upgrade/downgrade, remove, and "Refresh package lists". All of
it goes through `pkexec`. Reads (`dpkg-deb`, `dpkg-query`, `dpkg --compare-versions`,
`apt-get -s`) run as the normal user.

```
  drop / picker / %f argv (GApplication::open)
           │
           ▼
    deb.rs ── dpkg-deb --info / -f / -e ──▶ DebInfo { … }
           │                                 │
           │                                 ▼
           │                         ui/package_view (card)
           ▼
    apt.rs ── dpkg-query -W, dpkg --compare-versions
           │
           └── apt-get -s install --no-install-recommends <abs deb>
                                   │ (std thread → async_channel)
                                   ▼
                                Plan { … }
                                   │
                              [ Install ] clicked
                                   ▼
    runner.rs ── pkexec /usr/bin/env … /usr/bin/apt-get install -y <abs deb>
                                   │  Event stream (async_channel)
                                   ▼
                       ui/window.rs → ui/log_view + phase label + toast
```

## Modules

| File | Responsibility | May not |
| --- | --- | --- |
| `src/main.rs` | `AdwApplication` (id `io.github.debdrop.DebDrop`), `HANDLES_OPEN`, CSS load, single window ownership | contain apt command strings |
| `src/deb.rs` | Validation, `DebInfo`, control-field and dependency parsing, maintainer-script extraction | spawn privileged commands |
| `src/apt.rs` | Build argv, parse simulate plan, compute installed state/action, classify failures | touch UI types |
| `src/runner.rs` | Spawn child process groups, stream/capture output, define `Event` | interpret apt semantics |
| `src/ui/window.rs` | State machine, threading bridge, phase detection, dialogs, toasts, close guard | build apt argv |
| `src/ui/package_view.rs` | Metadata card, dependencies group, maintainer-scripts group | run processes |
| `src/ui/log_view.rs` | Monospace, read-only, auto-scrolling log buffer | — |

Rule of thumb: if a module needs both a `gtk::` type and a child process, the
design is wrong. `ui/window.rs` is the bridge: it moves results from std threads
onto the GTK main loop with `glib::spawn_future_local`.

## Data

```rust
pub struct DebInfo {
    pub path: PathBuf,
    pub package: String,             // Package
    pub version: String,             // Version
    pub architecture: String,        // Architecture
    pub section: Option<String>,
    pub priority: Option<String>,
    pub maintainer: Option<String>,
    pub installed_size_kb: Option<u64>,
    pub description: String,         // first line
    pub description_long: Option<String>, // remaining indented lines
    pub depends: Vec<Dep>,
    pub recommends: Vec<Dep>,
    pub scripts: Vec<String>,        // subset of preinst/postinst/prerm/postrm
    pub has_debconf: bool,           // config or templates present
    ctrl: TempDir,                   // private; kept alive for script_text()
}

pub struct Dep {
    pub name: String,
    pub constraint: Option<String>,  // e.g. ">= 2.34", kept as raw text
    pub alternatives: Vec<String>,   // names after `|`
}
// There is no comparison-operator enum: the constraint is display-only text.

pub struct Plan {
    pub new: Vec<String>,
    pub upgraded: Vec<String>,
    pub downgraded: Vec<String>,
    pub removed: Vec<String>,
    pub unresolved: Vec<String>,
    pub summary: Option<String>,     // apt's "N upgraded, M newly installed…" line
    pub detail: String,              // full simulate stdout + stderr
    pub timed_out: bool,
    pub code: i32,
    pub ok: bool,
    pub stale_lists: bool,
}
```

`DebInfo` is produced only from `dpkg-deb` output; the filename is never parsed
for metadata. The private `ctrl: TempDir` is created by `tempfile` and held for
the whole lifetime of `DebInfo`, so the maintainer-script viewer can call
`script_text()` at any time. It is removed automatically when the `DebInfo` is
dropped (loading another file, or quitting). Only the four known script names
are extracted into the viewer; the archive member list is never globbed blindly.

There is no SHA-256 or other checksum anywhere in the program, and no
`glib::Checksum` dependency.

## Command contract

Every external command is an argv vector passed to `std::process::Command`.
No `sh -c`, no interpolation, no string-built paths. Read commands are
unprivileged; root commands go through `pkexec` with absolute program paths.

| Purpose | argv |
| --- | --- |
| Validity gate / listing | `dpkg-deb --info <abs deb>` |
| Control fields | `dpkg-deb -f <abs deb>` |
| Extract control | `dpkg-deb -e <abs deb> <tempdir>` |
| Installed state | `dpkg-query -W '-f=${db:Status-Abbrev}\t${Version}\n' <pkg>` |
| Version order | `dpkg --compare-versions <a> lt <b>` (also `eq`) |
| Host architecture | `dpkg --print-architecture`, `dpkg --print-foreign-architectures` |
| Dry run | `apt-get -s install --no-install-recommends <abs deb>` |
| Install / reinstall / upgrade / downgrade | **root** `/usr/bin/pkexec /usr/bin/env DEBIAN_FRONTEND=noninteractive APT_LISTCHANGES_FRONTEND=none LC_ALL=C /usr/bin/apt-get install -y --no-install-recommends [--reinstall] [--allow-downgrades] <abs deb>` |
| Remove | **root** same pkexec/env prefix, `... /usr/bin/apt-get remove -y <pkg>` |
| Refresh lists | **root** same pkexec/env prefix, `... /usr/bin/apt-get update` |

`--reinstall` is added only for `Action::Reinstall`; `--allow-downgrades` only
for `Action::Downgrade` (`apt::install_argv`). `LC_ALL=C` is set on every child
whose output is parsed. `stdin` is `Stdio::null()` for all children; combined
with `DEBIAN_FRONTEND=noninteractive` and `APT_LISTCHANGES_FRONTEND=none`,
debconf questions take their defaults and apt never waits for input. The UI
surfaces the "Configuration" row when `has_debconf` is true.

## Threading and events

- Main thread: GTK only (including all `adw::`/`gtk::` widget calls).
- `apt::installed_state`/`action_for` and `apt::simulate` are dispatched on
  short-lived `std::thread`s. Each sends its result through an
  `async_channel::unbounded` sender; `ui/window.rs` consumes it with
  `glib::spawn_future_local`. A monotonically increasing sequence counter
  discards results from a previous load when a second file is opened quickly.
- One child transaction at a time is enforced by the UI's `busy` flag: while
  busy, the action buttons are disabled, new files are refused with a toast,
  and the close button is guarded.
- `runner::Event` has exactly two variants: `Line { text: String }` and
  `Exited { code: i32 }`. There is no `stream`, `Started`, or `Failed` variant.

`runner::spawn()` starts the child in its own process group
(`Command::process_group(0)`), pipes stdout and stderr, and starts one reader
thread per stream. Each reader splits on both `\n` and `\r`, strips ANSI CSI/OSC
escapes and other control characters, and caps a line at 4 KiB. A reaper thread
waits for the child and then **joins both reader threads** before sending
`Exited { code }`, so the exit event is guaranteed to be last and no log line is
lost. `runner::terminate(pid)` sends `SIGTERM` to the whole group
(`kill(-pgid, SIGTERM)`); it never uses SIGKILL.

`runner::capture()` runs one-shot queries to completion with a timeout, capturing
both streams (45 s in `apt::simulate`, `SIMULATE_TIMEOUT`). It also uses a process
group so a timed-out query can be SIGTERM'd cleanly.

## Phase detection

Phases live in `src/ui/window.rs` (`phase_index` / `phase_label`), not in the
runner. They are derived from output lines and only ever move forward:

| Pattern seen in a line | Phase label |
| --- | --- |
| `Reading package lists` / `Building dependency tree` | Resolving dependencies… |
| `Need to get` / `Get:` / `Fetching` | Downloading packages… |
| `Selecting previously` / `Preparing to unpack` / `Unpacking` | Unpacking… |
| `Setting up` / `Processing triggers` | Configuring… |

Before the first line of a root command the label reads
"Waiting for authentication…", because `pkexec` prompts before the child emits
anything. Unknown lines never reset the phase.

## Error taxonomy

`apt.rs::classify(code, output)` runs once, after the child exits, and returns
`(ErrorClass, hint)`. Detection is on stable substrings of the combined output
plus the exit code; `LC_ALL=C` keeps those substrings stable.

| Class | Detection | Hint |
| --- | --- | --- |
| `Auth` | exit code 126 / 127 (`is_auth`) | "Installation cancelled" — shown as a **toast**, not a banner |
| `Locked` | "could not get lock" / "unable to acquire the dpkg frontend lock" | "Another package manager is busy" |
| `StaleLists` | "unable to locate package" / "has no installation candidate" | "Package lists are out of date" + a "Refresh lists" banner button |
| `MissingDeps` | "unmet dependencies" / "is not installable" | "Dependencies cannot be satisfied" |
| `DiskFull` | "no space left on device" | "Not enough free disk space" |
| `BadArchive` | "is not a debian format archive" / "error in archive" | "This file is not a valid .deb package" |
| `Unknown` | anything else | "Installation failed (exit code N)" |

The full raw log is kept for every failure; the banner only adds the one-line
hint. The banner button is "Details" normally and "Refresh lists" for stale
package lists.

## Single instance and file opening

`main.rs` builds one `adw::Application` with
`gio::ApplicationFlags::HANDLES_OPEN`. `connect_activate` presents the single
window; `connect_open` presents it and forwards the first `GFile` path to
`AppWindow::load_path`. That is the path used for both `debdrop foo.deb` on the
command line and the desktop file's `Exec=debdrop %f`. Opening a second instance
with a file therefore loads it into the existing window rather than starting a
new one.

## Log

`ui/log_view.rs` owns a read-only monospace `GtkTextView`. Appends preserve
autoscroll only when the view is already at the bottom, and the buffer is trimmed
to the last 5 000 lines.

## Persistence

None. No config file, no state database, no recent-files list. The only
in-memory convenience is `last_dir`, used to seed the file dialog with the
directory of the most recently loaded `.deb`. Installation truth is re-queried
from `dpkg-query` after every transaction.

## Build

`cargo` only: no meson, no gresource compilation. The stylesheet is embedded with
`include_str!`. Declared dependencies (see `Cargo.toml`):

| Crate | Version | Feature |
| --- | --- | --- |
| `gtk4` | 0.11 | `v4_18` |
| `libadwaita` | 0.9 | `v1_7` |
| `async-channel` | 2 | — |
| `tempfile` | 3 | — |
| `libc` | 0.2 | — |

The release profile enables LTO, one codegen unit, and symbol stripping.
