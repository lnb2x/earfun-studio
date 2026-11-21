#!/bin/sh
set -eu
umask 022
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$APP_DIR"

VERSION=$(sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml | head -n 1)
case "$VERSION" in
    [0-9]*.[0-9]*.[0-9]*) ;;
    *)
        printf 'Không đọc được phiên bản hợp lệ từ Cargo.toml: "%s"\n' "$VERSION" >&2
        exit 1
        ;;
esac

cargo build --locked --release

mkdir -p dist

# 1. Build portable Ubuntu tar.gz bundle with normalized permissions
STAGE_TAR="$APP_DIR/target/package-tar/EarFunStudio-Ubuntu"
rm -rf "$APP_DIR/target/package-tar"
mkdir -p "$STAGE_TAR"
cp target/release/earfun-studio "$STAGE_TAR/earfun-studio"
cp -r assets docs run.sh install.sh uninstall.sh README.md LICENSE "$STAGE_TAR/"

find "$STAGE_TAR" -type d -exec chmod 755 {} +
find "$STAGE_TAR" -type f -exec chmod 644 {} +
chmod 755 "$STAGE_TAR/earfun-studio" "$STAGE_TAR/run.sh" "$STAGE_TAR/install.sh" "$STAGE_TAR/uninstall.sh"

TAR_OUT="dist/EarFunStudio-Ubuntu-${VERSION}-x86_64.tar.gz"
if tar --version 2>/dev/null | grep -q 'GNU tar'; then
    GZIP=-n tar --sort=name --owner=0 --group=0 --numeric-owner \
        -czf "$TAR_OUT" -C "$APP_DIR/target/package-tar" EarFunStudio-Ubuntu
else
    GZIP=-n tar -czf "$TAR_OUT" -C "$APP_DIR/target/package-tar" EarFunStudio-Ubuntu
fi
rm -rf "$APP_DIR/target/package-tar"
printf '%s\n' "$APP_DIR/$TAR_OUT"

# 2. Build Debian/Ubuntu .deb package if dpkg-deb is available
if command -v dpkg-deb >/dev/null 2>&1; then
    STAGE_DEB="$APP_DIR/target/package-deb/earfun-studio_${VERSION}_amd64"
    rm -rf "$APP_DIR/target/package-deb"
    mkdir -p "$STAGE_DEB/DEBIAN"
    mkdir -p "$STAGE_DEB/usr/bin"
    mkdir -p "$STAGE_DEB/usr/share/applications"
    mkdir -p "$STAGE_DEB/usr/share/icons/hicolor/scalable/apps"
    mkdir -p "$STAGE_DEB/usr/share/doc/earfun-studio"

    cp target/release/earfun-studio "$STAGE_DEB/usr/bin/earfun-studio"
    cp assets/earfun-studio.svg "$STAGE_DEB/usr/share/icons/hicolor/scalable/apps/vn.earfun.Studio.svg"
    cp assets/fonts/OFL.txt "$STAGE_DEB/usr/share/doc/earfun-studio/OFL.txt"

    cat > "$STAGE_DEB/usr/share/doc/earfun-studio/copyright" <<'EOF'
Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/
Upstream-Name: earfun-studio
Source: https://github.com/lnb2x/earfun-studio

Files: *
Copyright: 2026 EarFun Studio Contributors
License: MIT
 Permission is hereby granted, free of charge, to any person obtaining a copy
 of this software and associated documentation files (the "Software"), to deal
 in the Software without restriction, including without limitation the rights
 to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 copies of the Software, and to permit persons to whom the Software is
 furnished to do so, subject to the following conditions:
 .
 The above copyright notice and this permission notice shall be included in all
 copies or substantial portions of the Software.
 .
 THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 SOFTWARE.

Files: assets/fonts/*
Copyright: 2021 The Be Vietnam Pro Project Authors (https://github.com/bettergui/BeVietnamPro)
License: OFL-1.1
 This Font Software is licensed under the SIL Open Font License, Version 1.1.
 See /usr/share/doc/earfun-studio/OFL.txt for the full license text.
EOF

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

    INSTALLED_KB=$(du -sk "$STAGE_DEB/usr" | awk '{print $1}')

    cat > "$STAGE_DEB/DEBIAN/control" <<EOF
Package: earfun-studio
Version: ${VERSION}
Section: sound
Priority: optional
Architecture: amd64
Installed-Size: ${INSTALLED_KB}
Depends: libc6, libdbus-1-3, bluez, libxkbcommon0, libxkbcommon-x11-0, libwayland-client0, libgl1, libgtk-3-0t64 | libgtk-3-0, libglib2.0-0t64 | libglib2.0-0
Recommends: pipewire, wireplumber
Maintainer: EarFun Studio Contributors <noreply@github.com>
Homepage: https://github.com/lnb2x/earfun-studio
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
elif [ -n "${CI:-}" ]; then
    printf 'dpkg-deb is required in CI to build the .deb package.\n' >&2
    exit 1
fi
