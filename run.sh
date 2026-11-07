#!/bin/sh
set -eu
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$APP_DIR"
if [ ! -x .venv/bin/python ]; then
    printf '%s\n' 'Chạy ./install.sh trước để cài phụ thuộc.'
    exit 1
fi
exec .venv/bin/python -m earfun "$@"
