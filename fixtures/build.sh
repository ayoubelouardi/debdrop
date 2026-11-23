#!/usr/bin/env bash
# Build the disposable fixtures used by the manual test matrix.
# These packages install one script to /usr/bin and nothing else.
set -euo pipefail
cd "$(dirname "$0")"

SRC=hello-debdrop
OUT=build
rm -rf "$OUT"
mkdir -p "$OUT"

make_pkg() {
    local package="$1" version="$2" depends="${3:-}" extra="${4:-}"
    local dir="$OUT/${package}_${version}"
    rm -rf "$dir"
    cp -a "$SRC" "$dir"
    sed -i "s/^Package:.*/Package: $package/" "$dir/DEBIAN/control"
    sed -i "s/^Version:.*/Version: $version/" "$dir/DEBIAN/control"
    if [ -n "$depends" ]; then
        sed -i "/^Version:/a Depends: $depends" "$dir/DEBIAN/control"
    fi
    if [ "$extra" = "noscripts" ]; then
        rm -f "$dir/DEBIAN/postinst"
    elif [ "$extra" = "badpostinst" ]; then
        printf '#!/bin/sh\necho "deliberate failure from the fixture" >&2\nexit 1\n' > "$dir/DEBIAN/postinst"
    fi
    chmod 0755 "$dir/DEBIAN"
    chmod 0644 "$dir/DEBIAN/control"
    [ -f "$dir/DEBIAN/postinst" ] && chmod 0755 "$dir/DEBIAN/postinst"
    dpkg-deb --build --root-owner-group "$dir" "$OUT/${package}_${version}_all.deb" >/dev/null
}

make_pkg hello-debdrop 1.0.0
make_pkg hello-debdrop 1.1.0
make_pkg hello-debdrop 0.9.0
make_pkg debdrop-noscripts 1.0.0 "" noscripts
make_pkg debdrop-baddep 1.0.0 "nonexistent-debdrop-dep"
make_pkg debdrop-badpostinst 1.0.0 "" badpostinst

# debconf variant: config + templates in the control archive.
dir="$OUT/debdrop-debconf_1.0.0"
rm -rf "$dir"
cp -a "$SRC" "$dir"
sed -i "s/^Package:.*/Package: debdrop-debconf/" "$dir/DEBIAN/control"
chmod 0755 "$dir/DEBIAN"
cat > "$dir/DEBIAN/templates" <<'EOF'
Template: debdrop/test-question
Type: boolean
Default: true
Description: Test debconf question?
 Only used by the DebDrop test fixture.
EOF
cat > "$dir/DEBIAN/config" <<'EOF'
#!/bin/sh
set -e
. /usr/share/debconf/confmodule
db_input low debdrop/test-question || true
db_go || true
EOF
chmod 0755 "$dir/DEBIAN/config"
dpkg-deb --build --root-owner-group "$dir" "$OUT/debdrop-debconf_1.0.0_all.deb" >/dev/null

# GUI app variant: desktop file, icon, and Homepage in control.
dir="$OUT/hello-debdrop-gui_1.0.0"
rm -rf "$dir"
cp -a "$SRC" "$dir"
sed -i "s/^Package:.*/Package: hello-debdrop-gui/" "$dir/DEBIAN/control"
echo "Homepage: https://debdrop.example.com" >> "$dir/DEBIAN/control"
mkdir -p "$dir/usr/share/applications" "$dir/usr/share/pixmaps"
cat > "$dir/usr/share/applications/hello-debdrop.desktop" <<'EOF'
[Desktop Entry]
Type=Application
Name=Hello DebDrop GUI
GenericName=Sample Graphical Utility
Icon=hello-debdrop-icon
Exec=hello-debdrop
Categories=Utility;GTK;
EOF
cat > "$dir/usr/share/pixmaps/hello-debdrop-icon.svg" <<'EOF'
<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><rect width="64" height="64" rx="14" fill="#3584e4"/><circle cx="32" cy="32" r="16" fill="#ffffff"/></svg>
EOF
chmod 0755 "$dir/DEBIAN"
chmod 0644 "$dir/usr/share/applications/hello-debdrop.desktop" "$dir/usr/share/pixmaps/hello-debdrop-icon.svg"
dpkg-deb --build --root-owner-group "$dir" "$OUT/hello-debdrop-gui_1.0.0_all.deb" >/dev/null

echo "built:"
ls -1 "$OUT"/*.deb
