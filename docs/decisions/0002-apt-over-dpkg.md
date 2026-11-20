# 0002 — `apt-get install ./file.deb` instead of `dpkg -i`

Status: accepted · Date: 2026-11-20

## Context

`dpkg -i foo.deb` is the traditional path but fails whenever a dependency is
missing, leaving the package half-configured and the user to work out which
packages to install. `apt-get install ./foo.deb` (apt ≥ 1.1) reads the local file
from disk, resolves dependencies from the configured repositories, downloads
what is missing, and reports a coherent transaction. `gdebi` does something
similar but adds a dependency of its own.

## Decision

Use `apt-get install -y --no-install-recommends <absolute path>.deb` as the only
install path. Provide an unprivileged `apt-get -s install <path>` beforehand as
the preview. Keep `--allow-downgrades` conditional on an actual downgrade.

## Consequences

- Dependency resolution is apt's problem, not ours; the preview and the real run
  agree because they are the same resolver.
- Needs network access and non-stale package lists for packages not yet
  installed. That failure mode becomes an explicit "Refresh package lists"
  action instead of a dead end.
- `--no-install-recommends` keeps installs predictable; the card shows
  `Recommends` separately so the omission is visible.
- No support for fully offline dependency installation. Accepted: the offline
  simulate output names the missing package, and a future ADR can add "download
  dependencies elsewhere" if it ever matters.
- Adding a dependency is not "free": every extra package is another thing apt
  may download. `--no-install-recommends` is the guard, not a preference.
