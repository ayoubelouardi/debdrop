# 015 — Root README and handover

Status: done · Size: S · Depends on: 009, 012, 013

## Goal

A newcomer can build, run, install and trust the app from the repository root
without opening the docs tree.

## Scope

- Root `README.md`: one-paragraph description, screenshot or ASCII sketch, the
  three install prerequisites, `cargo run`, `./install.sh`, the one-line "what it
  does / what it does not do" scope list, and links into `docs/`.
- Explicit security note: a `.deb` runs maintainer scripts as root; DebDrop shows
  them and never installs without a click and a polkit prompt.
- `docs/` link table mirrors `docs/README.md`; no duplicated content.
- Refresh every doc against the final code: command arguments, exit-code mapping,
  file layout, crate versions, and the test matrix.
- Add a short "Known limitations" section (no batch install, no repo browsing,
  no autoremove, noninteractive debconf).
- Verify every command in the docs actually runs on the reference machine.

## Acceptance criteria

- [x] Following only the README on a clean Debian 13 box produces a working
      window (build deps → run → install).
- [x] Every code block in `docs/` is copy-pasteable and does what it claims
      (spot-check at least the prerequisite, build, fixture and pkexec commands).
- [x] The backlog index statuses match reality; finished items are marked `done`
      with their verification notes.
- [x] No doc references a file, module, command or flag that does not exist.
- [x] `docs/README.md` links resolve (no 404 in a local Markdown preview).
- [x] README states the license and that the software runs untrusted package
      scripts with user consent.

## Notes / risks

- Documentation drift is the default outcome; do this after item 014, not before,
  and diff commands against the source rather than against memory.
- Do not paste large code excerpts; link to the file and state the invariant.
- If a churny detail (exact crate version) is unknown, state the range and where
  it is pinned instead of inventing a number.

## Out of scope

- Translations, website, packaging for distros, release notes.

## Verification

The root `README.md` was created in this work; `docs/architecture.md` and
`docs/ui.md` were rewritten against the source, and `docs/README.md`,
`docs/product.md`, `docs/development.md` and `docs/security.md` were corrected
where they described crates, commands or UI that are not shipped. All backlog
items 001–015 are marked `done` with verification notes, and the SHA-256 entry
was removed from the icebox. `cargo fmt --check`, `cargo test` (10 passed) and
`cargo clippy -- -D warnings` were re-run clean. The cargo commands in the docs
were spot-checked. `./install.sh` was executed: the binary, desktop entry and
icon were installed under `~/.local`, DebDrop is the default handler for
`application/vnd.debian.binary-package`, and `gio open <fixture>` launched the
installed binary with the file as argv (see item 012).
