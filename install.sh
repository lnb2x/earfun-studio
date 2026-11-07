#!/bin/sh
# Builds or installs the standalone Rust binary and registers a desktop launcher for the current user.
set -eu
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$APP_DIR"

if [ ! -x "$APP_DIR/earfun-studio" ] && [ ! -x "$APP_DIR/target/release/earfun-studio" ]; then
    if ! command -v cargo >/dev/null 2>&1; then
        printf '%s\n' 'Cần Rust/Cargo để biên dịch từ mã nguồn, hoặc tải gói biên dịch sẵn từ GitHub Releases.'
        exit 1
    fi
    cargo build --release
fi

BIN_PATH="$APP_DIR/earfun-studio"
if [ ! -x "$BIN_PATH" ]; then
    BIN_PATH="$APP_DIR/target/release/earfun-studio"
fi

DATA_HOME="${XDG_DATA_HOME:-$HOME/.local/share}"
APPS_DIR="$DATA_HOME/applications"
mkdir -p "$APPS_DIR"

cat > "$APPS_DIR/vn.earfun.Studio.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=EarFun Studio
Comment=Điều khiển EarFun Air Pro 4 qua Bluetooth LE (Rust)
Exec="$BIN_PATH"
Icon=$APP_DIR/assets/earfun-studio.svg
Terminal=false
Categories=AudioVideo;Audio;
StartupNotify=true
StartupWMClass=vn.earfun.Studio
EOF

printf 'Đã cài đặt biểu tượng ứng dụng. Mở "EarFun Studio" từ menu ứng dụng hoặc chạy: %s\n' "$BIN_PATH"
