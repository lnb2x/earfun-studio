#!/bin/sh
set -eu
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$APP_DIR"
mkdir -p dist
# List inputs explicitly; never include venv, private logs or the Windows reference.
tar -czf dist/EarFunStudio-Ubuntu-1.2.0.tar.gz --exclude='__pycache__' --transform='s,^,EarFunStudio-Ubuntu/,' \
    earfun assets tests packaging requirements.txt run.sh install.sh uninstall.sh README.md docs
printf '%s\n' "$APP_DIR/dist/EarFunStudio-Ubuntu-1.2.0.tar.gz"
