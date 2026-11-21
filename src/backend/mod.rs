//! Asynchronous BLE backend owning device sessions and serializing TW600 operations.
//!
//! Organized into focused submodules:
//! - `transport`: `GattClient` trait, `BtleplugClient`, framed GATT I/O, and streamed packet collection.
//! - `commands`: Verified TW600 setters, multi-field readback loops, EQ/gesture writes, and restart plans.
//! - `lifecycle`: Adapter validation, BLE discovery, connection state machine, polling, and cleanup.

mod commands;
mod lifecycle;
mod transport;

pub use transport::{BoxFuture, BtleplugClient, GattClient};

use crate::protocol::{DeviceState, PhoneEntry, Reply};
use btleplug::platform::{Adapter, Peripheral};
use chrono::Local;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex as AsyncMutex};

const MAX_BACKEND_LOGS: usize = 2000;

#[inline]
pub(crate) fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
}

/// Granular Bluetooth & TW600 protocol lifecycle stage for accurate UI communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionPhase {
    #[default]
    Disconnected,
    PairedIdle,
    Scanning,
    OpeningBle,
    ResolvingGatt,
    InitializingProtocol,
    Ready,
    Reconnecting {
        attempt: u8,
        max_attempts: u8,
    },
}

impl ConnectionPhase {
    pub fn short_label(&self) -> String {
        match self {
            ConnectionPhase::Disconnected => "Chưa kết nối".to_string(),
            ConnectionPhase::PairedIdle => "Đã ghép đôi · Chờ kết nối".to_string(),
            ConnectionPhase::Scanning => "Đang quét BLE…".to_string(),
            ConnectionPhase::OpeningBle => "Đang mở kênh BLE…".to_string(),
            ConnectionPhase::ResolvingGatt => "Đang mở dịch vụ GATT…".to_string(),
            ConnectionPhase::InitializingProtocol => "Đang xác thực TW600…".to_string(),
            ConnectionPhase::Ready => "Sẵn sàng điều khiển".to_string(),
            ConnectionPhase::Reconnecting {
                attempt,
                max_attempts,
            } => format!("Đang nối lại ({}/{})…", attempt, max_attempts),
        }
    }
}

#[derive(Debug, Clone)]
pub enum BackendEvent {
    Devices(Vec<(String, String)>),
    EqSent { generation: u64, levels: Vec<i32> },
    Connection(ConnectionStatus),
    Phase(ConnectionPhase),
    ConnectedAddress(String),
    State(DeviceState),
    Phones(Vec<PhoneEntry>),
    Volume { percent: u8, label: String },
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

pub(crate) struct PendingRequest {
    pub(crate) vendor: u16,
    pub(crate) command: u16,
    pub(crate) tx: oneshot::Sender<Reply>,
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
    pub(crate) pending: Mutex<Option<PendingRequest>>,
    pub(crate) phone_stream: Mutex<Option<mpsc::UnboundedSender<Reply>>>,
    pub client: Mutex<Option<Arc<dyn GattClient>>>,
    pub devices: Mutex<BTreeMap<String, String>>,
    pub(crate) peripherals: Mutex<BTreeMap<String, Peripheral>>,
    pub(crate) adapter: AsyncMutex<Option<Adapter>>,
    pub op_lock: AsyncMutex<()>,
    pub wire_lock: AsyncMutex<()>,
    pub finding: AtomicBool,
    pub closing: AtomicBool,
    pub(crate) session_epoch: AtomicU64,
    pub address: Mutex<Option<String>>,
    pub request_timeout: Mutex<Duration>,
    pub step_sleep: Mutex<Duration>,
    pub find_duration: Mutex<Duration>,
    pub restart_deadline: Mutex<Duration>,
    pub(crate) connection_status: Mutex<ConnectionStatus>,
    pub(crate) phase: Mutex<ConnectionPhase>,
    pub(crate) finder_abort: Mutex<Option<tokio::task::AbortHandle>>,
    pub(crate) poll_abort: Mutex<Option<tokio::task::AbortHandle>>,
    pub(crate) reconnect_hook: Mutex<Option<ReconnectHook>>,
    pub(crate) query_hook: Mutex<Option<QueryHook>>,
}

impl BackendCore {
    pub fn new(emit: Arc<dyn Fn(BackendEvent) + Send + Sync>) -> Arc<Self> {
        Arc::new(Self {
            emit,
            log: Mutex::new(VecDeque::with_capacity(MAX_BACKEND_LOGS)),
            state: Mutex::new(DeviceState::default()),
            phones: Mutex::new(BTreeMap::new()),
            pending: Mutex::new(None),
            phone_stream: Mutex::new(None),
            client: Mutex::new(None),
            devices: Mutex::new(BTreeMap::new()),
            peripherals: Mutex::new(BTreeMap::new()),
            adapter: AsyncMutex::new(None),
            op_lock: AsyncMutex::new(()),
            wire_lock: AsyncMutex::new(()),
            finding: AtomicBool::new(false),
            closing: AtomicBool::new(false),
            session_epoch: AtomicU64::new(0),
            address: Mutex::new(None),
            request_timeout: Mutex::new(Duration::from_secs(4)),
            step_sleep: Mutex::new(Duration::from_secs(1)),
            find_duration: Mutex::new(Duration::from_secs(10)),
            restart_deadline: Mutex::new(Duration::from_secs(50)),
            connection_status: Mutex::new(ConnectionStatus::Disconnected),
            phase: Mutex::new(ConnectionPhase::Disconnected),
            finder_abort: Mutex::new(None),
            poll_abort: Mutex::new(None),
            reconnect_hook: Mutex::new(None),
            query_hook: Mutex::new(None),
        })
    }

    pub fn set_reconnect_hook(&self, hook: ReconnectHook) {
        *lock(&self.reconnect_hook) = Some(hook);
    }

    pub fn set_query_hook(&self, hook: QueryHook) {
        *lock(&self.query_hook) = Some(hook);
    }

    pub fn emit_event(&self, ev: BackendEvent) {
        (self.emit)(ev);
    }

    pub(crate) fn set_phase(&self, new_phase: ConnectionPhase) {
        let mut ph = lock(&self.phase);
        if *ph != new_phase {
            *ph = new_phase;
            drop(ph);
            self.emit_event(BackendEvent::Phase(new_phase));
        }
    }

    pub(crate) fn set_connection_status(&self, status: ConnectionStatus) {
        let mut cur = lock(&self.connection_status);
        if *cur != status {
            *cur = status.clone();
            drop(cur);
            self.emit_event(BackendEvent::Connection(status));
        }
    }

    pub fn record(&self, direction: &str, text: &str) {
        let ts = Local::now().format("%H:%M:%S");
        let line = format!("{} {} {}", ts, direction, text);
        {
            let mut log = lock(&self.log);
            if log.len() >= MAX_BACKEND_LOGS {
                log.pop_front();
            }
            log.push_back(line.clone());
        }
        self.emit_event(BackendEvent::Log(line));
    }

    pub fn is_connected(&self) -> bool {
        let has_client = lock(&self.client)
            .as_ref()
            .map(|c| c.is_connected())
            .unwrap_or(false);
        let has_version = lock(&self.state).version.is_some();
        has_client && has_version
    }

    pub fn supported(name: &str, uuids: &[String]) -> bool {
        name.eq_ignore_ascii_case("earfun air pro 4")
            || name.starts_with("TW600-")
            || uuids
                .iter()
                .any(|u| u.eq_ignore_ascii_case(crate::protocol::SERVICE_STR))
    }
}

pub struct BackendHandle {
    pub core: Arc<BackendCore>,
    runtime: tokio::runtime::Runtime,
    busy_tx: mpsc::UnboundedSender<i32>,
    wakeup: Option<Arc<dyn Fn() + Send + Sync>>,
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
            wakeup: None,
        }
    }

    pub fn set_wakeup(&mut self, wakeup: Arc<dyn Fn() + Send + Sync>) {
        self.wakeup = Some(wakeup);
    }

    pub fn spawn_task<F>(&self, fut: F)
    where
        F: Future<Output = Result<(), BackendError>> + Send + 'static,
    {
        struct BusyGuard {
            busy_tx: mpsc::UnboundedSender<i32>,
            wakeup: Option<Arc<dyn Fn() + Send + Sync>>,
        }

        impl Drop for BusyGuard {
            fn drop(&mut self) {
                let _ = self.busy_tx.send(-1);
                if let Some(w) = &self.wakeup {
                    w();
                }
            }
        }

        let _ = self.busy_tx.send(1);
        if let Some(w) = &self.wakeup {
            w();
        }
        let guard = BusyGuard {
            busy_tx: self.busy_tx.clone(),
            wakeup: self.wakeup.clone(),
        };
        let core = self.core.clone();
        self.runtime.spawn(async move {
            let _guard = guard;
            if let Err(e) = fut.await {
                let msg = e.to_string();
                core.record("!", &msg);
                core.emit_event(BackendEvent::Error(msg));
            }
        });
    }

    /// Run OS audio sink volume read/write off the UI thread.
    pub fn spawn_volume_task(&self, mac: String, percent: Option<u8>) {
        let core = self.core.clone();
        self.spawn_task(async move {
            let result = crate::audio::volume_async(&mac, percent)
                .await
                .map_err(BackendError::Runtime)?;
            let mut parsed = percent.unwrap_or(50);
            if let Some(second) = result.split_whitespace().nth(1) {
                if let Ok(frac) = second.parse::<f64>() {
                    parsed = (frac * 100.0).round().clamp(0.0, 100.0) as u8;
                }
            }
            let label = format!("{}% • Đầu ra EarFun", parsed);
            core.emit_event(BackendEvent::Volume {
                percent: parsed,
                label,
            });
            core.emit_event(BackendEvent::Notice(result));
            Ok(())
        });
    }

    pub fn shutdown_blocking(&self) {
        self.core.closing.store(true, Ordering::SeqCst);
        let core = self.core.clone();
        self.runtime.block_on(async move {
            if tokio::time::timeout(Duration::from_secs(3), core.disconnect())
                .await
                .is_err()
            {
                let _ =
                    tokio::time::timeout(Duration::from_secs(2), core.disconnect_internal()).await;
            }
        });
    }
}

/// A remembered device is never silently replaced by another nearby headset.
pub fn select_auto_device(devices: &BTreeMap<String, String>, preferred: &str) -> Option<String> {
    if !preferred.is_empty() {
        return devices
            .keys()
            .find(|mac| mac.eq_ignore_ascii_case(preferred))
            .cloned();
    }
    if devices.len() == 1 {
        devices.keys().next().cloned()
    } else {
        None
    }
}

#[cfg(test)]
mod connection_tests {
    use super::*;
    use crate::protocol as p;

    struct SlowClose;
    impl GattClient for SlowClose {
        fn is_connected(&self) -> bool {
            true
        }
        fn write_tx(&self, _: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
            Box::pin(async { Ok(()) })
        }
        fn close_session(&self) -> BoxFuture<'_, ()> {
            Box::pin(async { tokio::time::sleep(Duration::from_millis(5)).await })
        }
    }

    #[tokio::test]
    async fn timeout_in_polling_task_completes_cleanup_and_resets_ui() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let core = BackendCore::new(Arc::new(move |event| lock(&captured).push(event)));
        *lock(&core.client) = Some(Arc::new(SlowClose));
        *lock(&core.connection_status) = ConnectionStatus::Connected;
        lock(&core.state).version = Some("TW600-test".into());
        *lock(&core.request_timeout) = Duration::from_millis(5);
        let (start, ready) = oneshot::channel::<()>();
        let polling_core = core.clone();
        let task = tokio::spawn(async move {
            ready.await.unwrap();
            assert!(polling_core.request(&p::request(6, &[])).await.is_err());
        });
        *lock(&core.poll_abort) = Some(task.abort_handle());
        start.send(()).unwrap();
        task.await.expect("Polling must not cancel its own cleanup");
        assert!(!core.is_connected());
        assert!(lock(&events).iter().any(|event| matches!(
            event,
            BackendEvent::Connection(ConnectionStatus::Disconnected)
        )));
    }

    #[tokio::test]
    async fn stale_session_disconnect_does_not_clobber_reconnected_session() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let core = BackendCore::new(Arc::new(move |event| lock(&captured).push(event)));

        // Simulate Session 1 having been superseded by reconnected Session 2 (epoch = 2)
        core.session_epoch.store(2, Ordering::SeqCst);
        *lock(&core.client) = Some(Arc::new(SlowClose));
        *lock(&core.connection_status) = ConnectionStatus::Connected;
        lock(&core.state).version = Some("TW600-reconnected".into());
        let (tx, _rx) = oneshot::channel();
        *lock(&core.pending) = Some(PendingRequest {
            vendor: 10,
            command: 0x0206,
            tx,
        });

        // Late notification-stream end from stale Session 1 must be ignored completely
        core.disconnect_for_session(1).await;
        assert!(core.is_connected());
        assert_eq!(
            lock(&core.state).version.as_deref(),
            Some("TW600-reconnected")
        );
        assert!(lock(&core.pending).is_some());
        assert!(lock(&events).is_empty());

        // Matching epoch cleanup tears down Session 2 as expected
        core.disconnect_for_session(2).await;
        assert!(!core.is_connected());
        assert!(lock(&core.pending).is_none());
        assert!(lock(&events).iter().any(|event| matches!(
            event,
            BackendEvent::Connection(ConnectionStatus::Disconnected)
        )));
    }

    #[tokio::test(start_paused = true)]
    async fn poll_task_queries_every_interval() {
        use std::sync::Weak;

        struct PollFakeClient {
            core: Weak<BackendCore>,
            writes: Mutex<Vec<Vec<u8>>>,
        }

        impl GattClient for PollFakeClient {
            fn is_connected(&self) -> bool {
                true
            }
            fn write_tx(&self, packet: Vec<u8>) -> BoxFuture<'_, Result<(), String>> {
                Box::pin(async move {
                    lock(&self.writes).push(packet.clone());
                    let cmd = packet[3];
                    if let Some(c) = self.core.upgrade() {
                        let raw = p::packet(10, 0x8300 | (cmd as u16), &[0, 50]).unwrap();
                        c.notify(&raw);
                    }
                    Ok(())
                })
            }
            fn close_session(&self) -> BoxFuture<'_, ()> {
                Box::pin(async {})
            }
        }

        let core = BackendCore::new(Arc::new(|_| {}));
        lock(&core.state).version = Some("TW600-test".to_string());
        let client = Arc::new(PollFakeClient {
            core: Arc::downgrade(&core),
            writes: Mutex::new(Vec::new()),
        });
        *lock(&core.client) = Some(client.clone());

        let epoch = core.session_epoch.load(Ordering::SeqCst);
        core.start_poll_task(epoch);

        tokio::time::sleep(Duration::from_secs(11)).await;

        let writes = lock(&client.writes).clone();
        for &cmd in p::POLL_QUERIES {
            assert!(
                writes.contains(&p::request(cmd, &[])),
                "Expected poll query cmd {} in {:?}",
                cmd,
                writes
            );
        }
        core.disconnect().await;
    }

    #[test]
    fn panicking_task_releases_busy_counter() {
        let (busy_tx, mut busy_rx) = mpsc::unbounded_channel();
        let wakeups = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let wakeups_clone = wakeups.clone();
        let mut handle = BackendHandle::new(Arc::new(|_| {}), busy_tx);
        handle.set_wakeup(Arc::new(move || {
            wakeups_clone.fetch_add(1, Ordering::SeqCst);
        }));

        handle.spawn_task(async {
            panic!("simulated backend panic");
        });

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_time()
            .build()
            .unwrap();
        let first = rt
            .block_on(async { tokio::time::timeout(Duration::from_secs(2), busy_rx.recv()).await });
        assert_eq!(first.unwrap(), Some(1));
        let second = rt
            .block_on(async { tokio::time::timeout(Duration::from_secs(2), busy_rx.recv()).await });
        assert_eq!(second.unwrap(), Some(-1));
        assert!(wakeups.load(Ordering::SeqCst) >= 2);
    }
}
