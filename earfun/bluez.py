"""Linux dual-mode transport adapter for the pinned Bleak 3.0.2 backend.

BlueZ Device1.Connect may connect only BR/EDR, while cached GATT objects still
exist. Recover on a failed firmware read by requesting the remaining bearer.
Releasing this application's control session must not call Device1.Disconnect,
which also tears down the user's A2DP audio connection.
"""
import asyncio
from bleak import BleakClient
from bleak.backends.bluezdbus.client import BleakClientBlueZDBus
from bleak.backends.bluezdbus.utils import assert_reply
from dbus_fast import Message


class EarFunBluezBackend(BleakClientBlueZDBus):
    async def connect_control_bearer(self):
        if self._bus is None or self._device_path is None:
            raise ConnectionError('Phiên Bluetooth đã đóng.')
        reply = await asyncio.wait_for(self._bus.call(Message(
            destination='org.bluez', path=self._device_path,
            interface='org.bluez.Device1', member='Connect')), 25)
        assert_reply(reply)
        # ServicesResolved can be temporarily true for the Classic bearer.
        await asyncio.sleep(.5)
        self.services = None
        await asyncio.wait_for(self._get_services(), 12)

    async def disconnect(self):
        bus = self._bus
        if bus is None:
            return
        try:
            for path in list(self._notification_callbacks):
                try:
                    await asyncio.wait_for(bus.call(Message(
                        destination='org.bluez', path=path,
                        interface='org.bluez.GattCharacteristic1', member='StopNotify')), 2)
                except Exception:
                    # A physically lost link has nothing left to unsubscribe.
                    pass
        finally:
            # Complete (rather than cancel) Bleak's monitor: its cancellation
            # handler would send the same unwanted global Device1.Disconnect.
            if self._disconnect_monitor_event:
                self._disconnect_monitor_event.set()
                self._disconnect_monitor_event = None
            self._is_connected = False
            self._notification_callbacks.clear()
            self._cleanup_all()
            bus.disconnect()
            await bus.wait_for_disconnect()
            self._bus = None


class ControlClient(BleakClient):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, backend=EarFunBluezBackend, **kwargs)

    async def connect_control_bearer(self):
        # Keep use of Bleak backend internals isolated to this versioned adapter.
        await self._backend.connect_control_bearer()
