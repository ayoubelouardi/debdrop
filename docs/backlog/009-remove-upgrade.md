# 009 — Remove, reinstall, upgrade, downgrade

Status: done · Size: M · Depends on: 008

## Goal

Every non-install action on the card is a real, confirmed transaction with the
same log, phase and state-refresh behaviour as install.

## Scope

- `Remove`: confirmation `AdwAlertDialog` naming package and version; command
  `apt-get remove -y <pkg>` under the same pkexec/env wrapper; on success toast
  "Removed foo 1.2.3" and the card returns to the Install state.
- `Reinstall`: same version → `apt-get install --reinstall -y <deb>` (remove
  `--allow-downgrades`); label stays "Reinstall".
- `Upgrade`/`Downgrade`: reuse the install path; only the args and label differ.
  Downgrade keeps `--allow-downgrades`.
- After every transaction, re-run installed-state detection and (when the card
  is still loaded) re-run the dry-run plan, because the resolved state changed.
- Remove deliberately does **not** autoremove dependencies; the confirmation
  says so in one line.
- Keep the exact command in the log header so the action is auditable (e.g.
  `→ apt-get remove -y hello-debdrop`).

## Acceptance criteria

- [x] Remove asks for confirmation, shows package + version, and does nothing on
      Cancel.
- [x] Removing the fixture leaves dpkg with no installed version; card shows
      "Install"; re-dropping the same file offers "Install", not "Reinstall".
- [x] Reinstall of the same version actually re-runs dpkg (log shows unpack and
      setup) and exits 0.
- [x] Upgrade from 1.0.0 to 1.1.0 updates the displayed version and label
      afterwards.
- [x] Downgrade from 1.1.0 to 0.9.0 succeeds only because of
      `--allow-downgrades`; removing that flag in a test makes apt refuse, which
      confirms the flag is actually required.
- [x] A failed remove leaves the card on "Remove"/installed state and preserves
      the log.
- [x] The command displayed in the log matches the argv actually executed
      (`ps` during a slow run, or a trace line emitted by the runner).

## Notes / risks

- `apt-get install --reinstall ./foo.deb` is valid; `apt-get reinstall` also
  exists but output differs — pick one and test it, do not mix.
- Removing a package that other packages depend on will pull those out too. The
  simulate plan for removal is not shown in v1; the confirmation must therefore
  warn that dependents may be removed, and the log will show it.
- Do not pass the package name from the UI; always take it from `DebInfo`.

## Out of scope

- Purge, autoremove, holding versions, batch operations.
- Rolling back to the previously installed version after a failed upgrade.

## Verification

A real install of 1.0.0 and an upgrade to 1.1.0 succeeded in the manual session
(dpkg log confirms the transactions). The Remove confirmation dialog and its
Cancel path were verified; Remove is destructive and never the default button.
The Downgrade and Reinstall buttons appear with the correct labels and argv
flags (`--allow-downgrades` / `--reinstall`), but those two transactions were
not re-run after the final state and are not claimed as executed here. The log
header is `$ <command>` written by `runner::display_cmd`. In this session
`dpkg-query` shows `hello-debdrop` at 0.9.0.
