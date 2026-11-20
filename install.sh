#!/usr/bin/env bash
# User-local install. No root, no system paths.
set -euo pipefail
cd "$(dirname "$0")"

if [ "$(id -u)" -eq 0 ]; then
    echo "Run install.sh as your user, not as root." >&2
    exit 1
fi

BIN="${HOME}/.local/bin"
DATA="${XDG_DATA_HOME:-${HOME}/.local/share}"

refresh_caches() {
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "${DATA}/applications" >/dev/null 2>&1 || true
    else
        echo "note: update-desktop-database not found; menu may need a re-login"
    fi
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "${DATA}/icons/hicolor" >/dev/null 2>&1 || true
    fi
}

if [ "${1:-}" = "--uninstall" ]; then
    rm -f "${BIN}/debdrop" \
          "${DATA}/applications/debdrop.desktop" \
          "${DATA}/icons/hicolor/scalable/apps/debdrop.svg"
    refresh_caches
    echo "removed DebDrop from ${HOME}/.local"
    exit 0
fi

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo is required (rustup.rs)" >&2
    exit 1
fi

cargo build --release

install -Dm755 target/release/debdrop "${BIN}/debdrop"
install -Dm644 resources/debdrop.desktop "${DATA}/applications/debdrop.desktop"
install -Dm644 resources/debdrop.svg "${DATA}/icons/hicolor/scalable/apps/debdrop.svg"
refresh_caches

echo "installed:"
echo "  ${BIN}/debdrop"
echo "  ${DATA}/applications/debdrop.desktop"
echo "  ${DATA}/icons/hicolor/scalable/apps/debdrop.svg"
echo
echo "Ensure ${BIN} is on PATH, then launch DebDrop from your menu."
echo "Remove with: ./install.sh --uninstall"
