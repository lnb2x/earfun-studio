"""User-approved setting changes and their explicit dependencies."""
from dataclasses import dataclass

CODECS = ('ldac', 'aptx', 'adaptive', 'lossless')


@dataclass(frozen=True)
class Step:
    title: str
    setter: int
    query: int
    value: int
    wanted: tuple


@dataclass(frozen=True)
class Plan:
    title: str
    baseline: tuple
    queries: tuple
    steps: tuple
    target: tuple

    @property
    def confirmation(self):
        actions = ' → '.join(step.title for step in self.steps)
        return f'{actions}. Tai nghe có thể ngắt nhạc hoặc khởi động lại; ứng dụng sẽ tự kết nối lại và kiểm tra kết quả. Tiếp tục?'


def known(state, keys):
    if any(type(state.get(key)) is not bool for key in keys):
        raise ValueError('Chưa đọc đủ trạng thái tai nghe. Bấm Đọc lại trạng thái rồi thử lại.')
    return tuple((key, state[key]) for key in keys)


def codec_step(values):
    bits = sum(bit for key, bit in [('aptx', 1), ('adaptive', 2), ('ldac', 8), ('lossless', 16)] if values[key])
    return Step('Áp dụng codec đã chọn', 46, 47, bits, tuple((key, values[key]) for key in CODECS))


def codecs(state, wanted):
    baseline = known(state, ('dual',)+CODECS)
    known(wanted, CODECS)
    steps = []
    if wanted['ldac'] and state['dual']:
        steps.append(Step('Tắt kết nối hai thiết bị để dùng LDAC', 39, 38, 0, (('dual', False),)))
    if any(state[key] != wanted[key] for key in CODECS):
        steps.append(codec_step(wanted))
    return Plan('Đổi codec Bluetooth', baseline, (38, 47), tuple(steps), tuple((key, wanted[key]) for key in CODECS) + (('dual', False if wanted['ldac'] else state['dual']),))


def dual(state, on):
    baseline = known(state, ('dual',)+CODECS)
    steps = []
    if on and state['ldac']:
        wanted = {key: state[key] for key in CODECS}
        wanted['ldac'] = False
        step = codec_step(wanted)
        steps.append(Step('Tắt LDAC để dùng hai thiết bị', step.setter, step.query, step.value, step.wanted))
    if state['dual'] != on:
        steps.append(Step('Bật kết nối hai thiết bị' if on else 'Tắt kết nối hai thiết bị', 39, 38, int(on), (('dual', on),)))
    return Plan('Đổi kết nối đa điểm', baseline, (38, 47), tuple(steps), (('dual', on),) + tuple((key, False if on and key == 'ldac' else state[key]) for key in CODECS))


def restart_setting(state, key, on):
    baseline = known(state, (key,))
    if key == 'le_audio':
        step = Step('Bật LE Audio' if on else 'Dùng Bluetooth thông thường / Google Fast Pair', 75, 76, int(on), ((key, on),))
    elif key == 'ear':
        step = Step('Bật nhận biết đeo tai' if on else 'Tắt nhận biết đeo tai', 73, 74, int(not on), ((key, on),))
    else:
        raise ValueError('Thiết lập không có quy trình khởi động lại.')
    return Plan(step.title, baseline, (step.query,), () if state[key] == on else (step,), step.wanted)
