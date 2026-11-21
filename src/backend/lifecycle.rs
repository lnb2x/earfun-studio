//! Bluetooth adapter validation, discovery, connection lifecycle, background polling, and cleanup.

use super::{
    lock, select_auto_device, BackendCore, BackendError, BackendEvent, BtleplugClient,
    ConnectionPhase, ConnectionStatus,
};
use crate::protocol::{self as p, DeviceState, RX_UUID, SERVICE_UUID, TX_UUID};
use btleplug::api::{Central, CentralState, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager};
use futures::StreamExt;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(crate) const POLL_INTERVAL: Duration = Duration::from_secs(10);

impl BackendCore {
    pub(crate) fn start_poll_task(self: &Arc<Self>, epoch: u64) {
        let prev = lock(&self.poll_abort).take();
        if let Some(handle) = prev {
            if Some(handle.id()) != tokio::task::try_id() {
                handle.abort();
            }
        }
        let weak_poll = Arc::downgrade(self);
        let poll_task = tokio::spawn(async move {
            loop {
                tokio::time::sleep(POLL_INTERVAL).await;
                let Some(core) = weak_poll.upgrade() else {
                    break;
                };
                if !core.is_connected()
                    || core.closing.load(Ordering::SeqCst)
                    || core.session_epoch.load(Ordering::SeqCst) != epoch
                {
                    break;
                }
                if let Ok(_guard) = core.op_lock.try_lock() {
                    if core.is_connected() && core.session_epoch.load(Ordering::SeqCst) == epoch {
                        if let Err(e) = core.query_internal(false).await {
                            core.record("!", &e.to_string());
                            core.emit_event(BackendEvent::Error(e.to_string()));
                            core.disconnect_for_session(epoch).await;
                            break;
                        }
                    }
                };
            }
        });
        *lock(&self.poll_abort) = Some(poll_task.abort_handle());
    }

    pub(crate) fn ensure_polling(self: &Arc<Self>) {
        if !self.is_connected() || self.closing.load(Ordering::SeqCst) {
            return;
        }
        let needs_start = {
            let slot = lock(&self.poll_abort);
            match slot.as_ref() {
                None => true,
                Some(handle) => handle.is_finished(),
            }
        };
        if needs_start {
            let epoch = self.session_epoch.load(Ordering::SeqCst);
            self.start_poll_task(epoch);
        }
    }
    pub async fn disconnect_internal(&self) {
        self.disconnect_internal_inner(None).await;
    }

    pub(crate) async fn disconnect_for_session(&self, expected_epoch: u64) {
        self.disconnect_internal_inner(Some(expected_epoch)).await;
    }

    async fn disconnect_internal_inner(&self, expected_epoch: Option<u64>) {
        let client = {
            let mut slot = lock(&self.client);
            if let Some(expected) = expected_epoch {
                if self.session_epoch.load(Ordering::SeqCst) != expected {
                    return;
                }
            }
            self.session_epoch.fetch_add(1, Ordering::SeqCst);
            slot.take()
        };
        let had_session = client.is_some()
            || lock(&self.state).version.is_some()
            || *lock(&self.connection_status) != ConnectionStatus::Disconnected;

        let poll_opt = lock(&self.poll_abort).take();
        if let Some(handle) = poll_opt {
            if Some(handle.id()) != tokio::task::try_id() {
                handle.abort();
            }
        }
        let finder_opt = lock(&self.finder_abort).take();
        if let Some(handle) = finder_opt {
            if Some(handle.id()) != tokio::task::try_id() {
                handle.abort();
            }
        }
        let _ = lock(&self.pending).take();
        let _ = lock(&self.phone_stream).take();

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
        lock(&self.state).clone_from(&DeviceState::default());
        lock(&self.phones).clear();

        let idle_phase = if lock(&self.devices).is_empty() {
            ConnectionPhase::Disconnected
        } else {
            ConnectionPhase::PairedIdle
        };
        self.set_phase(idle_phase);

        if had_session {
            self.emit_event(BackendEvent::State(DeviceState::default()));
            self.emit_event(BackendEvent::Phones(Vec::new()));
            *lock(&self.connection_status) = ConnectionStatus::Disconnected;
            self.emit_event(BackendEvent::Connection(ConnectionStatus::Disconnected));
        }
    }

    pub async fn disconnect(&self) {
        let _guard = self.op_lock.lock().await;
        self.disconnect_internal().await;
    }

    /// Return a powered-on BLE adapter, invalidating any stale cached adapter if Bluetooth was toggled.
    async fn get_adapter(&self) -> Result<Adapter, BackendError> {
        let mut cached = self.adapter.lock().await;
        if let Some(adapter) = cached.as_ref() {
            if adapter.adapter_state().await.ok() == Some(CentralState::PoweredOn) {
                return Ok(adapter.clone());
            }
            *cached = None;
            lock(&self.peripherals).clear();
        }
        let manager = Manager::new()
            .await
            .map_err(|e| BackendError::Runtime(format!("Không mở được Bluetooth: {}", e)))?;
        let adapters = manager.adapters().await.map_err(|e| {
            BackendError::Runtime(format!("Không đọc được bộ thu Bluetooth: {}", e))
        })?;
        for adapter in adapters {
            if adapter.adapter_state().await.ok() == Some(CentralState::PoweredOn) {
                *cached = Some(adapter.clone());
                return Ok(adapter);
            }
        }
        Err(BackendError::Runtime(
            "Hãy bật Bluetooth trên máy rồi thử lại.".to_string(),
        ))
    }

    async fn collect_known_devices(&self, adapter: &Adapter) -> BTreeMap<String, String> {
        let mut found = BTreeMap::new();
        if let Ok(peripherals) = adapter.peripherals().await {
            for p in peripherals {
                if let Ok(Some(props)) = p.properties().await {
                    let name = props.local_name.unwrap_or_default();
                    let uuids: Vec<String> = props.services.iter().map(|u| u.to_string()).collect();
                    if Self::supported(&name, &uuids) {
                        let addr = props.address.to_string();
                        let display = if name.is_empty() { addr.clone() } else { name };
                        found.insert(addr.clone(), display);
                        lock(&self.peripherals).insert(addr, p);
                    }
                }
            }
        }
        found
    }

    pub async fn load_devices(&self) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        let adapter = self.get_adapter().await?;
        let found = self.collect_known_devices(&adapter).await;
        *lock(&self.devices) = found.clone();
        let list: Vec<(String, String)> = found.into_iter().collect();
        let has_any = !list.is_empty();
        self.emit_event(BackendEvent::Devices(list));
        if !self.is_connected() {
            self.set_phase(if has_any {
                ConnectionPhase::PairedIdle
            } else {
                ConnectionPhase::Disconnected
            });
        }
        if has_any {
            self.emit_event(BackendEvent::Notice(
                "Đã tìm thấy tai nghe đã ghép.".to_string(),
            ));
        }
        Ok(())
    }

    pub async fn scan(&self) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if self.closing.load(Ordering::SeqCst) {
            return Ok(());
        }
        if !self.is_connected() {
            self.set_phase(ConnectionPhase::Scanning);
        }
        self.emit_event(BackendEvent::Notice(
            "Đang quét Bluetooth trong 5 giây…".to_string(),
        ));
        let adapter = self.get_adapter().await?;
        let mut found = self.collect_known_devices(&adapter).await;
        adapter
            .start_scan(ScanFilter::default())
            .await
            .map_err(|e| BackendError::Connection(format!("Không quét được Bluetooth: {e}")))?;
        for _ in 0..20 {
            if self.closing.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        let _ = adapter.stop_scan().await;
        for (k, v) in self.collect_known_devices(&adapter).await {
            found.insert(k, v);
        }
        *lock(&self.devices) = found.clone();
        let count = found.len();
        let list: Vec<(String, String)> = found.into_iter().collect();
        self.emit_event(BackendEvent::Devices(list));
        if !self.is_connected() {
            self.set_phase(if count > 0 {
                ConnectionPhase::PairedIdle
            } else {
                ConnectionPhase::Disconnected
            });
        }
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

    pub async fn auto_connect(self: &Arc<Self>, preferred: &str) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        if self.is_connected() || self.closing.load(Ordering::SeqCst) {
            return Ok(());
        }
        self.set_connection_status(ConnectionStatus::Connecting);
        self.set_phase(ConnectionPhase::Scanning);
        self.emit_event(BackendEvent::Notice(
            "Đang tự tìm và kết nối EarFun Air Pro 4…".into(),
        ));
        let result = async {
            let adapter = self.get_adapter().await?;
            let mut found = self.collect_known_devices(&adapter).await;
            let mut target = select_auto_device(&found, preferred);
            if target.is_none() {
                adapter
                    .start_scan(ScanFilter::default())
                    .await
                    .map_err(|e| BackendError::Connection(format!("Không quét được Bluetooth: {e}")))?;
                for _ in 0..8 {
                    if self.closing.load(Ordering::SeqCst) {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    found.extend(self.collect_known_devices(&adapter).await);
                    target = select_auto_device(&found, preferred);
                    if (!preferred.is_empty() && target.is_some())
                        || self.closing.load(Ordering::SeqCst)
                    {
                        break;
                    }
                }
                let _ = adapter.stop_scan().await;
            }
            *lock(&self.devices) = found.clone();
            self.emit_event(BackendEvent::Devices(found.into_iter().collect()));
            let address = target.ok_or_else(|| {
                BackendError::Connection(
                    "Đang chờ tai nghe. Lấy tai nghe ra khỏi hộp và bật Bluetooth. Nếu có nhiều tai nghe, chọn thiết bị một lần.".into(),
                )
            })?;
            self.connect_internal(&address, true).await
        }
        .await;
        if result.is_err() {
            self.disconnect_internal().await;
        }
        result
    }

    pub async fn connect(self: &Arc<Self>, address: &str) -> Result<(), BackendError> {
        let _guard = self.op_lock.lock().await;
        self.connect_internal(address, true).await
    }

    pub(crate) async fn connect_internal(
        self: &Arc<Self>,
        address: &str,
        full: bool,
    ) -> Result<(), BackendError> {
        let has_existing = {
            let slot = lock(&self.client);
            self.session_epoch.fetch_add(1, Ordering::SeqCst);
            slot.is_some()
        };
        if has_existing {
            self.disconnect_internal().await;
        }
        self.set_connection_status(ConnectionStatus::Connecting);
        self.set_phase(ConnectionPhase::OpeningBle);
        lock(&self.state).clone_from(&DeviceState::default());

        let result: Result<(), BackendError> = tokio::time::timeout(Duration::from_secs(90), async {
            let adapter = self.get_adapter().await?;
            let mut peripheral = lock(&self.peripherals).get(address).cloned();
            if peripheral.is_none() {
                self.collect_known_devices(&adapter).await;
                peripheral = lock(&self.peripherals).get(address).cloned();
            }
            let peripheral = peripheral.ok_or_else(|| {
                BackendError::Connection(
                    "Chưa thấy tai nghe. Lấy tai nghe ra khỏi hộp; ứng dụng sẽ tự thử lại.".into(),
                )
            })?;

            for attempt in 0..2 {
                if attempt > 0 {
                    let old = {
                        let mut slot = lock(&self.client);
                        self.session_epoch.fetch_add(1, Ordering::SeqCst);
                        slot.take()
                    };
                    if let Some(old) = old {
                        old.close_session().await;
                    }
                    self.set_connection_status(ConnectionStatus::Connecting);
                    self.set_phase(ConnectionPhase::OpeningBle);
                }
                if self.closing.load(Ordering::SeqCst) {
                    return Err(BackendError::Connection("Đã dừng kết nối.".into()));
                }

                // Dual-mode earbuds can be Connected on BR/EDR audio with no LE bearer yet.
                let opened =
                    tokio::time::timeout(Duration::from_secs(30), peripheral.connect()).await;
                match opened {
                    Ok(Ok(())) => {}
                    other => {
                        let detail = match other {
                            Ok(Err(e)) => e.to_string(),
                            _ => "Kết nối Bluetooth quá thời gian chờ".into(),
                        };
                        self.record("!", &format!("Mở kênh BLE: {detail}"));
                    }
                }

                self.set_phase(ConnectionPhase::ResolvingGatt);
                let mut discovered = false;
                for _ in 0..6 {
                    if self.closing.load(Ordering::SeqCst) {
                        return Err(BackendError::Connection("Đã dừng kết nối.".into()));
                    }
                    match tokio::time::timeout(
                        Duration::from_secs(8),
                        peripheral.discover_services(),
                    )
                    .await
                    {
                        Ok(Ok(()))
                            if peripheral.characteristics().iter().any(|c| c.uuid == TX_UUID)
                                && peripheral.characteristics().iter().any(|c| c.uuid == RX_UUID) =>
                        {
                            discovered = true;
                            break;
                        }
                        _ => tokio::time::sleep(Duration::from_millis(500)).await,
                    }
                }
                if !discovered {
                    if attempt == 0 {
                        self.record("!", "Chưa khám phá được GATT; thử mở lại kênh BLE.");
                        continue;
                    }
                    return Err(BackendError::Connection(
                        "Không khám phá được dịch vụ BLE TW600 (service discovery timed out). Lấy tai nghe ra khỏi hộp, đóng EarFun Audio trên điện thoại và thử lại. Nếu vừa ghép lại trên Windows, cần đồng bộ cả khóa Bluetooth LE giữa hai hệ điều hành.".into(),
                    ));
                }

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

                let mut stream = peripheral
                    .notifications()
                    .await
                    .map_err(|e| BackendError::Connection(e.to_string()))?;
                if let Err(e) =
                    tokio::time::timeout(Duration::from_secs(8), peripheral.subscribe(&rx_char))
                        .await
                        .map_err(|_| {
                            BackendError::Connection("Đăng ký thông báo BLE quá thời gian chờ.".into())
                        })?
                {
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

                let connected_flag = Arc::new(AtomicBool::new(true));
                let session_closed = Arc::new(AtomicBool::new(false));
                let flag_for_notify = connected_flag.clone();
                let closed_for_notify = session_closed.clone();
                let weak_core = Arc::downgrade(self);

                let session_id = {
                    let mut slot = lock(&self.client);
                    let next_epoch = self.session_epoch.fetch_add(1, Ordering::SeqCst) + 1;
                    let notify_task = tokio::spawn(async move {
                        while let Some(notification) = stream.next().await {
                            if notification.uuid == RX_UUID {
                                if let Some(core) = weak_core.upgrade() {
                                    core.notify(&notification.value);
                                } else {
                                    return;
                                }
                            }
                        }
                        // Stream terminated unexpectedly without intentional `close_session`
                        if !closed_for_notify.load(Ordering::SeqCst) {
                            flag_for_notify.store(false, Ordering::SeqCst);
                            if let Some(core) = weak_core.upgrade() {
                                if !core.closing.load(Ordering::SeqCst)
                                    && core.session_epoch.load(Ordering::SeqCst) == next_epoch
                                {
                                    core.record("!", "Luồng thông báo BLE đã đóng.");
                                    core.disconnect_for_session(next_epoch).await;
                                }
                            }
                        }
                    });

                    let client_arc = Arc::new(BtleplugClient {
                        peripheral: peripheral.clone(),
                        tx_char,
                        rx_char,
                        connected_flag,
                        session_closed,
                        notify_abort: Mutex::new(Some(notify_task.abort_handle())),
                    });
                    *slot = Some(client_arc);
                    next_epoch
                };

                self.set_phase(ConnectionPhase::InitializingProtocol);
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
                        *lock(&self.address) = Some(address.to_string());
                        self.emit_event(BackendEvent::ConnectedAddress(address.to_string()));
                        if full {
                            self.query_internal(true).await?;
                            self.set_phase(ConnectionPhase::Ready);
                        }
                        self.set_connection_status(ConnectionStatus::Connected);

                        if full {
                            self.start_poll_task(session_id);
                        }
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
        })
        .await
        .unwrap_or_else(|_| {
            Err(BackendError::Connection(
                "Kết nối BLE quá thời gian chờ; ứng dụng sẽ thử lại. Kiểm tra tai nghe và khóa ghép đôi dual boot.".into(),
            ))
        });

        if result.is_err() {
            self.disconnect_internal().await;
        }
        result
    }
}
