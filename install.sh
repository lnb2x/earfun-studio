#!/bin/sh
# Installs in this folder and registers a launcher for the current user only.
set -eu
APP_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$APP_DIR"
if ! python3 -c "import gi; gi.require_version('Gtk', '4.0'); from gi.repository import Gtk" 2>/dev/null; then
    printf '%s\n' 'Cần GTK 4 và Python. Chạy:' 'sudo apt install python3 python3-venv python3-gi gir1.2-gtk-4.0 bluez wireplumber' 'Sau đó chạy lại ./install.sh'
    exit 1
fi
if ! python3 -m venv --system-site-packages .venv; then
    printf '%s\n' 'Hãy cài python3-venv rồi thử lại: sudo apt install python3-venv'
    exit 1
fi
.venv/bin/python -m pip install -r requirements.txt
python3 - "$APP_DIR" <<'PY'
import os
import sys
from pathlib import Path
app = Path(sys.argv[1])
data = Path(os.environ.get('XDG_DATA_HOME', Path.home()/'.local/share'))
applications = data/'applications'
applications.mkdir(parents=True, exist_ok=True)
# Desktop Entry Exec quoting: backslashes and reserved characters inside quotes.
def quoted(value):
    value = str(value).replace('%', '%%')
    for ch in ('\\', '"', '`', '$'):
        value = value.replace(ch, '\\'+ch)
    return '"'+value+'"'
entry = '\n'.join(['[Desktop Entry]', 'Type=Application', 'Name=EarFun Studio',
    'Comment=Điều khiển EarFun Air Pro 4 qua Bluetooth LE',
    'Exec='+quoted(app/'run.sh'), 'Icon='+str(app/'assets/earfun-studio.svg'),
    'Terminal=false', 'Categories=AudioVideo;Audio;', 'StartupNotify=true',
    'StartupWMClass=vn.earfun.Studio', ''])
(applications/'vn.earfun.Studio.desktop').write_text(entry)
print('Đã cài. Mở “EarFun Studio” từ menu ứng dụng hoặc chạy: '+str(app/'run.sh'))
PY
