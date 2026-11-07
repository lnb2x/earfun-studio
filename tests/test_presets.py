import copy
import unittest
from earfun import presets, protocol


class PresetTests(unittest.TestCase):
    def test_apk_catalog_identity_and_exact_reference_curves(self):
        self.assertEqual(len(presets.BUILTINS), 32)
        self.assertEqual(len({row['key'] for row in presets.BUILTINS}), 32)
        curves = {row['key']: row['levels'] for row in presets.BUILTINS}
        self.assertEqual(curves['earfun:24'], [4, 9, 0, 0, 0, 0, 0, 0, 0, 0])
        self.assertEqual(curves['earfun:29'], [0, 0, 0, 1, 4, 5, 4, 2, 0, 0])
        self.assertEqual(curves['fernanda:201'], [-2, 2, 0, 1, 2, 3, -1, -3, -4, 0])
        self.assertEqual(curves['fernanda:202'], [-7, -3, -1, -2, 0, 2, 2, -1, -2, 1])
        self.assertEqual(presets.CATALOG['source']['version'], '20.1.61')
        # An APK level of 9 means +5 dB, not +9 dB or a clamped +6 dB.
        bass = protocol.equalizer(curves['earfun:24'])
        self.assertEqual(bass[1][9:11], bytes.fromhex('012c'))
        for levels in curves.values():
            self.assertEqual(len(protocol.equalizer(levels)), 10)

    def test_personal_names_cannot_shadow_official_values(self):
        options = presets.options({'Rock': [9]*10})
        official = next(row for row in options if row[0] == 'earfun:5')
        personal = next(row for row in options if row[0] == 'custom:Rock')
        self.assertNotEqual(official[1], personal[1])
        self.assertEqual(official[2], [4, 5, 2, 0, 0, 1, 2, 3, 3, 2])
        self.assertEqual(personal[2], [9]*10)

    def test_export_import_is_lossless_and_rejects_wrong_model(self):
        levels = list(range(-5, 5))
        document = presets.encode('Tên tiếng Việt', levels)
        self.assertEqual(presets.decode(document), ('Tên tiếng Việt', levels))
        for key, bad in [('model', 'OTHER'), ('frequencies', [100]*10),
                         ('levels', [True]*10), ('levels', [11]*10),
                         ('levels', [0]*9), ('version', 2), ('name', '')]:
            invalid = copy.deepcopy(document)
            invalid[key] = bad
            with self.assertRaises(ValueError):
                presets.decode(invalid)

    def test_fast_pair_is_inverse_of_le_audio_and_unknown_language_stays_unknown(self):
        self.assertEqual(protocol.state(protocol.Reply(10, 0x34c, 0, b'\0')),
                         {'le_audio': False, 'google_fast_pair': True})
        self.assertEqual(protocol.state(protocol.Reply(10, 0x34c, 0, b'\1')),
                         {'le_audio': True, 'google_fast_pair': False})
        self.assertEqual(protocol.state(protocol.Reply(10, 0x371, 0, b'\0')), {'voice_language': 'EN'})
        self.assertEqual(protocol.state(protocol.Reply(10, 0x371, 0, b'\xff')), {})
        self.assertNotIn(113, protocol.QUERIES, 'Unsupported optional voice query must not break startup')
