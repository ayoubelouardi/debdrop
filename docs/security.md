# Security and privilege model

## The uncomfortable truth

A `.deb` is not data. Installing one runs maintainer scripts (`preinst`,
`postinst`, `prerm`, `postrm`) **as root**, and can register services, users,
kernel modules, and cron jobs. No GUI can make that safe. DebDrop's job is to
make the privilege step visible, explicit, and hard to trigger by accident —
never to imply the package has been vetted.

Consequences for the product:

- The app never auto-installs. A human clicks **Install**, sees a polkit prompt,
  and authenticates. No command-line flag, no drag-and-drop-target, and no
  `%f` invocation may start a transaction.
- When maintainer scripts are present, the "Maintainer scripts" group lists their
  names (e.g. `postinst`) with the subtitle "Runs with root privileges during
  installation" and a read-only **View…** dialog.
- DebDrop does not claim the package is signed, trusted, or verified. It reports
  what dpkg/apt report, and nothing more.

## Privilege boundary

v1 uses stock `pkexec` with the generic `org.freedesktop.policykit.exec` action.
There is no custom polkit policy, no root helper binary, and nothing setuid.

```
user process (no privilege)
  └─ pkexec  ──[polkit auth via lxpolkit]──▶  root
       └─ env … apt-get install -y /abs/path/foo.deb
```

Why no helper: a helper installed to `/usr/libexec` with a narrow policy is more
work, needs a root install step, and gains little while the helper would simply
wrap `apt-get`. The privilege surface is therefore exactly "authenticated user
may run apt-get", which is what they could already do with sudo.

Cost of that choice: the polkit dialog identifies DebDrop only by the pkexec
caller, and authentication is per-transaction (polkit may cache briefly). If the
product later needs fine-grained actions ("install only, never remove"), the
trigger to revisit is recorded in `decisions/0003-pkexec-escalation.md`.

## Input handling

- All external commands are argv vectors; **no shell is ever involved**. A path
  containing `;`, `$(…)`, spaces, or newlines is just an argument.
- Paths are canonicalized with `std::fs::canonicalize` and must be regular files
  before `dpkg-deb` is called.
- Files must have a `.deb` suffix (case-insensitive) and `dpkg-deb --info` must
  succeed before any field is shown. Failure means the file is rejected, not
  partially parsed. The desktop entry registers
  `application/vnd.debian.binary-package` so a file manager offers DebDrop.
- The package name used for `apt-get remove` comes from `dpkg-deb`, is checked
  against Debian's allowed character set (`[a-z0-9][a-z0-9+.-]+`), and is passed
  as a single argument. A malformed name rejects the file at load time.
- `dpkg-deb -e` extraction goes to a `tempfile::TempDir` (mode 0700) held privately
  by `DebInfo`. It is removed automatically when that `DebInfo` is dropped — on
  loading another file or on app exit — not when the viewer dialog closes.
- No network access of our own: only apt fetches, using the system's configured
  sources and trust model.
- No secrets are read, stored, or logged. The polkit password never passes
  through DebDrop.

## What we refuse to do

| Refusal | Reason |
| --- | --- |
| Run the app as root | GTK/DBus/session assumptions break; attack surface explodes |
| Ship a setuid binary | Unnecessary; pkexec exists and is audited |
| Store or proxy the sudo/polkit password | Out of scope, unauditable |
| Edit `/etc/apt/sources.list*` | Enormous blast radius, needs its own threat model |
| `apt-get autoremove`/`purge` implicitly | Removing packages the user did not name |
| Install from a URL | Trust and TLS handled by apt's own sources, not by us |
| Hide the apt output | The log is the audit trail; it is always available after a run |
| Install anything without a click | Drag-and-drop is inspection only |

## UI-level guards

- Busy state disables all action buttons; a second transaction cannot start.
- Closing the window mid-transaction asks for confirmation and warns that
  interrupting dpkg may leave the system half-configured (`dpkg --configure -a`).
- Remove requires an explicit confirmation dialog naming the package and its
  version, because "remove" is the destructive action with the least visual
  weight otherwise.
- Errors always keep the raw log; classification only adds a hint above it.

## Residual risk accepted in v1

1. A malicious maintainer script runs as root with the user's consent. Mitigated
   only by visibility, not by containment.
2. Dependency downloads use the system's configured (and possibly stale or
   compromised) mirrors. Same risk as `apt-get install` in a terminal.
3. `--allow-downgrades` can be used to install an older, vulnerable version.
   Shown explicitly as **Downgrade** on the button, never silently.
4. polkit's default agent behaviour varies by desktop; a prompt rendered behind
   the window may look like a hang. Mitigation: the busy view says "Waiting for
   authentication…" between spawn and the first line of output.
5. No audit log of past installs beyond the in-session log.
