#!/bin/sh
set -eu
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$APP_DIR"

if [ -x "$APP_DIR/earfun-studio" ]; then
    exec "$APP_DIR/earfun-studio" "$@"
elif [ -x "$APP_DIR/target/release/earfun-studio" ]; then
    exec "$APP_DIR/target/release/earfun-studio" "$@"
else
    exec cargo run --release -- "$@"
fi
