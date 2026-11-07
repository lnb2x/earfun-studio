"""One asyncio loop owns BlueZ and serializes all device operations."""
import asyncio
import threading
import time
from collections import deque
from bleak import BleakScanner
from bleak.exc import BleakDBusError
from .bluez import ControlClient
from dbus_fast.aio import MessageBus
from dbus_fast import BusType, Message, MessageType
from bleak.backends.device import BLEDevice
from . import protocol as p


class CommandRejected(RuntimeError):
    pass


class Backend:
    def __init__(self, emit):
        self.emit = emit
        self.loop = asyncio.new_event_loop()
        self.thread = threading.Thread(target=self.loop.run_forever, daemon=True)
        self.thread.start()
        self.client = None
        self.devices = {}
        self.state = {}
        self.phones = {}
        self.log = deque(maxlen=2000)
        self.pending = None
        self.lock = asyncio.Lock()
        self.wire_lock = asyncio.Lock()
        self.poll_task = None
        self.finder_task = None
        self.finding = False
        self.closing = False
        self.address = None

    def submit(self, coro):
        future = asyncio.run_coroutine_threadsafe(coro, self.loop)
        def done(f):
            try:
                f.result()
            except Exception as ex:
                self.record('!', str(ex) or type(ex).__name__)
                self.emit('error', str(ex) or type(ex).__name__)
        future.add_done_callback(done)
        return future

    def record(self, direction, text):
        line = time.strftime('%H:%M:%S') + ' ' + direction + ' ' + text
        self.log.append(line)
        self.emit('log', line)

    @property
    def connected(self):
        return self.client is not None and self.client.is_connected and 'version' in self.state

    async def known_devices(self):
        """Cached paired devices are available immediately, without scanning."""
        found = {}
        bus = await MessageBus(bus_type=BusType.SYSTEM).connect()
        try:
            reply = await bus.call(Message(destination='org.bluez', path='/',
                interface='org.freedesktop.DBus.ObjectManager', member='GetManagedObjects'))
            if reply.message_type == MessageType.ERROR:
                raise RuntimeError('Không truy cập được BlueZ: ' + str(reply.body))
            for path, interfaces in reply.body[0].items():
                props = interfaces.get('org.bluez.Device1')
                if not props:
                    continue
                props = {k: v.value for k, v in props.items()}
                name = props.get('Name', props.get('Alias', ''))
                if self.supported(name, props.get('UUIDs', [])):
                    address = props['Address']
                    found[address] = BLEDevice(address, name, {'path': path, 'props': props})
        finally:
            bus.disconnect()
        return found

    async def load_devices(self):
        async with self.lock:
            self.devices = await self.known_devices()
            self.emit('devices', [(d.address, d.name or d.address) for d in self.devices.values()])
            if self.devices:
                self.emit('notice', 'Đã tìm thấy tai nghe đã ghép. Bấm Kết nối để mở bảng điều khiển.')

    async def scan(self):
        async with self.lock:
            self.emit('notice', 'Đang quét Bluetooth trong 7 giây…')
            found = await self.known_devices()
            async with BleakScanner() as scanner:
                await asyncio.sleep(7)
                for device, adv in scanner.discovered_devices_and_advertisement_data.values():
                    name = adv.local_name or device.name or ''
                    if self.supported(name, adv.service_uuids):
                        found[device.address] = device
            self.devices = found
            self.emit('devices', [(d.address, d.name or d.address) for d in found.values()])
            self.emit('notice', f'Tìm thấy {len(found)} tai nghe. ' + ('Chọn thiết bị rồi kết nối.' if found else 'Lấy tai nghe ra khỏi hộp và ghép trong Cài đặt Bluetooth.'))

    @staticmethod
    def supported(name, uuids):
        return name.lower() == 'earfun air pro 4' or name.startswith('TW600-') or p.SERVICE in [u.lower() for u in uuids]

    async def connect(self, address):
        async with self.lock:
            await self._connect(address)

    async def _connect(self, address, full=True):
        if self.client:
            await self._disconnect()
        self.emit('connection', 'connecting')
        candidate = ControlClient(self.devices.get(address, address), disconnected_callback=self.lost, timeout=25)
        self.client = candidate
        self.state.clear()
        try:
            await asyncio.wait_for(candidate.connect(), 35)
            for attempt in range(2):
                try:
                    if not candidate.services.get_service(p.SERVICE):
                        raise ConnectionError('BlueZ chưa mở dịch vụ điều khiển TW600.')
                    await candidate.start_notify(p.RX, self.notify)
                    version = await self.request(p.request(9))
                    break
                except (BleakDBusError, ConnectionError) as error:
                    recoverable = isinstance(error, ConnectionError) or (
                        'not connected' in str(error).lower()
                        or error.dbus_error == 'org.bluez.Error.NotConnected')
                    if attempt or not recoverable or not candidate.is_connected:
                        raise
                    self.record('!', 'Mở lại kênh BLE sau lỗi: ' + str(error))
                    self.emit('notice', 'Đang mở kênh Bluetooth LE điều khiển tai nghe…')
                    await candidate.connect_control_bearer()
                    try:
                        await candidate.stop_notify(p.RX)
                    except BleakDBusError:
                        pass
            firmware = p.state(version).get('version', '')
            if not firmware.startswith('TW600-'):
                raise RuntimeError('Firmware không xác nhận TW600: ' + firmware)
            self.apply(version)
            self.address = address
            self.emit('connected_address', address)
            if full:
                await self._query(True)
            self.emit('connection', 'connected')
            self.poll_task = asyncio.create_task(self.poll())
        except BaseException:
            await self._disconnect()
            raise


    def lost(self, client):
        if client is not self.client:
            return
        self.state.clear()
        self.phones.clear()
        if self.pending and not self.pending[2].done():
            self.pending[2].set_exception(ConnectionError('Bluetooth đã ngắt kết nối'))
        self.emit('state', {})
        self.emit('phones', [])
        self.emit('connection', 'disconnected')

    def notify(self, _characteristic, data):
        self.record('<<', bytes(data).hex(' '))
        reply = p.decode(data)
        if reply is None:
            return
        if self.pending:
            vendor, command, future = self.pending
            if (reply.vendor, reply.command) == (vendor, command) and not future.done():
                future.set_result(reply)
        # Firmware is validated by connect before any settings become available.
        if 'version' in self.state:
            self.apply(reply)

    def apply(self, reply):
        update = p.state(reply)
        phone = update.pop('phone', None)
        if phone:
            if phone['num'] == 0:
                self.phones.clear()
            self.phones[phone['num']] = phone
            self.emit('phones', list(self.phones.values()))
        self.state.update(update)
        self.emit('state', dict(self.state))

    async def write(self, packet):
        if not self.client or not self.client.is_connected:
            raise ConnectionError('Kết nối tai nghe trước khi điều chỉnh.')
        self.record('>>', packet.hex(' '))
        await asyncio.wait_for(self.client.write_gatt_char(p.TX, packet, response=True), 5)

    async def request(self, packet):
        async with self.wire_lock:
            vendor, command = int.from_bytes(packet[:2], 'big'), int.from_bytes(packet[2:4], 'big')
            future = self.loop.create_future()
            self.pending = vendor, command, future
            try:
                await self.write(packet)
                reply = await asyncio.wait_for(future, 4)
                if reply.status:
                    raise CommandRejected(f'Tai nghe từ chối lệnh {command:04X} (mã {reply.status}).')
                return reply
            except (TimeoutError, ConnectionError):
                # A late reply must never acknowledge a later command.
                await self._disconnect()
                raise RuntimeError('Bluetooth không phản hồi; đã đóng kênh điều khiển. Hãy kết nối lại.')
            finally:
                self.pending = None

    async def send(self, packet):
        async with self.wire_lock:
            try:
                await self.write(packet)
                await asyncio.sleep(.08)
            except Exception:
                await self._disconnect()
                raise

    async def _query(self, full):
        for command in p.QUERIES if full else (6, 7, 23, 21):
            try:
                self.apply(await self.request(p.request(command)))
            except CommandRejected as ex:
                self.record('?', str(ex))
        if full:
            await self._phones()
        self.emit('notice', 'Đã đọc trạng thái trực tiếp từ tai nghe.')

    async def query(self):
        async with self.lock:
            if self.connected:
                await self._query(True)

    async def poll(self):
        try:
            while self.connected:
                await asyncio.sleep(10)
                if not self.lock.locked():
                    async with self.lock:
                        if self.connected:
                            await self._query(False)
        except asyncio.CancelledError:
            pass
        except Exception as ex:
            self.record('!', str(ex))
            self.emit('error', str(ex))
            await self._disconnect()

    async def set_value(self, setter, query, value, key, wanted):
        async with self.lock:
            await self._set(setter, query, value, key, wanted)

    async def _set(self, setter, query, value, key, wanted):
        if not self.connected:
            raise ConnectionError('Kết nối tai nghe trước khi điều chỉnh.')
        await self.send(p.request(setter, value))
        for _ in range(8):
            await asyncio.sleep(.4)
            reply = await self.request(p.request(query))
            self.apply(reply)
            if p.state(reply).get(key) == wanted:
                self.emit('notice', 'Tai nghe đã xác nhận thay đổi.')
                return
        raise RuntimeError('Tai nghe chưa xác nhận giá trị mới. Giao diện hiển thị trạng thái đọc lại.')

    async def direct(self, command, *payload):
        async with self.lock:
            if not self.connected:
                raise ConnectionError('Kết nối tai nghe trước khi điều chỉnh.')
            await self.send(p.request(command, *payload))
            self.emit('notice', 'Đã gửi lệnh. Thiết bị có thể ngắt kết nối; hãy đọc lại để kiểm tra.')

    async def set_key(self, side, gesture, action):
        async with self.lock:
            if not self.connected:
                raise ConnectionError('Chưa kết nối tai nghe.')
            await self.send(p.gesture_packet(side, gesture, action))
            await asyncio.sleep(.3)
            reply = await self.request(p.request(11))
            self.apply(reply)
            if p.state(reply).get('keys', {}).get(side+'.'+gesture) != action:
                raise RuntimeError('Tai nghe chưa xác nhận thao tác cảm ứng mới.')
            self.emit('notice', 'Đã xác nhận thao tác cảm ứng.')

    async def set_eq(self, levels):
        packets = p.equalizer(levels)
        async with self.lock:
            if not self.connected:
                raise ConnectionError('Chưa kết nối tai nghe.')
            for packet in packets:
                await self.send(packet)
            self.emit('notice', 'Đã ghi đủ 10 gói EQ. Chưa có xác nhận đọc lại đường cong EQ.')

    async def codecs(self, values):
        async with self.lock:
            if not all(k in self.state for k in ('ldac', 'aptx', 'adaptive', 'lossless', 'dual')):
                raise RuntimeError('Chưa đọc đủ trạng thái codec / đa điểm.')
            if values['ldac'] and self.state['dual']:
                raise RuntimeError('Tắt đa điểm trước khi bật LDAC.')
            bits = sum(bit for key, bit in [('aptx', 1), ('adaptive', 2), ('ldac', 8), ('lossless', 16)] if values[key])
            await self._set(46, 47, bits, 'ldac', values['ldac'])
            if any(self.state.get(k) != v for k, v in values.items()):
                raise RuntimeError('Tai nghe chưa xác nhận đủ các codec.')

    async def run_plan(self, plan):
        """Execute only the changes shown in the confirmation, once each."""
        async with self.lock:
            if self.closing:
                raise RuntimeError('Ứng dụng đang đóng; chưa gửi thay đổi.')
            if not self.connected:
                raise ConnectionError('Kết nối tai nghe trước khi thay đổi thiết lập.')
            if not plan.steps:
                self.emit('notice', 'Tai nghe đã ở thiết lập này; không cần gửi lại.')
                return
            for command in plan.queries:
                self.apply(await self.request(p.request(command)))
            if any(self.state.get(key) != value for key, value in plan.baseline):
                raise RuntimeError('Thiết lập tai nghe đã đổi trong lúc chờ xác nhận. Hãy chọn lại theo trạng thái mới.')
            completed = []
            try:
                for index, step in enumerate(plan.steps, 1):
                    if self.closing:
                        raise RuntimeError('Đã dừng quy trình vì ứng dụng đang đóng.')
                    self.emit('notice', f'{index}/{len(plan.steps)} · {step.title}…')
                    await self._run_restart_step(step)
                    completed.append(step.title)
                await self._query(True)
                final = dict(plan.target)
                if any(self.state.get(key) != value for key, value in final.items()):
                    raise RuntimeError('Trạng thái cuối khác lựa chọn đã xác nhận.')
                self.emit('notice', 'Đã hoàn tất và đọc lại xác nhận: ' + plan.title + '.')
            except Exception as error:
                detail = ('Đã hoàn tất: ' + '; '.join(completed) + '. ') if completed else ''
                raise RuntimeError(detail + 'Chưa xác nhận hoàn tất toàn bộ thay đổi. ' + str(error)) from error

    async def _run_restart_step(self, step):
        address = self.address
        # A write may reach the device even if its acknowledgement is lost.
        # Never resend setters during reconnection or readback retries.
        try:
            await self.send(p.request(step.setter, step.value))
        except Exception as error:
            self.record('?', 'Kiểm tra sau khi gửi lệnh bị gián đoạn: ' + str(error))
        deadline = self.loop.time() + 50
        for attempt in range(25):
            if self.loop.time() >= deadline:
                break
            if self.closing:
                raise RuntimeError('Đã dừng quy trình vì ứng dụng đang đóng.')
            await asyncio.sleep(1)
            try:
                if not self.connected:
                    if not address:
                        raise ConnectionError('Không còn địa chỉ tai nghe để kết nối lại.')
                    self.emit('notice', f'Tai nghe đang khởi động lại. Đang kết nối lại (lần {attempt+1})…')
                    await asyncio.wait_for(self._connect(address, full=False), min(12, max(1, deadline-self.loop.time())))
                reply = await self.request(p.request(step.query))
                self.apply(reply)
                if all(self.state.get(key) == value for key, value in step.wanted):
                    return
            except CommandRejected:
                raise
            except Exception as error:
                self.record('?', 'Đọc lại sau thay đổi: ' + str(error))
        raise RuntimeError('Tai nghe chưa xác nhận bước “' + step.title + '”. Trạng thái đang hiển thị là phản hồi đọc được gần nhất.')

    async def disable_touch(self, left, right):
        async with self.lock:
            if not all(key in self.state for key in ('dis_l', 'dis_r')):
                raise RuntimeError('Chưa đọc đủ trạng thái khóa cảm ứng hai tai.')
            value = 1 if left and right else 2 if left else 3 if right else 0
            await self._set(52, 53, value, 'dis_l', left)
            if self.state.get('dis_r') != right:
                raise RuntimeError('Chưa xác nhận khóa cảm ứng tai phải.')

    async def _phones(self):
        self.phones.clear()
        self.emit('phones', [])
        try:
            reply = await self.request(p.request(44))
            # Streamed entries are applied by notify; don't re-apply index 0 here.
            if 'version' not in self.state:
                self.apply(reply)
        except CommandRejected as ex:
            self.record('?', str(ex))

    async def phones_action(self, command=44, num=None):
        async with self.lock:
            if not self.connected:
                raise ConnectionError('Chưa kết nối tai nghe.')
            if command in (41, 43) and sum(x['connected'] for x in self.phones.values()) >= 2:
                raise RuntimeError('Đã kết nối hai thiết bị. Ngắt một thiết bị trước.')
            if command != 44:
                await self.send(p.request(command, *([] if num is None else [num])))
                await asyncio.sleep(1)
            await self._phones()
            self.emit('notice', 'Đã đọc danh sách thiết bị.' if command != 43 else 'Đã mở ghép nối. Dùng Cài đặt Bluetooth trên thiết bị mới, rồi bấm Làm mới.')

    async def find(self, mask):
        async with self.lock:
            if self.finder_task:
                self.finder_task.cancel()
                self.finder_task = None
            # Mark before write: a failed ACK can still mean the sound started.
            self.finding = self.finding or bool(mask)
            if mask:
                self.finder_task = asyncio.create_task(self.stop_later())
            await self.send(p.request(50, mask))
            self.finding = bool(mask)
            self.emit('notice', 'Đang phát tiếng tìm; tự dừng sau 10 giây.' if mask else 'Đã gửi dừng tiếng tìm.')

    async def stop_later(self):
        await asyncio.sleep(10)
        self.finder_task = None
        try:
            await self.find(0)
        except Exception as ex:
            self.emit('error', 'Không gửi được dừng tiếng tìm: ' + str(ex))

    async def _disconnect(self):
        if self.poll_task and self.poll_task is not asyncio.current_task():
            self.poll_task.cancel()
        self.poll_task = None
        if self.finder_task:
            self.finder_task.cancel()
            self.finder_task = None
        client = self.client
        if client:
            try:
                if self.finding and client.is_connected:
                    await asyncio.wait_for(client.write_gatt_char(p.TX, p.request(50, 0), response=True), 2)
                await asyncio.wait_for(client.disconnect(), 5)
            except Exception as ex:
                self.record('!', 'Đóng Bluetooth: ' + str(ex))
        self.finding = False
        self.client = None
        self.state.clear()
        self.phones.clear()
        self.emit('state', {})
        self.emit('phones', [])
        self.emit('connection', 'disconnected')

    async def disconnect(self):
        async with self.lock:
            await self._disconnect()
