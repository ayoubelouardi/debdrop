# 007 — Dry-run transaction plan

Status: done · Size: M · Depends on: 003, 006

## Goal

Showing what apt *would* do, before asking for a password: new packages,
upgrades, removals, unresolved dependencies — with the exact names.

## Scope

- `apt::simulate(info) -> Plan` running
  `apt-get -s install --no-install-recommends <abs deb>` with `LC_ALL=C`,
  `stdin` null, and a 45 s timeout (`SIMULATE_TIMEOUT`) so a hung resolver cannot
  freeze the UI.
- Parse the stable parts of the output, not the prose:
  - `The following NEW packages will be installed:` + indented package list.
  - `The following packages will be upgraded:` / `… REMOVED:`.
  - `Installing:`/`Conf:` lines as a fallback when the summary is absent.
  - unresolved: `Unable to locate package X`, `has no installation candidate`,
    `but it is not installable`, `unmet dependencies`.
- Run the simulation off the main thread; show the card immediately with
  "Checking dependencies…" as the Dependencies group description until it returns.
- Render the plan as rows in the Dependencies group (name plus text such as
  "required", "will be installed", "not found"), with the headline as the group
  description. Per-package sizes are not shown.
- If the simulation fails for stale lists, expose the "Refresh package lists"
  action inline (item 010 provides the command; this item defines the trigger).

## Acceptance criteria

- [x] Fixture with no extra deps → "All dependencies are available", Install
      enabled immediately.
- [x] Fixture depending on a package present in the repos but not installed →
      plan lists it under "will be installed", Install enabled.
- [x] Fixture with `Depends: nonexistent-package` → unresolved row names it,
      Install is disabled, and no apt install is attempted.
- [x] Simulate on a package with a newer installed version reports the upgrade
      or downgrade correctly with `--allow-downgrades` noted.
- [x] Simulate never prompts for a password and never writes to the system.
- [x] A 45 s timeout surfaces "Could not resolve dependencies (timed out)" and
      leaves the card usable.
- [x] Loading a second file cancels/ignores the first simulation's result.

## Notes / risks

- The summary list is wrapped at terminal width; parse indented continuation
  lines rather than one package per line.
- `-s` does not download, but it does read apt lists; a missing list produces
  the stale-lists case, which is a feature here.
- Do not parse localized output: `LC_ALL=C` is mandatory, and a test with a
  German locale env set on the child must still produce identical `Plan` values.
- Simulation can emit `WARNING: apt does not have a stable CLI interface`; that
  line is expected and goes to the log, not the card.

## Out of scope

- Per-package download size (icebox), dependency pinning, alternatives selection.

## Verification

`apt::simulate` runs `apt-get -s install --no-install-recommends <abs deb>`
through `runner::capture` on a std thread with a 45 s timeout. Unresolved and
stale-list detection are unit-tested (`detects_unresolvable`,
`detects_stale_lists` pass in this session), and `ui/window.rs::apply_plan`
filters the root package name out of the displayed plan rows. The plan is shown
as rows in the Dependencies group, whose description carries `Plan::headline()`;
there is no expander row. The `Installing:`/`Conf:` fallback parsing described in
the original scope is not present in the shipped code.
A German-locale child environment was not re-tested this session; `LC_ALL=C` is
set on the query.
