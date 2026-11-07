//! Asynchronous BLE backend owning device sessions and serializing TW600 operations.

use crate::protocol::{
    self as p, DeviceState, PhoneEntry, Reply, StateUpdate, RX_UUID, SERVICE_STR, SERVICE_UUID,
    TX_UUID,
};
use crate::workflows::{Plan, Step};
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use chrono::Local;
use futures::StreamExt;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    Devices(Vec<(String, String)>),
    Connection(ConnectionStatus),
    ConnectedAddress(String),
    State(DeviceState),
    Phones(Vec<PhoneEntry>),
    Notice(String),
    Error(String),
    Log(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackendError {
    CommandRejected(String),
    Connection(String),
    Runtime(String),
}

impl std::fmt::Display for BackendError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendError::CommandRejected(s)
            | BackendError::Connection(s)
            | BackendError::Runtime(s) => write!(f, "{}", s),
        }
    }
}

impl std::error::Error for BackendError {}

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait GattClient: Send + Sync {
    fn is_connected(&self) -> bool;
    fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>>;
    fn close_session(&self) -> BoxFuture<'_, ()>;
}

struct PendingRequest {
    vendor: u16,
    command: u16,
    tx: oneshot::Sender<Reply>,
}

pub struct BtleplugClient {
    peripheral: Peripheral,
    tx_char: Characteristic,
    rx_char: Characteristic,
    connected_flag: AtomicBool,
    notify_abort: Mutex<Option<tokio::task::AbortHandle>>,
}

impl GattClient for BtleplugClient {
    fn is_connected(&self) -> bool {
        self.connected_flag.load(Ordering::SeqCst)
    }

    fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
        Box::pin(async move {
            if !self.is_connected() {
                return Err("Bluetooth đã ngắt kết nối".to_string());
            }
            self.peripheral
                .write(&self.tx_char, &packet, WriteType::WithResponse)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn close_session(&self) -> BoxFuture<'_, ()> {
        Box::pin(async move {
            self.connected_flag.store(false, Ordering::SeqCst);
            let abort_opt = self.notify_abort.lock().unwrap().take();
            if let Some(handle) = abort_opt {
                handle.abort();
            }
            let _ = tokio::time::timeout(
                Duration::from_secs(2),
                self.peripheral.unsubscribe(&self.rx_char),
            )
            .await;
        })
    }
}

pub type ReconnectHook =
    Arc<dyn Fn(String, bool) -> BoxFuture<'static, Result<(), BackendError>> + Send + Sync>;
pub type QueryHook =
    Arc<dyn Fn(bool) -> BoxFuture<'static, Result<(), BackendError>> + Send + Sync>;

pub struct BackendCore {
    emit: Arc<dyn Fn(BackendEvent) + Send + Sync>,
    pub log: Mutex<VecDeque<String>>,
    pub state: Mutex<DeviceState>,
    pub phones: Mutex<BTreeMap<u8, PhoneEntry>>,
    pending: Mutex<Option<PendingRequest>>,
    pub client: Mutex<Option<Arc<dyn GattClient>>>,
    pub devices: Mutex<BTreeMap<String, String>>,
    peripherals: Mutex<BTreeMap<String, Peripheral>>,
    pub op_lock: AsyncMutex<()>,
    pub wire_lock: AsyncMutex<()>,
    pub finding: AtomicBool,
    pub closing: AtomicBool,
    pub address: Mutex<Option<String>>,
    pub request_timeout: Mutex<Duration>,
    pub step_sleep: Mutex<Duration>,
    finder_abort: Mutex<Option<tokio::task::AbortHandle>>,
    poll_abort: Mutex<Option<tokio::task::AbortHandle>>,
    reconnect_hook: Mutex<Option<ReconnectHook>>,
    query_hook: Mutex<Option<QueryHook>>,
}

impl BackendCore {
    pub fn new(emit: Arc<dyn Fn(BackendEvent) + Send + Sync>) -> Arc<Self> {
        Arc::new(Self {
            emit,
            log: Mutex::new(VecDeque::with_capacity(2000)),
            state: Mutex::new(DeviceState::default()),
            phones: Mutex::new(BTreeMap::new()),
            pending: Mutex::new(None),
            client: Mutex::new(None),
            devices: Mutex::new(BTreeMap::new()),
            peripherals: Mutex::new(BTreeMap::new()),
            op_lock: AsyncMutex::new(()),
            wire_lock: AsyncMutex::new(()),
            finding: AtomicBool::new(false),
            closing: AtomicBool::new(false),
            address: Mutex::new(None),
            request_timeout: Mutex::new(Duration::from_secs(4)),
            step_sleep: Mutex::new(Duration::from_secs(1)),
            finder_abort: Mutex::new(None),
            poll_abort: Mutex::new(None),
            reconnect_hook: Mutex::new(None),
            query_hook: Mutex::new(None),
        })
    }

    pub fn set_reconnect_hook(&self, hook: ReconnectHook) {
        *self.reconnect_hook.lock().unwrap() = Some(hook);
    }

    pub fn set_query_hook(&self, hook: QueryHook) {
        *self.query_hook.lock().unwrap() = Some(hook);
    }

    pub fn emit_event(&self, ev: BackendEvent) {
        (self.emit)(ev);
    }

    pub fn record(&self, direction: &str, text: &str) {
        let ts = Local::now().format("%H:%M:%S");
        let line = format!("{} {} {}", ts, direction, text);
        {
            let mut log = self.log.lock().unwrap();
            if log.len() >= 2000 {
                log.pop_front();
            }
            log.push_back(line.clone());
        }
        self.emit_event(BackendEvent::Log(line));
    }

    pub fn is_connected(&self) -> bool {
        let has_client = self
            .client
            .lock()
            .unwrap()
            .as_ref()
            .map(|c| c.is_connected())
            .unwrap_or(false);
        let has_version = self.state.lock().unwrap().version.is_some();
        has_client && has_version
    }

    pub fn supported(name: &str, uuids: &[String]) -> bool {
        name.eq_ignore_ascii_case("earfun air pro 4")
            || name.starts_with("TW600-")
            || uuids.iter().any(|u| u.eq_ignore_ascii_case(SERVICE_STR))
    }

    pub fn notify(&self, data: &[u8]) {
        self.record("<<", &p::hex_encode_spaced(data));
        let Some(reply) = p::decode(data) else {
            return;
        };
        {
            let mut pending = self.pending.lock().unwrap();
            if let Some(req) = pending.as_ref() {
                if req.vendor == reply.vendor && req.command == reply.command {
                    let req = pending.take().unwrap();
                    let _ = req.tx.send(reply.clone());
                }
            }
        }
        let has_version = self.state.lock().unwrap().version.is_some();
        if has_version {
            self.apply(&reply);
        }
    }

    pub fn apply(&self, reply: &Reply) -> StateUpdate {
        let update = p::state(reply);
        if let Some(phone) = &update.phone {
            let list = {
                let mut phones = self.phones.lock().unwrap();
                if phone.num == 0 {
                    phones.clear();
                }
                phones.insert(phone.num, phone.clone());
                phones.values().cloned().collect::<Vec<_>>()
            };
            self.emit_event(BackendEvent::Phones(list));
        }
        let snapshot = {
            let mut st = self.state.lock().unwrap();
            st.merge(&update);
            st.clone()
        };
        self.emit_event(BackendEvent::State(snapshot));
        update
    }

    pub fn lost(&self) {
        self.state.lock().unwrap().clone_from(&DeviceState::default());
        self.phones.lock().unwrap().clear();
        let _ = self.pending.lock().unwrap().take();
        self.emit_event(BackendEvent::State(DeviceState::default()));
        self.emit_event(BackendEvent::Phones(Vec::new()));
        self.emit_event(BackendEvent::Connection(ConnectionStatus::Disconnected));
    }

    pub async fn write(&self, packet: &[u8]) -> Result<(), BackendError> {
        let client = self.client.lock().unwrap().clone();
        let Some(client) = client else {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        };
        if !client.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        self.record(">>", &p::hex_encode_spaced(packet));
        match tokio::time::timeout(Duration::from_secs(5), client.write_tx(packet.to_vec())).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(e)) => Err(BackendError::Connection(e)),
            Err(_) => Err(BackendError::Runtime("Ghi GATT quá thời gian chờ".to_string())),
        }
    }

    pub async fn request(&self, packet: &[u8]) -> Result<Reply, BackendError> {
        let _guard = self.wire_lock.lock().await;
        let vendor = u16::from_be_bytes([packet[0], packet[1]]);
        let command = u16::from_be_bytes([packet[2], packet[3]]);
        let (tx, rx) = oneshot::channel();
        {
            *self.pending.lock().unwrap() = Some(PendingRequest {
                vendor,
                command,
                tx,
            });
        }
        let timeout_dur = *self.request_timeout.lock().unwrap();
        let write_res = self.write(packet).await;
        if let Err(e) = write_res {
            *self.pending.lock().unwrap() = None;
            self.disconnect_internal().await;
            return Err(e);
        }

        match tokio::time::timeout(timeout_dur, rx).await {
            Ok(Ok(reply)) => {
                *self.pending.lock().unwrap() = None;
                if reply.status != 0 {
                    return Err(BackendError::CommandRejected(format!(
                        "Tai nghe từ chối lệnh {:04X} (mã {}).",
                        command, reply.status
                    )));
                }
                Ok(reply)
            }
            _ => {
                *self.pending.lock().unwrap() = None;
                self.disconnect_internal().await;
                Err(BackendError::Runtime(
                    "Bluetooth không phản hồi; đã đóng kênh điều khiển. Hãy kết nối lại."
                        .to_string(),
                ))
            }
        }
    }

    pub async fn send(&self, packet: &[u8]) -> Result<(), BackendError> {
        let _guard = self.wire_lock.lock().await;
        if let Err(e) = self.write(packet).await {
            self.disconnect_internal().await;
            return Err(e);
        }
        let sleep_dur = (*self.step_sleep.lock().unwrap()).min(Duration::from_millis(80));
        tokio::time::sleep(sleep_dur).await;
        Ok(())
    }

    pub async fn query_internal(&self, full: bool) -> Result<(), BackendError> {
        let hook_opt = self.query_hook.lock().unwrap().clone();
        if let Some(hook) = hook_opt {
            return hook(full).await;
        }
        let commands = if full { p::QUERIES } else { p::POLL_QUERIES };
        for &cmd in commands {
            match self.request(&p::request(cmd, &[])).await {
                Ok(reply) => {
                    self.apply(&reply);
                }
                Err(BackendError::CommandRejected(msg)) => {
                    self.record("?", &msg);
                }
                Err(e) => return Err(e),
            }
        }
        if full {
            self.phones_internal().await?;
        }
        self.emit_event(BackendEvent::Notice(
            "Đã đọc trạng thái trực tiếp từ tai nghe.".to_string(),
        ));
        Ok(())
    }

    pub async fn query(&self) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if self.is_connected() {
            self.query_internal(true).await?;
        }
        Ok(())
    }

    pub async fn set_bool_value(
        &self,
        setter: u8,
        query: u8,
        value: u8,
        key: &str,
        wanted: bool,
    ) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        self.set_bool_internal(setter, query, value, key, wanted).await
    }

    async fn set_bool_internal(
        &self,
        setter: u8,
        query: u8,
        value: u8,
        key: &str,
        wanted: bool,
    ) -> Result<(), BackendError> {
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        self.send(&p::request(setter, &[value])).await?;
        let wait_dur = (*self.step_sleep.lock().unwrap()).min(Duration::from_millis(400));
        for _ in 0..8 {
            tokio::time::sleep(wait_dur).await;
            let reply = self.request(&p::request(query, &[])).await?;
            self.apply(&reply);
            if self.state.lock().unwrap().get_bool(key) == Some(wanted) {
                self.emit_event(BackendEvent::Notice(
                    "Tai nghe đã xác nhận thay đổi.".to_string(),
                ));
                return Ok(());
            }
        }
        Err(BackendError::Runtime(
            "Tai nghe chưa xác nhận giá trị mới. Giao diện hiển thị trạng thái đọc lại."
                .to_string(),
        ))
    }

    pub async fn set_string_value(
        &self,
        setter: u8,
        query: u8,
        value: u8,
        key: &str,
        wanted: &str,
    ) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        self.send(&p::request(setter, &[value])).await?;
        let wait_dur = (*self.step_sleep.lock().unwrap()).min(Duration::from_millis(400));
        for _ in 0..8 {
            tokio::time::sleep(wait_dur).await;
            let reply = self.request(&p::request(query, &[])).await?;
            let upd = self.apply(&reply);
            let actual = match key {
                "noise" => upd.noise.as_deref(),
                "sub" => upd.sub.as_deref(),
                "amb" => upd.amb.as_deref(),
                "mic" => upd.mic.as_deref(),
                _ => None,
            };
            if actual == Some(wanted) {
                self.emit_event(BackendEvent::Notice(
                    "Tai nghe đã xác nhận thay đổi.".to_string(),
                ));
                return Ok(());
            }
        }
        Err(BackendError::Runtime(
            "Tai nghe chưa xác nhận giá trị mới. Giao diện hiển thị trạng thái đọc lại."
                .to_string(),
        ))
    }

    pub async fn set_u8_value(
        &self,
        setter: u8,
        query: u8,
        value: u8,
        key: &str,
        wanted: u8,
    ) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        self.send(&p::request(setter, &[value])).await?;
        let wait_dur = (*self.step_sleep.lock().unwrap()).min(Duration::from_millis(400));
        for _ in 0..8 {
            tokio::time::sleep(wait_dur).await;
            let reply = self.request(&p::request(query, &[])).await?;
            let upd = self.apply(&reply);
            let actual = match key {
                "prompt" => upd.prompt,
                "hearing_limit" => upd.hearing_limit,
                "balance" => upd.balance,
                _ => None,
            };
            if actual == Some(wanted) {
                self.emit_event(BackendEvent::Notice(
                    "Tai nghe đã xác nhận thay đổi.".to_string(),
                ));
                return Ok(());
            }
        }
        Err(BackendError::Runtime(
            "Tai nghe chưa xác nhận giá trị mới. Giao diện hiển thị trạng thái đọc lại."
                .to_string(),
        ))
    }

    pub async fn direct(&self, command: u8, payload: &[u8]) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi điều chỉnh.".to_string(),
            ));
        }
        self.send(&p::request(command, payload)).await?;
        self.emit_event(BackendEvent::Notice(
            "Đã gửi lệnh. Thiết bị có thể ngắt kết nối; hãy đọc lại để kiểm tra.".to_string(),
        ));
        Ok(())
    }

    pub async fn set_key(&self, side: &str, gesture: &str, action: &str) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection("Chưa kết nối tai nghe.".to_string()));
        }
        let pkt = p::gesture_packet(side, gesture, action).map_err(BackendError::Runtime)?;
        self.send(&pkt).await?;
        let wait_dur = (*self.step_sleep.lock().unwrap()).min(Duration::from_millis(300));
        tokio::time::sleep(wait_dur).await;
        let reply = self.request(&p::request(11, &[])).await?;
        let upd = self.apply(&reply);
        let field = format!("{}.{}", side, gesture);
        if upd.keys.get(&field).map(|s| s.as_str()) != Some(action) {
            return Err(BackendError::Runtime(
                "Tai nghe chưa xác nhận thao tác cảm ứng mới.".to_string(),
            ));
        }
        self.emit_event(BackendEvent::Notice(
            "Đã xác nhận thao tác cảm ứng.".to_string(),
        ));
        Ok(())
    }

    pub async fn set_eq(&self, levels: &[i32]) -> Result<(), BackendError> {
        let packets = p::equalizer(levels).map_err(BackendError::Runtime)?;
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection("Chưa kết nối tai nghe.".to_string()));
        }
        for pkt in packets {
            self.send(&pkt).await?;
        }
        self.emit_event(BackendEvent::Notice(
            "Đã ghi đủ 10 gói EQ. Chưa có xác nhận đọc lại đường cong EQ.".to_string(),
        ));
        Ok(())
    }

    pub async fn codecs(&self, values: &BTreeMap<String, bool>) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        let (has_all, dual_on) = {
            let st = self.state.lock().unwrap();
            let all = ["ldac", "aptx", "adaptive", "lossless", "dual"]
                .iter()
                .all(|k| st.get_bool(k).is_some());
            (all, st.dual.unwrap_or(false))
        };
        if !has_all {
            return Err(BackendError::Runtime(
                "Chưa đọc đủ trạng thái codec / đa điểm.".to_string(),
            ));
        }
        let wanted_ldac = *values.get("ldac").unwrap_or(&false);
        if wanted_ldac && dual_on {
            return Err(BackendError::Runtime(
                "Tắt đa điểm trước khi bật LDAC.".to_string(),
            ));
        }
        let mut bits: u8 = 0;
        for (key, bit) in [("aptx", 1u8), ("adaptive", 2u8), ("ldac", 8u8), ("lossless", 16u8)] {
            if *values.get(key).unwrap_or(&false) {
                bits |= bit;
            }
        }
        self.set_bool_internal(46, 47, bits, "ldac", wanted_ldac).await?;
        let st = self.state.lock().unwrap();
        for (k, &v) in values {
            if st.get_bool(k) != Some(v) {
                return Err(BackendError::Runtime(
                    "Tai nghe chưa xác nhận đủ các codec.".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub async fn disable_touch(&self, left: bool, right: bool) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        {
            let st = self.state.lock().unwrap();
            if st.dis_l.is_none() || st.dis_r.is_none() {
                return Err(BackendError::Runtime(
                    "Chưa đọc đủ trạng thái khóa cảm ứng hai tai.".to_string(),
                ));
            }
        }
        let value = if left && right {
            1
        } else if left {
            2
        } else if right {
            3
        } else {
            0
        };
        self.set_bool_internal(52, 53, value, "dis_l", left).await?;
        if self.state.lock().unwrap().dis_r != Some(right) {
            return Err(BackendError::Runtime(
                "Chưa xác nhận khóa cảm ứng tai phải.".to_string(),
            ));
        }
        Ok(())
    }

    async fn phones_internal(&self) -> Result<(), BackendError> {
        self.phones.lock().unwrap().clear();
        self.emit_event(BackendEvent::Phones(Vec::new()));
        match self.request(&p::request(44, &[])).await {
            Ok(reply) => {
                if self.state.lock().unwrap().version.is_none() {
                    self.apply(&reply);
                }
                Ok(())
            }
            Err(BackendError::CommandRejected(msg)) => {
                self.record("?", &msg);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub async fn phones_action(&self, command: u8, num: Option<u8>) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if !self.is_connected() {
            return Err(BackendError::Connection("Chưa kết nối tai nghe.".to_string()));
        }
        if command == 41 || command == 43 {
            let connected_count = self
                .phones
                .lock()
                .unwrap()
                .values()
                .filter(|p| p.connected)
                .count();
            if connected_count >= 2 {
                return Err(BackendError::Runtime(
                    "Đã kết nối hai thiết bị. Ngắt một thiết bị trước.".to_string(),
                ));
            }
        }
        if command != 44 {
            let payload: Vec<u8> = num.into_iter().collect();
            self.send(&p::request(command, &payload)).await?;
            let sleep_dur = *self.step_sleep.lock().unwrap();
            tokio::time::sleep(sleep_dur).await;
        }
        self.phones_internal().await?;
        self.emit_event(BackendEvent::Notice(if command != 43 {
            "Đã đọc danh sách thiết bị.".to_string()
        } else {
            "Đã mở ghép nối. Dùng Cài đặt Bluetooth trên thiết bị mới, rồi bấm Làm mới."
                .to_string()
        }));
        Ok(())
    }

    async fn stop_find(&self) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        let abort_opt = self.finder_abort.lock().unwrap().take();
        if let Some(handle) = abort_opt {
            handle.abort();
        }
        self.send(&p::request(50, &[0])).await?;
        self.finding.store(false, Ordering::SeqCst);
        self.emit_event(BackendEvent::Notice("Đã gửi dừng tiếng tìm.".to_string()));
        Ok(())
    }

    pub async fn find(self: &Arc<Self>, mask: u8) -> Result<(), BackendError> {
        if mask == 0 {
            return self.stop_find().await;
        }
        let _guard = self.op_lock.lock().await;
        let abort_opt = self.finder_abort.lock().unwrap().take();
        if let Some(handle) = abort_opt {
            handle.abort();
        }
        self.finding.store(true, Ordering::SeqCst);
        let weak = Arc::downgrade(self);
        let join = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(10)).await;
            if let Some(strong) = weak.upgrade() {
                if let Err(e) = strong.stop_find().await {
                    strong.emit_event(BackendEvent::Error(format!(
                        "Không gửi được dừng tiếng tìm: {}",
                        e
                    )));
                }
            }
        });
        *self.finder_abort.lock().unwrap() = Some(join.abort_handle());
        self.send(&p::request(50, &[mask])).await?;
        self.finding.store(true, Ordering::SeqCst);
        self.emit_event(BackendEvent::Notice(
            "Đang phát tiếng tìm; tự dừng sau 10 giây.".to_string(),
        ));
        Ok(())
    }

    pub async fn run_plan(self: &Arc<Self>, plan: &Plan) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if self.closing.load(Ordering::SeqCst) {
            return Err(BackendError::Runtime(
                "Ứng dụng đang đóng; chưa gửi thay đổi.".to_string(),
            ));
        }
        if !self.is_connected() {
            return Err(BackendError::Connection(
                "Kết nối tai nghe trước khi thay đổi thiết lập.".to_string(),
            ));
        }
        if plan.steps.is_empty() {
            self.emit_event(BackendEvent::Notice(
                "Tai nghe đã ở thiết lập này; không cần gửi lại.".to_string(),
            ));
            return Ok(());
        }
        for &cmd in &plan.queries {
            let reply = self.request(&p::request(cmd, &[])).await?;
            self.apply(&reply);
        }
        {
            let st = self.state.lock().unwrap();
            if plan
                .baseline
                .iter()
                .any(|(k, v)| st.get_bool(k) != Some(*v))
            {
                return Err(BackendError::Runtime(
                    "Thiết lập tai nghe đã đổi trong lúc chờ xác nhận. Hãy chọn lại theo trạng thái mới."
                        .to_string(),
                ));
            }
        }

        let mut completed: Vec<String> = Vec::new();
        for (idx, step) in plan.steps.iter().enumerate() {
            if self.closing.load(Ordering::SeqCst) {
                let err = "Đã dừng quy trình vì ứng dụng đang đóng.";
                let detail = if completed.is_empty() {
                    String::new()
                } else {
                    format!("Đã hoàn tất: {}. ", completed.join("; "))
                };
                return Err(BackendError::Runtime(format!(
                    "{}Chưa xác nhận hoàn tất toàn bộ thay đổi. {}",
                    detail, err
                )));
            }
            self.emit_event(BackendEvent::Notice(format!(
                "{}/{} · {}…",
                idx + 1,
                plan.steps.len(),
                step.title
            )));
            if let Err(e) = self.run_restart_step(step).await {
                let detail = if completed.is_empty() {
                    String::new()
                } else {
                    format!("Đã hoàn tất: {}. ", completed.join("; "))
                };
                return Err(BackendError::Runtime(format!(
                    "{}Chưa xác nhận hoàn tất toàn bộ thay đổi. {}",
                    detail, e
                )));
            }
            completed.push(step.title.clone());
        }

        if let Err(e) = self.query_internal(true).await {
            let detail = if completed.is_empty() {
                String::new()
            } else {
                format!("Đã hoàn tất: {}. ", completed.join("; "))
            };
            return Err(BackendError::Runtime(format!(
                "{}Chưa xác nhận hoàn tất toàn bộ thay đổi. {}",
                detail, e
            )));
        }

        {
            let st = self.state.lock().unwrap();
            if plan.target.iter().any(|(k, v)| st.get_bool(k) != Some(*v)) {
                let detail = if completed.is_empty() {
                    String::new()
                } else {
                    format!("Đã hoàn tất: {}. ", completed.join("; "))
                };
                return Err(BackendError::Runtime(format!(
                    "{}Chưa xác nhận hoàn tất toàn bộ thay đổi. Trạng thái cuối khác lựa chọn đã xác nhận.",
                    detail
                )));
            }
        }

        self.emit_event(BackendEvent::Notice(format!(
            "Đã hoàn tất và đọc lại xác nhận: {}.",
            plan.title
        )));
        Ok(())
    }

    async fn run_restart_step(self: &Arc<Self>, step: &Step) -> Result<(), BackendError> {
        let address = self.address.lock().unwrap().clone();
        if let Err(e) = self.send(&p::request(step.setter, &[step.value])).await {
            self.record("?", &format!("Kiểm tra sau khi gửi lệnh bị gián đoạn: {}", e));
        }

        let step_sleep = *self.step_sleep.lock().unwrap();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(50);

        for attempt in 0..25 {
            if tokio::time::Instant::now() >= deadline {
                break;
            }
            if self.closing.load(Ordering::SeqCst) {
                return Err(BackendError::Runtime(
                    "Đã dừng quy trình vì ứng dụng đang đóng.".to_string(),
                ));
            }
            tokio::time::sleep(step_sleep).await;

            if !self.is_connected() {
                let Some(ref addr) = address else {
                    return Err(BackendError::Connection(
                        "Không còn địa chỉ tai nghe để kết nối lại.".to_string(),
                    ));
                };
                self.emit_event(BackendEvent::Notice(format!(
                    "Tai nghe đang khởi động lại. Đang kết nối lại (lần {})…",
                    attempt + 1
                )));
                let hook_opt = self.reconnect_hook.lock().unwrap().clone();
                let reconn_res = if let Some(hook) = hook_opt {
                    hook(addr.clone(), false).await
                } else {
                    self.connect_internal(addr, false).await
                };
                if let Err(e) = reconn_res {
                    self.record("?", &format!("Đọc lại sau thay đổi: {}", e));
                    continue;
                }
            }

            match self.request(&p::request(step.query, &[])).await {
                Ok(reply) => {
                    self.apply(&reply);
                    let matched = {
                        let st = self.state.lock().unwrap();
                        step.wanted.iter().all(|(k, v)| st.get_bool(k) == Some(*v))
                    };
                    if matched {
                        return Ok(());
                    }
                }
                Err(BackendError::CommandRejected(msg)) => {
                    return Err(BackendError::CommandRejected(msg));
                }
                Err(e) => {
                    self.record("?", &format!("Đọc lại sau thay đổi: {}", e));
                }
            }
        }

        Err(BackendError::Runtime(format!(
            "Tai nghe chưa xác nhận bước “{}”. Trạng thái đang hiển thị là phản hồi đọc được gần nhất.",
            step.title
        )))
    }

    pub async fn disconnect_internal(&self) {
        let poll_opt = self.poll_abort.lock().unwrap().take();
        if let Some(handle) = poll_opt {
            handle.abort();
        }
        let finder_opt = self.finder_abort.lock().unwrap().take();
        if let Some(handle) = finder_opt {
            handle.abort();
        }
        let client = self.client.lock().unwrap().take();
        if let Some(client) = client {
            if self.finding.load(Ordering::SeqCst) && client.is_connected() {
                let _ = tokio::time::timeout(
                    Duration::from_secs(2),
                    client.write_tx(p::request(50, &[0])),
                )
                .await;
            }
            client.close_session().await;
        }
        self.finding.store(false, Ordering::SeqCst);
        self.state.lock().unwrap().clone_from(&DeviceState::default());
        self.phones.lock().unwrap().clear();
        self.emit_event(BackendEvent::State(DeviceState::default()));
        self.emit_event(BackendEvent::Phones(Vec::new()));
        self.emit_event(BackendEvent::Connection(ConnectionStatus::Disconnected));
    }

    pub async fn disconnect(&self) {
        let _guard = self.op_lock.lock().await;
        self.disconnect_internal().await;
    }

    async fn get_adapter() -> Result<Adapter, BackendError> {
        let manager = Manager::new()
            .await
            .map_err(|e| BackendError::Runtime(format!("Không mở được Bluetooth: {}", e)))?;
        let adapters = manager
            .adapters()
            .await
            .map_err(|e| BackendError::Runtime(format!("Không đọc được bộ thu Bluetooth: {}", e)))?;
        adapters.into_iter().next().ok_or_else(|| {
            BackendError::Runtime("Không tìm thấy bộ thu Bluetooth trên máy.".to_string())
        })
    }

    async fn collect_known_devices(&self, adapter: &Adapter) -> BTreeMap<String, String> {
        let mut found = BTreeMap::new();
        if let Ok(peripherals) = adapter.peripherals().await {
            for p in peripherals {
                if let Ok(Some(props)) = p.properties().await {
                    let name = props.local_name.unwrap_or_default();
                    let uuids: Vec<String> =
                        props.services.iter().map(|u| u.to_string()).collect();
                    if Self::supported(&name, &uuids) {
                        let addr = props.address.to_string();
                        let display = if name.is_empty() {
                            addr.clone()
                        } else {
                            name
                        };
                        found.insert(addr.clone(), display);
                        self.peripherals.lock().unwrap().insert(addr, p);
                    }
                }
            }
        }
        found
    }

    pub async fn load_devices(&self) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        let adapter = Self::get_adapter().await?;
        let found = self.collect_known_devices(&adapter).await;
        *self.devices.lock().unwrap() = found.clone();
        let list: Vec<(String, String)> = found.into_iter().collect();
        let has_any = !list.is_empty();
        self.emit_event(BackendEvent::Devices(list));
        if has_any {
            self.emit_event(BackendEvent::Notice(
                "Đã tìm thấy tai nghe đã ghép. Bấm Kết nối để mở bảng điều khiển.".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn scan(&self) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        self.emit_event(BackendEvent::Notice(
            "Đang quét Bluetooth trong 5 giây…".to_string(),
        ));
        let adapter = Self::get_adapter().await?;
        let mut found = self.collect_known_devices(&adapter).await;
        let _ = adapter.start_scan(ScanFilter::default()).await;
        tokio::time::sleep(Duration::from_secs(5)).await;
        let _ = adapter.stop_scan().await;
        for (k, v) in self.collect_known_devices(&adapter).await {
            found.insert(k, v);
        }
        *self.devices.lock().unwrap() = found.clone();
        let count = found.len();
        let list: Vec<(String, String)> = found.into_iter().collect();
        self.emit_event(BackendEvent::Devices(list));
        self.emit_event(BackendEvent::Notice(format!(
            "Tìm thấy {} tai nghe. {}",
            count,
            if count > 0 {
                "Chọn thiết bị rồi kết nối."
            } else {
                "Lấy tai nghe ra khỏi hộp và ghép trong Cài đặt Bluetooth."
            }
        )));
        Ok(())
    }

    pub async fn connect(self: &Arc<Self>, address: &str) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        self.connect_internal(address, true).await
    }

    async fn connect_internal(
        self: &Arc<Self>,
        address: &str,
        full: bool,
    ) -> Result<(), BackendError> {
        let has_existing = self.client.lock().unwrap().is_some();
        if has_existing {
            self.disconnect_internal().await;
        }
        self.emit_event(BackendEvent::Connection(ConnectionStatus::Connecting));
        self.state.lock().unwrap().clone_from(&DeviceState::default());

        let peripheral_opt = self.peripherals.lock().unwrap().get(address).cloned();
        let peripheral = match peripheral_opt {
            Some(p) => p,
            None => {
                let adapter = Self::get_adapter().await?;
                let _ = self.collect_known_devices(&adapter).await;
                let p_opt = self.peripherals.lock().unwrap().get(address).cloned();
                match p_opt {
                    Some(p) => p,
                    None => {
                        self.emit_event(BackendEvent::Connection(ConnectionStatus::Disconnected));
                        return Err(BackendError::Connection(
                            "Không tìm thấy thiết bị Bluetooth đã chọn. Hãy bấm Tìm thiết bị."
                                .to_string(),
                        ));
                    }
                }
            }
        };

        let result: Result<(), BackendError> = async {
            for attempt in 0..2 {
                let is_conn = peripheral.is_connected().await.unwrap_or(false);
                if !is_conn || attempt > 0 {
                    tokio::time::timeout(Duration::from_secs(25), peripheral.connect())
                        .await
                        .map_err(|_| {
                            BackendError::Connection("Kết nối Bluetooth quá thời gian chờ.".to_string())
                        })?
                        .map_err(|e| BackendError::Connection(e.to_string()))?;
                }

                tokio::time::timeout(Duration::from_secs(12), peripheral.discover_services())
                    .await
                    .map_err(|_| {
                        BackendError::Connection("Đọc dịch vụ GATT quá thời gian chờ.".to_string())
                    })?
                    .map_err(|e| BackendError::Connection(e.to_string()))?;

                let chars = peripheral.characteristics();
                let tx_char = chars
                    .iter()
                    .find(|c| c.uuid == TX_UUID && c.service_uuid == SERVICE_UUID)
                    .or_else(|| chars.iter().find(|c| c.uuid == TX_UUID))
                    .cloned();
                let rx_char = chars
                    .iter()
                    .find(|c| c.uuid == RX_UUID && c.service_uuid == SERVICE_UUID)
                    .or_else(|| chars.iter().find(|c| c.uuid == RX_UUID))
                    .cloned();

                let (Some(tx_char), Some(rx_char)) = (tx_char, rx_char) else {
                    if attempt == 0 {
                        self.record("!", "Chưa thấy dịch vụ GATT TW600, thử mở lại kênh BLE…");
                        self.emit_event(BackendEvent::Notice(
                            "Đang mở kênh Bluetooth LE điều khiển tai nghe…".to_string(),
                        ));
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    return Err(BackendError::Connection(
                        "Thiết bị chưa mở dịch vụ điều khiển TW600.".to_string(),
                    ));
                };

                if let Err(e) = peripheral.subscribe(&rx_char).await {
                    if attempt == 0 {
                        self.record("!", &format!("Mở lại kênh BLE sau lỗi: {}", e));
                        self.emit_event(BackendEvent::Notice(
                            "Đang mở kênh Bluetooth LE điều khiển tai nghe…".to_string(),
                        ));
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        continue;
                    }
                    return Err(BackendError::Connection(e.to_string()));
                }

                let mut stream = peripheral
                    .notifications()
                    .await
                    .map_err(|e| BackendError::Connection(e.to_string()))?;
                let weak_core = Arc::downgrade(self);
                let notify_task = tokio::spawn(async move {
                    while let Some(notification) = stream.next().await {
                        if notification.uuid == RX_UUID {
                            if let Some(core) = weak_core.upgrade() {
                                core.notify(&notification.value);
                            } else {
                                break;
                            }
                        }
                    }
                });

                let client_arc = Arc::new(BtleplugClient {
                    peripheral: peripheral.clone(),
                    tx_char,
                    rx_char,
                    connected_flag: AtomicBool::new(true),
                    notify_abort: Mutex::new(Some(notify_task.abort_handle())),
                });
                *self.client.lock().unwrap() = Some(client_arc);

                match self.request(&p::request(9, &[])).await {
                    Ok(version_reply) => {
                        let upd = p::state(&version_reply);
                        let fw = upd.version.unwrap_or_default();
                        if !fw.starts_with("TW600-") {
                            return Err(BackendError::Runtime(format!(
                                "Firmware không xác nhận TW600: {}",
                                fw
                            )));
                        }
                        self.apply(&version_reply);
                        *self.address.lock().unwrap() = Some(address.to_string());
                        self.emit_event(BackendEvent::ConnectedAddress(address.to_string()));
                        if full {
                            self.query_internal(true).await?;
                        }
                        self.emit_event(BackendEvent::Connection(ConnectionStatus::Connected));

                        let weak_poll = Arc::downgrade(self);
                        let poll_task = tokio::spawn(async move {
                            loop {
                                tokio::time::sleep(Duration::from_secs(10)).await;
                                let Some(core) = weak_poll.upgrade() else {
                                    break;
                                };
                                if !core.is_connected() {
                                    break;
                                }
                                if let Ok(_guard) = core.op_lock.try_lock() {
                                    if core.is_connected() {
                                        if let Err(e) = core.query_internal(false).await {
                                            core.record("!", &e.to_string());
                                            core.emit_event(BackendEvent::Error(e.to_string()));
                                            core.disconnect_internal().await;
                                            break;
                                        }
                                    }
                                };
                            }
                        });
                        *self.poll_abort.lock().unwrap() = Some(poll_task.abort_handle());
                        return Ok(());
                    }
                    Err(e) => {
                        if attempt == 0 {
                            self.record("!", &format!("Mở lại kênh BLE sau lỗi: {}", e));
                            self.emit_event(BackendEvent::Notice(
                                "Đang mở kênh Bluetooth LE điều khiển tai nghe…".to_string(),
                            ));
                            continue;
                        }
                        return Err(e);
                    }
                }
            }
            Err(BackendError::Connection(
                "Không thể kết nối tới tai nghe.".to_string(),
            ))
        }
        .await;

        if result.is_err() {
            self.disconnect_internal().await;
        }
        result
    }
}

pub struct BackendHandle {
    pub core: Arc<BackendCore>,
    runtime: tokio::runtime::Runtime,
    busy_tx: mpsc::UnboundedSender<i32>,
}

impl BackendHandle {
    pub fn new(
        emit: Arc<dyn Fn(BackendEvent) + Send + Sync>,
        busy_tx: mpsc::UnboundedSender<i32>,
    ) -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime");
        let core = BackendCore::new(emit);
        Self {
            core,
            runtime,
            busy_tx,
        }
    }

    pub fn spawn_task<F>(&self, fut: F)
    where
        F: Future<Output = Result<(), BackendError>> + Send + 'static,
    {
        let _ = self.busy_tx.send(1);
        let busy_tx = self.busy_tx.clone();
        let core = self.core.clone();
        self.runtime.spawn(async move {
            if let Err(e) = fut.await {
                let msg = e.to_string();
                core.record("!", &msg);
                core.emit_event(BackendEvent::Error(msg));
            }
            let _ = busy_tx.send(-1);
        });
    }

    pub fn shutdown_blocking(&self) {
        self.core.closing.store(true, Ordering::SeqCst);
        let core = self.core.clone();
        self.runtime.block_on(async move {
            let _ = tokio::time::timeout(Duration::from_secs(4), core.disconnect()).await;
        });
    }
}
