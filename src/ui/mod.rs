//! Cross-platform GUI for EarFun Studio (Windows & Ubuntu) built with eframe/egui.
//! Modularized light lavender-indigo design system with responsive layouts and low-friction UX.

pub mod header;
pub mod icons;
pub mod pages;
pub mod theme;
pub mod widgets;

#[cfg(test)]
mod tests;

pub use icons::{build_app_icon, draw_vector_icon, IconKind};

use crate::backend::{BackendEvent, BackendHandle, ConnectionStatus};
use crate::presets::{self, PresetOption};
use crate::protocol::{self as p, DeviceState, PhoneEntry};
use crate::settings::Settings;
use crate::ui::icons::draw_spinner;
use crate::ui::theme::*;
use crate::ui::widgets::{primary_button, secondary_button};
use crate::workflows::Plan;
use eframe::egui::{self, Color32, Key, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const MAX_UI_LOGS: usize = 2000;

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
            Page::Equalizer => "Tinh chỉnh trực quan 10 dải tần số",
            Page::Gestures => "Tùy biến thao tác chạm cho từng bên tai",
            Page::Multipoint => "Quản lý kết nối đồng thời hai thiết bị",
            Page::Earbuds => "Âm thanh, đàm thoại, codec Bluetooth và tìm tai nghe",
            Page::Application => "Tùy chọn kết nối, phím tắt và nhật ký Bluetooth LE",
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

struct ConfirmModal {
    message: String,
    just_opened: bool,
    action: Box<dyn FnOnce(&mut StudioApp) + Send>,
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
    pub(crate) user_disconnected: bool,
    pub(crate) state: DeviceState,
    pub(crate) phones: Vec<PhoneEntry>,
    pub(crate) devices: Vec<(String, String)>,
    pub(crate) selected_device_idx: usize,
    pub(crate) address: String,
    pub(crate) notice: String,
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
    confirm_modal: Option<ConfirmModal>,
    shutting_down: bool,
}

impl StudioApp {
    /// Initialize `StudioApp` and optionally configure `eframe` fonts & theme.
    pub fn new(cc: Option<&eframe::CreationContext<'_>>, smoke: bool, connect_last: bool) -> Self {
        if let Some(cc) = cc {
            configure_fonts(&cc.egui_ctx);
            configure_style(&cc.egui_ctx);
        }

        let settings = Settings::new(None);
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (busy_tx, busy_rx) = mpsc::unbounded_channel();

        let emit = Arc::new(move |ev: BackendEvent| {
            let _ = event_tx.send(ev);
        });
        let backend = BackendHandle::new(emit, busy_tx);

        let preset_options = presets::options(&settings.data.presets);
        let eq_levels = settings.data.eq.clone();
        let address = settings.data.last_mac.clone();
        let startup_connect = connect_last || settings.data.auto_connect;

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
            user_disconnected: false,
            state: DeviceState::default(),
            phones: Vec::new(),
            devices: Vec::new(),
            selected_device_idx: 0,
            address,
            notice: "Đang chuẩn bị kết nối tai nghe…".to_string(),
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
        self.notice = text.into();
        self.notice_is_error = is_error;
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

    pub(crate) fn apply_backend_event(&mut self, ev: BackendEvent) {
        match ev {
            BackendEvent::Devices(list) => {
                self.devices = list;
                self.selected_device_idx = self
                    .devices
                    .iter()
                    .position(|(mac, _)| *mac == self.address)
                    .unwrap_or(0);
            }
            BackendEvent::Connection(status) => {
                if status == ConnectionStatus::Disconnected {
                    self.connection_generation += 1;
                    self.eq_sent_levels = None;
                    self.eq_sending_levels = None;
                    self.find_until = None;
                    self.manual_query_pending = false;
                }
                self.connection = status;
                match self.connection {
                    ConnectionStatus::Connected => {
                        self.show_notice("Đã kết nối EarFun Air Pro 4.", false);
                    }
                    ConnectionStatus::Disconnected => {
                        self.show_notice("Chưa kết nối tai nghe.", false);
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
                self.show_notice(msg, false);
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

    pub(crate) fn ask_confirm<F>(&mut self, message: impl Into<String>, action: F)
    where
        F: FnOnce(&mut StudioApp) + Send + 'static,
    {
        if self.confirm_modal.is_some() {
            return;
        }
        self.confirm_modal = Some(ConfirmModal {
            message: message.into(),
            just_opened: true,
            action: Box::new(action),
        });
    }

    pub(crate) fn execute_plan(&mut self, plan_res: Result<Plan, String>) {
        match plan_res {
            Err(e) => self.show_notice(e, true),
            Ok(plan) => {
                if plan.steps.is_empty() {
                    self.show_notice("Tai nghe đã ở thiết lập này.", false);
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

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.confirm_modal.is_some() || ctx.wants_keyboard_input() {
            return;
        }
        let mut manual_refresh = false;
        ctx.input(|input| {
            if input.modifiers.alt {
                for (idx, key) in [
                    Key::Num1,
                    Key::Num2,
                    Key::Num3,
                    Key::Num4,
                    Key::Num5,
                    Key::Num6,
                ]
                .into_iter()
                .enumerate()
                {
                    if input.key_pressed(key) {
                        self.page = Page::ALL[idx];
                    }
                }
            }
            if input.key_pressed(Key::F5) && self.is_enabled() {
                manual_refresh = true;
            }
        });
        if manual_refresh {
            self.trigger_manual_query();
        }
    }

    fn render_confirm_modal(&mut self, ctx: &egui::Context) {
        let (msg, allow_keys) = match self.confirm_modal.as_mut() {
            Some(m) => {
                let allow = !m.just_opened;
                m.just_opened = false;
                (m.message.clone(), allow)
            }
            None => return,
        };

        let mut choice: Option<bool> = None;

        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("modal_backdrop_scrim"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    rect,
                    Rounding::ZERO,
                    Color32::from_rgba_unmultiplied(24, 30, 48, 68),
                );
                if allow_keys && resp.clicked() {
                    choice = Some(false);
                }
            });

        if allow_keys {
            ctx.input(|i| {
                if i.key_pressed(Key::Escape) {
                    choice = Some(false);
                } else if i.key_pressed(Key::Enter) {
                    choice = Some(true);
                }
            });
        }

        egui::Window::new("Xác nhận thay đổi")
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(430.0)
            .min_width(380.0)
            .show(ctx, |ui| {
                ui.set_max_width(430.0);
                ui.add_space(6.0);
                ui.label(RichText::new(&msg).size(13.0).color(TEXT_PRIMARY));
                ui.add_space(16.0);
                ui.horizontal_wrapped(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if primary_button(ui, "Tiếp tục", true).clicked() {
                            choice = Some(true);
                        }
                        if secondary_button(ui, "Hủy", true).clicked() {
                            choice = Some(false);
                        }
                    });
                });
            });

        if let Some(confirmed) = choice {
            if let Some(modal) = self.confirm_modal.take() {
                if confirmed {
                    (modal.action)(self);
                } else {
                    let st = self.state.clone();
                    self.sync_state_to_drafts(&st);
                }
            }
        }
    }

    pub fn render_panels(&mut self, ctx: &egui::Context) {
        self.handle_shortcuts(ctx);
        let bg_interactive = self.confirm_modal.is_none();

        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .exact_width(214.0)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 16.0,
                right: 8.0,
                top: 16.0,
                bottom: 16.0,
            }))
            .show(ctx, |ui| {
                ui.add_enabled_ui(bg_interactive, |ui| {
                    self.render_sidebar(ui);
                });
            });

        egui::TopBottomPanel::bottom("footer_panel")
            .resizable(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 10.0,
                right: 18.0,
                top: 4.0,
                bottom: 12.0,
            }))
            .show(ctx, |ui| {
                let (bg, border, fg, icon) = if self.notice_is_error {
                    (DANGER_BG, DANGER_BORDER, DANGER_TEXT, IconKind::Alert)
                } else {
                    (
                        Color32::from_rgba_unmultiplied(255, 255, 255, 185),
                        CARD_BORDER,
                        TEXT_SUBTITLE,
                        IconKind::Check,
                    )
                };
                egui::Frame::none()
                    .fill(bg)
                    .stroke(Stroke::new(1.0_f32, border))
                    .rounding(Rounding::same(11.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                    .show(ctx_ui(ui), |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_wrapped(|ui| {
                            if self.busy > 0 && !self.notice_is_error {
                                draw_spinner(ui, 14.0, ACCENT_BLUE);
                            } else {
                                let (icon_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                draw_vector_icon(
                                    ui.painter(),
                                    icon_rect,
                                    icon,
                                    if self.notice_is_error {
                                        DANGER_TEXT
                                    } else {
                                        ACCENT_BLUE
                                    },
                                );
                            }
                            ui.add(
                                egui::Label::new(RichText::new(&self.notice).size(11.5).color(fg))
                                    .selectable(true),
                            );
                        });
                    });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 10.0,
                right: 18.0,
                top: 16.0,
                bottom: 4.0,
            }))
            .show(ctx, |ui| {
                ui.add_enabled_ui(bg_interactive, |ui| {
                    self.render_top_header(ui);
                    ui.add_space(10.0);
                    egui::ScrollArea::vertical()
                        .id_salt("main_content")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add_space(2.0);
                            match self.page {
                                Page::Overview => self.render_overview(ui),
                                Page::Equalizer => self.render_equalizer(ui),
                                Page::Gestures => self.render_gestures(ui),
                                Page::Multipoint => self.render_multipoint(ui),
                                Page::Earbuds => self.render_earbuds(ui),
                                Page::Application => self.render_application(ui, ctx),
                            }
                            ui.add_space(8.0);
                        });
                });
            });

        self.render_confirm_modal(ctx);
    }
}

#[inline]
fn ctx_ui(ui: &mut egui::Ui) -> &mut egui::Ui {
    ui
}

impl eframe::App for StudioApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [229.0 / 255.0, 237.0 / 255.0, 247.0 / 255.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.style().visuals.dark_mode || ctx.style().visuals.panel_fill != BG_MID {
            configure_style(ctx);
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
        } else {
            if self.last_reconnect_check.elapsed() >= Duration::from_secs(8) {
                self.last_reconnect_check = Instant::now();
                if self.settings.data.auto_connect
                    && !self.user_disconnected
                    && self.connection == ConnectionStatus::Disconnected
                    && self.busy == 0
                {
                    let addr = self.address.clone();
                    let core = self.backend.core.clone();
                    self.backend
                        .spawn_task(async move { core.auto_connect(&addr).await });
                }
            }
            ctx.request_repaint_after(Duration::from_millis(250));
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
    configure_style(&ctx);
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
            assert!(
                !ctx.style().visuals.dark_mode,
                "Theme must remain Light mode"
            );
        }
    }
    for page in Page::ALL {
        println!("UI OK: {}", page.title());
    }
    app.backend.shutdown_blocking();
}
