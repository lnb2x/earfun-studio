//! BLE GATT transport, framed wire request/reply matching, and streamed notification collection.

use super::{
    lock, BackendCore, BackendError, BackendEvent, ConnectionPhase, ConnectionStatus,
    PendingRequest,
};
use crate::protocol::{self as p, DeviceState, PhoneEntry, Reply, StateUpdate};
use btleplug::api::{Characteristic, Peripheral as _, WriteType};
use btleplug::platform::Peripheral;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait GattClient: Send + Sync {
    fn is_connected(&self) -> bool;
    fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>>;
    fn close_session(&self) -> BoxFuture<'_, ()>;
}

pub struct BtleplugClient {
    pub(crate) peripheral: Peripheral,
    pub(crate) tx_char: Characteristic,
    pub(crate) rx_char: Characteristic,
    pub(crate) connected_flag: Arc<AtomicBool>,
    pub(crate) session_closed: Arc<AtomicBool>,
    pub(crate) notify_abort: Mutex<Option<tokio::task::AbortHandle>>,
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
            self.session_closed.store(true, Ordering::SeqCst);
            self.connected_flag.store(false, Ordering::SeqCst);
            let abort_opt = lock(&self.notify_abort).take();
            if let Some(handle) = abort_opt {
                if Some(handle.id()) != tokio::task::try_id() {
                    handle.abort();
                }
            }
            let _ = tokio::time::timeout(
                Duration::from_secs(2),
                self.peripheral.unsubscribe(&self.rx_char),
            )
            .await;
            #[cfg(target_os = "windows")]
            {
                let _ = tokio::time::timeout(Duration::from_secs(2), self.peripheral.disconnect())
                    .await;
            }
        })
    }
}

impl BackendCore {
    pub fn notify(&self, data: &[u8]) {
        self.record("<<", &p::hex_encode_spaced(data));
        let Some(reply) = p::decode(data) else {
            return;
        };

        // Route streamed paired-device replies (`cmd 44` / `0x032c`) to active collector if present
        if reply.vendor == 10 && reply.command == 0x032c {
            if let Some(tx) = lock(&self.phone_stream).as_ref() {
                let _ = tx.send(reply);
                return;
            }
        }

        // Route solicited reply exclusively to its awaiting `request()` caller
        let matched = {
            let mut pending = lock(&self.pending);
            if pending
                .as_ref()
                .is_some_and(|req| req.vendor == reply.vendor && req.command == reply.command)
            {
                pending.take()
            } else {
                None
            }
        };
        if let Some(req) = matched {
            let _ = req.tx.send(reply);
            return;
        }

        // Apply unsolicited push notifications only when session handshake is complete
        if self.is_connected() && reply.status == 0 {
            self.apply(&reply);
        }
    }

    pub fn apply(&self, reply: &Reply) -> StateUpdate {
        let update = p::state(reply);
        if let Some(phone) = &update.phone {
            let list = {
                let mut phones = lock(&self.phones);
                if phone.num == 0 {
                    phones.clear();
                }
                phones.insert(phone.num, phone.clone());
                phones.values().cloned().collect::<Vec<_>>()
            };
            self.emit_event(BackendEvent::Phones(list));
        }
        let snapshot = {
            let mut st = lock(&self.state);
            st.merge(&update);
            st.clone()
        };
        self.emit_event(BackendEvent::State(snapshot));
        update
    }

    pub fn lost(&self) {
        lock(&self.state).clone_from(&DeviceState::default());
        lock(&self.phones).clear();
        let _ = lock(&self.pending).take();
        let _ = lock(&self.phone_stream).take();
        self.emit_event(BackendEvent::State(DeviceState::default()));
        self.emit_event(BackendEvent::Phones(Vec::new()));
        let idle_phase = if lock(&self.devices).is_empty() {
            ConnectionPhase::Disconnected
        } else {
            ConnectionPhase::PairedIdle
        };
        self.set_phase(idle_phase);
        *lock(&self.connection_status) = ConnectionStatus::Disconnected;
        self.emit_event(BackendEvent::Connection(ConnectionStatus::Disconnected));
    }

    pub async fn write(&self, packet: &[u8]) -> Result<(), BackendError> {
        let client = lock(&self.client).clone();
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
            Err(_) => Err(BackendError::Runtime(
                "Ghi GATT quá thời gian chờ".to_string(),
            )),
        }
    }

    pub async fn request(&self, packet: &[u8]) -> Result<Reply, BackendError> {
        if packet.len() < 4 {
            return Err(BackendError::Runtime(
                "Gói truy vấn Bluetooth không hợp lệ (< 4 byte).".to_string(),
            ));
        }
        let _guard = self.wire_lock.lock().await;
        let vendor = u16::from_be_bytes([packet[0], packet[1]]);
        let command = u16::from_be_bytes([packet[2], packet[3]]);
        let (tx, rx) = oneshot::channel();
        {
            *lock(&self.pending) = Some(PendingRequest {
                vendor,
                command,
                tx,
            });
        }
        let timeout_dur = *lock(&self.request_timeout);
        if let Err(e) = self.write(packet).await {
            *lock(&self.pending) = None;
            self.disconnect_internal().await;
            return Err(e);
        }

        match tokio::time::timeout(timeout_dur, rx).await {
            Ok(Ok(reply)) => {
                *lock(&self.pending) = None;
                if reply.status != 0 {
                    return Err(BackendError::CommandRejected(format!(
                        "Tai nghe từ chối lệnh {:04X} (mã {}).",
                        command, reply.status
                    )));
                }
                Ok(reply)
            }
            _ => {
                *lock(&self.pending) = None;
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
        let sleep_dur = (*lock(&self.step_sleep)).min(Duration::from_millis(80));
        tokio::time::sleep(sleep_dur).await;
        Ok(())
    }

    /// Collect all streamed paired-device packets (`cmd 44` / `0x032c`) within a bounded settle window.
    pub(crate) async fn phones_internal(&self) -> Result<(), BackendError> {
        let _guard = self.wire_lock.lock().await;
        let (tx, mut rx) = mpsc::unbounded_channel();
        *lock(&self.phone_stream) = Some(tx);

        let timeout_dur = *lock(&self.request_timeout);
        if let Err(e) = self.write(&p::request(44, &[])).await {
            *lock(&self.phone_stream) = None;
            self.disconnect_internal().await;
            return Err(e);
        }

        let first = match tokio::time::timeout(timeout_dur, rx.recv()).await {
            Ok(Some(reply)) => reply,
            _ => {
                *lock(&self.phone_stream) = None;
                self.disconnect_internal().await;
                return Err(BackendError::Runtime(
                    "Bluetooth không phản hồi khi đọc danh sách thiết bị.".to_string(),
                ));
            }
        };

        if first.status != 0 {
            *lock(&self.phone_stream) = None;
            let msg = format!("Tai nghe từ chối lệnh 032C (mã {}).", first.status);
            self.record("?", &msg);
            return Ok(());
        }

        let mut collected: BTreeMap<u8, PhoneEntry> = BTreeMap::new();
        if let Some(ph) = p::state(&first).phone {
            collected.insert(ph.num, ph);
        }

        let settle =
            (*lock(&self.step_sleep)).clamp(Duration::from_millis(45), Duration::from_millis(260));
        while let Ok(Some(next_reply)) = tokio::time::timeout(settle, rx.recv()).await {
            if next_reply.status == 0 {
                if let Some(ph) = p::state(&next_reply).phone {
                    collected.insert(ph.num, ph);
                }
            }
        }
        *lock(&self.phone_stream) = None;

        let list: Vec<PhoneEntry> = collected.values().cloned().collect();
        *lock(&self.phones) = collected;
        self.emit_event(BackendEvent::Phones(list));
        Ok(())
    }
}
