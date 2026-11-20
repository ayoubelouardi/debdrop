# 0001 — GTK4 + libadwaita instead of a web-based shell

Status: accepted · Date: 2026-11-20

## Context

The app is a single-window utility that runs a handful of local commands and
streams their output. The reference machine is Debian 13 on sway/Wayland with
GTK 4.18 and libadwaita 1.7 already installed; only the `-dev` packages are
missing. Alternatives considered: Tauri (WebKitGTK 4.1 not installed, adds a
second runtime and a JS toolchain), Electron (~200 MB before any code),
Python + GTK3 (available, but weaker tooling for a long-lived app and awkward
distribution).

## Decision

Rust + `gtk4-rs` + `libadwaita-rs`, one binary, no web runtime, CSS embedded as
a string.

## Consequences

- Drag & drop, file dialogs, and a native "system utility" feel come free and
  behave correctly under Wayland.
- Dark/light and system accent colour follow the desktop without custom theming.
- `cargo` is the entire build system; no meson, no gresource compilation.
- Distribution is a single ~5–10 MB binary plus a `.desktop` file and an SVG.
- Cost: GTK4/libadwaita API churn and Rust-GObject ownership rules; contributors
  need the `-dev` packages.
- Revisit if: the UI needs to be embedded in a browser or shipped to
  non-GTK systems. At that point add a new ADR rather than rewriting this one.
