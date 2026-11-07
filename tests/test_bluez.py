import asyncio
import unittest
from unittest.mock import patch
from dbus_fast import Message, MessageType
from bleak.exc import BleakDBusError
from earfun.backend import Backend
from earfun.bluez import EarFunBluezBackend
from earfun import protocol as p
import test_core


class OpenClient(test_core.FakeClient):
    def __init__(self, backend, missing_service=False, reject_firmware=False):
        super().__init__(backend)
        self.services = self
        self.recovered = False
        self.recoveries = 0
        self.missing = missing_service
        self.reject_firmware = reject_firmware
        self.notifications = 0

    async def connect(self):
        self.is_connected = True

    def get_service(self, uuid):
        return not self.missing or self.recovered

    async def start_notify(self, *args):
        self.notifications += 1

    async def stop_notify(self, *args):
        pass

    async def connect_control_bearer(self):
        self.recovered = True
        self.recoveries += 1

    async def write_gatt_char(self, uuid, packet, response):
        self.writes.append(packet)
        if not self.recovered:
            raise BleakDBusError('org.bluez.Error.Failed', ['Not connected'])
        if packet == p.request(9):
            version = b'OTHER-model' if self.reject_firmware else b'TW600-hardware-test'
            self.backend.notify(None, bytes.fromhex('000a830900') + version)


class ConnectRecoveryTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        fixture = test_core.BackendTests()
        await fixture.asyncSetUp()
        self.backend = fixture.backend
        self.backend.client = None
        self.backend.state.clear()
        self.backend.devices = {}
        self.queries = []
        async def read(full):
            self.queries.append(full)
        self.backend._query = read

    async def asyncTearDown(self):
        await self.backend.disconnect()

    async def test_cached_gatt_but_only_classic_connected_recovers(self):
        b = self.backend
        client = OpenClient(b)
        with patch('earfun.backend.ControlClient', return_value=client):
            await b.connect('AA:BB:CC:DD:EE:FF')
        self.assertEqual(client.recoveries, 1)
        self.assertEqual(client.writes, [p.request(9), p.request(9)])
        self.assertTrue(b.connected)
        self.assertEqual(self.queries, [True])

    async def test_missing_gatt_service_is_refreshed_before_retry(self):
        b = self.backend
        client = OpenClient(b, missing_service=True)
        with patch('earfun.backend.ControlClient', return_value=client):
            await b.connect('AA:BB:CC:DD:EE:FF')
        self.assertTrue(b.connected)
        self.assertEqual(client.recoveries, 1)
        self.assertEqual(client.writes, [p.request(9)])

    async def test_wrong_model_never_becomes_connected(self):
        b = self.backend
        client = OpenClient(b, reject_firmware=True)
        client.recovered = True
        with patch('earfun.backend.ControlClient', return_value=client):
            with self.assertRaises(RuntimeError):
                await b.connect('AA:BB:CC:DD:EE:FF')
        self.assertFalse(b.connected)
        self.assertEqual(self.queries, [])


class ReleaseTests(unittest.IsolatedAsyncioTestCase):
    async def test_releasing_control_does_not_disconnect_a2dp(self):
        calls = []
        class Bus:
            async def call(self, message):
                calls.append(message.member)
                return Message(message_type=MessageType.METHOD_RETURN, reply_serial=1)
            def disconnect(self):
                calls.append('close-local-bus')
            async def wait_for_disconnect(self):
                pass
        client = EarFunBluezBackend('AA:BB:CC:DD:EE:FF', bluez={}, timeout=25)
        client._bus = Bus()
        client._is_connected = True
        client._notification_callbacks['/device/rx'] = lambda _: None
        monitor = client._disconnect_monitor_event = asyncio.Event()
        await client.disconnect()
        self.assertEqual(calls, ['StopNotify', 'close-local-bus'])
        self.assertNotIn('Disconnect', calls)
        self.assertTrue(monitor.is_set())
        self.assertFalse(client.is_connected)
        self.assertIsNone(client._bus)
