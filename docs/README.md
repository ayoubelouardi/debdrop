# DebDrop documentation

DebDrop is a small GTK4/libadwaita application for installing local `.deb` files on
Debian-based systems. Drop a file, see what it really is, install it with one
privilege prompt.

## Contents

| Document | What it answers |
| --- | --- |
| [product.md](product.md) | What v1 does, who it is for, what it deliberately does not do |
| [architecture.md](architecture.md) | Modules, data flow, external commands, error taxonomy |
| [security.md](security.md) | Trust model, privilege boundary, what we refuse to do |
| [ui.md](ui.md) | Window states, widget tree, copy, keyboard, accessibility |
| [development.md](development.md) | Prerequisites, build, fixtures, manual test matrix, packaging |
| [decisions/](decisions/) | ADRs — one file per irreversible-ish choice |
| [backlog/](backlog/) | Ordered, individually shippable work items with acceptance criteria |

## Conventions

- **Docs are written before the code** and updated in the same change that makes
  them wrong. A stale doc is a bug.
- **ADRs are append-only.** If a decision changes, add a new ADR that supersedes
  the old one; never rewrite history.
- **The backlog is the source of truth for work.** Each item has a stable ID,
  a status, and testable acceptance criteria. IDs are never reused or renumbered.
- **No claim without a command.** If a doc says "apt resolves dependencies", it
  says which command proves it.
- **Environment facts** in these docs describe the reference machine (Debian 13,
  sway/Wayland, GTK 4.18.6, libadwaita 1.7.6). Where a version matters, it is
  stated inline.
