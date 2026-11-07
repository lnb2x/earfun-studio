"""TW600 wire format ported from the supplied EarFun Studio C# source."""
from dataclasses import dataclass
import struct

SERVICE = '00001100-d102-11e1-9b23-00025b00a5a5'
TX = '00001101-d102-11e1-9b23-00025b00a5a5'
RX = '00001102-d102-11e1-9b23-00025b00a5a5'
QUERIES = (9, 6, 7, 23, 21, 59, 61, 19, 38, 57, 49, 47, 74, 53, 11, 77, 4, 76, 84, 86)
GAINS = (-6, -5, -4, -3.5, -3, -2.5, -2, -1.5, -1, -.5, 0, .5, 1, 1.5, 2, 2.5, 3, 3.5, 4, 5, 6)
FREQUENCIES = (31.5, 63, 125, 250, 500, 1000, 2000, 4000, 8000, 16000)
OPERATIONS = {'Phát / tạm dừng': 1, 'Tăng âm lượng': 2, 'Giảm âm lượng': 3,
              'Bài trước': 4, 'Bài tiếp': 5, 'Trợ lý giọng nói': 6, 'Chế độ game': 7,
              'Đổi chế độ nghe': 8, 'ANC / bình thường': 9, 'Xuyên âm / bình thường': 10,
              'ANC / xuyên âm': 11, 'Gọi lại': 13, 'Không gán': 16}
GESTURES = {'Chạm một lần': 1, 'Chạm hai lần': 2, 'Chạm ba lần': 3, 'Nhấn giữ': 17}


def packet(vendor, command, payload=b''):
    if len(payload) > 508:
        raise ValueError('Gói Bluetooth quá dài')
    return struct.pack('>HH', vendor, command) + bytes(payload)


def request(command, *payload):
    return packet(10, 0x300 | command, payload)


@dataclass(frozen=True)
class Reply:
    vendor: int
    command: int
    status: int
    payload: bytes


def decode(raw):
    if len(raw) < 5:
        return None
    streamed = len(raw) >= 31 and raw[:5] == b'\x00\x0a\x03\x2c\x00'
    if not raw[2] & 128 and not streamed:
        return None
    vendor, cmd, status = struct.unpack('>HHB', raw[:5])
    return Reply(vendor, cmd & 0x7fff, status, bytes(raw[5:]))


def gesture_packet(side, gesture, action):
    if side not in ('L', 'R'):
        raise ValueError('Tai nghe không hợp lệ')
    values = [GESTURES[gesture], OPERATIONS[action]]
    return request(10, *(values + [0, 0] if side == 'L' else [0, 0] + values))


def equalizer(levels):
    if len(levels) != 10 or any(type(x) is not int or not -10 <= x <= 10 for x in levels):
        raise ValueError('EQ cần 10 mức nguyên trong khoảng -10…10')
    return [packet(29, 0x0e01, struct.pack('>BhHhH', 161+i, -360, int(f*3),
                                         int(GAINS[levels[i]+10]*60), int(.7*4096)))
            for i, f in enumerate(FREQUENCIES)]


def state(reply):
    if reply.vendor != 10 or reply.command >> 8 != 3 or reply.status:
        return {}
    cmd, data = reply.command & 255, reply.payload
    if cmd == 9:
        return {'version': data.decode('utf-8', errors='replace').rstrip('\0')}
    if cmd == 44:
        if len(data) < 26:
            return {}
        return {'phone': {'num': data[0], 'name': data[1:20].decode('utf-8', errors='replace').rstrip('\0'),
                          'mac': data[22:25].hex().upper(), 'connected': data[25] == 1}}
    if cmd == 11:
        reverse = {v: k for k, v in OPERATIONS.items()}
        keys = {}
        for g, gesture in enumerate(GESTURES):
            for side, name in enumerate(('L', 'R')):
                pos = 1 + g*4 + side*2
                if pos < len(data):
                    keys[name+'.'+gesture] = reverse.get(data[pos], f'0x{data[pos]:02X}')
        return {'keys': keys} if keys else {}
    if not data:
        return {}
    v = data[0]
    if cmd in (6, 7, 23):
        return {{6: 'bat_l', 7: 'bat_r', 23: 'bat_box'}[cmd]: v} if v <= 100 else {}
    enums = {21: ('noise', ('normal', 'anc', 'ambient', 'adaptive', 'wind')),
             59: ('sub', ('EAR_ADAPTIVE', 'WIND', 'DEFAULT', 'BALANCE', 'ADAPTIVE')),
             61: ('amb', ('DEFAULT', 'TOTAL')), 57: ('prompt', (100, 75, 50, 25, 0)),
             49: ('mic', ('AUTO', 'L', 'R')), 77: ('codec', ('SBC', 'AAC', 'aptX', 'aptX HD', 'aptX Adaptive', 'aptX Lossless', 'LDAC'))}
    if cmd in enums:
        key, values = enums[cmd]
        idx = v-1 if cmd == 77 else v
        return {key: values[idx]} if 0 <= idx < len(values) else {}
    bools = {19: 'game', 38: 'dual', 74: 'ear', 76: 'le_audio'}
    if cmd == 76 and v <= 1:
        return {'le_audio': v == 1, 'google_fast_pair': v == 0}
    if cmd == 113:
        return {'voice_language': ('EN', 'ZH', 'DE', 'JA')[v]} if v < 4 else {}
    if cmd in bools:
        return {bools[cmd]: (v == 0 if cmd == 74 else v == 1)} if v <= 1 else {}
    if cmd == 47:
        return {k: bool(v & b) for k, b in [('aptx', 1), ('adaptive', 2), ('ldac', 8), ('lossless', 16)]}
    if cmd == 53:
        return {'dis_l': v in (1, 2), 'dis_r': v in (1, 3)} if v <= 3 else {}
    if cmd == 51:
        return {'find_l': bool(v & 1), 'find_r': bool(v & 2)}
    if cmd in (4, 84, 86):
        return {{4: 'api', 84: 'hearing_limit', 86: 'balance'}[cmd]: v}
    return {}
