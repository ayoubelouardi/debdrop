# 0003 — `pkexec` directly, no root helper or custom polkit policy

Status: accepted · Date: 2026-11-20

## Context

The app must run `apt-get` as root from a GUI. Options:

1. Run the whole app as root — breaks GTK session/theme/DBus assumptions and
   makes every rendering bug a root bug.
2. Ship a setuid helper — unaudited privilege surface, packaging complexity.
3. Install a root helper in `/usr/libexec` with a custom polkit action —
   narrower policy, but requires a system-wide install step and a second binary.
4. Call stock `pkexec` with `org.freedesktop.policykit.exec`.

The session already runs `lxpolkit`, so (4) shows a password dialog with no setup.

## Decision

(4). `pkexec /usr/bin/env DEBIAN_FRONTEND=noninteractive APT_LISTCHANGES_FRONTEND=none LC_ALL=C /usr/bin/apt-get install …`,
with absolute program paths, inherited stdin closed, and stdout/stderr piped.

## Consequences

- Zero install-time privilege footprint: `./install.sh` writes only to
  `~/.local`.
- The authorization scope is "this user may run this command", which equals what
  `sudo apt-get` already grants. We do not claim finer granularity.
- Exit codes 126/127 mean "not authorized / dismissed" and are mapped to a calm
  "Installation cancelled", not an error.
- No way to distinguish install from remove in the auth dialog; a custom policy
  would. Revisit triggers: users needing per-action policy, kiosk/managed
  deployment, or a pre-authenticated session.
- If a desktop has no polkit agent, the app must detect the failure and explain
  it; the fallback (`sudo -A` + zenity askpass) is a runner-level swap, not an
  architectural change.
- pkexec sanitizes the environment, so everything apt needs is passed explicitly
  through `/usr/bin/env`; relying on the user's `DEBIAN_FRONTEND` would be a bug.
