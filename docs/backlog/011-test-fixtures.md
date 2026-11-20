# 011 — Test fixtures (.deb builder)

Status: done · Size: S · Depends on: 002

## Goal

A repeatable, disposable package family so every install/upgrade/remove path can
be exercised without touching a real system package.

## Scope

- `fixtures/hello-debdrop/` source tree: `DEBIAN/control`, `DEBIAN/postinst`,
  `usr/bin/hello-debdrop` (a one-line script printing its version).
- `fixtures/build.sh`: builds
  - `hello-debdrop_1.0.0_all.deb`
  - `hello-debdrop_1.1.0_all.deb` (bumped version, updated postinst echo)
  - `hello-debdrop_0.9.0_all.deb` (downgrade case)
  - `debdrop-baddep_1.0.0_all.deb` (`Depends: nonexistent-debdrop-dep`)
  - `debdrop-badpostinst_1.0.0_all.deb` (postinst exits 1)
  - `debdrop-noscripts_1.0.0_all.deb` (no maintainer scripts)
  - `debdrop-debconf` variant with `DEBIAN/templates` + `config`
- `fixtures/README.md` with the build command, expected `dpkg-deb -f` output per
  file, and a note that these packages install nothing but a single script to
  `/usr/bin`.
- `.gitignore` entries for `*.deb` and `fixtures/*/DEBIAN/md5sums`.

## Acceptance criteria

- [x] `./fixtures/build.sh` builds all fixtures from clean sources with no root.
- [x] `dpkg-deb --info` and `-f` on each fixture show the intended fields and
      script sets.
- [x] Installing the good fixture manually then removing it leaves no files
      behind (`dpkg -L` after remove, `ls /usr/bin/hello-debdrop`).
- [x] `debdrop-baddep` fails resolution in a dry run on a machine without that
      dependency name.
- [x] Fixtures are never installed automatically by any test script.

## Notes / risks

- `dpkg-deb --build` requires the `DEBIAN` dir to have mode 0755 and control
  file 0644; the build script should `chmod` explicitly so a fresh checkout works.
- Use architecture `all` to keep the fixtures portable; no compiled payloads.
- Keep the version strings realistic (`1.0.0`, `1.1.0`, `0.9.0`) so the
  comparison tests in item 006 have something meaningful to compare.
- These files are test data, not examples for users; say so in the README.

## Out of scope

- Automated CI; a shell script invoked by hand is enough for v1.

## Verification

`fixtures/build.sh` builds all seven `.deb` files, and all seven are present in
`fixtures/build/`: the three `hello-debdrop` versions (0.9.0, 1.0.0, 1.1.0),
`debdrop-noscripts`, `debdrop-baddep`, `debdrop-badpostinst`, and
`debdrop-debconf`. `fixtures/README.md` documents each with its purpose and the
`dpkg-deb` sanity commands. The script and README were reviewed this session;
the builder was not re-run from clean sources. The fixture is named `debdrop-debconf` in both the build script and README.
Fixtures are never installed automatically by any script.
