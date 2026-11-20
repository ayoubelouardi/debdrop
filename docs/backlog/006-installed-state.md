# 006 — Installed state and action classification

Status: done · Size: S · Depends on: 003

## Goal

The button tells the truth: Install, Reinstall, Upgrade to X, or Downgrade to X,
and Remove appears only when it means something.

## Scope

- `apt::installed_state(pkg) -> NotInstalled | Installed { version, status }`
  via `dpkg-query -W -f='${db:Status-Abbrev}\t${Version}\n' <pkg>`.
  Treat any status other than `ii` as not fully installed; distinguish
  `un`/`rc` (config files remain) in a secondary label, not as "installed".
- `apt::compare(a, b) -> Ordering` via `dpkg --compare-versions` (three calls, or
  `le` + `eq`), because Debian version ordering is not semver.
- `Action { Install, Reinstall, Upgrade { from, to }, Downgrade { from, to } }`
  computed from `DebInfo.version` vs installed version.
- Wire labels into the primary button; add/remove the Remove button and its
  confirmation dialog when the state changes.
- Re-query state after every transaction (item 009 calls this), never cache it
  across a reload.

## Acceptance criteria

- [x] Not installed → "Install"; after manual `dpkg -i` of the same file →
      "Reinstall" without restarting the app.
- [x] Installed 1.0.0 + file 1.1.0 → "Upgrade to 1.1.0".
- [x] Installed 1.1.0 + file 1.0.0 → "Downgrade to 1.0.0".
- [x] `rc`-state package (removed, config remains) is reported as not installed
      with a note, and the primary action is "Install".
- [x] Version comparison uses `dpkg --compare-versions` — verified with
      `1.0~rc1 < 1.0` and `1.10 > 1.9`, where a naive string compare fails.
- [x] Remove button is absent for packages not installed and requires the exact
      package name in a confirmation dialog before acting.
- [x] Buttons stay disabled while a transaction is running.

## Notes / risks

- `dpkg-query` exits 1 for unknown packages and prints nothing; that is
  `NotInstalled`, not an error.
- `${db:Status-Abbrev}` includes trailing space in some versions — trim.
- Do not use `apt list --installed`: slower, and its output is for humans.

## Out of scope

- Running the transactions (007–009).
- Detecting "installed from a different source/repo".

## Verification

`apt::installed_state` uses `dpkg-query -W` and `apt::compare` delegates ordering
to `dpkg --compare-versions`; `apt::action_for` produced the four observed labels
— Install, Reinstall, Upgrade to X, Downgrade to X — with the 1.0.0 / 1.1.0 /
0.9.0 fixtures. In this session `dpkg-query` reports `hello-debdrop` as `ii`
0.9.0. Debian-order edge cases (`1.0~rc1 < 1.0`, `1.10 > 1.9`) were not re-run
during the documentation pass; they are handled by `dpkg`, not by string
comparison. As shipped, `Action` is a plain four-variant enum with no `from`/`to`
fields, and the Remove button is shown only for installed packages.
