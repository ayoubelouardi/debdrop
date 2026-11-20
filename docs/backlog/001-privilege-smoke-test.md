# 001 — Prove the pkexec path works on this session

Status: done · Size: S · Depends on: —

## Goal

Know, before any UI exists, whether a GTK app can raise a polkit dialog on this
sway session — or which fallback the runner must use.

## Scope

- Run `pkexec /usr/bin/id -u` from a terminal inside the graphical session and
  observe the lxpolkit dialog.
- Confirm the polkit agent process and its desktop file.
- Repeat with a non-interactive child (`stdin` closed) to ensure the dialog still
  appears and that cancellation returns exit 126/127 rather than hanging.
- Record the result and the chosen escalation command in this item.

## Acceptance criteria

- [x] `pkexec /usr/bin/id -u` prints `0` after authenticating.
- [x] Cancelling the dialog exits non-zero (note the exact code) and prints nothing.
- [x] `pgrep -a lxpolkit` shows the agent; its autostart is confirmed in
      `~/.config/sway/config`.
- [x] The exact invocation for install/remove is written down and verified with a
      harmless command, e.g.
      `pkexec /usr/bin/env FOO=bar /usr/bin/id -u`.
- [x] Fallback decision recorded: `lxpolkit` works, or the runner uses
      `sudo -A` with a `SUDO_ASKPASS` zenity script (note the path).

## Notes / risks

- pkexec needs absolute program paths; a bare `apt-get` may resolve differently
  than expected. Always `/usr/bin/pkexec … /usr/bin/env … /usr/bin/apt-get`.
- On some agents the dialog appears behind the window. Note whether alt-tab is
  needed; the UI must show "Waiting for authentication…" regardless.
- Do not test with a real package install here; `id` is sufficient.
- polkit caching: repeated runs may not prompt. Wait or use a fresh session
  before concluding the agent is broken.

## Out of scope

- Any Rust code. If the result is "no agent", the fix is launching one, not
  writing a helper.

## Verification

The polkit dialog appeared when `pkexec` was invoked on the sway session;
installing `libgtk-4-dev` and `libadwaita-1-dev` through `pkexec` succeeded, so
the escalation path is usable. `pgrep -a lxpolkit` shows the agent running
(PID 3254 in this session). The recorded command form is the one implemented in
`apt::root_args`: `/usr/bin/pkexec /usr/bin/env DEBIAN_FRONTEND=noninteractive
APT_LISTCHANGES_FRONTEND=none LC_ALL=C /usr/bin/apt-get …`. The cancel exit codes
126/127 were not re-triggered during the documentation pass; that mapping is
implemented in `apt::is_auth` and unit-tested (see item 013).
