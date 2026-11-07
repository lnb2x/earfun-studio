"""Control only the selected Bluetooth sink, never the default speaker."""
import json
import subprocess


def run(*args):
    return subprocess.check_output(args, text=True, timeout=5)


def find_sink(objects, mac):
    normalized = mac.replace(':', '').lower()
    if len(normalized) != 12:
        return None
    for obj in objects:
        props = obj.get('info', {}).get('props', {})
        if props.get('media.class') != 'Audio/Sink':
            continue
        if props.get('device.api') != 'bluez5' and not props.get('node.name', '').startswith('bluez_output.'):
            continue
        identities = [props.get('api.bluez5.address', ''), props.get('api.bluez5.path', ''), props.get('node.name', '')]
        if any(normalized in s.replace(':', '').replace('_', '').lower() for s in identities):
            return str(obj['id'])
    return None


def volume(mac, percent=None):
    sink = find_sink(json.loads(run('pw-dump')), mac)
    if sink is None:
        raise RuntimeError('Chưa có đầu ra âm thanh EarFun. Kết nối tai nghe trong Cài đặt Bluetooth Ubuntu.')
    if percent is not None:
        # Confirm the node still belongs to this address before writing volume.
        identity = run('wpctl', 'inspect', sink).replace(':', '').replace('_', '').lower()
        if mac.replace(':', '').lower() not in identity:
            raise RuntimeError('Đầu ra âm thanh đã thay đổi. Hãy đọc lại âm lượng.')
        run('wpctl', 'set-volume', sink, f'{max(0, min(100, int(percent)))}%')
    return run('wpctl', 'get-volume', sink).strip()
