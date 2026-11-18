#!/bin/sh
set -eu
DATA_HOME="${XDG_DATA_HOME:-}"
if [ -z "$DATA_HOME" ]; then
    HOME_DIR="${HOME:-}"
    if [ -z "$HOME_DIR" ]; then
        printf '%s\n' 'Biến môi trường HOME hoặc XDG_DATA_HOME chưa được thiết lập.' >&2
        exit 1
    fi
    DATA_HOME="$HOME_DIR/.local/share"
fi
rm -f "$DATA_HOME/applications/vn.earfun.Studio.desktop"
printf '%s\n' 'Đã gỡ biểu tượng ứng dụng. Tệp thực thi và cấu hình cá nhân vẫn được giữ nguyên.'
