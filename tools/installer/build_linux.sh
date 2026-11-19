#!/usr/bin/env bash
# Build Constellations Linux Packages (AppImage & .deb, P9-01, EDGE_CASES §13).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="0.1.0"
OUT="$ROOT/out/installer"
APPDIR="$OUT/Constellations.AppDir"

echo "=== Step 1: Cargo build release ==="
cd "$ROOT"
cargo build --release -p shell

echo "=== Step 2: Stage AppDir ==="
rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
mkdir -p "$APPDIR/usr/lib/constellations"
mkdir -p "$APPDIR/usr/share/applications"
mkdir -p "$APPDIR/usr/share/icons/hicolor/256x256/apps"

cp "$ROOT/target/release/constellations" "$APPDIR/usr/bin/"
cp -r "$ROOT/ui" "$APPDIR/usr/lib/constellations/"

# Desktop file
cat <<EOF > "$APPDIR/usr/share/applications/constellations.desktop"
[Desktop Entry]
Name=Constellations
Comment=Web browser with tabs drawn as a trail of stars
Exec=constellations %U
Icon=constellations
Type=Application
Categories=Network;WebBrowser;
MimeType=text/html;text/xml;application/xhtml+xml;x-scheme-handler/http;x-scheme-handler/https;
StartupWMClass=constellations
EOF

cp "$APPDIR/usr/share/applications/constellations.desktop" "$APPDIR/constellations.desktop"

# AppRun script
cat <<'EOF' > "$APPDIR/AppRun"
#!/bin/sh
HERE="$(dirname "$(readlink -f "${0}")")"
export PATH="${HERE}/usr/bin:${PATH}"
export LD_LIBRARY_PATH="${HERE}/usr/lib:${HERE}/usr/lib/constellations:${LD_LIBRARY_PATH}"
exec "${HERE}/usr/bin/constellations" "$@"
EOF
chmod +x "$APPDIR/AppRun"

echo "AppDir staged at $APPDIR"

echo "=== Step 3: Package .deb ==="
DEBDIR="$OUT/deb/constellations_${VERSION}_amd64"
rm -rf "$DEBDIR"
mkdir -p "$DEBDIR/DEBIAN"
mkdir -p "$DEBDIR/usr/bin"
mkdir -p "$DEBDIR/usr/share/constellations"
mkdir -p "$DEBDIR/usr/share/applications"

cat <<EOF > "$DEBDIR/DEBIAN/control"
Package: constellations
Version: ${VERSION}
Section: web
Priority: optional
Architecture: amd64
Maintainer: Constellations Authors <noreply@constellations.browser>
Description: A desktop web browser with tabs drawn as a trail of stars.
 Floating command bar at the bottom and private-first design.
EOF

cp "$ROOT/target/release/constellations" "$DEBDIR/usr/bin/"
cp -r "$ROOT/ui" "$DEBDIR/usr/share/constellations/"
cp "$APPDIR/usr/share/applications/constellations.desktop" "$DEBDIR/usr/share/applications/"

if command -v dpkg-deb >/dev/null 2>&1; then
    dpkg-deb --build "$DEBDIR" "$OUT/constellations_${VERSION}_amd64.deb"
    echo "Built $OUT/constellations_${VERSION}_amd64.deb"
else
    echo "dpkg-deb not found; .deb structure prepared at $DEBDIR."
fi
