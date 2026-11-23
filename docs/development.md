# Development

## Prerequisites

Debian 13 reference machine. Runtime libs for GTK4 and libadwaita are already
present; the `-dev` packages are not:

```
sudo apt install libgtk-4-dev libadwaita-1-dev pkg-config build-essential
```

Verify:

```
pkg-config --modversion gtk4 libadwaita-1   # expect 4.18.x / 1.7.x
rustc --version && cargo --version          # 1.93 / 1.93 on the reference box
```

## Build and run

```
cargo run                 # debug, opens the window
cargo build --release     # target/release/debdrop
cargo fmt --check && cargo clippy -- -D warnings
```

Or use the POSIX `/bin/sh` developer utility suite:

```sh
./utilities.sh clean              # Clean target/, fixtures/build/, and temporary files
./utilities.sh test [filter]      # Rebuild fixtures and run cargo test
./utilities.sh check             # Fast cargo check
./utilities.sh build             # Compile optimized release binary
./utilities.sh run [file.deb]    # Run DebDrop in dev mode
./utilities.sh fixtures          # Rebuild disposable test fixtures
./utilities.sh inspect <file.deb> # Terminal inspection of .deb metadata, scripts, and desktop entries
./utilities.sh simulate <file.deb># Unprivileged APT transaction simulation (apt-get -s)
./utilities.sh size              # Report stripped binary size
./utilities.sh ci                # Full clean, fixture build, check, test, and release compile
```

Run under sway like any Wayland client; nothing about the app is compositor
specific. For widget debugging:

```
GTK_DEBUG=interactive cargo run
G_MESSAGES_DEBUG=all cargo run
```

## Verifying the privilege path first

Before touching UI code, prove polkit works in this session:

```
pkexec /usr/bin/id -u        # should print 0 after the lxpolkit dialog
```

If no dialog appears, `pkexec` fails with "No authentication agent found" or
hangs. Check `pgrep -a lxpolkit`. Fallbacks, in order: start `lxpolkit`, use
`/usr/bin/polkit-gnome-authentication-agent-1`, or switch the runner to
`sudo -A` with `SUDO_ASKPASS` pointing at a zenity script. Record the outcome in
backlog item 001 before proceeding.

## Test fixtures

Never test the install path with a package you did not build. `fixtures/` holds
throwaway packages (see `fixtures/README.md` for all seven); the base source tree is:

```
fixtures/hello-debdrop/
  DEBIAN/control
  DEBIAN/postinst
  usr/bin/hello-debdrop
```

```
Package: hello-debdrop
Version: 1.0.0
Architecture: all
Maintainer: DebDrop Test <test@example.invalid>
Description: DebDrop test fixture
 Installs nothing useful. Used to exercise install, upgrade and remove.
```

Build and install manually to confirm expectations:

```
dpkg-deb --build fixtures/hello-debdrop fixtures/hello-debdrop_1.0.0_all.deb
dpkg-deb --info fixtures/hello-debdrop_1.0.0_all.deb
```

Ship a second version (`1.1.0`) to exercise **Upgrade**, and a third (`0.9.0`) to
exercise **Downgrade**. A fourth fixture with `Depends: nonexistent-package` is
the offline-failure case.

## Manual test matrix

Run after every change that touches commands or state transitions.

| # | Scenario | Expected |
| --- | --- | --- |
| 1 | Launch, no args | Empty state, no root prompt |
| 2 | Drop fixture | Card with correct version/arch/size, script warning, Install |
| 3 | Pick via dialog | Same as 2 |
| 4 | `debdrop fixture.deb` | Loads that file at startup |
| 5 | Drop `README.md` | Inline rejection, no parse, no prompt |
| 6 | Drop truncated `.deb` | "Not a valid .deb", no prompt |
| 7 | Install fixture, cancel auth | "Installation cancelled", state unchanged |
| 8 | Install fixture, authenticate | Log streams, toast, dpkg shows `ii` |
| 9 | Re-drop installed fixture | Primary reads Reinstall, Remove visible |
| 10 | Install 1.1.0 over 1.0.0 | Primary reads Upgrade to 1.1.0 |
| 11 | Install 0.9.0 over 1.1.0 | Primary reads Downgrade to 0.9.0, apt gets `--allow-downgrades` |
| 12 | Remove fixture | Confirm dialog, log, state flips back to Install |
| 13 | Install `Depends: nonexistent` with network off | Simulation names it, install not attempted |
| 14 | Clear apt lists, then simulate | Stale-lists hint + Refresh action |
| 15 | Hold apt lock (`flock /var/lib/dpkg/lock-frontend sleep 60`) | "Another package manager is busy" |
| 16 | Close window mid-install | Confirmation, no silent kill |
| 17 | 500-line log | Log scrolls, "Show last log" reopens it, no UI freeze |

State after every run must be re-read from `dpkg-query`, not inferred. If a test
passes only because of cached UI state, the test is invalid.

## Packaging

`./install.sh` (user-local, no root):

```
~/.local/bin/debdrop
~/.local/share/applications/debdrop.desktop
~/.local/share/icons/hicolor/scalable/apps/debdrop.svg
```

then `update-desktop-database ~/.local/share/applications`. The desktop file
declares `MimeType=application/vnd.debian.binary-package;` and
`Exec=debdrop %f`. Uninstall is the same script with `--uninstall`.

Do not add a system-wide install path in v1; it complicates updates and gives no
benefit for a single-user utility.

## Troubleshooting

| Symptom | Cause | Fix |
| --- | --- | --- |
| `Package gtk4 was not found` at build | `libgtk-4-dev` missing | `pkg-config --modversion gtk4` |
| Window opens but is blank | CSS/stack state bug | `GTK_DEBUG=interactive`, check stack child |
| No polkit dialog | no agent on the session | start `lxpolkit`; see item 001 |
| Prompt appears behind window | polkit agent behaviour | alt-tab; keep "Waiting for authentication…" visible |
| apt says "Could not get lock" | unattended-upgrades | wait or stop it; do not delete lock files |
| Install succeeded but UI says failed | exit code from `env`, not apt | ensure pkexec runs `env` with absolute paths |

## Conventions

- No `unwrap()` on I/O, child spawn, or parsing; convert to a typed error and a
  user-facing hint.
- Every user-visible string lives in `ui.rs`/`ui/*` at the call site; no
  templating layer in v1.
- Keep modules under ~400 lines. If `apt.rs` starts growing parsers for four
  different commands, split per command.
- New external command ⇒ new row in the table in `architecture.md` and a test in
  the matrix above.
