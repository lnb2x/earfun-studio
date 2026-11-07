import asyncio
import json
import struct
import tempfile
import unittest
from collections import deque
from pathlib import Path
from unittest.mock import patch
from earfun import protocol as p
from earfun.audio import find_sink
from earfun.settings import Settings
from earfun.backend import Backend, CommandRejected


class ProtocolTests(unittest.TestCase):
    def test_reference_packets(self):
        # Exact bytes from the supplied C# protocol, not a round-trip assertion.
        self.assertEqual(p.request(20, 1).hex(), '000a031401')
        self.assertEqual(p.gesture_packet('L', 'Nhấn giữ', 'Phát / tạm dừng').hex(), '000a030a11010000')
        self.assertEqual(p.gesture_packet('R', 'Chạm hai lần', 'Bài tiếp').hex(), '000a030a00000205')
        eq = p.equalizer([0]*10)
        self.assertEqual(eq[0].hex(), '001d0e01a1fe98005e00000b33')
        self.assertEqual(eq[-1].hex(), '001d0e01aafe98bb8000000b33')
        self.assertEqual(len(eq), 10)
        self.assertEqual(p.equalizer([-10]*10)[0][9:11], b'\xfe\x98')
        self.assertEqual(p.equalizer([10]*10)[0][9:11], b'\x01\x68')

    def test_invalid_packets_and_gains(self):
        for data in (b'', b'\x00\x0a\x83', b'\x00\x0a\x03\x06\x00'):
            self.assertIsNone(p.decode(data))
        for levels in ([0]*9, [11]*10, [1.5]*10):
            with self.assertRaises(ValueError):
                p.equalizer(levels)
        with self.assertRaises(ValueError):
            p.gesture_packet('X', 'Chạm một lần', 'Phát / tạm dừng')

    def test_decode_state_unknown_and_rejected(self):
        battery = p.decode(bytes.fromhex('000a83060057'))
        self.assertEqual(p.state(battery), {'bat_l': 87})
        for raw in ('000a830600ff', '000a83060157', '001d83060057', '000a831500ff'):
            self.assertEqual(p.state(p.decode(bytes.fromhex(raw))), {})
        self.assertEqual(p.state(p.decode(bytes.fromhex('000a834a0000'))), {'ear': True})
        self.assertEqual(p.state(p.decode(bytes.fromhex('000a832f001b'))),
                         {'aptx': True, 'adaptive': True, 'ldac': True, 'lossless': True})

    def test_streamed_phones_and_short_keys(self):
        payload = bytes([0]) + b'Ubuntu'.ljust(19, b'\0') + b'\0\0' + bytes.fromhex('aabbcc') + bytes([1])
        reply = p.decode(bytes.fromhex('000a032c00') + payload)
        phone = p.state(reply)['phone']
        self.assertEqual(phone['name'], 'Ubuntu')
        self.assertEqual(phone['mac'], 'AABBCC')
        self.assertTrue(phone['connected'])
        self.assertEqual(p.state(p.Reply(10, 0x30b, 0, b'\0')), {})
        self.assertEqual(p.state(p.Reply(10, 0x30b, 0, bytes([1, 1, 2, 5]))),
                         {'keys': {'L.Chạm một lần': 'Phát / tạm dừng', 'R.Chạm một lần': 'Bài tiếp'}})


class StorageAndAudioTests(unittest.TestCase):
    def test_settings_validation_and_atomic_save(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'settings.json'
            path.write_text('{bad')
            settings = Settings(path)
            self.assertEqual(settings.data['eq'], [0]*10)
            settings.data['presets'] = {'Riêng': [3]*10}
            settings.save()
            self.assertEqual(Settings(path).data['presets']['Riêng'], [3]*10)
            path.write_text(json.dumps({'eq': [90]*10, 'presets': {'bad': [1]}, 'auto_connect': 'false'}))
            settings = Settings(path)
            self.assertEqual(settings.data['eq'], [0]*10)
            self.assertEqual(settings.data['presets'], {})
            self.assertFalse(settings.data['auto_connect'])

    def test_never_select_speakers_or_another_bluetooth_sink(self):
        def node(i, name, media='Audio/Sink'):
            return {'id': i, 'info': {'props': {'node.name': name, 'media.class': media, 'object.serial': i+100}}}
        objects = [node(1, 'alsa_output.AA_BB_CC_DD_EE_FF'),
                   node(2, 'bluez_output.11_22_33_44_55_66.1'),
                   node(3, 'bluez_output.AA_BB_CC_DD_EE_FF.1', 'Audio/Source')]
        self.assertIsNone(find_sink(objects, 'AA:BB:CC:DD:EE:FF'))
        objects.append(node(4, 'bluez_output.AA_BB_CC_DD_EE_FF.1'))
        self.assertEqual(find_sink(objects, 'AA:BB:CC:DD:EE:FF'), '4')
        self.assertIsNone(find_sink(objects, ''))


class FakeClient:
    is_connected = True

    def __init__(self, backend):
        self.backend = backend
        self.writes = []
        self.noise = 0
        self.reject = False
        self.wrong_vendor_first = False

    async def write_gatt_char(self, uuid, packet, response):
        self.writes.append(packet)
        vendor, command = struct.unpack('>HH', packet[:4])
        if command == 0x314:
            self.noise = packet[4]
        payload = bytes([self.noise]) if command == 0x315 else b''
        raw = struct.pack('>HHB', vendor, command | 0x8000, int(self.reject)) + payload
        if self.wrong_vendor_first:
            self.backend.notify(None, b'\x00\x1d' + raw[2:])
        self.backend.notify(None, raw)

    async def disconnect(self):
        self.is_connected = False


class BackendTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        # No BlueZ or physical device access in unit tests.
        b = self.backend = Backend.__new__(Backend)
        b.loop = asyncio.get_running_loop()
        b.emit = lambda *_: None
        b.log = deque(maxlen=20)
        b.state = {'version': 'TW600-test'}
        b.phones = {}
        b.pending = None
        b.poll_task = b.finder_task = None
        b.finding = False
        b.lock = asyncio.Lock()
        b.wire_lock = asyncio.Lock()
        b.client = FakeClient(b)

    async def test_setter_reads_actual_device_state(self):
        b = self.backend
        await b.set_value(20, 21, 1, 'noise', 'anc')
        self.assertEqual(b.state['noise'], 'anc')
        self.assertEqual(b.client.writes, [bytes.fromhex('000a031401'), bytes.fromhex('000a0315')])

    async def test_request_matches_vendor_and_reports_rejection(self):
        b = self.backend
        b.client.wrong_vendor_first = True
        response = await b.request(p.request(21))
        self.assertEqual(response.vendor, 10)
        b.client.reject = True
        with self.assertRaises(CommandRejected):
            await b.request(p.request(21))
        self.assertIsNone(b.pending)

    async def test_disconnect_stops_finder_first(self):
        b = self.backend
        client = b.client
        b.finding = True
        await b.disconnect()
        self.assertEqual(client.writes[-1], bytes.fromhex('000a033200'))
        self.assertFalse(client.is_connected)
        self.assertEqual(b.state, {})

    async def test_no_eq_write_when_disconnected(self):
        b = self.backend
        b.client.is_connected = False
        with self.assertRaises(ConnectionError):
            await b.set_eq([0]*10)
        self.assertEqual(b.client.writes, [])

    async def test_timeout_closes_connection_and_clears_pending(self):
        b = self.backend
        client = b.client
        async def silent(*args, **kwargs):
            pass
        client.write_gatt_char = silent
        wait_for = asyncio.wait_for
        async def fast_timeout(aw, timeout):
            return await wait_for(aw, min(timeout, .02))
        with patch('earfun.backend.asyncio.wait_for', fast_timeout):
            with self.assertRaises(RuntimeError):
                await b.request(p.request(21))
        self.assertFalse(client.is_connected)
        self.assertIsNone(b.pending)
        self.assertEqual(b.state, {})

    async def test_ldac_conflict_has_no_writes(self):
        b = self.backend
        b.state.update({'dual': True, 'ldac': False, 'aptx': True, 'adaptive': True, 'lossless': True})
        with self.assertRaises(RuntimeError):
            await b.codecs({'ldac': True, 'aptx': True, 'adaptive': True, 'lossless': True})
        self.assertEqual(b.client.writes, [])


if __name__ == '__main__':
    unittest.main()
