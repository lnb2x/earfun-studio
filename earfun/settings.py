import json
import os
from pathlib import Path


class Settings:
    def __init__(self, path=None):
        self.path = Path(path) if path else Path(os.environ.get('XDG_CONFIG_HOME', Path.home()/'.config'))/'earfun-studio'/'settings.json'
        self.data = {'auto_connect': False, 'last_mac': '', 'eq': [0]*10, 'presets': {}}
        try:
            raw = json.loads(self.path.read_text())
            if isinstance(raw, dict):
                self.data.update(raw)
        except (OSError, ValueError):
            pass
        valid = lambda v: isinstance(v, list) and len(v) == 10 and all(type(x) is int and -10 <= x <= 10 for x in v)
        if not valid(self.data['eq']):
            self.data['eq'] = [0]*10
        presets = self.data['presets']
        self.data['presets'] = {k: v for k, v in presets.items() if isinstance(k, str) and valid(v)} if isinstance(presets, dict) else {}
        self.data['auto_connect'] = self.data['auto_connect'] is True
        if not isinstance(self.data['last_mac'], str):
            self.data['last_mac'] = ''

    def save(self):
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temporary = self.path.with_suffix('.tmp')
        temporary.write_text(json.dumps(self.data, ensure_ascii=False, indent=2))
        temporary.replace(self.path)
