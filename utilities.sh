#!/bin/sh
# DebDrop Developer & Maintenance Utilities Suite
# Fully POSIX / ash / dash / sh compliant.
set -eu

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# ANSI color codes for friendly terminal output
if [ -t 1 ]; then
    BOLD="\033[1m"
    GREEN="\033[32m"
    YELLOW="\033[33m"
    BLUE="\033[34m"
    CYAN="\033[36m"
    MAGENTA="\033[35m"
    RED="\033[31m"
    RESET="\033[0m"
else
    BOLD=""
    GREEN=""
    YELLOW=""
    BLUE=""
    CYAN=""
    MAGENTA=""
    RED=""
    RESET=""
fi

log_info() {
    printf "${BLUE}${BOLD}[INFO]${RESET} %s\n" "$*"
}

log_success() {
    printf "${GREEN}${BOLD}[SUCCESS]${RESET} %s\n" "$*"
}

log_warn() {
    printf "${YELLOW}${BOLD}[WARN]${RESET} %s\n" "$*"
}

log_error() {
    printf "${RED}${BOLD}[ERROR]${RESET} %s\n" "$*" >&2
}

cmd_clean() {
    log_info "Cleaning cargo build artifacts..."
    if command -v cargo >/dev/null 2>&1; then
        cargo clean
    else
        rm -rf target
    fi

    log_info "Cleaning test fixture builds..."
    rm -rf fixtures/build

    log_info "Cleaning temporary deb and lock artifacts..."
    find . -maxdepth 2 -type f -name "*.deb" ! -path "./fixtures/*" -exec rm -f {} +
    rm -f .*.swp *~ *.bak clean.sh dev.sh

    log_success "Clean completed successfully!"
}

cmd_test() {
    log_info "Ensuring test fixtures are built..."
    cmd_fixtures

    log_info "Running test suite..."
    cargo test "$@"
    log_success "All tests passed!"
}

cmd_check() {
    log_info "Running compiler checks..."
    cargo check
    log_success "Compiler check passed!"
}

cmd_build() {
    log_info "Compiling release binary with LTO and symbols stripped..."
    cargo build --release
    if [ -f "target/release/debdrop" ]; then
        SIZE=$(ls -lh target/release/debdrop | awk '{print $5}')
        log_success "Release binary ready: target/release/debdrop (${BOLD}${GREEN}$SIZE${RESET})"
    fi
}

cmd_run() {
    log_info "Running DebDrop in development mode..."
    cargo run -- "$@"
}

cmd_fixtures() {
    log_info "Building disposable test fixtures..."
    if [ -f "./fixtures/build.sh" ]; then
        sh ./fixtures/build.sh
        log_success "Fixtures built in fixtures/build/"
    else
        log_error "fixtures/build.sh not found!"
        return 1
    fi
}

cmd_inspect() {
    DEB_FILE="${1:-}"
    if [ -z "$DEB_FILE" ]; then
        log_error "Usage: $0 inspect <path/to/package.deb>"
        return 1
    fi

    if [ ! -f "$DEB_FILE" ]; then
        log_error "File not found: $DEB_FILE"
        return 1
    fi

    printf "${CYAN}${BOLD}=== Inspecting %s ===${RESET}\n" "$DEB_FILE"
    
    printf "\n${YELLOW}${BOLD}--- Control Information ---${RESET}\n"
    dpkg-deb -f "$DEB_FILE" Package Version Architecture Section Installed-Size Maintainer Homepage Description 2>/dev/null || dpkg-deb -I "$DEB_FILE"

    printf "\n${YELLOW}${BOLD}--- Maintainer Scripts (runs as root) ---${RESET}\n"
    SCRIPTS=$(dpkg-deb -I "$DEB_FILE" 2>/dev/null | grep -E "preinst|postinst|prerm|postrm" || true)
    if [ -n "$SCRIPTS" ]; then
        printf "%s\n" "$SCRIPTS"
    else
        printf "${GREEN}None (Clean declarative package)${RESET}\n"
    fi

    printf "\n${YELLOW}${BOLD}--- Desktop Integration ---${RESET}\n"
    DESKTOP=$(dpkg-deb -c "$DEB_FILE" | grep -E "applications/.*\.desktop" || true)
    if [ -n "$DESKTOP" ]; then
        printf "%s\n" "$DESKTOP"
    else
        printf "No .desktop application entry\n"
    fi

    printf "\n${YELLOW}${BOLD}--- Total Package Files ---${RESET}\n"
    COUNT=$(dpkg-deb -c "$DEB_FILE" | wc -l)
    printf "Total files and directories: %s\n" "$COUNT"
}

cmd_simulate() {
    DEB_FILE="${1:-}"
    if [ -z "$DEB_FILE" ]; then
        log_error "Usage: $0 simulate <path/to/package.deb>"
        return 1
    fi

    if [ ! -f "$DEB_FILE" ]; then
        log_error "File not found: $DEB_FILE"
        return 1
    fi

    ABS_PATH="$(cd "$(dirname "$DEB_FILE")" && pwd)/$(basename "$DEB_FILE")"
    log_info "Running unprivileged APT transaction simulation for: $ABS_PATH"
    apt-get -s install --no-install-recommends "$ABS_PATH"
}

cmd_size() {
    if [ ! -f "target/release/debdrop" ]; then
        log_info "Release binary not found; building now..."
        cmd_build
    fi
    SIZE=$(ls -lh target/release/debdrop | awk '{print $5}')
    printf "${BOLD}DebDrop Binary Footprint:${RESET}\n"
    printf "  Location:      target/release/debdrop\n"
    printf "  Stripped Size: %s\n" "$SIZE"
    if command -v file >/dev/null 2>&1; then
        file target/release/debdrop
    fi
}

cmd_install() {
    log_info "Running local installation via install.sh..."
    ./install.sh
}

cmd_uninstall() {
    log_info "Running local uninstallation via install.sh..."
    ./install.sh --uninstall
}

cmd_fmt() {
    log_info "Checking code formatting..."
    if cargo fmt --version >/dev/null 2>&1; then
        cargo fmt --check
        log_success "Formatting is clean!"
    else
        log_warn "cargo-fmt not installed, skipping."
    fi
}

cmd_clippy() {
    log_info "Running clippy lints..."
    if cargo clippy --version >/dev/null 2>&1; then
        cargo clippy -- -D warnings
        log_success "Clippy check clean!"
    else
        log_warn "cargo-clippy not installed, skipping."
    fi
}

cmd_ci() {
    log_info "Running complete CI validation pipeline..."
    cmd_clean
    cmd_fixtures
    cmd_check
    cmd_test
    cmd_build
    cmd_size
    log_success "Full CI pipeline passed cleanly! 🚀"
}

cmd_deb() {
    log_info "Building a distributable .deb package of DebDrop itself..."

    if ! command -v cargo-deb > /dev/null 2>&1; then
        log_warn "cargo-deb not found. Installing now (this may take a minute)..."
        cargo install cargo-deb
        log_success "cargo-deb installed!"
    fi

    log_info "Building release binary first..."
    cargo build --release

    log_info "Packaging with cargo-deb..."
    cargo deb

    DEB_FILE=$(find target/debian -name "*.deb" -newer target/release/debdrop 2>/dev/null | head -1)
    if [ -n "$DEB_FILE" ]; then
        SIZE=$(ls -lh "$DEB_FILE" | awk '{print $5}')
        log_success "Distributable .deb ready: ${BOLD}${GREEN}$DEB_FILE${RESET} (${SIZE})"
        printf "\n${YELLOW}${BOLD}Tip:${RESET} Install it with:  sudo dpkg -i %s\n" "$DEB_FILE"
    else
        log_warn "Build completed. Check target/debian/ for the .deb file."
    fi
}

show_help() {
    printf "${BOLD}${MAGENTA}DebDrop Developer & Maintenance Utilities${RESET}\n"
    printf "POSIX / ash / dash / sh compatible runner for builds, tests, cleanups, and package tools.\n\n"
    printf "${BOLD}Usage:${RESET} %s [command] [options]\n\n" "$0"
    printf "${BOLD}Commands:${RESET}\n"
    printf "  ${CYAN}clean${RESET}              Remove target/, fixtures/build/, and temporary files\n"
    printf "  ${CYAN}test${RESET} [filter]      Build fixtures and run test suite (e.g. test deb)\n"
    printf "  ${CYAN}check${RESET}             Run fast cargo check\n"
    printf "  ${CYAN}build${RESET}             Build optimized release binary (target/release/debdrop)\n"
    printf "  ${CYAN}run${RESET} [file.deb]    Run DebDrop in development mode\n"
    printf "  ${CYAN}fixtures${RESET}          Rebuild all disposable test fixtures in fixtures/build/\n"
    printf "  ${CYAN}inspect${RESET} <file.deb> Terminal utility to inspect a .deb's metadata and scripts\n"
    printf "  ${CYAN}simulate${RESET} <file.deb>Run unprivileged APT dry-run transaction simulation\n"
    printf "  ${CYAN}size${RESET}              Report release binary size and file information\n"
    printf "  ${CYAN}install${RESET}           Compile release and install user-local (~/.local)\n"
    printf "  ${CYAN}uninstall${RESET}         Uninstall user-local binary and desktop files\n"
    printf "  ${CYAN}fmt${RESET}               Check code formatting with cargo fmt\n"
    printf "  ${CYAN}clippy${RESET}            Run cargo clippy lints\n"
    printf "  ${CYAN}ci${RESET}                Full pipeline: clean, fixtures, check, test, build\n"
    printf "  ${CYAN}deb${RESET}               Build distributable .deb package (installs cargo-deb if needed)\n"
    printf "  ${CYAN}help${RESET}              Show this help message\n\n"
    printf "${BOLD}Examples:${RESET}\n"
    printf "  %s clean\n" "$0"
    printf "  %s test\n" "$0"
    printf "  %s inspect fixtures/build/hello-debdrop-gui_1.0.0_all.deb\n" "$0"
    printf "  %s simulate fixtures/build/hello-debdrop_1.0.0_all.deb\n" "$0"
    printf "  %s run fixtures/build/hello-debdrop-gui_1.0.0_all.deb\n\n" "$0"
}

ACTION="${1:-help}"
shift 2>/dev/null || true

case "$ACTION" in
    clean)
        cmd_clean
        ;;
    test)
        cmd_test "$@"
        ;;
    check)
        cmd_check
        ;;
    build)
        cmd_build
        ;;
    run)
        cmd_run "$@"
        ;;
    fixtures)
        cmd_fixtures
        ;;
    inspect)
        cmd_inspect "$@"
        ;;
    simulate)
        cmd_simulate "$@"
        ;;
    size)
        cmd_size
        ;;
    install)
        cmd_install
        ;;
    uninstall)
        cmd_uninstall
        ;;
    fmt)
        cmd_fmt
        ;;
    clippy)
        cmd_clippy
        ;;
    ci)
        cmd_ci
        ;;
    deb)
        cmd_deb
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        log_error "Unknown command: $ACTION"
        show_help
        exit 1
        ;;
esac
