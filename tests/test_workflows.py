import asyncio
import unittest
from unittest.mock import patch
import test_core
from earfun import protocol as p, workflows as w


class PlannerTests(unittest.TestCase):
    def setUp(self):
        self.state = {'dual': True, 'ldac': False, 'aptx': True, 'adaptive': True, 'lossless': True}

    def test_ldac_has_explicit_multi_device_dependency(self):
        wanted = {key: self.state[key] for key in w.CODECS}
        wanted['ldac'] = True
        plan = w.codecs(self.state, wanted)
        self.assertEqual([step.setter for step in plan.steps], [39, 46])
        self.assertIn('Tắt kết nối hai thiết bị', plan.confirmation)
        self.assertEqual(dict(plan.target), {**wanted, 'dual': False})

    def test_multi_device_plan_preserves_every_other_codec(self):
        self.state.update(dual=False, ldac=True)
        plan = w.dual(self.state, True)
        self.assertEqual([(s.setter, s.value) for s in plan.steps], [(46, 19), (39, 1)])
        self.assertEqual(dict(plan.target)['aptx'], True)
        self.assertEqual(dict(plan.target)['ldac'], False)

    def test_unchanged_and_unknown_settings_do_not_create_writes(self):
        self.assertEqual(w.dual(self.state, True).steps, ())
        with self.assertRaises(ValueError):
            w.dual({}, True)
        with self.assertRaises(ValueError):
            w.codecs(self.state, {'ldac': True})

    def test_one_bluetooth_mode_represents_both_inverse_settings(self):
        plan = w.restart_setting({'le_audio': True}, 'le_audio', False)
        self.assertEqual((plan.steps[0].setter, plan.steps[0].query, plan.steps[0].value), (75, 76, 0))


class Device:
    def __init__(self):
        self.state = {'version': 'TW600-test', 'dual': True, 'ldac': False,
                      'aptx': True, 'adaptive': True, 'lossless': True}
        self.writes = []
        self.fail_codec = False


class Session:
    def __init__(self, backend, device):
        self.backend, self.device = backend, device
        self.is_connected = True

    async def write_gatt_char(self, uuid, packet, response):
        cmd = int.from_bytes(packet[2:4], 'big') & 255
        device = self.device
        if cmd in (39, 46):
            device.writes.append((cmd, packet[4]))
            if cmd == 39:
                device.state['dual'] = bool(packet[4])
                # Simulate a reboot exactly after a successful setter write.
                self.is_connected = False
                self.backend.lost(self)
            elif not device.fail_codec:
                bits = packet[4]
                device.state.update({key: bool(bits & bit) for key, bit in
                                    [('aptx', 1), ('adaptive', 2), ('ldac', 8), ('lossless', 16)]})
            return
        if cmd == 38:
            value = int(device.state['dual'])
        elif cmd == 47:
            value = sum(bit for key, bit in [('aptx', 1), ('adaptive', 2), ('ldac', 8), ('lossless', 16)] if device.state[key])
        else:
            raise AssertionError(f'Unexpected command {cmd}')
        self.backend.notify(None, p.packet(10, 0x8300 | cmd, bytes([0, value])))

    async def disconnect(self):
        self.is_connected = False


class WorkflowTests(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        fixture = test_core.BackendTests()
        await fixture.asyncSetUp()
        self.b = fixture.backend
        self.b.devices = {}
        self.b.address = 'AA:BB:CC:DD:EE:FF'
        self.b.closing = False
        self.device = Device()
        self.b.state = dict(self.device.state)
        self.b.client = Session(self.b, self.device)
        self.reconnections = 0
        async def reconnect(address, full=True):
            self.reconnections += 1
            self.b.client = Session(self.b, self.device)
            self.b.state = dict(self.device.state)
        async def refresh(full):
            self.b.state = dict(self.device.state)
        self.b._connect = reconnect
        self.b._query = refresh
        original_sleep = asyncio.sleep
        async def instant(_):
            await original_sleep(0)
        self.sleep_patch = patch('earfun.backend.asyncio.sleep', instant)
        self.sleep_patch.start()

    async def asyncTearDown(self):
        self.sleep_patch.stop()
        await self.b.disconnect()

    def ldac_plan(self):
        wanted = {key: self.b.state[key] for key in w.CODECS}
        wanted['ldac'] = True
        return w.codecs(self.b.state, wanted)

    async def test_reboot_reconnects_then_continues_without_resending_setter(self):
        plan = self.ldac_plan()
        await self.b.run_plan(plan)
        self.assertEqual(self.device.writes, [(39, 0), (46, 27)])
        self.assertEqual(self.reconnections, 1)
        self.assertTrue(self.b.state['ldac'])
        self.assertFalse(self.b.state['dual'])

    async def test_external_change_during_confirmation_causes_zero_writes(self):
        plan = self.ldac_plan()
        self.device.state['aptx'] = False
        with self.assertRaisesRegex(RuntimeError, 'đã đổi'):
            await self.b.run_plan(plan)
        self.assertEqual(self.device.writes, [])

    async def test_partial_failure_reports_completed_step_and_never_resends(self):
        plan = self.ldac_plan()
        self.device.fail_codec = True
        with self.assertRaisesRegex(RuntimeError, 'Đã hoàn tất: Tắt kết nối hai thiết bị'):
            await self.b.run_plan(plan)
        self.assertEqual(self.device.writes, [(39, 0), (46, 27)])
        self.assertFalse(self.device.state['ldac'])
        self.assertFalse(self.device.state['dual'])

    async def test_closing_stops_retry_without_another_write(self):
        plan = self.ldac_plan()
        self.b.closing = True
        # No setter may be sent once closing has already been requested.
        with self.assertRaises(RuntimeError):
            await self.b.run_plan(plan)
        self.assertEqual(self.device.writes, [])
