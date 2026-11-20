# Product — DebDrop v1

## Problem

On Debian the "just install this .deb" path is a terminal ritual:
`sudo dpkg -i foo.deb` produces a dependency wall, `apt-get install ./foo.deb`
works but gives no preview, and neither tells you what the package will do,
whether it is already installed, or why it failed. File managers hand `.deb`
files to `gdebi` or nothing at all.

DebDrop makes that path a two-step GUI flow: **drop → inspect → install**, with
the real apt transaction and its real output, not a fake progress bar.

## User

Single user on a Debian/Ubuntu desktop, comfortable in a terminal but unwilling
to keep one open for a routine install. Reference machine: Debian 13, sway,
Wayland, `lxpolkit` running, no GNOME session.

## v1 scope

In:

- Drag & drop a `.deb` onto the window, or pick one with a file dialog.
- Validate it is a real Debian binary package before trusting any of its metadata.
- Preview: name, version, architecture, section, maintainer, installed size,
  description, declared dependencies, and whether they are satisfied.
- Warn when the package ships maintainer scripts or debconf templates, with the
  scripts viewable read-only.
- Detect the installed state and adapt the primary action: Install / Reinstall /
  Upgrade / Downgrade.
- Install through `apt-get install ./file.deb` as root via a `pkexec` polkit
  prompt. Stream real output into an in-window log with coarse phase labels.
- Remove the same package by name (`apt-get remove`), no autoremove.
- Refresh apt package lists from the UI when resolution fails because indexes
  are stale.

Out (v1):

- Browsing the repositories, searching, or installing by package name.
- A browser of all installed packages.
- Flatpak, Snap, AppImage, `.rpm`, source packages.
- Batch/multi-file queues, dependency pinning, holding/version pinning.
- Autoremove, purge, or cleaning up orphaned dependencies.
- Sandboxing, signature verification beyond what apt/dpkg already do, or any
  attempt to inspect what a maintainer script will *do*.
- Auto-update of DebDrop itself, telemetry, network access of its own.

## Happy path

1. Empty window: "Drop a `.deb` to inspect it" + `Choose File…`.
2. Drop `foo_1.2.3_amd64.deb`.
3. Card appears with metadata, dependency/plan rows, and a plan headline from a
   dry run ("2 new packages, 1 upgraded").
4. Click **Install** → polkit password dialog → busy view with log.
5. `Resolving dependencies… Unpacking… Configuring…` → toast "Installed foo 1.2.3".
6. Card flips to installed; **Remove** and **Reinstall** become available.

## Failure paths that must be good

| Situation | Expected behaviour |
| --- | --- |
| Not a `.deb` / corrupt archive | Inline rejection before any parsing, no root prompt |
| Dependency missing and offline | Simulation names the exact missing package; install is not attempted |
| Stale package lists | "Unable to locate package" is translated into a Refresh action |
| Auth dialog dismissed | Exit 126/127 → "Installation cancelled" toast, no error wall |
| apt lock held | "Another package manager is busy" banner hint |
| dpkg unpack/config failure | Full log preserved (reopen with "Show last log"), state re-read from dpkg |
| Already installed, same version | Primary action reads **Reinstall**, not **Install** |

## Success criteria

- A first-time user installs a valid `.deb` without reading documentation.
- Every number and status shown before install comes from `dpkg-deb`/`dpkg-query`,
  never from the filename.
- After install or removal the UI re-reads state from `dpkg-query` instead of
  assuming success from the exit code alone.
- No operation runs through a shell; no privilege is requested for read-only steps.
- The binary stays a single small executable with no runtime beyond GTK4 +
  libadwaita, and starts in well under a second.
