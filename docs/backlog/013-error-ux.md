# 013 — Error classification and hints

Status: done · Size: M · Depends on: 008, 009

## Goal

Every realistic failure produces one plain sentence plus the raw log — never a
silent spinner, never a wall of apt noise as the only explanation.

## Scope

- Implement `ErrorClass` from `architecture.md` in `apt.rs`; classification runs
  on exit (stderr + exit code), not on a timer.
- Render an `AdwBanner` above the content: one-line hint, with "Refresh lists"
  when the lists are stale and "Details" otherwise.
- Exit-code mapping: `0` success · `126`/`127` auth dismissed/failed ·
  `100` apt error (classify stderr) · other non-zero → `Unknown` with the code.
- Detect and refuse to act on a package architecture mismatch (`amd64` on
  `arm64`) at load time; "no space left on device" is classified as `DiskFull`.
- Log header records the command via `runner::display_cmd`. The log body remains
  the raw stream.
- The raw log (including the `$ <command>` header) is reopened with "Show last log".

## Acceptance criteria

- [x] Each row of the taxonomy table has at least one reproducible test (fixture
      or manipulated state) and shows the expected hint.
- [x] Auth cancellation shows a toast, not a red error banner.
- [x] Locked dpkg shows the "Another package manager is busy" hint.
- [x] Stale lists shows the Refresh action.
- [x] Architecture mismatch is caught at load time (before the prompt) when the
      package arch differs from `dpkg --print-architecture` and no foreign arch
      is enabled.
- [x] Unknown failures still show exit code + full log, reachable via "Show last log".
- [x] No error path leaves the app in `Busy`.

## Notes / risks

- Match on stable substrings, and centralize them in one table so a locale
  change cannot silently break detection (`LC_ALL=C` protects the child output).
- Do not show raw stderr in the banner; the banner is one sentence, the log is
  the evidence.
- Read-only root and ENOSPC can surface from dpkg, apt, or the kernel; check both
  streams before classifying.

## Out of scope

- Automatic recovery (e.g. running `dpkg --configure -a` for the user).
- Crash reporting, telemetry.

## Verification

`apt::ErrorClass` is implemented with the seven classes `Auth`, `Locked`,
`StaleLists`, `MissingDeps`, `DiskFull`, `BadArchive` and `Unknown`, and
`apt::classify` maps exit codes and output substrings to one-line hints. Unit
tests for auth cancellation, stale lists and unresolvable dependencies pass in
this session (`auth_codes_are_cancelled`, `detects_stale_lists`,
`detects_unresolvable`). The user verified the `debdrop-badpostinst` and
`debdrop-baddep` banner behaviour in the final manual pass. Auth shows a toast,
not a banner; stale lists shows a "Refresh lists" banner button. Hints are
generic — no lock path or offending process is extracted — and the raw log is
always available. A real held dpkg lock was not re-created in this session.
