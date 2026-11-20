# 003 — Parse and validate `.deb` files

Status: done · Size: M · Depends on: 002

## Goal

Turn a path into a trustworthy `DebInfo`, or reject it with a precise reason,
without any privilege.

## Scope

- `deb::load(path) -> Result<DebInfo, DebError>`:
  - canonicalize; require a regular file; require `.deb` suffix (case-insensitive).
  - run `dpkg-deb --info` first — success is the validity gate.
  - run `dpkg-deb -f <path>` once and parse the full control block, including
    folded continuation lines for the long description.
- Parse `Depends`/`Recommends` into `Dep { name, constraint }`, splitting on
  commas and `|` alternatives (keep all alternatives for display; the first is
  the preferred one).
- Detect scripts from the `--info` listing (`preinst`, `postinst`, `prerm`,
  `postrm`, `config`, `templates`) and expose `has_debconf`.
- Validate the package name against `^[a-z0-9][a-z0-9+.-]+$`; expose the reason
  in `DebError` if it fails.
- No checksum/hash field: `DebInfo` contains only dpkg-reported metadata.
- Unit tests over literal control texts: folded description, alternatives,
  versioned deps, empty fields, CRLF input.

## Acceptance criteria

- [x] A valid fixture yields name, version, arch, section, maintainer,
      installed size, short + long description, and parsed deps.
- [x] Filename is never used as a metadata source (test with a renamed file whose
      name contradicts its control fields).
- [x] A directory, a non-`.deb`, a truncated archive, and an empty file each
      produce a distinct `DebError` with no panic.
- [x] `dpkg-deb`'s stderr is captured and surfaced, not printed to the console.
- [x] Parsing a `.deb` performs zero privileged operations (verify with a log or
      by running as a user with no polkit agent — it must still work).
- [x] Tests pass with `cargo test`.

## Notes / risks

- `dpkg-deb -f` output is field-per-line with leading-space continuations; do not
  use a TOML/YAML parser.
- `Description:` first line is the synopsis; the rest is indented. Preserve blank
  lines inside it.
- Localized output would break parsing; pass `LC_ALL=C` to every child.
- `--info` listing format varies slightly across dpkg versions; match on file
  names, not on byte counts.

## Out of scope

- Dependency satisfaction (item 007) and installed state (item 006).
- Extracting payload contents or verifying signatures.

## Verification

All seven built fixtures in `fixtures/build/` load through `deb::load`. The
fixture Maintainer field contains `DebDrop Test <test@example.invalid>`; it
renders correctly because `ui/package_view.rs` applies `markup_escape_text` to
every value before setting a row title/subtitle. `cargo test` passes 10 tests,
including `loads_a_real_fixture_when_present` against
`hello-debdrop_1.0.0_all.deb`. No checksum is collected or displayed, by design
(see `docs/architecture.md`).
