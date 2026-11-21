# DebDrop Roadmap & Technical Vision

Welcome to the DebDrop engineering roadmap. DebDrop is designed to become the gold standard for inspecting and installing Debian packages (`.deb`) on modern Linux workstations.

---

## 🎯 Vision & Core Pillars

1. **⚡ Uncompromising Performance**: Pure native Rust compiled down to machine code with zero Electron, zero Python, and sub-50ms cold starts. Minimal memory footprint (<30MB RSS under active rendering).
2. **🛡️ Absolute Transparency & Safety**: No black-box operations. Users must be able to inspect what a package contains, what dependencies it pulls, and what maintainer root scripts it executes before giving root polkit credentials.
3. **🎨 Modern Desktop Craft**: Deep, first-class Libadwaita / GTK4 integration honoring GNOME Human Interface Guidelines, smooth Wayland drag-and-drop, and dynamic adaptive dark/light theming.
4. **🔒 Minimal Privilege Boundary**: Completely unprivileged inspection; privilege elevation is requested only for the transaction execution via Polkit. Zero telemetry, zero analytics, zero external network queries of its own.

---

## 🗺️ Milestone Matrix

| Milestone | Focus | Target Status |
| :--- | :--- | :--- |
| **v0.1** | Core Foundation & Transaction Transparency | **Shipped (Current)** |
| **v0.2** | Deep Inspection & Cryptographic Integrity | **In Progress 🚧** |
| **v0.3** | Power Workflows, Batch Queues & Conflicts | **Planned 📋** |
| **v0.4** | Headless CLI Companion & File Manager Synergy | **Planned 📋** |
| **v1.0** | Universal Distribution, PPA & Sandbox Portals | **Future Target 🎯** |

---

## 📦 Phase Details

### Phase 1: v0.1 — Foundations & Core Safety (Shipped ✅)
The fundamental inspection and execution loop:
- [x] **Safe Archive Parsing**: Extract control fields directly with `dpkg-deb -I` without shell execution.
- [x] **Real APT Simulation**: Dry-run preview with `apt-get -s` to list newly installed, upgraded, downgraded, or unresolvable dependencies.
- [x] **Installed State Detection**: Query `dpkg-query` to distinguish between Fresh Install, Reinstall, Upgrade, and Downgrade.
- [x] **Root Maintainer Script Discovery**: Detect and preview `preinst`, `postinst`, `prerm`, and `postrm` scripts in read-only dialogs before execution.
- [x] **Streaming Live Log**: Live asynchronous stdout/stderr capture from `pkexec apt-get` with coarse phase tracking (`Unpacking…`, `Configuring…`).
- [x] **Safe Termination**: Graceful cancellation handling and lock detection.

---

### Phase 2: v0.2 — Deep Inspection & Cryptographic Integrity (In Progress 🚧)
Giving users surgical visibility into archive payloads and provenance:
- [ ] **Internal File Tree Explorer**:
  - Interactive file browser showing all files and directories packaged in `data.tar.*` (via `dpkg-deb -c`).
  - Search and filter files (e.g. identify if binaries are installed to `/usr/bin`, systemd services installed to `/etc/systemd`, etc.).
  - File size distribution visualization.
- [ ] **Cryptographic Hash & Integrity Verification**:
  - Compute SHA-256, SHA-512, and MD5 checksums of the `.deb` archive with one-click copy.
  - Match archive hashes against known repository indices when available.
  - Inspect internal file `md5sums` control data to ensure payload integrity.
- [ ] **Version Diffing Engine**:
  - If a package is already installed, show a side-by-side diff of changed, added, and removed files between the local system and the incoming `.deb`.
- [ ] **Enhanced APT Stale Index Healing**:
  - Automatically detect partial index update states and offer seamless one-click list refresh before re-evaluating the plan.

---

### Phase 3: v0.3 — Power Workflows & Batch Queuing (Planned 📋)
Optimizing multi-package installation and dependency conflict resolution:
- [ ] **Multi-Deb Drag & Drop Queue**:
  - Drag multiple `.deb` files at once or drop a folder of packages.
  - Automatically compute topological dependency ordering across dropped files.
  - Single atomic installation transaction for the entire queue.
- [ ] **Interactive Dependency Conflict Solver**:
  - Clear visual explanation when a package breaks existing packages (`Conflicts:`, `Breaks:`).
  - Show alternate providers for virtual packages.
- [ ] **Post-Transaction Orphan Cleaner (Opt-in)**:
  - Dry-run preview of leftover orphaned dependencies with an optional one-click clean.

---

### Phase 4: v0.4 — CLI Companion & Desktop Ecosystem (Planned 📋)
Expanding DebDrop beyond the main GUI window:
- [ ] **`debdrop-cli` Headless Companion**:
  - Standalone headless CLI binary sharing the same core Rust crates (`debdrop-core`).
  - Terminal output formatting with colored diffs and script previews.
  - Perfect for CI/CD pipelines, container builds, and terminal purists.
- [ ] **File Manager Extensions**:
  - Native contextual actions for GNOME Files (Nautilus), Nemo, Dolphin, and Thunar.
  - "Inspect with DebDrop" default handler registration.
- [ ] **Interactive Debconf Bridge**:
  - Terminal popup or native dialog for packages that require interactive debconf question handling instead of silent defaults.

---

### Phase 5: v1.0 — Universal Distribution & Packaging (Future 🎯)
Making DebDrop effortlessly available across all Linux ecosystems:
- [ ] **Official Upstream Debian & Ubuntu Packaging**:
  - Debian ITP (Intent to Package) submission for official Debian repositories.
  - Dedicated Ubuntu PPA with automated launchpad builds.
- [ ] **Arch Linux & Derivatives**:
  - AUR package for Arch / Manjaro users managing Debian packages in chroots or containers.
- [ ] **Flatpak / XDG Desktop Portal Integration**:
  - Sandbox distribution with host-exec portal bridge for root transaction execution.
- [ ] **Localization (i18n)**:
  - Community translations for the UI and error taxonomies via Gettext.

---

## 🤝 Contributing to the Roadmap

Have ideas, bug reports, or feature requests?
- Open an issue on GitHub to discuss design proposals.
- Check out existing [ADRs (Architecture Decision Records)](decisions/) for technical context on our architectural choices.
- Join the discussion in our backlog items under [`docs/backlog/`](backlog/).
