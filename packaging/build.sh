#!/bin/sh
set -eu
umask 022
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$APP_DIR"

VERSION="1.3.0"
cargo build --release

mkdir -p dist

# 1. Build portable Ubuntu tar.gz bundle
STAGE_TAR="$APP_DIR/target/package-tar/EarFunStudio-Ubuntu"
rm -rf "$APP_DIR/target/package-tar"
mkdir -p "$STAGE_TAR"
cp target/release/earfun-studio "$STAGE_TAR/earfun-studio"
chmod 755 "$STAGE_TAR/earfun-studio"
cp -r assets docs run.sh install.sh uninstall.sh README.md "$STAGE_TAR/"

tar -czf "dist/EarFunStudio-Ubuntu-${VERSION}-x86_64.tar.gz" -C "$APP_DIR/target/package-tar" EarFunStudio-Ubuntu
rm -rf "$APP_DIR/target/package-tar"
printf '%s\n' "$APP_DIR/dist/EarFunStudio-Ubuntu-${VERSION}-x86_64.tar.gz"

# 2. Build Debian/Ubuntu .deb package if dpkg-deb is available
if command -v dpkg-deb >/dev/null 2>&1; then
    STAGE_DEB="$APP_DIR/target/package-deb/earfun-studio_${VERSION}_amd64"
    rm -rf "$APP_DIR/target/package-deb"
    mkdir -p "$STAGE_DEB/DEBIAN"
    mkdir -p "$STAGE_DEB/usr/bin"
    mkdir -p "$STAGE_DEB/usr/share/applications"
    mkdir -p "$STAGE_DEB/usr/share/icons/hicolor/scalable/apps"

    cp target/release/earfun-studio "$STAGE_DEB/usr/bin/earfun-studio"
    cp assets/earfun-studio.svg "$STAGE_DEB/usr/share/icons/hicolor/scalable/apps/vn.earfun.Studio.svg"

    cat > "$STAGE_DEB/usr/share/applications/vn.earfun.Studio.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=EarFun Studio
Comment=Điều khiển EarFun Air Pro 4 qua Bluetooth LE
Exec=/usr/bin/earfun-studio
Icon=vn.earfun.Studio
Terminal=false
Categories=AudioVideo;Audio;
StartupNotify=true
StartupWMClass=vn.earfun.Studio
EOF

    cat > "$STAGE_DEB/DEBIAN/control" <<EOF
Package: earfun-studio
Version: ${VERSION}
Section: sound
Priority: optional
Architecture: amd64
Depends: libc6, libdbus-1-3, bluez, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libgl1
Recommends: pipewire, wireplumber
Maintainer: EarFun Studio Contributors <noreply@github.com>
Description: EarFun Studio cho EarFun Air Pro 4 (TW600)
 Ứng dụng desktop tiếng Việt viết bằng Rust điều khiển tai nghe
 EarFun Air Pro 4 (TW600) qua Bluetooth LE.
EOF

    find "$STAGE_DEB" -type d -exec chmod 755 {} +
    find "$STAGE_DEB" -type f -exec chmod 644 {} +
    chmod 755 "$STAGE_DEB/usr/bin/earfun-studio"

    dpkg-deb --build --root-owner-group "$STAGE_DEB" "dist/earfun-studio_${VERSION}_amd64.deb" >/dev/null
    rm -rf "$APP_DIR/target/package-deb"
    printf '%s\n' "$APP_DIR/dist/earfun-studio_${VERSION}_amd64.deb"
fi
