# ADR 0004 — Custom Polkit Action for Privilege Escalation

**Status:** Accepted  
**Date:** 2026-11-23  
**Supersedes:** Parts of [ADR 0003](0003-pkexec-escalation.md) (which used the generic `org.freedesktop.policykit.exec` action)

---

## Context

The original design (ADR 0003) escalated privileges through `pkexec` using the **generic** Polkit action `org.freedesktop.policykit.exec`. This action is shared across all programs that use `pkexec` without a registered custom policy. While functional, it has two significant drawbacks:

1. **Audit trail ambiguity.** System administrators auditing `/var/log/auth.log` see `org.freedesktop.policykit.exec`, which gives no information about *what* program requested elevation or *why*.
2. **Policy granularity.** There is no way to grant DebDrop-specific overrides (e.g., allow installation in a kiosk environment) without accidentally relaxing policy for every other `pkexec` user.

The code review flagged this as a high-priority issue for v0.2.

---

## Decision

We introduce a dedicated Polkit action file at:

```
resources/io.github.debdrop.policy
```

Installed to the system Polkit actions directory by `install.sh`:

```
/usr/share/polkit-1/actions/io.github.debdrop.policy
```

The file defines three named actions:

| Action ID | Purpose |
|-----------|---------|
| `io.github.debdrop.install` | Install, reinstall, upgrade, or downgrade a `.deb` |
| `io.github.debdrop.remove` | Remove an installed package |
| `io.github.debdrop.update` | Refresh APT package lists |

Each action is **annotated** with:

```xml
<annotate key="org.freedesktop.policykit.exec.path">/usr/bin/apt-get</annotate>
```

This annotation binds the action to exactly `/usr/bin/apt-get`. No other binary can claim this action ID.

---

## Default Policy

```
allow_inactive: auth_admin
allow_active:   auth_admin_keep
```

- **`auth_admin`** — requires a password from an administrator account (not just the invoking user) for inactive (remote) sessions.
- **`auth_admin_keep`** — same, but caches the authorization for a short window (default: 5 minutes) in active desktop sessions, reducing repeated prompts during multi-step operations.

This matches the behavior of GNOME Software and similar trusted desktop installers.

---

## Consequences

**Positive:**
- Auth log entries now read `io.github.debdrop.install` instead of the generic action — immediately useful for security audits.
- Administrators can write targeted Polkit rules (`.rules` JS files) for DebDrop specifically (e.g., allowing members of a custom group to install without a password).
- Lays the groundwork for `--action-id` support if `pkexec` gains it in a future version.

**Negative:**
- The `.policy` file must be installed to `/usr/share/polkit-1/actions/` — a system-wide path requiring `sudo`. This is handled by the system-wide `.deb` package (see `cargo-deb` configuration in `Cargo.toml`), **not** by the user-local `install.sh`.
- Local-install users (via `install.sh`) will still fall back to the generic action until they separately install the policy file. We document this in `docs/installation.md`.

---

## Relationship to ADR 0003

ADR 0003 remains authoritative for:
- The choice of `pkexec` over `gksudo`/`sudo` frontends.
- The `argv` vector (no shell) mandate.
- The process group termination strategy for orphaned children.

This ADR extends ADR 0003 specifically on the **action ID** question.

---

## References

- [Polkit reference manual — Writing Polkit Actions](https://www.freedesktop.org/software/polkit/docs/latest/polkit.8.html)
- [freedesktop.org — PolicyKit Policy Configuration DTD](http://www.freedesktop.org/standards/PolicyKit/1/policyconfig.dtd)
- `resources/io.github.debdrop.policy` — the implemented action file
- `Cargo.toml` — `[package.metadata.deb]` section for `.deb` asset installation
