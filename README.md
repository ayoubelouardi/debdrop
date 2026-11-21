<div align="center">

  <img src="resources/debdrop.svg" width="128" height="128" alt="DebDrop Icon" />

  # DebDrop

  **The blazingly fast, transparent `.deb` package inspector and installer for Linux.**

  [![Rust 2021](https://img.shields.io/badge/Rust-2021%20%7C%201.85%2B-orange.svg?style=for-the-badge&logo=rust)](Cargo.toml)
  [![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg?style=for-the-badge)](LICENSE)
  [![GTK4 & Libadwaita](https://img.shields.io/badge/GUI-GTK4%20%2F%20Libadwaita-blueviolet.svg?style=for-the-badge&logo=gnome)](https://gnome.pages.gitlab.gnome.org/libadwaita/)
  [![Platform](https://img.shields.io/badge/Platform-Debian%20%7C%20Ubuntu-red.svg?style=for-the-badge&logo=debian)](https://www.debian.org)
  [![Zero Telemetry](https://img.shields.io/badge/Telemetry-Zero-success.svg?style=for-the-badge)](#security--privacy)
  [![Speed](https://img.shields.io/badge/Cold%20Start-%3C50ms-brightgreen.svg?style=for-the-badge)](#why-debdrop)

  <p>
    <b>Drop a <code>.deb</code> file. See its real metadata, inspect root maintainer scripts, and preview APT's transaction plan before a single file touches your system.</b>
  </p>

  <p>
    <a href="#quick-start">Quick Start</a> &bull;
    <a href="#why-debdrop">Why DebDrop?</a> &bull;
    <a href="#features">Key Features</a> &bull;
    <a href="#-engineering-roadmap">Roadmap</a> &bull;
    <a href="#documentation">Documentation</a>
  </p>

</div>

---

## ⚡ The Problem & The Solution

Installing a standalone `.deb` package on modern Linux shouldn't be a gamble:

- Running `sudo dpkg -i foo.deb` fails on missing dependencies, leaving `dpkg` in an unconfigured state.
- Running `apt-get install ./foo.deb` gives no preview, leaves you guessing what maintainer scripts run as root, and clutters your shell.
- Legacy GUI tools like `gdebi` feel outdated, and modern "Software Centers" take seconds to open, consume hundreds of megabytes of RAM, and obscure technical details behind generic progress bars.

**DebDrop** brings modern elegance, speed, and absolute transparency to local Debian package installation.

```text
 ┌──────────────────────┐
 │   Drop foo.deb       │   ◀── Instant metadata validation via dpkg-deb (unprivileged)
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐
 │   Inspect & Preview  │   ◀── Real APT transaction simulation (apt-get -s)
 │  • Check dependencies│   ◀── Warns if scripts (preinst/postinst) run as root
 │  • View root scripts │
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐
 │    Install with      │
 │    Single Polkit     │   ◀── Streams live, unbuffered apt output into sleek terminal
 └──────────┬───────────┘
            ▼
 ┌──────────────────────┐
 │   Done (< 1 second)  │   ◀── Re-reads live system state directly from dpkg-query
 └──────────────────────┘
```

---

## 📊 Why DebDrop?

| Feature | DebDrop | `gdebi` | GNOME Software | `sudo dpkg -i` |
| :--- | :---: | :---: | :---: | :---: |
| **Startup Time** | **< 50ms** | ~400ms | 3,000ms+ | Instant (CLI) |
| **Memory Footprint** | **~25 MB** | ~65 MB | 250 MB+ | ~5 MB |
| **Maintainer Script Inspector** | **Yes (view pre/postinst)** | No | No | No |
| **APT Dry-Run Simulation** | **Yes (`apt-get -s`)** | Basic | No | No (Fails on missing deps) |
| **Interface** | **Native GTK4 / Libadwaita** | Ancient GTK3 | GTK4 / Store | Terminal only |
| **Live Streaming Output** | **Yes (Terminal Card)** | Terminal popup | Generic bar | Terminal text |
| **Privilege Separation** | **Read-only until click** | Varies | Background daemon | Runs entire command as root |
| **Telemetry & Background Services**| **Zero** | Zero | PackageKit daemon | Zero |

---

## ✨ Features

- ⚡ **Blazingly Fast Native Binary**: Written in safe, modern Rust using GTK4 and Libadwaita. Instant cold startup with near-zero resource utilization.
- 🛡️ **Maintainer Script Safety Inspection**: Debian packages can execute arbitrary code as root during installation (`preinst`, `postinst`, `prerm`, `postrm`). DebDrop flags them immediately and lets you read their source before authenticating.
- 🔬 **Real APT Transaction Simulation**: Before any changes occur, DebDrop runs an unprivileged `apt-get -s` dry run. You see exactly what packages will be added, upgraded, downgraded, or removed.
- 📜 **Unbuffered Live Terminal Log**: No fake indeterminate progress meters. Watch APT's real stdout/stderr stream directly into an embedded developer terminal with phase indicators (`Resolving…`, `Unpacking…`, `Configuring…`).
- 🎨 **Adaptive Libadwaita UI**: Seamless integration with GNOME desktop environments, honoring system accent colors and automatic dark/light mode switching.
- 🔄 **Smart State Detection**: Automatically detects whether the package is new, already installed, an upgrade, a downgrade, or a reinstall.

---

## 🚀 Engineering Roadmap

We are actively developing DebDrop with a clear community roadmap. Check out our comprehensive [**Roadmap Document (`/docs/roadmap`)**](docs/roadmap.md) for full architectural milestones.

```
  v0.1: Foundations ──▶ v0.2: Deep Inspection ──▶ v0.3: Batch & Queues ──▶ v0.4: CLI & Ecosystem ──▶ v1.0: Universal
    [ Shipped ✅ ]          [ In Progress 🚧 ]          [ Planned 📋 ]           [ Planned 📋 ]          [ Future 🎯 ]
```

### 🌟 Roadmap Highlights

- **Phase 1: Foundations & Core Safety (v0.1 — Shipped ✅)**
  - Unprivileged metadata extraction (`dpkg-deb`)
  - APT dry-run dependency solver and conflict detection
  - Polkit privilege escalation with streaming live log
- **Phase 2: Deep Inspection & Cryptographic Verification (v0.2 — In Progress 🚧)**
  - 📦 **Internal Archive File Tree Browser**: Explore packaged files (`data.tar.*`) before installation without extracting them to disk.
  - 🔐 **Cryptographic Integrity**: SHA-256 / SHA-512 checksum calculation and verification against GPG keyrings.
  - 🔄 **Installed Version Diffing**: Compare local installed package files with the incoming `.deb`.
- **Phase 3: Power Workflows & Batch Queues (v0.3 — Planned 📋)**
  - Multi-deb drag & drop queue with automatic topological dependency resolution.
  - Interactive dependency conflict visualizer.
- **Phase 4: CLI Companion & Desktop Ecosystem (v0.4 — Planned 📋)**
  - `debdrop-cli` headless companion tool for terminal power users and automated CI scripts.
  - Desktop file manager extensions (Nautilus, Nemo, Dolphin, Thunar).

👉 Read the complete [**DebDrop Roadmap (`docs/roadmap.md`)**](docs/roadmap.md) to explore the design specifications and contribute!

---

## 🛠️ Quick Start

### Prerequisites (Debian 13 / Ubuntu 24.04+)

DebDrop requires Rust and the GTK4 / Libadwaita development headers:

```sh
sudo apt install cargo libgtk-4-dev libadwaita-1-dev pkg-config build-essential
```

Ensure a Polkit authentication agent (e.g. `lxpolkit` or `gnome-shell`) is running for privileged package operations.

### Build and Run

```sh
# Clone the repository
git clone https://github.com/debdrop/debdrop.git
cd debdrop

# Run in development mode
cargo run

# Or launch directly with a package
cargo run -- path/to/package.deb
```

### Install into `~/.local`

The included installer compiles an optimized release binary and installs the executable, desktop file, and high-resolution icons into `~/.local`:

```sh
./install.sh

# To uninstall later
./install.sh --uninstall
```

---

## 🖥️ Usage

- **Drag & Drop**: Drag any `.deb` file directly onto the DebDrop window.
- **File Dialog**: Click `Choose File…` or press <kbd>Ctrl</kbd> + <kbd>O</kbd>.
- **Command Line**: Pass one or more files directly:
  ```sh
  debdrop my-app_1.0.0_amd64.deb
  ```
- **Desktop "Open With"**: Right-click any `.deb` in your file manager and select **DebDrop**.

---

## 🔒 Security & Privacy

DebDrop follows a strict zero-trust security architecture:

1. **Unprivileged by Default**: Parsing archives, extracting control files, reading maintainer scripts, and checking dependencies are performed as your regular unprivileged user.
2. **Transparent Root Elevation**: When you click **Install** or **Remove**, DebDrop requests root permissions through Polkit (`pkexec apt-get`). DebDrop never retains or caches credentials.
3. **Zero Telemetry**: DebDrop never connects to third-party servers, never reports usage telemetry, and makes no network requests of its own.

---

## 📚 Documentation

Detailed documentation and Architecture Decision Records (ADRs) are located in [`docs/`](docs/):

- 🗺️ [**Engineering Roadmap**](docs/roadmap.md) — Future releases, specifications, and milestones.
- 📐 [**Architecture Overview**](docs/architecture.md) — Internal design, thread boundaries, and external command contracts.
- 🛡️ [**Security Model**](docs/security.md) — Trust boundaries, privilege escalation, and isolation.
- 🎨 [**UI Specification**](docs/ui.md) — Widget hierarchy, keyboard navigation, and accessibility.
- 🧪 [**Development & Testing**](docs/development.md) — Test fixtures, manual test matrix, and release checklist.
- 📝 [**Decisions (ADRs)**](docs/decisions/) — Historical rationale behind engineering choices.

---

## 📄 License

DebDrop is open-source software licensed under the [MIT License](LICENSE).
