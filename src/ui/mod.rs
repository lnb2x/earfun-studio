//! Cross-platform GUI for EarFun Studio (Windows & Ubuntu) built with eframe/egui.
//! Modular design system with Light/Dark/System themes, overlap-proof responsive layouts,
//! granular BLE connection lifecycle states, and event-driven zero-idle-repaint scheduling.

pub mod header;
pub mod icons;
pub mod pages;
mod shell;
pub mod theme;
pub mod widgets;

#[cfg(test)]
mod tests;

pub use icons::{build_app_icon, draw_vector_icon, IconKind};

use crate::backend::{BackendEvent, BackendHandle, ConnectionPhase, ConnectionStatus};
use crate::presets::{self, PresetOption};
use crate::protocol::{self as p, DeviceState, PhoneEntry};
use crate::settings::{Settings, ThemeMode};
use crate::ui::theme::*;
use crate::workflows::Plan;
use eframe::egui::{self, Pos2, Rect, Vec2};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const MAX_UI_LOGS: usize = 2000;

/// Semantic tone for footer status notices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoticeTone {
    Info,
    Success,
    Warning,
    Error,
}

/// Primary navigation sections of EarFun Studio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Overview,
    Equalizer,
    Gestures,
    Multipoint,
    Earbuds,
    Application,
}

impl Page {
    pub const ALL: [Page; 6] = [
        Page::Overview,
        Page::Equalizer,
        Page::Gestures,
        Page::Multipoint,
        Page::Earbuds,
        Page::Application,
    ];

    pub fn title(&self) -> &'static str {
        match self {
            Page::Overview => "Tổng quan",
            Page::Equalizer => "Equalizer",
            Page::Gestures => "Cảm ứng",
            Page::Multipoint => "Hai thiết bị",
            Page::Earbuds => "Tai nghe",
            Page::Application => "Ứng dụng",
        }
    }

    pub fn subtitle(&self) -> &'static str {
        match self {
            Page::Overview => "Kiểm soát chống ồn, pin và chế độ nghe tức thì",
            Page::Equalizer => "Tinh chỉnh trực quan 10 dải tần số theo phần cứng TW600",
            Page::Gestures => "Tùy biến thao tác chạm và khóa cảm ứng từng bên tai",
            Page::Multipoint => "Quản lý kết nối đồng thời hai thiết bị Bluetooth",
            Page::Earbuds => "Âm thanh, đàm thoại, codec Bluetooth và tìm tai nghe",
            Page::Application => "Giao diện, tự kết nối, phím tắt và nhật ký chẩn đoán BLE",
        }
    }

    pub fn icon(&self) -> IconKind {
        match self {
            Page::Overview => IconKind::Overview,
            Page::Equalizer => IconKind::Eq,
            Page::Gestures => IconKind::Touch,
            Page::Multipoint => IconKind::Devices,
            Page::Earbuds => IconKind::Settings,
            Page::Application => IconKind::App,
        }
    }
}

/// Filter tabs for the Equalizer preset picker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetCategory {
    All,
    EarFun,
    Fernanda,
    Custom,
}

pub(crate) struct ConfirmModal {
    pub(crate) message: String,
    pub(crate) just_opened: bool,
    pub(crate) action: Box<dyn FnOnce(&mut StudioApp) + Send>,
}

/// Main application state for EarFun Studio.
pub struct StudioApp {
    pub smoke: bool,
    pub smoke_step: usize,
    pub smoke_timer: Instant,
    pub(crate) manual_query_pending: bool,
    pub(crate) settings: Settings,
    pub(crate) backend: BackendHandle,
    event_rx: mpsc::UnboundedReceiver<BackendEvent>,
    busy_rx: mpsc::UnboundedReceiver<i32>,
    pub(crate) busy: i32,
    pub page: Page,
    pub(crate) connection: ConnectionStatus,
    pub(crate) phase: ConnectionPhase,
    pub(crate) user_disconnected: bool,
    pub(crate) state: DeviceState,
    pub(crate) phones: Vec<PhoneEntry>,
    pub(crate) devices: Vec<(String, String)>,
    pub(crate) selected_device_idx: usize,
    pub(crate) address: String,
    pub(crate) notice: String,
    pub(crate) notice_tone: NoticeTone,
    pub(crate) notice_is_error: bool,
    pub(crate) logs: VecDeque<String>,
    pub(crate) log_filter: String,
    pub(crate) connection_generation: u64,

    // Balance & Volume
    pub(crate) draft_balance: u8,
    pub(crate) balance_dirty: bool,
    pub(crate) volume_value: u8,
    pub(crate) volume_label: String,

    // Gestures state
    pub(crate) draft_keys: BTreeMap<String, usize>,

    // Equalizer state
    pub(crate) preset_options: Vec<PresetOption>,
    pub(crate) selected_preset_idx: usize,
    pub(crate) eq_category: PresetCategory,
    pub(crate) preset_search: String,
    pub(crate) eq_levels: Vec<i32>,
    pub(crate) preset_name_input: String,
    pub(crate) eq_sent_levels: Option<Vec<i32>>,
    pub(crate) eq_sending_levels: Option<Vec<i32>>,

    // Find My Earbuds countdown
    pub(crate) find_until: Option<Instant>,

    // Reconnect timer
    pub(crate) last_reconnect_check: Instant,

    // Confirmation dialog
    pub(crate) confirm_modal: Option<ConfirmModal>,
    shutting_down: bool,
}

impl StudioApp {
    /// Initialize `StudioApp` and optionally configure `eframe` fonts, theme, and event wakeup.
    pub fn new(cc: Option<&eframe::CreationContext<'_>>, smoke: bool, connect_last: bool) -> Self {
        let settings = Settings::new(None);
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (busy_tx, busy_rx) = mpsc::unbounded_channel();

        let repaint_ctx = cc.map(|c| c.egui_ctx.clone());
        let repaint_for_emit = repaint_ctx.clone();

        let emit = Arc::new(move |ev: BackendEvent| {
            let _ = event_tx.send(ev);
            if let Some(ctx) = &repaint_for_emit {
                ctx.request_repaint();
            }
        });
        let mut backend = BackendHandle::new(emit, busy_tx);

        if let Some(cc) = cc {
            configure_fonts(&cc.egui_ctx);
            apply_theme(&cc.egui_ctx, settings.data.theme);
            let ctx_for_busy = cc.egui_ctx.clone();
            backend.set_wakeup(Arc::new(move || {
                ctx_for_busy.request_repaint();
            }));
        }

        let preset_options = presets::options(&settings.data.presets);
        let eq_levels = settings.data.eq.clone();
        let address = settings.data.last_mac.clone();
        let startup_connect = connect_last || settings.data.auto_connect;

        let (initial_notice, initial_tone) = if let Some(warn) = settings.load_warning.clone() {
            (warn, NoticeTone::Warning)
        } else if !smoke && startup_connect {
            (
                "Đang chuẩn bị kết nối tai nghe…".to_string(),
                NoticeTone::Info,
            )
        } else {
            (
                "Sẵn sàng • Bấm Quét BLE hoặc chọn tai nghe để bắt đầu.".to_string(),
                NoticeTone::Info,
            )
        };

        let mut draft_keys = BTreeMap::new();
        for side in ["L", "R"] {
            for (gesture, _) in p::GESTURES {
                draft_keys.insert(format!("{}.{}", side, gesture), 0);
            }
        }

        let mut app = Self {
            smoke,
            smoke_step: 0,
            smoke_timer: Instant::now(),
            manual_query_pending: false,
            settings,
            backend,
            event_rx,
            busy_rx,
            busy: 0,
            page: Page::Overview,
            connection: ConnectionStatus::Disconnected,
            phase: ConnectionPhase::Disconnected,
            user_disconnected: false,
            state: DeviceState::default(),
            phones: Vec::new(),
            devices: Vec::new(),
            selected_device_idx: 0,
            address,
            notice: initial_notice,
            notice_tone: initial_tone,
            notice_is_error: false,
            logs: VecDeque::with_capacity(MAX_UI_LOGS),
            log_filter: String::new(),
            connection_generation: 0,
            draft_balance: 50,
            balance_dirty: false,
            volume_value: 50,
            volume_label: "— • Đầu ra EarFun".to_string(),
            draft_keys,
            preset_options,
            selected_preset_idx: 0,
            eq_category: PresetCategory::All,
            preset_search: String::new(),
            eq_levels,
            preset_name_input: String::new(),
            eq_sent_levels: None,
            eq_sending_levels: None,
            find_until: None,
            last_reconnect_check: Instant::now(),
            confirm_modal: None,
            shutting_down: false,
        };

        app.sync_eq_picker();

        if !smoke {
            let core = app.backend.core.clone();
            let preferred = app.address.clone();
            if startup_connect {
                app.backend
                    .spawn_task(async move { core.auto_connect(&preferred).await });
            } else {
                app.backend
                    .spawn_task(async move { core.load_devices().await });
            }
        }

        app
    }

    pub fn is_connected(&self) -> bool {
        self.connection == ConnectionStatus::Connected
    }

    pub fn is_enabled(&self) -> bool {
        self.is_connected() && self.busy == 0
    }

    pub fn has_unsent_eq(&self) -> bool {
        self.eq_sent_levels.as_ref() != Some(&self.eq_levels)
    }

    pub fn show_notice(&mut self, text: impl Into<String>, is_error: bool) {
        self.set_notice(
            text,
            if is_error {
                NoticeTone::Error
            } else {
                NoticeTone::Success
            },
        );
    }

    pub fn set_notice(&mut self, text: impl Into<String>, tone: NoticeTone) {
        self.notice = text.into();
        self.notice_tone = tone;
        self.notice_is_error = tone == NoticeTone::Error;
    }

    pub fn set_theme_mode(&mut self, ctx: &egui::Context, mode: ThemeMode) {
        if self.settings.data.theme != mode {
            self.settings.data.theme = mode;
            apply_theme(ctx, mode);
            self.save_settings();
        }
    }

    pub fn cycle_theme_mode(&mut self, ctx: &egui::Context) {
        let next = match self.settings.data.theme {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::System,
            ThemeMode::System => ThemeMode::Light,
        };
        self.set_theme_mode(ctx, next);
        self.set_notice(
            format!("Đã chuyển giao diện sang chế độ: {}.", next.label()),
            NoticeTone::Info,
        );
    }

    pub(crate) fn trigger_manual_query(&mut self) {
        self.manual_query_pending = true;
        let core = self.backend.core.clone();
        self.backend.spawn_task(async move { core.query().await });
    }

    pub fn save_settings(&mut self) {
        if self.smoke {
            return;
        }
        if let Err(e) = self.settings.save() {
            self.show_notice(format!("Không lưu được cấu hình: {}", e), true);
        }
    }

    pub fn apply_backend_event(&mut self, ev: BackendEvent) {
        match ev {
            BackendEvent::Devices(list) => {
                self.devices = list;
                self.selected_device_idx = self
                    .devices
                    .iter()
                    .position(|(mac, _)| mac.eq_ignore_ascii_case(&self.address))
                    .unwrap_or(0);
                if !self.is_connected() && self.connection != ConnectionStatus::Connecting {
                    self.phase = if self.devices.is_empty() {
                        ConnectionPhase::Disconnected
                    } else {
                        ConnectionPhase::PairedIdle
                    };
                }
            }
            BackendEvent::Phase(phase) => {
                self.phase = phase;
            }
            BackendEvent::Connection(status) => {
                if status == ConnectionStatus::Disconnected {
                    self.connection_generation += 1;
                    self.eq_sent_levels = None;
                    self.eq_sending_levels = None;
                    self.find_until = None;
                    self.manual_query_pending = false;
                    if matches!(
                        self.phase,
                        ConnectionPhase::Ready
                            | ConnectionPhase::OpeningBle
                            | ConnectionPhase::ResolvingGatt
                            | ConnectionPhase::InitializingProtocol
                            | ConnectionPhase::Reconnecting { .. }
                    ) {
                        self.phase = if self.devices.is_empty() {
                            ConnectionPhase::Disconnected
                        } else {
                            ConnectionPhase::PairedIdle
                        };
                    }
                } else if status == ConnectionStatus::Connected {
                    self.phase = ConnectionPhase::Ready;
                }
                self.connection = status;
                match self.connection {
                    ConnectionStatus::Connected => {
                        self.set_notice(
                            "Đã kết nối và đồng bộ EarFun Air Pro 4.",
                            NoticeTone::Success,
                        );
                    }
                    ConnectionStatus::Disconnected => {
                        self.set_notice("Chưa kết nối tai nghe.", NoticeTone::Info);
                    }
                    ConnectionStatus::Connecting => {}
                }
            }
            BackendEvent::ConnectedAddress(addr) => {
                self.address = addr.clone();
                self.settings.data.last_mac = addr;
                self.save_settings();
            }
            BackendEvent::State(new_state) => {
                self.sync_state_to_drafts(&new_state);
                self.state = new_state;
            }
            BackendEvent::Phones(phones) => {
                self.phones = phones;
            }
            BackendEvent::Volume { percent, label } => {
                self.volume_value = percent;
                self.volume_label = label;
            }
            BackendEvent::Notice(msg) => {
                if msg.starts_with("Đang phát tiếng tìm") {
                    self.find_until = Some(Instant::now() + Duration::from_secs(10));
                } else if msg.starts_with("Đã gửi dừng tiếng tìm") {
                    self.find_until = None;
                }
                if msg == "Đã đọc trạng thái trực tiếp từ tai nghe." {
                    if self.manual_query_pending {
                        self.manual_query_pending = false;
                    } else if self.busy == 0 {
                        return;
                    }
                }
                let tone = if msg.starts_with("Đang ") {
                    NoticeTone::Info
                } else {
                    NoticeTone::Success
                };
                self.set_notice(msg, tone);
            }
            BackendEvent::Error(msg) => {
                self.eq_sending_levels = None;
                self.find_until = None;
                self.manual_query_pending = false;
                let snapshot = self.state.clone();
                self.sync_state_to_drafts(&snapshot);
                self.show_notice(msg, true);
            }
            BackendEvent::EqSent { generation, levels } => {
                if generation == self.connection_generation && self.is_connected() {
                    self.eq_sending_levels = None;
                    self.eq_sent_levels = Some(levels);
                }
            }
            BackendEvent::Log(line) => {
                if self.logs.len() >= MAX_UI_LOGS {
                    self.logs.pop_front();
                }
                self.logs.push_back(line);
            }
        }
    }

    fn process_events(&mut self) {
        while let Ok(ev) = self.event_rx.try_recv() {
            self.apply_backend_event(ev);
        }
        while let Ok(delta) = self.busy_rx.try_recv() {
            self.busy = (self.busy + delta).max(0);
        }
    }

    pub(crate) fn sync_state_to_drafts(&mut self, new_state: &DeviceState) {
        let clamped_hw_bal = new_state.balance.map(|v| v.min(100));
        if !self.balance_dirty || Some(self.draft_balance) == clamped_hw_bal {
            self.balance_dirty = false;
            self.draft_balance = clamped_hw_bal.unwrap_or(50);
        }

        for (field, draft_idx) in &mut self.draft_keys {
            let actual = new_state.keys.get(field).map(|s| s.as_str());
            *draft_idx = actual
                .and_then(|act| p::OPERATIONS.iter().position(|(name, _)| *name == act))
                .map(|i| i + 1)
                .unwrap_or(0);
        }
    }

    pub fn sync_eq_picker(&mut self) {
        if self.selected_preset_idx < self.preset_options.len()
            && self.preset_options[self.selected_preset_idx]
                .levels
                .as_ref()
                == Some(&self.eq_levels)
        {
            return;
        }
        self.selected_preset_idx = self
            .preset_options
            .iter()
            .position(|opt| opt.levels.as_ref() == Some(&self.eq_levels))
            .unwrap_or(0);
    }

    pub fn refresh_presets(&mut self, select_key: Option<&str>) {
        self.preset_options = presets::options(&self.settings.data.presets);
        if let Some(key) = select_key {
            if let Some(pos) = self.preset_options.iter().position(|o| o.key == key) {
                self.selected_preset_idx = pos;
                return;
            }
        }
        self.sync_eq_picker();
    }

    pub fn set_eq(&mut self, levels: Vec<i32>, select_key: Option<&str>) {
        if let Ok(valid) = presets::validate_levels(&levels) {
            self.eq_levels = valid;
            if let Some(k) = select_key {
                self.selected_preset_idx = self
                    .preset_options
                    .iter()
                    .position(|o| o.key == k)
                    .unwrap_or(0);
            } else {
                self.sync_eq_picker();
            }
        }
    }

    pub fn persist_draft_eq(&mut self) {
        if self.settings.data.eq != self.eq_levels {
            self.settings.data.eq = self.eq_levels.clone();
            self.save_settings();
        }
    }

    pub fn toggle_connect(&mut self) {
        if self.is_connected() {
            self.user_disconnected = true;
            let core = self.backend.core.clone();
            self.backend.spawn_task(async move {
                core.disconnect().await;
                Ok(())
            });
            return;
        }
        if self.selected_device_idx >= self.devices.len() {
            self.show_notice("Bấm Quét BLE và chọn tai nghe trước.", true);
            return;
        }
        let addr = self.devices[self.selected_device_idx].0.clone();
        self.address = addr.clone();
        self.user_disconnected = false;
        let core = self.backend.core.clone();
        self.backend
            .spawn_task(async move { core.connect(&addr).await });
    }

    pub(crate) fn execute_plan(&mut self, plan_res: Result<Plan, String>) {
        match plan_res {
            Err(e) => self.show_notice(e, true),
            Ok(plan) => {
                if plan.steps.is_empty() {
                    self.set_notice("Tai nghe đã ở thiết lập này.", NoticeTone::Info);
                    return;
                }
                let msg = plan.confirmation();
                self.ask_confirm(msg, move |app| {
                    let core = app.backend.core.clone();
                    app.backend
                        .spawn_task(async move { core.run_plan(&plan).await });
                });
            }
        }
    }
}

impl eframe::App for StudioApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        let c = if visuals.dark_mode {
            Palette::dark().bg_mid
        } else {
            Palette::light().bg_mid
        };
        c.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let desired_dark = resolve_dark_mode(ctx, self.settings.data.theme);
        if ctx.style().visuals.dark_mode != desired_dark {
            apply_theme(ctx, self.settings.data.theme);
        }
        paint_window_background(ctx);

        self.process_events();

        if self.smoke {
            if self.smoke_timer.elapsed() >= Duration::from_millis(100) {
                self.smoke_timer = Instant::now();
                if self.smoke_step < Page::ALL.len() {
                    let p = Page::ALL[self.smoke_step];
                    self.page = p;
                    println!("UI OK: {}", p.title());
                    self.smoke_step += 1;
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }
            ctx.request_repaint_after(Duration::from_millis(40));
        } else if self.settings.data.auto_connect
            && !self.user_disconnected
            && self.connection == ConnectionStatus::Disconnected
        {
            if self.last_reconnect_check.elapsed() >= Duration::from_secs(8) {
                self.last_reconnect_check = Instant::now();
                if self.busy == 0 {
                    let addr = self.address.clone();
                    let core = self.backend.core.clone();
                    self.backend
                        .spawn_task(async move { core.auto_connect(&addr).await });
                }
            }
            let wait_rem = Duration::from_secs(8)
                .saturating_sub(self.last_reconnect_check.elapsed())
                .max(Duration::from_millis(250));
            ctx.request_repaint_after(wait_rem);
        }

        if ctx.input(|i| i.viewport().close_requested()) && !self.shutting_down {
            self.shutting_down = true;
            self.settings.data.eq = self.eq_levels.clone();
            self.save_settings();
            self.backend.shutdown_blocking();
        }

        self.render_panels(ctx);
    }
}

/// Run an offscreen headless layout pass over all 6 pages across compact and standard viewports.
pub fn run_headless_smoke_test() {
    let ctx = egui::Context::default();
    configure_fonts(&ctx);
    apply_theme(&ctx, ThemeMode::Light);
    let mut app = StudioApp::new(None, true, false);
    for viewport_size in [Vec2::new(860.0, 620.0), Vec2::new(1060.0, 760.0)] {
        for page in Page::ALL {
            app.page = page;
            let raw_input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, viewport_size)),
                ..Default::default()
            };
            let _ = ctx.run(raw_input, |ctx| {
                paint_window_background(ctx);
                app.render_panels(ctx);
            });
        }
    }
    for page in Page::ALL {
        println!("UI OK: {}", page.title());
    }
    app.backend.shutdown_blocking();
}
