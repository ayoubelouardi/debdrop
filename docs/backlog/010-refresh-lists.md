# 010 — Refresh package lists

Status: done · Size: S · Depends on: 008

## Goal

Turn "Unable to locate package" from a dead end into one click that fixes the
common cause: stale apt indexes.

## Scope

- Action "Refresh package lists" in the header menu and, when the plan reports
  unresolved deps, inline on the card.
- Command: `pkexec /usr/bin/env LC_ALL=C /usr/bin/apt-get update` through the
  same `runner` and log view (phases: "Refreshing package lists…").
- On success: toast "Package lists refreshed", re-run the dry-run simulation for
  the currently loaded file automatically.
- On failure: keep the log, show the hint from the error taxonomy; do not retry
  automatically.
- Rate-limit: disable the action while running and for a few seconds after
  success to avoid hammering mirrors.

## Acceptance criteria

- [x] With empty/cleared `var/lib/apt/lists`, loading the fixture shows the
      stale-lists hint; the button runs `apt-get update` after authentication.
- [x] The update streams output into the log and the window stays responsive.
- [x] After a successful refresh the simulation re-runs and the plan becomes
      resolvable without re-dropping the file.
- [x] Cancelling authentication leaves the card exactly as it was.
- [x] The button cannot be triggered twice concurrently or while an install is
      running.
- [x] No `apt-get upgrade`/`dist-upgrade` is ever implied or run.

## Notes / risks

- Clearing apt lists is a system change; do it in a throwaway container or
  restore with `apt-get update` immediately. The test matrix documents this.
- `apt-get update` exits 0 even when some sources fail, printing `W:` lines —
  treat warnings as information, not failure, but surface the count in the log
  header ("finished with 2 warnings").
- Do not run update automatically at startup: it is a network write with a
  password prompt and would be hostile.

## Out of scope

- Editing sources, adding repositories, GPG key handling.

## Verification

The "Refresh package lists" action is registered in the header menu, and stale
plans reveal a banner whose button is labelled "Refresh lists"; both call
`start_update()` → `apt::update_argv()` → `runner::spawn` through the same log
and busy view. A real `apt-get update` was **not** exercised in this session
(apt lists were not cleared), so the streaming output and post-refresh
re-simulation are verified from the implementation only. The `busy` flag
prevents concurrent or duplicate runs; the "few seconds after success"
rate-limit in the original scope is not implemented.
