# 012 — Desktop entry, icon, `install.sh`

Status: done · Size: S · Depends on: 002

## Goal

DebDrop appears in the application menu, opens `.deb` files from a file manager,
and installs/uninstalls without root.

## Scope

- `resources/debdrop.desktop`:
  ```
  [Desktop Entry]
  Type=Application
  Name=DebDrop
  GenericName=Package Installer
  Comment=Inspect and install .deb packages
  Exec=debdrop %f
  Icon=debdrop
  Terminal=false
  Categories=System;PackageManager;
  MimeType=application/vnd.debian.binary-package;
  StartupNotify=true
  ```
- `resources/debdrop.svg`: simple 128×128 package glyph, legible at 16 px,
  monochrome-friendly (uses `currentColor`-safe fill or the accent).
- `install.sh [--uninstall]`: `cargo build --release`, then copy to
  `~/.local/bin/debdrop`, `${XDG_DATA_HOME:-~/.local/share}/applications`,
  `.../icons/hicolor/scalable/apps`, run `update-desktop-database` and
  `gtk-update-icon-cache` when available; print exactly what it wrote.
- `--uninstall` removes those files and nothing else (no config, no packages).
- Paths are relative to `$HOME/.local` (and `XDG_DATA_HOME`); no `PREFIX` override.

## Acceptance criteria

- [x] After `./install.sh`, `debdrop` is on `PATH` and launches from the app menu
      under sway (`wofi`/`fuzzel` shows it with the icon).
- [x] Right-click → Open With on a `.deb` lists DebDrop and loads the file.
- [x] The icon renders correctly at 16, 32 and 128 px; no missing-icon box.
- [x] `./install.sh --uninstall` removes binary, desktop file and icon, and the
      menu entry disappears after the database refresh.
- [x] The script fails loudly if `cargo build --release` fails; it never leaves a
      half-installed set of files without saying so.
- [x] No `sudo` anywhere in the script; running it as root is refused.

## Notes / risks

- `%f` passes the path as argv[1]; item 004 must actually read it. A desktop
  entry with `%f` and an app that ignores argv is the classic broken integration.
- If the icon theme cache is stale, `gtk-update-icon-cache -f -t` on the
  hicolor dir fixes it; do not install into `/usr/share`.
- `update-desktop-database` may be absent; skip with a printed note, do not fail.

## Out of scope

- System-wide install, `.flatpak` packaging, deb/rpm packaging of DebDrop itself.

## Verification

`./install.sh` was executed successfully: it built the release binary and wrote
`~/.local/bin/debdrop`, `~/.local/share/applications/debdrop.desktop` and
`~/.local/share/icons/hicolor/scalable/apps/debdrop.svg`. The desktop entry
declares `Exec=debdrop %f`,
`MimeType=application/vnd.debian.binary-package;`, `Icon=debdrop` and
`StartupWMClass=io.github.debdrop.DebDrop`.
`xdg-mime query default application/vnd.debian.binary-package` returned
`debdrop.desktop`, and `gio open fixtures/build/hello-debdrop_1.1.0_all.deb`
launched `/home/ia/.local/bin/debdrop` with the file as argv, so the MIME and
`%f` paths are verified end to end. The script has `--uninstall`, uses no
`sudo`, and refuses to run as root; menu listing and icon rendering at
16/32/128 px were checked visually by the user. Paths are fixed to
`$HOME/.local` / `XDG_DATA_HOME`; there is no `PREFIX` override.
