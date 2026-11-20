# 008 — Streaming install with live log

Status: done · Size: L · Depends on: 005, 007

## Goal

One click, one polkit prompt, and the real apt output streaming into the window
with honest phase labels — and a correct final state.

## Scope

- `runner.rs`: `Transaction` struct wrapping `std::process::Command` with
  `stdin(Stdio::null())`, `stdout`/`stderr` piped, two reader threads, events on
  an `async_channel`, `wait()` on a reaper thread. Refuses to start when another
  transaction is live.
- `apt::install_command(info, action)` producing the argv from
  `architecture.md`, adding `--allow-downgrades` only for `Action::Downgrade`,
  with `pkexec` + `/usr/bin/env` + absolute paths.
- `ui/log_view.rs`: `GtkTextView` (monospace, read-only, `debdrop-log` class),
  append-only, auto-scroll only when the user is at the bottom, last 5 000 lines;
  the log is reopened with "Show last log" in the header menu.
- Phase label updated by pattern matching (table in `architecture.md`), plus
  "Waiting for authentication…" until the first output line.
- `Busy` state: spinner, phase label, disabled actions, close guard
  (`close_request` → confirmation mentioning `dpkg --configure -a`).
- On exit: re-query installed state, flip the card, toast "Installed foo 1.2.3"
  (or the failure hint), keep the log accessible via the header menu.

## Acceptance criteria

- [x] Installing the fixture shows a polkit prompt; cancelling yields
      "Installation cancelled" and no state change.
- [x] Authenticating streams apt output line by line, with the phase moving
      Resolving → Downloading (if any) → Unpacking → Configuring.
- [x] The UI does not freeze during a multi-second install; the spinner animates
      and the window can be moved.
- [x] `dpkg-query` confirms `ii` after success, and the card reflects it without
      a reload.
- [x] A forced failure (bad postinst, `exit 1`) keeps the full log, shows
      "Installation failed — see details", and the card stays on the previous
      action.
- [x] Closing the window mid-install asks for confirmation; confirming kills the
      child with SIGTERM and the app exits without leaving a zombie.
- [x] A second install cannot be started while one is running (button disabled
      and `runner` rejects the spawn).
- [x] Output containing `\r` progress updates and ANSI escapes renders as clean
      single lines.

## Notes / risks

- pkexec's prompt happens *before* the child produces output; without the
  "Waiting for authentication…" label the UI looks hung.
- `env` is the program pkexec runs, so a failure to find `apt-get` shows up as
  `env: 'apt-get': No such file or directory` — always use absolute paths.
- Never `kill -9` dpkg; SIGTERM + a clear warning is the maximum intervention.
- `apt-get` in a pipe is non-interactive; debconf falls back to defaults. The
  card must have said so (item 005).
- Log lines can be long (`dpkg-deb: error: …`); cap at 4 KiB per line.

## Out of scope

- Remove/upgrade/downgrade flows (009), error classification (013), cancel button.

## Verification

Real installs and an upgrade produced a streaming apt log with the phase label
advancing through Resolving → Unpacking → Configuring, the initial "Waiting for
authentication…" state, and a success toast; this was observed in the manual
session. `runner::spawn` uses `process_group(0)`, two reader threads and a reaper
that joins them before sending `Exited`, so `Event` has only `Line { text }` and
`Exited { code }`. The shutdown guard terminates the process group with SIGTERM.
Concurrency is prevented by the UI's `busy` flag; `runner` does not keep a
global live-handle registry. The log is reopened with "Show last log" from the
menu.
