# DebDrop

DebDrop is a small GTK4 / libadwaita application for inspecting and installing
local `.deb` packages on Debian-based systems. Drop a file (or pick one, or pass
it on the command line), see its real metadata and what apt plans to do, then
install with a single polkit prompt. The log you see is apt's real output.

## Prerequisites

On Debian 13 you need Rust/cargo and the GTK4/libadwaita development headers:

```sh
sudo apt install cargo libgtk-4-dev libadwaita-1-dev pkg-config build-essential
```

A running polkit authentication agent (e.g. `lxpolkit`) is required for the
install/remove/refresh actions.

## Build and run

```sh
cargo run                     # debug build, opens the window
cargo build --release         # target/release/debdrop
cargo test                    # unit tests
```

Pass a `.deb` file to load it directly: `cargo run -- foo_1.0.0_all.deb`.

## Install (user-local)

```sh
./install.sh                  # builds release and installs into ~/.local
./install.sh --uninstall      # removes those files
```

The script installs `debdrop`, a desktop entry, and an icon under `~/.local`;
it does not use `sudo` and refuses to run as root. Ensure `~/.local/bin` is on
your `PATH`.

## Scope

**Does:** validate a `.deb` with `dpkg-deb`; show its metadata, dependencies and
maintainer-script names; preview apt's transaction with `apt-get -s`; detect the
installed version and offer Install / Reinstall / Upgrade / Downgrade / Remove;
install and remove through `pkexec apt-get` with a streaming log; refresh apt
package lists.

**Does not:** browse repositories or installed packages, install by package
name, handle Flatpak/Snap/AppImage/`.rpm`, install multiple files at once,
autoremove or purge, edit apt sources, or phone home.

## Security note

A `.deb` is not inert data: its maintainer scripts (`preinst`, `postinst`,
`prerm`, `postrm`) run **as root** during installation. DebDrop shows those
script names and lets you read their contents before you decide. Nothing is
installed without an explicit click and a polkit password prompt; all read-only
inspection runs as the normal user. DebDrop adds no signature or trust check
beyond what apt/dpkg already do.

## Documentation

- [Documentation index](docs/README.md)
- [Product scope](docs/product.md)
- [Architecture](docs/architecture.md)
- [Security model](docs/security.md)
- [UI](docs/ui.md)
- [Development and manual test matrix](docs/development.md)
- [Decisions](docs/decisions/) · [Backlog](docs/backlog/)

## Known limitations

- No batch install or multi-file queue.
- No repository browsing or search.
- No autoremove or purge.
- debconf questions are answered noninteractively with defaults.
- No SHA-256 or other checksum is computed or displayed.
- A polkit authentication agent must be present for privileged actions.

## License

MIT.
