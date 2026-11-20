# Backlog

Ordered work for DebDrop v1. Each item is small enough for one focused session and
independently verifiable. The index below is the only status board; item files
carry the detail.

## Statuses

| Status | Meaning |
| --- | --- |
| `todo` | Not started, ready when dependencies are `done` |
| `doing` | Actively being worked; only one at a time on the critical path |
| `blocked` | Waiting on a decision or an external condition (state which) |
| `done` | All acceptance criteria ticked, manual test recorded, docs updated |

Update both the item file and the table in the same change. Never renumber an
item; if scope changes, edit the item or add a new one that supersedes it.

## Index

| ID | Title | Size | Depends on | Status |
| --- | --- | --- | --- | --- |
| [001](001-privilege-smoke-test.md) | Prove the pkexec path works on this session | S | — | done |
| [002](002-scaffold-app.md) | Scaffold the Rust/GTK4 app and window | M | 001 | done |
| [003](003-parse-deb.md) | Parse and validate `.deb` files | M | 002 | done |
| [004](004-load-files.md) | Load files: drag & drop, picker, `%f` | M | 003 | done |
| [005](005-package-card.md) | Package card and script viewer | M | 003, 004 | done |
| [006](006-installed-state.md) | Installed state and action classification | S | 003 | done |
| [007](007-dry-run-plan.md) | Dry-run transaction plan | M | 003, 006 | done |
| [008](008-streaming-install.md) | Streaming install with live log | L | 005, 007 | done |
| [009](009-remove-upgrade.md) | Remove, reinstall, upgrade, downgrade | M | 008 | done |
| [010](010-refresh-lists.md) | Refresh package lists | S | 008 | done |
| [011](011-test-fixtures.md) | Test fixtures (.deb builder) | S | 002 | done |
| [012](012-desktop-integration.md) | Desktop entry, icon, `install.sh` | S | 002 | done |
| [013](013-error-ux.md) | Error classification and hints | M | 008, 009 | done |
| [014](014-polish-a11y.md) | Empty states, accessibility, polish | S | 005, 008 | done |
| [015](015-readme-handover.md) | Root README and handover | S | 009, 012, 013 | done |

Critical path: 001 → 002 → 003 → 004 → 005/006 → 007 → 008 → 009. Items 010–012
and 011 can run in parallel once their dependencies are done.

## Item template

```md
# NNN — Title

Status: todo · Size: S/M/L · Depends on: …

## Goal
One sentence: what the user can do after this that they could not before.

## Scope
- Bullets, concrete.

## Acceptance criteria
- [ ] Observable, testable, no "works correctly".

## Notes / risks
Non-obvious mechanics, commands, failure modes.

## Out of scope
Things deliberately left for another item.
```

## Definition of done

- `cargo fmt --check` and `cargo clippy -- -D warnings` clean; `cargo test` green.
- Acceptance criteria ticked. Where a criterion is satisfied by implementation
  plus a unit test but was not exercised against a live apt/polkit transaction,
  the **Verification** note says exactly what was and was not observed.
- Relevant rows in the manual test matrix (`development.md`) exercised, or
  explicitly marked as not exercised in the item's **Verification** note.
- Docs updated where behaviour or scope changed; new ADR if a decision forked.

## Icebox

Unscheduled, not v1. Promote to an item only with a reason and a date.

- Installed-packages browser with search and remove.
- Batch install of multiple `.deb` files in one apt transaction.
- "Open containing folder" and recent files.
- Dark/light override and compact layout for small screens.
- Flatpak/AppImage awareness (detect and explain "this is not a .deb").
- Localized strings via `gettext`.
- System-wide install with a custom polkit action and a narrow helper.
- Dependency download preview (sizes per package) from the simulate output.
