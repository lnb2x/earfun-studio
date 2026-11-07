#!/bin/sh
set -eu
rm -f "${XDG_DATA_HOME:-$HOME/.local/share}/applications/vn.earfun.Studio.desktop"
printf '%s\n' 'Đã gỡ biểu tượng ứng dụng. Mã nguồn, .venv và cấu hình cá nhân vẫn nằm trên máy.'
