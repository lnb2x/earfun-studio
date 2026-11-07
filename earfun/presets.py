"""TW600 EQ facts, transcribed from the owner's EarFun Audio 20.1.61 APK."""
import json
from pathlib import Path
from .protocol import FREQUENCIES


def validate_levels(levels):
    if not isinstance(levels, list) or len(levels) != 10 or any(type(v) is not int or not -10 <= v <= 10 for v in levels):
        raise ValueError('Bản chỉnh cần 10 mức nguyên từ −10 đến +10 theo bảng gain TW600.')
    return list(levels)


CATALOG = json.loads((Path(__file__).resolve().parent.parent/'assets'/'eq-presets.json').read_text())
BUILTINS = tuple(CATALOG['presets'])
for _preset in BUILTINS:
    validate_levels(_preset['levels'])


def options(custom):
    return [('draft', 'Tùy chỉnh (bản nháp)', None)] + [
        (p['key'], p['group']+' · '+p['name'], list(p['levels'])) for p in BUILTINS
    ] + [('custom:'+name, 'Cá nhân · '+name, list(levels)) for name, levels in custom.items()]


def encode(name, levels):
    return {'format': 'earfun-studio.eq', 'version': 1, 'model': 'TW600',
            'name': name.strip() or 'Bản chỉnh nhập', 'frequencies': list(FREQUENCIES),
            'levels': validate_levels(levels)}


def decode(document):
    if not isinstance(document, dict) or document.get('format') != 'earfun-studio.eq' or document.get('version') != 1:
        raise ValueError('Không phải tệp EQ EarFun Studio phiên bản 1.')
    if document.get('model') != 'TW600' or document.get('frequencies') != list(FREQUENCIES):
        raise ValueError('Tệp EQ không đúng mẫu TW600 hoặc tần số 10 dải.')
    name = document.get('name')
    if not isinstance(name, str) or not name.strip() or len(name) > 100:
        raise ValueError('Tên bản chỉnh phải có từ 1 đến 100 ký tự.')
    return name.strip(), validate_levels(document.get('levels'))
