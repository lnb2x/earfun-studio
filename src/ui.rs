//! Cross-platform GUI for EarFun Studio (Windows & Ubuntu) built with eframe/egui.

use crate::audio;
use crate::backend::{BackendEvent, BackendHandle, ConnectionStatus};
use crate::presets::{self, PresetOption};
use crate::protocol::{self as p, DeviceState, PhoneEntry};
use crate::settings::Settings;
use crate::workflows::{self, Plan};
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Pos2, RichText, Rounding, Sense,
    Stroke, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/BeVietnamPro-Regular.ttf");
const FONT_MEDIUM: &[u8] = include_bytes!("../assets/fonts/BeVietnamPro-Medium.ttf");
const FONT_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/BeVietnamPro-SemiBold.ttf");

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
            Page::Overview => "Âm thanh theo cách của bạn",
            Page::Equalizer => "Tinh chỉnh từng dải âm",
            Page::Gestures => "Thao tác quen thuộc, theo cách của bạn",
            Page::Multipoint => "Quản lý các thiết bị đang kết nối",
            Page::Earbuds => "Cá nhân hóa tai nghe của bạn",
            Page::Application => "Tùy chọn và thông tin ứng dụng",
        }
    }

    pub fn icon_glyph(&self) -> &'static str {
        match self {
            Page::Overview => "◉",
            Page::Equalizer => "≡",
            Page::Gestures => "◎",
            Page::Multipoint => "⇄",
            Page::Earbuds => "🎧",
            Page::Application => "⚙",
        }
    }
}

struct ConfirmModal {
    message: String,
    action: Box<dyn FnOnce(&mut StudioApp) + Send>,
}

pub struct StudioApp {
    pub smoke: bool,
    pub smoke_step: usize,
    pub smoke_timer: Instant,
    pub connect_last_pending: bool,
    pub settings: Settings,
    pub backend: BackendHandle,
    event_rx: mpsc::UnboundedReceiver<BackendEvent>,
    busy_rx: mpsc::UnboundedReceiver<i32>,
    pub busy: i32,
    pub page: Page,
    pub connection: ConnectionStatus,
    pub user_disconnected: bool,
    pub state: DeviceState,
    pub phones: Vec<PhoneEntry>,
    pub devices: Vec<(String, String)>,
    pub selected_device_idx: usize,
    pub address: String,
    pub notice: String,
    pub notice_is_error: bool,
    pub logs: Vec<String>,
    pub connection_generation: u64,

    // Choice drafts
    pub draft_sub: usize,
    pub draft_amb: usize,
    pub draft_prompt: usize,
    pub draft_mic: usize,
    pub draft_hearing_limit: usize,
    pub draft_le_audio: usize,
    pub dirty_choices: BTreeSet<String>,

    // Balance & Volume
    pub draft_balance: u8,
    pub balance_dirty: bool,
    pub volume_value: u8,
    pub volume_label: String,

    // Gestures drafts
    pub draft_keys: BTreeMap<String, usize>,
    pub dirty_keys: BTreeSet<String>,

    // Equalizer state
    pub preset_options: Vec<PresetOption>,
    pub selected_preset_idx: usize,
    pub preset_search: String,
    pub eq_levels: Vec<i32>,
    pub preset_name_input: String,
    pub eq_sent_levels: Option<Vec<i32>>,
    pub eq_sending_levels: Option<Vec<i32>>,

    // Reconnect timer
    pub last_reconnect_check: Instant,

    // Confirmation dialog
    confirm_modal: Option<ConfirmModal>,
    shutting_down: bool,
}

impl StudioApp {
    pub fn new(cc: Option<&eframe::CreationContext<'_>>, smoke: bool, connect_last: bool) -> Self {
        if let Some(cc) = cc {
            Self::configure_fonts(&cc.egui_ctx);
            Self::configure_style(&cc.egui_ctx);
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
            connect_last_pending: connect_last,
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
            notice: "Lấy tai nghe ra khỏi hộp. Bấm Tìm thiết bị để bắt đầu.".to_string(),
            notice_is_error: false,
            logs: Vec::new(),
            connection_generation: 0,
            draft_sub: 0,
            draft_amb: 0,
            draft_prompt: 0,
            draft_mic: 0,
            draft_hearing_limit: 0,
            draft_le_audio: 0,
            dirty_choices: BTreeSet::new(),
            draft_balance: 50,
            balance_dirty: false,
            volume_value: 50,
            volume_label: "— • Đầu ra EarFun".to_string(),
            draft_keys,
            dirty_keys: BTreeSet::new(),
            preset_options,
            selected_preset_idx: 0,
            preset_search: String::new(),
            eq_levels,
            preset_name_input: String::new(),
            eq_sent_levels: None,
            eq_sending_levels: None,
            last_reconnect_check: Instant::now(),
            confirm_modal: None,
            shutting_down: false,
        };

        app.sync_eq_picker();

        if !smoke {
            let core = app.backend.core.clone();
            app.backend.spawn_task(async move { core.load_devices().await });
        }

        app
    }

    fn configure_fonts(ctx: &egui::Context) {
        let mut fonts = FontDefinitions::default();
        fonts.font_data.insert(
            "BeVietnamPro-Regular".to_owned(),
            FontData::from_static(FONT_REGULAR),
        );
        fonts.font_data.insert(
            "BeVietnamPro-Medium".to_owned(),
            FontData::from_static(FONT_MEDIUM),
        );
        fonts.font_data.insert(
            "BeVietnamPro-SemiBold".to_owned(),
            FontData::from_static(FONT_SEMIBOLD),
        );

        fonts
            .families
            .entry(FontFamily::Proportional)
            .or_default()
            .insert(0, "BeVietnamPro-Regular".to_owned());
        fonts
            .families
            .entry(FontFamily::Name("Medium".into()))
            .or_default()
            .push("BeVietnamPro-Medium".to_owned());
        fonts
            .families
            .entry(FontFamily::Name("SemiBold".into()))
            .or_default()
            .push("BeVietnamPro-SemiBold".to_owned());

        ctx.set_fonts(fonts);
    }

    fn configure_style(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = Color32::from_rgb(227, 235, 247); // #e3ebf7
        visuals.window_fill = Color32::WHITE;
        visuals.override_text_color = Some(Color32::from_rgb(33, 34, 35)); // #212223
        visuals.widgets.noninteractive.bg_fill = Color32::WHITE;
        visuals.widgets.noninteractive.bg_stroke =
            Stroke::new(1.0_f32, Color32::from_rgb(237, 241, 248));
        visuals.widgets.noninteractive.rounding = Rounding::same(12.0);

        visuals.widgets.inactive.bg_fill = Color32::from_rgb(244, 246, 252);
        visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(244, 246, 252);
        visuals.widgets.inactive.bg_stroke =
            Stroke::new(1.0_f32, Color32::from_rgb(226, 231, 241));
        visuals.widgets.inactive.rounding = Rounding::same(10.0);

        visuals.widgets.hovered.bg_fill = Color32::from_rgb(237, 241, 250);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(237, 241, 250);
        visuals.widgets.hovered.bg_stroke =
            Stroke::new(1.0_f32, Color32::from_rgb(158, 163, 215));
        visuals.widgets.hovered.rounding = Rounding::same(10.0);

        visuals.widgets.active.bg_fill = Color32::from_rgb(228, 234, 251);
        visuals.widgets.active.weak_bg_fill = Color32::from_rgb(228, 234, 251);
        visuals.widgets.active.bg_stroke =
            Stroke::new(1.0_f32, Color32::from_rgb(102, 133, 209));
        visuals.widgets.active.rounding = Rounding::same(10.0);

        visuals.selection.bg_fill = Color32::from_rgb(85, 115, 188); // #5573bc
        visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(12.0, 7.0);
        ctx.set_style(style);
    }

    pub fn is_connected(&self) -> bool {
        self.connection == ConnectionStatus::Connected
    }

    pub fn is_enabled(&self) -> bool {
        self.is_connected() && self.busy == 0
    }

    pub fn show_notice(&mut self, text: impl Into<String>, is_error: bool) {
        self.notice = text.into();
        self.notice_is_error = is_error;
    }

    pub fn save_settings(&mut self) {
        if let Err(e) = self.settings.save() {
            self.show_notice(format!("Không lưu được cấu hình: {}", e), true);
        }
    }

    fn process_events(&mut self) {
        while let Ok(delta) = self.busy_rx.try_recv() {
            self.busy = (self.busy + delta).max(0);
        }

        while let Ok(ev) = self.event_rx.try_recv() {
            match ev {
                BackendEvent::Devices(list) => {
                    self.devices = list;
                    self.selected_device_idx = self
                        .devices
                        .iter()
                        .position(|(mac, _)| *mac == self.address)
                        .unwrap_or(0);
                    if self.connect_last_pending && !self.devices.is_empty() && !self.is_connected()
                    {
                        self.connect_last_pending = false;
                        self.toggle_connect();
                    }
                }
                BackendEvent::Connection(status) => {
                    if status == ConnectionStatus::Disconnected {
                        self.connection_generation += 1;
                        self.eq_sent_levels = None;
                        self.eq_sending_levels = None;
                    }
                    self.connection = status;
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
                    if msg == "Đã đọc trạng thái trực tiếp từ tai nghe." && self.busy == 0 {
                        continue;
                    }
                    self.show_notice(msg, false);
                }
                BackendEvent::Error(msg) => {
                    self.show_notice(msg, true);
                }
                BackendEvent::Log(line) => {
                    if self.logs.len() >= 2000 {
                        self.logs.remove(0);
                    }
                    self.logs.push(line);
                }
            }
        }
    }

    fn sync_state_to_drafts(&mut self, new_state: &DeviceState) {
        const SUB_KEYS: &[&str] = &["DEFAULT", "BALANCE", "ADAPTIVE", "EAR_ADAPTIVE", "WIND"];
        if !self.dirty_choices.contains("sub") {
            self.draft_sub = new_state
                .sub
                .as_deref()
                .and_then(|v| SUB_KEYS.iter().position(|&k| k == v))
                .map(|i| i + 1)
                .unwrap_or(0);
        } else if let Some(v) = new_state.sub.as_deref() {
            if self.draft_sub > 0 && SUB_KEYS.get(self.draft_sub - 1) == Some(&v) {
                self.dirty_choices.remove("sub");
            }
        }

        const AMB_KEYS: &[&str] = &["DEFAULT", "TOTAL"];
        if !self.dirty_choices.contains("amb") {
            self.draft_amb = new_state
                .amb
                .as_deref()
                .and_then(|v| AMB_KEYS.iter().position(|&k| k == v))
                .map(|i| i + 1)
                .unwrap_or(0);
        } else if let Some(v) = new_state.amb.as_deref() {
            if self.draft_amb > 0 && AMB_KEYS.get(self.draft_amb - 1) == Some(&v) {
                self.dirty_choices.remove("amb");
            }
        }

        const PROMPT_VALS: &[u8] = &[100, 75, 50, 25, 0];
        if !self.dirty_choices.contains("prompt") {
            self.draft_prompt = new_state
                .prompt
                .and_then(|v| PROMPT_VALS.iter().position(|&k| k == v))
                .map(|i| i + 1)
                .unwrap_or(0);
        } else if let Some(v) = new_state.prompt {
            if self.draft_prompt > 0 && PROMPT_VALS.get(self.draft_prompt - 1) == Some(&v) {
                self.dirty_choices.remove("prompt");
            }
        }

        const MIC_KEYS: &[&str] = &["AUTO", "L", "R"];
        if !self.dirty_choices.contains("mic") {
            self.draft_mic = new_state
                .mic
                .as_deref()
                .and_then(|v| MIC_KEYS.iter().position(|&k| k == v))
                .map(|i| i + 1)
                .unwrap_or(0);
        } else if let Some(v) = new_state.mic.as_deref() {
            if self.draft_mic > 0 && MIC_KEYS.get(self.draft_mic - 1) == Some(&v) {
                self.dirty_choices.remove("mic");
            }
        }

        if !self.dirty_choices.contains("hearing_limit") {
            self.draft_hearing_limit = new_state
                .hearing_limit
                .filter(|&v| v <= 5)
                .map(|v| (v as usize) + 1)
                .unwrap_or(0);
        } else if let Some(v) = new_state.hearing_limit {
            if self.draft_hearing_limit == (v as usize) + 1 {
                self.dirty_choices.remove("hearing_limit");
            }
        }

        if !self.dirty_choices.contains("le_audio") {
            self.draft_le_audio = match new_state.le_audio {
                Some(false) => 1,
                Some(true) => 2,
                None => 0,
            };
        } else if let Some(v) = new_state.le_audio {
            let idx = if v { 2 } else { 1 };
            if self.draft_le_audio == idx {
                self.dirty_choices.remove("le_audio");
            }
        }

        if !self.balance_dirty || Some(self.draft_balance) == new_state.balance {
            self.balance_dirty = false;
            self.draft_balance = new_state.balance.unwrap_or(50);
        }

        for (field, draft_idx) in &mut self.draft_keys {
            let actual = new_state.keys.get(field).map(|s| s.as_str());
            let actual_idx = actual
                .and_then(|act| p::OPERATIONS.iter().position(|(name, _)| *name == act))
                .map(|i| i + 1)
                .unwrap_or(0);
            if self.dirty_keys.contains(field) {
                if *draft_idx == actual_idx && actual_idx != 0 {
                    self.dirty_keys.remove(field);
                }
            } else {
                *draft_idx = actual_idx;
            }
        }
    }

    pub fn sync_eq_picker(&mut self) {
        if self.selected_preset_idx < self.preset_options.len() {
            if self.preset_options[self.selected_preset_idx].levels.as_ref()
                == Some(&self.eq_levels)
            {
                return;
            }
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
            self.show_notice("Bấm Tìm thiết bị và chọn tai nghe trước.", true);
            return;
        }
        let addr = self.devices[self.selected_device_idx].0.clone();
        self.address = addr.clone();
        self.user_disconnected = false;
        let core = self.backend.core.clone();
        self.backend
            .spawn_task(async move { core.connect(&addr).await });
    }

    fn ask_confirm<F>(&mut self, message: impl Into<String>, action: F)
    where
        F: FnOnce(&mut StudioApp) + Send + 'static,
    {
        self.confirm_modal = Some(ConfirmModal {
            message: message.into(),
            action: Box::new(action),
        });
    }

    fn execute_plan(&mut self, plan_res: Result<Plan, String>) {
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

    fn draw_card<R>(
        ui: &mut egui::Ui,
        title: Option<&str>,
        subtitle: Option<&str>,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> R {
        egui::Frame::none()
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(237, 241, 248)))
            .rounding(Rounding::same(18.0))
            .inner_margin(egui::Margin::same(18.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if let Some(t) = title {
                    ui.label(
                        RichText::new(t)
                            .font(FontId::new(16.5, FontFamily::Name("SemiBold".into())))
                            .color(Color32::from_rgb(33, 34, 35)),
                    );
                    if let Some(sub) = subtitle {
                        ui.label(
                            RichText::new(sub)
                                .size(12.0)
                                .color(Color32::from_rgb(104, 103, 118)),
                        );
                    }
                    ui.add_space(6.0);
                }
                add_contents(ui)
            })
            .inner
    }

    fn primary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
        let fill = if enabled {
            Color32::from_rgb(85, 115, 188)
        } else {
            Color32::from_rgb(180, 192, 220)
        };
        let btn = egui::Button::new(
            RichText::new(text)
                .font(FontId::new(13.0, FontFamily::Name("Medium".into())))
                .color(Color32::WHITE),
        )
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(102, 133, 209)))
        .rounding(Rounding::same(10.0));
        ui.add_enabled(enabled, btn)
    }

    fn danger_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
        let btn = egui::Button::new(
            RichText::new(text)
                .font(FontId::new(13.0, FontFamily::Name("Medium".into())))
                .color(Color32::from_rgb(170, 75, 72)),
        )
        .fill(Color32::from_rgb(255, 244, 243))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(241, 217, 214)))
        .rounding(Rounding::same(10.0));
        ui.add_enabled(enabled, btn)
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 235))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(237, 241, 248)))
            .rounding(Rounding::same(20.0))
            .inner_margin(egui::Margin::same(14.0))
            .show(ui, |ui| {
                ui.set_min_height(ui.available_height());
                ui.horizontal(|ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(230, 236, 251))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(9.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("♫")
                                    .size(18.0)
                                    .color(Color32::from_rgb(66, 91, 152)),
                            );
                        });
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            RichText::new("EarFun Studio")
                                .font(FontId::new(15.5, FontFamily::Name("SemiBold".into()))),
                        );
                        ui.label(
                            RichText::new("Air Pro 4")
                                .size(11.5)
                                .color(Color32::from_rgb(104, 103, 118)),
                        );
                    });
                });

                ui.add_space(18.0);

                for nav_page in Page::ALL {
                    let active = self.page == nav_page;
                    let fill = if active {
                        Color32::from_rgb(232, 237, 249)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let stroke = if active {
                        Stroke::new(1.0_f32, Color32::from_rgb(198, 209, 236))
                    } else {
                        Stroke::NONE
                    };
                    let text_color = if active {
                        Color32::from_rgb(66, 91, 152)
                    } else {
                        Color32::from_rgb(110, 114, 130)
                    };
                    let font = if active {
                        FontId::new(13.5, FontFamily::Name("SemiBold".into()))
                    } else {
                        FontId::new(13.5, FontFamily::Proportional)
                    };

                    let btn = egui::Button::new(
                        RichText::new(format!("{}   {}", nav_page.icon_glyph(), nav_page.title()))
                            .font(font)
                            .color(text_color),
                    )
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(Rounding::same(11.0))
                    .min_size(Vec2::new(ui.available_width(), 38.0));

                    if ui.add(btn).on_hover_text(nav_page.subtitle()).clicked() {
                        self.page = nav_page;
                    }
                }

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.label(
                        RichText::new("EarFun Studio 1.3 · Rust")
                            .size(10.5)
                            .color(Color32::from_rgb(120, 122, 135)),
                    );
                    let status_text = match self.connection {
                        ConnectionStatus::Connected => "Bluetooth LE • Đã kết nối",
                        ConnectionStatus::Connecting => "Bluetooth LE • Đang kết nối…",
                        ConnectionStatus::Disconnected => "Bluetooth LE • Chưa kết nối",
                    };
                    ui.label(
                        RichText::new(status_text)
                            .size(11.0)
                            .color(Color32::from_rgb(104, 103, 118)),
                    );
                    if ui
                        .button(
                            RichText::new("Bluetooth hệ thống")
                                .size(12.0)
                                .color(Color32::from_rgb(98, 105, 125)),
                        )
                        .clicked()
                    {
                        if let Err(e) = audio::open_bluetooth_settings() {
                            self.show_notice(e, true);
                        }
                    }
                });
            });
    }

    fn render_header_and_summary(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                ui.label(
                    RichText::new(self.page.title())
                        .font(FontId::new(24.0, FontFamily::Name("SemiBold".into()))),
                );
                ui.label(
                    RichText::new(self.page.subtitle())
                        .size(12.5)
                        .color(Color32::from_rgb(104, 103, 118)),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_scan = self.busy == 0 && !self.is_connected();
                if ui
                    .add_enabled(can_scan, egui::Button::new("Tìm thiết bị"))
                    .clicked()
                {
                    let core = self.backend.core.clone();
                    self.backend.spawn_task(async move { core.scan().await });
                }

                ui.add_enabled_ui(can_scan, |ui| {
                    let selected_label = if self.devices.is_empty() {
                        "Chọn tai nghe".to_string()
                    } else {
                        self.devices
                            .get(self.selected_device_idx)
                            .map(|(_, name)| name.clone())
                            .unwrap_or_else(|| "Chọn tai nghe".to_string())
                    };
                    egui::ComboBox::from_id_salt("device_picker")
                        .width(220.0)
                        .selected_text(selected_label)
                        .show_ui(ui, |ui| {
                            for (i, (_, name)) in self.devices.iter().enumerate() {
                                ui.selectable_value(&mut self.selected_device_idx, i, name);
                            }
                        });
                });
            });
        });

        ui.add_space(6.0);

        ui.columns(5, |cols| {
            Self::draw_card(&mut cols[0], None, None, |ui| {
                ui.label(
                    RichText::new("EarFun Air Pro 4")
                        .font(FontId::new(15.0, FontFamily::Name("SemiBold".into()))),
                );
                let badge_str = match self.connection {
                    ConnectionStatus::Connected => "Đã kết nối",
                    ConnectionStatus::Connecting => "Đang kết nối…",
                    ConnectionStatus::Disconnected => "Chưa kết nối",
                };
                let codec_str = self.state.codec.as_deref().unwrap_or("—");
                ui.label(
                    RichText::new(format!("{} • {}", badge_str, codec_str))
                        .size(11.5)
                        .color(Color32::from_rgb(80, 107, 170)),
                );
            });

            Self::draw_card(&mut cols[1], None, None, |ui| {
                ui.label(
                    RichText::new("Điều khiển BLE")
                        .size(12.0)
                        .color(Color32::from_rgb(104, 103, 118)),
                );
                let btn_label = match self.connection {
                    ConnectionStatus::Connected => "Ngắt kết nối",
                    ConnectionStatus::Connecting => "Đang kết nối…",
                    ConnectionStatus::Disconnected => "Kết nối",
                };
                if Self::primary_button(ui, btn_label, self.busy == 0).clicked() {
                    self.toggle_connect();
                }
            });

            let bat_items = [
                ("Tai trái", self.state.bat_l),
                ("Tai phải", self.state.bat_r),
                ("Hộp sạc", self.state.bat_box),
            ];
            for (i, (title, val)) in bat_items.into_iter().enumerate() {
                Self::draw_card(&mut cols[i + 2], None, None, |ui| {
                    ui.label(
                        RichText::new(title)
                            .size(12.0)
                            .color(Color32::from_rgb(104, 103, 118)),
                    );
                    let text = val.map(|v| format!("{}%", v)).unwrap_or_else(|| "—".to_string());
                    ui.label(
                        RichText::new(text)
                            .font(FontId::new(22.0, FontFamily::Name("SemiBold".into()))),
                    );
                    let frac = val.map(|v| (v as f32) / 100.0).unwrap_or(0.0);
                    ui.add(
                        egui::ProgressBar::new(frac)
                            .desired_height(5.0)
                            .fill(Color32::from_rgb(142, 172, 210)),
                    );
                });
            }
        });
    }

    fn render_overview(&mut self, ui: &mut egui::Ui) {
        let enabled = self.is_enabled();
        let current_noise = self.state.noise.clone();

        Self::draw_card(ui, Some("Kiểm soát tiếng ồn"), None, |ui| {
            ui.columns(3, |cols| {
                let modes = [
                    ("normal", "Bình thường", "Âm thanh nguyên bản", 0u8),
                    ("anc", "Chống ồn", "Tập trung vào âm nhạc", 1u8),
                    ("ambient", "Xuyên âm", "Nghe môi trường xung quanh", 2u8),
                ];
                for (i, (key, title, desc, code)) in modes.into_iter().enumerate() {
                    let is_active = current_noise.as_deref() == Some(key);
                    let fill = if is_active {
                        Color32::from_rgb(233, 238, 252)
                    } else {
                        Color32::from_rgb(247, 249, 252)
                    };
                    let stroke = if is_active {
                        Stroke::new(1.5_f32, Color32::from_rgb(140, 156, 210))
                    } else {
                        Stroke::new(1.0_f32, Color32::from_rgb(231, 235, 243))
                    };
                    let btn = egui::Button::new(
                        RichText::new(format!("{}\n{}", title, desc))
                            .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                    )
                    .fill(fill)
                    .stroke(stroke)
                    .rounding(Rounding::same(12.0))
                    .min_size(Vec2::new(cols[i].available_width(), 54.0));

                    if cols[i].add_enabled(enabled, btn).clicked() {
                        let core = self.backend.core.clone();
                        let k_owned = key.to_string();
                        self.backend.spawn_task(async move {
                            core.set_string_value(20, 21, code, "noise", &k_owned).await
                        });
                    }
                }
            });

            ui.add_space(10.0);

            ui.columns(2, |cols| {
                let sub_options = [
                    ("—", "", 0u8),
                    ("Mạnh", "DEFAULT", 2u8),
                    ("Cân bằng", "BALANCE", 3u8),
                    ("Thích ứng môi trường", "ADAPTIVE", 4u8),
                    ("Thích ứng tai", "EAR_ADAPTIVE", 0u8),
                    ("Giảm tiếng gió", "WIND", 1u8),
                ];
                let sub_avail = enabled && self.state.sub.is_some();
                cols[0].label(
                    RichText::new("Mức chống ồn")
                        .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                );
                cols[0].horizontal(|ui| {
                    ui.add_enabled_ui(sub_avail, |ui| {
                        let prev = self.draft_sub;
                        egui::ComboBox::from_id_salt("choice_sub")
                            .width(200.0)
                            .selected_text(sub_options[self.draft_sub].0)
                            .show_ui(ui, |ui| {
                                for (idx, (label, _, _)) in sub_options.iter().enumerate() {
                                    ui.selectable_value(&mut self.draft_sub, idx, *label);
                                }
                            });
                        if self.draft_sub != prev {
                            self.dirty_choices.insert("sub".to_string());
                        }
                    });
                    let wanted_key = sub_options[self.draft_sub].1;
                    let can_apply = sub_avail
                        && self.draft_sub > 0
                        && self.state.sub.as_deref() != Some(wanted_key);
                    if ui
                        .add_enabled(can_apply, egui::Button::new("Áp dụng"))
                        .clicked()
                    {
                        let wire_val = sub_options[self.draft_sub].2;
                        let wanted = wanted_key.to_string();
                        let core = self.backend.core.clone();
                        self.backend.spawn_task(async move {
                            core.set_string_value(58, 59, wire_val, "sub", &wanted)
                                .await
                        });
                    }
                });

                let amb_options = [
                    ("—", "", 0u8),
                    ("Mặc định", "DEFAULT", 0u8),
                    ("Tự nhiên", "TOTAL", 1u8),
                ];
                let amb_avail = enabled && self.state.amb.is_some();
                cols[1].label(
                    RichText::new("Chế độ xuyên âm")
                        .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                );
                cols[1].horizontal(|ui| {
                    ui.add_enabled_ui(amb_avail, |ui| {
                        let prev = self.draft_amb;
                        egui::ComboBox::from_id_salt("choice_amb")
                            .width(200.0)
                            .selected_text(amb_options[self.draft_amb].0)
                            .show_ui(ui, |ui| {
                                for (idx, (label, _, _)) in amb_options.iter().enumerate() {
                                    ui.selectable_value(&mut self.draft_amb, idx, *label);
                                }
                            });
                        if self.draft_amb != prev {
                            self.dirty_choices.insert("amb".to_string());
                        }
                    });
                    let wanted_key = amb_options[self.draft_amb].1;
                    let can_apply = amb_avail
                        && self.draft_amb > 0
                        && self.state.amb.as_deref() != Some(wanted_key);
                    if ui
                        .add_enabled(can_apply, egui::Button::new("Áp dụng"))
                        .clicked()
                    {
                        let wire_val = amb_options[self.draft_amb].2;
                        let wanted = wanted_key.to_string();
                        let core = self.backend.core.clone();
                        self.backend.spawn_task(async move {
                            core.set_string_value(60, 61, wire_val, "amb", &wanted)
                                .await
                        });
                    }
                });
            });
        });

        ui.columns(2, |cols| {
            Self::draw_card(&mut cols[0], None, None, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Chế độ chơi game")
                                .font(FontId::new(13.5, FontFamily::Name("Medium".into()))),
                        );
                        ui.label(
                            RichText::new("Giảm độ trễ khi xem và chơi")
                                .size(11.5)
                                .color(Color32::from_rgb(104, 103, 118)),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut game_on = self.state.game.unwrap_or(false);
                        let avail = enabled && self.state.game.is_some();
                        if ui.add_enabled(avail, egui::Checkbox::new(&mut game_on, "")).changed() {
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_bool_value(18, 19, u8::from(game_on), "game", game_on)
                                    .await
                            });
                        }
                    });
                });
            });

            let mut trigger_ear: Option<bool> = None;
            Self::draw_card(&mut cols[1], None, None, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("Nhận biết đeo tai")
                                .font(FontId::new(13.5, FontFamily::Name("Medium".into()))),
                        );
                        ui.label(
                            RichText::new("Phản hồi khi đeo hoặc tháo tai nghe")
                                .size(11.5)
                                .color(Color32::from_rgb(104, 103, 118)),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut ear_on = self.state.ear.unwrap_or(false);
                        let avail = enabled && self.state.ear.is_some();
                        if ui.add_enabled(avail, egui::Checkbox::new(&mut ear_on, "")).changed() {
                            trigger_ear = Some(ear_on);
                        }
                    });
                });
            });
            if let Some(on) = trigger_ear {
                let plan_res = workflows::restart_setting(&self.state, "ear", on);
                self.execute_plan(plan_res);
            }
        });

        Self::draw_card(ui, None, None, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Âm lượng đầu ra tai nghe")
                            .font(FontId::new(13.5, FontFamily::Name("Medium".into()))),
                    );
                    ui.label(
                        RichText::new(&self.volume_label)
                            .size(11.5)
                            .color(Color32::from_rgb(104, 103, 118)),
                    );
                });
                if ui
                    .add_enabled(enabled, egui::Button::new("↻"))
                    .on_hover_text("Đọc âm lượng")
                    .clicked()
                {
                    self.read_or_set_volume(None);
                }
                ui.add(egui::Slider::new(&mut self.volume_value, 0..=100).suffix("%"));
                if Self::primary_button(ui, "Áp dụng", enabled).clicked() {
                    let v = self.volume_value;
                    self.read_or_set_volume(Some(v));
                }
                if ui.button("Cài đặt âm thanh").clicked() {
                    if let Err(e) = audio::open_sound_settings() {
                        self.show_notice(e, true);
                    }
                }
            });
        });
    }

    fn read_or_set_volume(&mut self, value: Option<u8>) {
        match audio::volume(&self.address, value) {
            Ok(result) => {
                if let Some(second) = result.split_whitespace().nth(1) {
                    if let Ok(frac) = second.parse::<f64>() {
                        self.volume_value = (frac * 100.0).round().clamp(0.0, 100.0) as u8;
                    }
                }
                self.volume_label = format!("{}% • Đầu ra EarFun", self.volume_value);
                self.show_notice(result, false);
            }
            Err(e) => {
                self.show_notice(e, true);
            }
        }
    }

    fn render_equalizer(&mut self, ui: &mut egui::Ui) {
        let enabled = self.is_enabled();
        let mut reset_clicked = false;
        let mut load_clicked = false;
        let mut save_clicked = false;
        let mut apply_clicked = false;
        let mut rename_clicked = false;
        let mut delete_clicked = false;
        let mut import_clicked = false;
        let mut export_clicked = false;
        let mut sliders_changed = false;

        Self::draw_card(
            ui,
            Some("Bộ EQ EarFun Audio"),
            Some("30 mẫu EarFun và 2 mẫu Fernanda cho Air Pro 4 • Chọn mẫu để nạp bản nháp"),
            |ui| {
                ui.horizontal(|ui| {
                    let current_label = self
                        .preset_options
                        .get(self.selected_preset_idx)
                        .map(|o| o.label.clone())
                        .unwrap_or_else(|| "Tùy chỉnh (bản nháp)".to_string());

                    let prev_idx = self.selected_preset_idx;
                    egui::ComboBox::from_id_salt("preset_picker")
                        .width(260.0)
                        .selected_text(current_label)
                        .show_ui(ui, |ui| {
                            ui.text_edit_singleline(&mut self.preset_search);
                            let q = self.preset_search.trim().to_lowercase();
                            for (i, opt) in self.preset_options.iter().enumerate() {
                                if q.is_empty() || opt.label.to_lowercase().contains(&q) {
                                    ui.selectable_value(&mut self.selected_preset_idx, i, &opt.label);
                                }
                            }
                        });
                    if self.selected_preset_idx != prev_idx {
                        load_clicked = true;
                    }
                    if ui.button("Nạp bản chỉnh").clicked() {
                        load_clicked = true;
                    }
                    if ui.button("Đặt lại").clicked() {
                        reset_clicked = true;
                    }
                });

                ui.add_space(6.0);
                Self::draw_eq_curve(ui, &self.eq_levels);
                ui.add_space(6.0);

                ui.columns(10, |cols| {
                    for (i, &freq) in p::FREQUENCIES.iter().enumerate() {
                        cols[i].vertical_centered(|ui| {
                            let gain = p::GAINS[(self.eq_levels[i] + 10) as usize];
                            let gain_str = if gain.abs() < 1e-6 {
                                "0 dB".to_string()
                            } else {
                                format!("{:+} dB", gain)
                            };
                            ui.label(
                                RichText::new(gain_str)
                                    .size(11.0)
                                    .color(Color32::from_rgb(80, 107, 170)),
                            );
                            let slider = egui::Slider::new(&mut self.eq_levels[i], -10..=10)
                                .vertical()
                                .show_value(false);
                            if ui.add_sized([24.0, 130.0], slider).changed() {
                                sliders_changed = true;
                            }
                            let freq_label = if freq >= 1000.0 {
                                format!("{}k", (freq / 1000.0) as u32)
                            } else if (freq - 31.5).abs() < 0.1 {
                                "31.5".to_string()
                            } else {
                                format!("{}", freq as u32)
                            };
                            ui.label(
                                RichText::new(freq_label)
                                    .size(11.0)
                                    .color(Color32::from_rgb(104, 103, 118)),
                            );
                        });
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.preset_name_input)
                            .hint_text("Tên bản chỉnh")
                            .desired_width(180.0),
                    );
                    if ui.button("Lưu trên máy").clicked() {
                        save_clicked = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if Self::primary_button(ui, "Gửi tới tai nghe", enabled).clicked() {
                            apply_clicked = true;
                        }
                    });
                });

                ui.horizontal(|ui| {
                    if ui.button("Đổi tên").clicked() {
                        rename_clicked = true;
                    }
                    if ui.button("Xóa bản riêng").clicked() {
                        delete_clicked = true;
                    }
                    if ui.button("Nhập từ tệp…").clicked() {
                        import_clicked = true;
                    }
                    if ui.button("Xuất bản nháp…").clicked() {
                        export_clicked = true;
                    }
                });

                let name = self
                    .preset_options
                    .get(self.selected_preset_idx)
                    .map(|o| o.label.as_str())
                    .unwrap_or("Tùy chỉnh (bản nháp)");
                let status = if let Some(sending) = &self.eq_sending_levels {
                    if *sending == self.eq_levels {
                        "Đang gửi bản này…"
                    } else {
                        "Đang gửi bản trước • Bản này chưa gửi"
                    }
                } else if self.eq_sent_levels.as_ref() == Some(&self.eq_levels) {
                    "Đã gửi trong phiên này • Chưa đọc lại được EQ"
                } else {
                    "Chưa gửi"
                };
                ui.label(
                    RichText::new(format!("Bản nháp: {} • {}", name, status))
                        .size(11.5)
                        .color(Color32::from_rgb(104, 103, 118)),
                );
            },
        );

        ui.label(
            RichText::new(
                "Đường cong hiển thị bản chỉnh trên máy. Chưa hỗ trợ đọc lại EQ từ tai nghe.",
            )
            .size(11.5)
            .color(Color32::from_rgb(104, 103, 118)),
        );

        if sliders_changed {
            self.sync_eq_picker();
        }
        if reset_clicked {
            self.set_eq(vec![0; 10], None);
        }
        if load_clicked {
            if let Some(opt) = self.preset_options.get(self.selected_preset_idx).cloned() {
                if let Some(levels) = opt.levels {
                    self.set_eq(levels, Some(&opt.key));
                    if let Some(custom_name) = opt.key.strip_prefix("custom:") {
                        self.preset_name_input = custom_name.to_string();
                    }
                }
            }
        }
        if save_clicked {
            self.handle_save_preset();
        }
        if rename_clicked {
            self.handle_rename_preset();
        }
        if delete_clicked {
            self.handle_delete_preset();
        }
        if import_clicked {
            self.handle_import_preset();
        }
        if export_clicked {
            self.handle_export_preset();
        }
        if apply_clicked {
            self.handle_apply_eq();
        }
    }

    fn draw_eq_curve(ui: &mut egui::Ui, levels: &[i32]) {
        let desired = Vec2::new(ui.available_width(), 108.0);
        let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, Rounding::same(13.0), Color32::from_rgb(248, 250, 254));

        let left = rect.left() + 38.0;
        let right = rect.right() - 18.0;
        let top = rect.top() + 13.0;
        let bottom = rect.bottom() - 13.0;
        let y_for_gain = |gain: f64| -> f32 {
            top + ((6.0 - gain) as f32 / 12.0) * (bottom - top)
        };

        for db in [6, 3, 0, -3, -6] {
            let yy = y_for_gain(db as f64);
            let label = if db > 0 {
                format!("+{}", db)
            } else {
                db.to_string()
            };
            painter.text(
                Pos2::new(rect.left() + 8.0, yy),
                egui::Align2::LEFT_CENTER,
                label,
                FontId::new(10.0, FontFamily::Proportional),
                Color32::from_rgb(124, 130, 150),
            );
            painter.line_segment(
                [Pos2::new(left, yy), Pos2::new(right, yy)],
                Stroke::new(1.0_f32, Color32::from_rgb(226, 233, 246)),
            );
        }

        let mut points = Vec::with_capacity(10);
        for (i, &lv) in levels.iter().enumerate() {
            let gain = p::GAINS[(lv.clamp(-10, 10) + 10) as usize];
            let x = left + (right - left) * ((i as f32 + 0.5) / 10.0);
            let y = y_for_gain(gain);
            painter.line_segment(
                [Pos2::new(x, top), Pos2::new(x, bottom)],
                Stroke::new(1.0_f32, Color32::from_rgb(237, 241, 249)),
            );
            points.push(Pos2::new(x, y));
        }

        let stroke = Stroke::new(2.5_f32, Color32::from_rgb(102, 133, 209));
        for i in 0usize..9usize {
            let p0 = points[i.saturating_sub(1)];
            let p1 = points[i];
            let p2 = points[i + 1];
            let p3 = points[(i + 2).min(9)];

            let c1 = Pos2::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
            let c2 = Pos2::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);

            let mut prev = p1;
            for step in 1..=12 {
                let t = step as f32 / 12.0;
                let inv = 1.0 - t;
                let x = inv * inv * inv * p1.x
                    + 3.0 * inv * inv * t * c1.x
                    + 3.0 * inv * t * t * c2.x
                    + t * t * t * p2.x;
                let y = inv * inv * inv * p1.y
                    + 3.0 * inv * inv * t * c1.y
                    + 3.0 * inv * t * t * c2.y
                    + t * t * t * p2.y;
                let next = Pos2::new(x, y);
                painter.line_segment([prev, next], stroke);
                prev = next;
            }
        }

        for pt in points {
            painter.circle(
                pt,
                3.5,
                Color32::WHITE,
                Stroke::new(1.8_f32, Color32::from_rgb(102, 133, 209)),
            );
        }
    }

    fn handle_save_preset(&mut self) {
        let name = self.preset_name_input.trim().to_string();
        if name.is_empty() || name.chars().count() > 100 {
            self.show_notice("Nhập tên bản chỉnh từ 1 đến 100 ký tự.", true);
            return;
        }
        let levels = self.eq_levels.clone();
        let exists = self.settings.data.presets.contains_key(&name);
        let do_save = move |app: &mut StudioApp| {
            app.settings.data.presets.insert(name.clone(), levels.clone());
            app.settings.data.eq = levels;
            app.save_settings();
            app.refresh_presets(Some(&format!("custom:{}", name)));
            app.show_notice("Đã lưu bản chỉnh trên máy.", false);
        };
        if exists {
            let prompt = format!("Ghi đè bản chỉnh riêng “{}”?", self.preset_name_input.trim());
            self.ask_confirm(prompt, do_save);
        } else {
            do_save(self);
        }
    }

    fn selected_custom_name(&mut self) -> Option<String> {
        let key = self
            .preset_options
            .get(self.selected_preset_idx)
            .map(|o| o.key.as_str())
            .unwrap_or("");
        if let Some(custom) = key.strip_prefix("custom:") {
            Some(custom.to_string())
        } else {
            self.show_notice("Chọn một bản chỉnh trong nhóm Cá nhân trước.", true);
            None
        }
    }

    fn handle_rename_preset(&mut self) {
        let Some(old) = self.selected_custom_name() else {
            return;
        };
        let new_name = self.preset_name_input.trim().to_string();
        if new_name.is_empty() || new_name.chars().count() > 100 {
            self.show_notice("Nhập tên mới vào ô Tên bản chỉnh (1–100 ký tự).", true);
            return;
        }
        if new_name != old && self.settings.data.presets.contains_key(&new_name) {
            self.show_notice("Tên mới đã tồn tại. Hãy dùng tên khác.", true);
            return;
        }
        if let Some(levels) = self.settings.data.presets.remove(&old) {
            self.settings.data.presets.insert(new_name.clone(), levels);
            self.save_settings();
            self.refresh_presets(Some(&format!("custom:{}", new_name)));
            self.show_notice("Đã đổi tên bản chỉnh riêng.", false);
        }
    }

    fn handle_delete_preset(&mut self) {
        let Some(name) = self.selected_custom_name() else {
            return;
        };
        let prompt = format!("Xóa bản chỉnh riêng “{}” trên máy?", name);
        self.ask_confirm(prompt, move |app| {
            app.settings.data.presets.remove(&name);
            app.save_settings();
            app.refresh_presets(None);
            app.show_notice(
                "Đã xóa bản chỉnh riêng; bản nháp đang chỉnh vẫn được giữ.",
                false,
            );
        });
    }

    fn handle_export_preset(&mut self) {
        let fallback_name = self
            .preset_options
            .get(self.selected_preset_idx)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| "Bản chỉnh EQ".to_string());
        let name = if self.preset_name_input.trim().is_empty() {
            fallback_name
        } else {
            self.preset_name_input.trim().to_string()
        };
        let doc = match presets::encode(&name, &self.eq_levels) {
            Ok(d) => d,
            Err(e) => {
                self.show_notice(e, true);
                return;
            }
        };
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Xuất bản nháp EQ")
            .set_file_name("earfun-eq.json")
            .add_filter("Bản chỉnh EQ (*.json)", &["json"])
            .save_file()
        {
            match serde_json::to_string_pretty(&doc)
                .map_err(|e| e.to_string())
                .and_then(|s| std::fs::write(&path, format!("{}\n", s)).map_err(|e| e.to_string()))
            {
                Ok(()) => self.show_notice("Đã xuất bản nháp EQ ra tệp.", false),
                Err(e) => self.show_notice(e, true),
            }
        }
    }

    fn handle_import_preset(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Nhập bản chỉnh EQ EarFun Studio")
            .add_filter("Bản chỉnh EQ (*.json)", &["json"])
            .pick_file()
        {
            match std::fs::read(&path) {
                Ok(bytes) => {
                    if bytes.len() > 65536 {
                        self.show_notice("Tệp EQ quá lớn (tối đa 64 KiB).", true);
                        return;
                    }
                    let res = serde_json::from_slice::<serde_json::Value>(&bytes)
                        .map_err(|e| e.to_string())
                        .and_then(|v| presets::decode_value(&v));
                    match res {
                        Ok((name, levels)) => {
                            self.set_eq(levels, None);
                            self.preset_name_input = name;
                            self.show_notice(
                                "Đã nhập vào bản nháp. Bấm Lưu trên máy hoặc Gửi tới tai nghe để áp dụng.",
                                false,
                            );
                        }
                        Err(e) => self.show_notice(e, true),
                    }
                }
                Err(e) => self.show_notice(e.to_string(), true),
            }
        }
    }

    fn handle_apply_eq(&mut self) {
        let levels = self.eq_levels.clone();
        self.settings.data.eq = levels.clone();
        self.save_settings();
        self.eq_sending_levels = Some(levels.clone());
        let core = self.backend.core.clone();
        let levels_for_task = levels.clone();
        self.backend
            .spawn_task(async move { core.set_eq(&levels_for_task).await });
        self.eq_sending_levels = None;
        self.eq_sent_levels = Some(levels);
    }

    fn render_gestures(&mut self, ui: &mut egui::Ui) {
        let enabled = self.is_enabled();
        let mut save_req: Option<(String, String, String)> = None;
        let mut lock_req: Option<(bool, bool)> = None;

        ui.columns(2, |cols| {
            for (col_idx, (side, title)) in
                [("L", "Tai trái"), ("R", "Tai phải")].into_iter().enumerate()
            {
                Self::draw_card(
                    &mut cols[col_idx],
                    Some(title),
                    Some("Chọn chức năng cho từng thao tác."),
                    |ui| {
                        for (gesture, _) in p::GESTURES {
                            let field = format!("{}.{}", side, gesture);
                            let current_idx = *self.draft_keys.get(&field).unwrap_or(&0);
                            let selected_label = if current_idx == 0 {
                                "—"
                            } else {
                                p::OPERATIONS[current_idx - 1].0
                            };

                            ui.label(
                                RichText::new(*gesture)
                                    .font(FontId::new(12.5, FontFamily::Name("Medium".into()))),
                            );
                            ui.horizontal(|ui| {
                                ui.add_enabled_ui(enabled, |ui| {
                                    let mut new_idx = current_idx;
                                    egui::ComboBox::from_id_salt(&field)
                                        .width(210.0)
                                        .selected_text(selected_label)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut new_idx, 0, "—");
                                            for (i, (op_name, _)) in p::OPERATIONS.iter().enumerate()
                                            {
                                                ui.selectable_value(&mut new_idx, i + 1, *op_name);
                                            }
                                        });
                                    if new_idx != current_idx {
                                        self.draft_keys.insert(field.clone(), new_idx);
                                        self.dirty_keys.insert(field.clone());
                                    }
                                });

                                let idx_now = *self.draft_keys.get(&field).unwrap_or(&0);
                                let wanted = if idx_now >= 1 && idx_now <= p::OPERATIONS.len() {
                                    Some(p::OPERATIONS[idx_now - 1].0)
                                } else {
                                    None
                                };
                                let actual = self.state.keys.get(&field).map(|s| s.as_str());
                                let can_save = enabled && wanted.is_some() && wanted != actual;
                                if ui.add_enabled(can_save, egui::Button::new("Lưu")).clicked() {
                                    if let Some(act) = wanted {
                                        save_req = Some((
                                            side.to_string(),
                                            gesture.to_string(),
                                            act.to_string(),
                                        ));
                                    }
                                }
                            });
                            ui.add_space(4.0);
                        }

                        ui.separator();
                        let both_known = self.state.dis_l.is_some() && self.state.dis_r.is_some();
                        let mut locked = if side == "L" {
                            self.state.dis_l.unwrap_or(false)
                        } else {
                            self.state.dis_r.unwrap_or(false)
                        };
                        if ui
                            .add_enabled(
                                enabled && both_known,
                                egui::Checkbox::new(&mut locked, "Tắt cảm ứng bên này"),
                            )
                            .changed()
                        {
                            let l = if side == "L" {
                                locked
                            } else {
                                self.state.dis_l.unwrap_or(false)
                            };
                            let r = if side == "R" {
                                locked
                            } else {
                                self.state.dis_r.unwrap_or(false)
                            };
                            lock_req = Some((l, r));
                        }
                    },
                );
            }
        });

        if let Some((side, gesture, action)) = save_req {
            let core = self.backend.core.clone();
            self.backend
                .spawn_task(async move { core.set_key(&side, &gesture, &action).await });
        }
        if let Some((l, r)) = lock_req {
            let core = self.backend.core.clone();
            self.backend
                .spawn_task(async move { core.disable_touch(l, r).await });
        }
    }

    fn render_multipoint(&mut self, ui: &mut egui::Ui) {
        let enabled = self.is_enabled();
        let mut trigger_dual: Option<bool> = None;
        let mut pair_new = false;

        Self::draw_card(
            ui,
            Some("Kết nối đa điểm"),
            Some("Nghe và chuyển đổi giữa tối đa hai thiết bị."),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Cho phép kết nối hai thiết bị")
                            .font(FontId::new(13.5, FontFamily::Name("Medium".into()))),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut dual_on = self.state.dual.unwrap_or(false);
                        let avail = enabled && self.state.dual.is_some();
                        if ui.add_enabled(avail, egui::Checkbox::new(&mut dual_on, "")).changed() {
                            trigger_dual = Some(dual_on);
                        }
                    });
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(enabled, egui::Button::new("Làm mới danh sách"))
                        .clicked()
                    {
                        let core = self.backend.core.clone();
                        self.backend
                            .spawn_task(async move { core.phones_action(44, None).await });
                    }
                    if Self::primary_button(ui, "Ghép thiết bị mới", enabled).clicked() {
                        pair_new = true;
                    }
                });
            },
        );

        let mut phone_cmd: Option<(u8, u8)> = None;
        Self::draw_card(ui, Some("Thiết bị đã lưu"), None, |ui| {
            if self.phones.is_empty() {
                ui.label(
                    RichText::new("Chưa có dữ liệu thiết bị.")
                        .size(12.0)
                        .color(Color32::from_rgb(104, 103, 118)),
                );
            } else {
                for phone in &self.phones {
                    ui.horizontal(|ui| {
                        let name = if phone.name.is_empty() {
                            "Thiết bị"
                        } else {
                            &phone.name
                        };
                        let status = if phone.connected {
                            "Đang kết nối"
                        } else {
                            "Đã lưu"
                        };
                        ui.label(format!("{} · {} ({})", name, phone.mac, status));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let cmd = if phone.connected { 42u8 } else { 41u8 };
                            let label = if phone.connected { "Ngắt" } else { "Kết nối" };
                            if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
                                phone_cmd = Some((cmd, phone.num));
                            }
                        });
                    });
                    ui.separator();
                }
            }
        });

        ui.label(
            RichText::new(
                "Danh sách do tai nghe cung cấp. Ngắt thiết bị có thể làm gián đoạn âm thanh.",
            )
            .size(11.5)
            .color(Color32::from_rgb(104, 103, 118)),
        );

        if let Some(on) = trigger_dual {
            let plan_res = workflows::dual(&self.state, on);
            self.execute_plan(plan_res);
        }
        if pair_new {
            self.ask_confirm(
                "Cho tai nghe tìm thiết bị Bluetooth mới? Tiếp tục?",
                |app| {
                    let core = app.backend.core.clone();
                    app.backend
                        .spawn_task(async move { core.phones_action(43, None).await });
                },
            );
        }
        if let Some((cmd, num)) = phone_cmd {
            self.ask_confirm(
                "Thay đổi kết nối thiết bị này có thể ngắt âm thanh. Tiếp tục?",
                move |app| {
                    let core = app.backend.core.clone();
                    app.backend
                        .spawn_task(async move { core.phones_action(cmd, Some(num)).await });
                },
            );
        }
    }

    fn render_earbuds(&mut self, ui: &mut egui::Ui) {
        let enabled = self.is_enabled();
        let mut trigger_le_audio: Option<bool> = None;
        let mut trigger_codec: Option<(&'static str, bool)> = None;
        let mut find_req: Option<u8> = None;
        let mut reset_req = false;

        Self::draw_card(ui, Some("Âm thanh & cuộc gọi"), None, |ui| {
            let prompt_opts = [
                ("—", None),
                ("100%", Some((100u8, 0u8))),
                ("75%", Some((75u8, 1u8))),
                ("50%", Some((50u8, 2u8))),
                ("25%", Some((25u8, 3u8))),
                ("Tắt", Some((0u8, 4u8))),
            ];
            let prompt_avail = enabled && self.state.prompt.is_some();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Âm lượng thông báo")
                        .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wanted = prompt_opts[self.draft_prompt].1;
                    let can_apply = prompt_avail
                        && wanted.is_some()
                        && wanted.map(|(v, _)| v) != self.state.prompt;
                    if ui
                        .add_enabled(can_apply, egui::Button::new("Áp dụng"))
                        .clicked()
                    {
                        if let Some((wanted_val, wire_idx)) = wanted {
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_u8_value(56, 57, wire_idx, "prompt", wanted_val)
                                    .await
                            });
                        }
                    }
                    ui.add_enabled_ui(prompt_avail, |ui| {
                        let prev = self.draft_prompt;
                        egui::ComboBox::from_id_salt("choice_prompt")
                            .width(180.0)
                            .selected_text(prompt_opts[self.draft_prompt].0)
                            .show_ui(ui, |ui| {
                                for (i, (lbl, _)) in prompt_opts.iter().enumerate() {
                                    ui.selectable_value(&mut self.draft_prompt, i, *lbl);
                                }
                            });
                        if self.draft_prompt != prev {
                            self.dirty_choices.insert("prompt".to_string());
                        }
                    });
                });
            });

            let lang_text = match self.state.voice_language.as_deref() {
                Some("EN") => "English",
                Some("ZH") => "中文",
                Some("DE") => "Deutsch",
                Some("JA") => "日本語",
                _ => "EarFun Audio",
            };
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Gói giọng thông báo")
                            .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                    );
                    ui.label(
                        RichText::new(
                            "Cài bằng ứng dụng trên điện thoại; bản desktop chưa hỗ trợ.",
                        )
                        .size(11.5)
                        .color(Color32::from_rgb(104, 103, 118)),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(lang_text)
                            .size(12.5)
                            .color(Color32::from_rgb(104, 103, 118)),
                    );
                });
            });

            let mic_opts = [
                ("—", None),
                ("Tự động", Some(("AUTO", 0u8))),
                ("Tai trái", Some(("L", 1u8))),
                ("Tai phải", Some(("R", 2u8))),
            ];
            let mic_avail = enabled && self.state.mic.is_some();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Micro cuộc gọi")
                        .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wanted = mic_opts[self.draft_mic].1;
                    let can_apply = mic_avail
                        && wanted.is_some()
                        && wanted.map(|(k, _)| k) != self.state.mic.as_deref();
                    if ui
                        .add_enabled(can_apply, egui::Button::new("Áp dụng"))
                        .clicked()
                    {
                        if let Some((wanted_k, wire_idx)) = wanted {
                            let wanted_s = wanted_k.to_string();
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_string_value(48, 49, wire_idx, "mic", &wanted_s)
                                    .await
                            });
                        }
                    }
                    ui.add_enabled_ui(mic_avail, |ui| {
                        let prev = self.draft_mic;
                        egui::ComboBox::from_id_salt("choice_mic")
                            .width(180.0)
                            .selected_text(mic_opts[self.draft_mic].0)
                            .show_ui(ui, |ui| {
                                for (i, (lbl, _)) in mic_opts.iter().enumerate() {
                                    ui.selectable_value(&mut self.draft_mic, i, *lbl);
                                }
                            });
                        if self.draft_mic != prev {
                            self.dirty_choices.insert("mic".to_string());
                        }
                    });
                });
            });

            let hl_opts = ["—", "Tắt", "90 dB", "80 dB", "70 dB", "60 dB", "50 dB"];
            let hl_avail = enabled && self.state.hearing_limit.is_some();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Giới hạn âm lượng")
                        .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wanted = if self.draft_hearing_limit >= 1 {
                        Some((self.draft_hearing_limit - 1) as u8)
                    } else {
                        None
                    };
                    let can_apply =
                        hl_avail && wanted.is_some() && wanted != self.state.hearing_limit;
                    if ui
                        .add_enabled(can_apply, egui::Button::new("Áp dụng"))
                        .clicked()
                    {
                        if let Some(v) = wanted {
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_u8_value(83, 84, v, "hearing_limit", v).await
                            });
                        }
                    }
                    ui.add_enabled_ui(hl_avail, |ui| {
                        let prev = self.draft_hearing_limit;
                        egui::ComboBox::from_id_salt("choice_hl")
                            .width(180.0)
                            .selected_text(hl_opts[self.draft_hearing_limit])
                            .show_ui(ui, |ui| {
                                for (i, lbl) in hl_opts.iter().enumerate() {
                                    ui.selectable_value(&mut self.draft_hearing_limit, i, *lbl);
                                }
                            });
                        if self.draft_hearing_limit != prev {
                            self.dirty_choices.insert("hearing_limit".to_string());
                        }
                    });
                });
            });

            let bal_avail = enabled && self.state.balance.is_some();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Cân bằng trái/phải (50 = giữa)")
                        .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_apply = bal_avail && Some(self.draft_balance) != self.state.balance;
                    if ui
                        .add_enabled(can_apply, egui::Button::new("Áp dụng"))
                        .clicked()
                    {
                        let v = self.draft_balance;
                        let core = self.backend.core.clone();
                        self.backend.spawn_task(async move {
                            core.set_u8_value(85, 86, v, "balance", v).await
                        });
                    }
                    ui.add_enabled_ui(bal_avail, |ui| {
                        if ui
                            .add(egui::Slider::new(&mut self.draft_balance, 0..=100))
                            .changed()
                        {
                            self.balance_dirty = true;
                        }
                    });
                });
            });
        });

        Self::draw_card(
            ui,
            Some("Kết nối Bluetooth"),
            Some("Codec cần được cả hệ điều hành và bộ phát hỗ trợ."),
            |ui| {
                let le_opts = ["—", "Thông thường · Google Fast Pair", "LE Audio"];
                let le_avail = enabled && self.state.le_audio.is_some();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Chế độ Bluetooth")
                            .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let wanted = match self.draft_le_audio {
                            1 => Some(false),
                            2 => Some(true),
                            _ => None,
                        };
                        let can_apply =
                            le_avail && wanted.is_some() && wanted != self.state.le_audio;
                        if ui
                            .add_enabled(can_apply, egui::Button::new("Áp dụng"))
                            .clicked()
                        {
                            trigger_le_audio = wanted;
                        }
                        ui.add_enabled_ui(le_avail, |ui| {
                            let prev = self.draft_le_audio;
                            egui::ComboBox::from_id_salt("choice_le_audio")
                                .width(230.0)
                                .selected_text(le_opts[self.draft_le_audio])
                                .show_ui(ui, |ui| {
                                    for (i, lbl) in le_opts.iter().enumerate() {
                                        ui.selectable_value(&mut self.draft_le_audio, i, *lbl);
                                    }
                                });
                            if self.draft_le_audio != prev {
                                self.dirty_choices.insert("le_audio".to_string());
                            }
                        });
                    });
                });

                let codecs_known = ["aptx", "adaptive", "lossless", "ldac", "dual"]
                    .iter()
                    .all(|k| self.state.get_bool(k).is_some());
                for (key, label) in [
                    ("ldac", "LDAC"),
                    ("aptx", "aptX"),
                    ("adaptive", "aptX Adaptive"),
                    ("lossless", "aptX Lossless"),
                ] {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(label)
                                .font(FontId::new(13.0, FontFamily::Name("Medium".into()))),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut val = self.state.get_bool(key).unwrap_or(false);
                            if ui
                                .add_enabled(
                                    enabled && codecs_known,
                                    egui::Checkbox::new(&mut val, ""),
                                )
                                .changed()
                            {
                                trigger_codec = Some((key, val));
                            }
                        });
                    });
                }
            },
        );

        Self::draw_card(
            ui,
            Some("Tìm tai nghe"),
            Some("Tháo tai nghe khỏi tai. Cần giữ kết nối để gửi lệnh dừng sau 10 giây."),
            |ui| {
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(enabled, egui::Button::new("Phát tiếng tai trái"))
                        .clicked()
                    {
                        find_req = Some(1);
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("Phát tiếng tai phải"))
                        .clicked()
                    {
                        find_req = Some(2);
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("Dừng tìm"))
                        .clicked()
                    {
                        let core = self.backend.core.clone();
                        self.backend.spawn_task(async move { core.find(0).await });
                    }
                });
            },
        );

        Self::draw_card(
            ui,
            Some("Cài đặt nâng cao"),
            Some("Khôi phục sẽ xóa thiết lập và các thiết bị đã ghép trên tai nghe."),
            |ui| {
                if Self::danger_button(ui, "Khôi phục cài đặt gốc…", enabled).clicked() {
                    reset_req = true;
                }
            },
        );

        if let Some(on) = trigger_le_audio {
            let plan_res = workflows::restart_setting(&self.state, "le_audio", on);
            self.execute_plan(plan_res);
        }
        if let Some((codec_key, on)) = trigger_codec {
            let mut wanted = BTreeMap::new();
            for &k in workflows::CODECS {
                if let Some(v) = self.state.get_bool(k) {
                    wanted.insert(k.to_string(), v);
                }
            }
            wanted.insert(codec_key.to_string(), on);
            let plan_res = workflows::codecs(&self.state, &wanted);
            self.execute_plan(plan_res);
        }
        if let Some(mask) = find_req {
            self.ask_confirm(
                "Bạn đã tháo tai nghe khỏi tai? Giữ kết nối để gửi dừng sau 10 giây.",
                move |app| {
                    let core = app.backend.core.clone();
                    app.backend
                        .spawn_task(async move { core.find(mask).await });
                },
            );
        }
        if reset_req {
            self.ask_confirm(
                "Xóa cài đặt và các kết nối đã ghép trên tai nghe? Bạn sẽ cần ghép lại Bluetooth.",
                |app| {
                    let core = app.backend.core.clone();
                    app.backend
                        .spawn_task(async move { core.direct(54, &[]).await });
                },
            );
        }
    }

    fn render_application(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let enabled = self.is_enabled();

        Self::draw_card(ui, Some("Tùy chọn ứng dụng"), None, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Tự kết nối lại khi ứng dụng đang chạy")
                        .font(FontId::new(13.5, FontFamily::Name("Medium".into()))),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .checkbox(&mut self.settings.data.auto_connect, "")
                        .changed()
                    {
                        self.save_settings();
                    }
                });
            });
            ui.label(
                RichText::new("Đóng cửa sổ sẽ thoát ứng dụng và giải phóng phiên điều khiển BLE.")
                    .size(11.5)
                    .color(Color32::from_rgb(104, 103, 118)),
            );
        });

        Self::draw_card(
            ui,
            Some("Thông tin & nhật ký"),
            Some("Chỉ lưu trên máy. Nhật ký có thể chứa tên và địa chỉ thiết bị Bluetooth."),
            |ui| {
                let fw = self.state.version.as_deref().unwrap_or("—");
                let codec = self.state.codec.as_deref().unwrap_or("—");
                ui.label(
                    RichText::new(format!("Firmware {}  /  Codec {}", fw, codec))
                        .size(12.0)
                        .color(Color32::from_rgb(104, 103, 118)),
                );

                egui::Frame::none()
                    .fill(Color32::from_rgb(248, 250, 254))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(222, 228, 239)))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(8.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .max_height(180.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let log_str = self.logs.join("\n");
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(log_str)
                                            .monospace()
                                            .size(11.0)
                                            .color(Color32::from_rgb(104, 103, 118)),
                                    )
                                    .selectable(true),
                                );
                            });
                    });

                ui.horizontal(|ui| {
                    if ui.button("Xuất nhật ký…").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Lưu nhật ký Bluetooth")
                            .set_file_name("earfun-bluetooth.log")
                            .save_file()
                        {
                            match std::fs::write(&path, self.logs.join("\n")) {
                                Ok(()) => self.show_notice("Đã xuất nhật ký.", false),
                                Err(e) => self.show_notice(e.to_string(), true),
                            }
                        }
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("Đọc lại trạng thái"))
                        .clicked()
                    {
                        let core = self.backend.core.clone();
                        self.backend.spawn_task(async move { core.query().await });
                    }
                });
            },
        );

        Self::draw_card(
            ui,
            Some("EarFun Studio 1.3 (Rust)"),
            Some("Ứng dụng độc lập cho Windows và Ubuntu. Viết bằng Rust (eframe/egui + btleplug)."),
            |ui| {
                ui.label(
                    RichText::new(
                        "EQ cá nhân hóa theo thính lực, firmware/gói giọng nói, tham gia Auracast, AI/dịch thuật và hồ sơ đám mây hiện vẫn cần EarFun Audio chính hãng.",
                    )
                    .size(12.0)
                    .color(Color32::from_rgb(104, 103, 118)),
                );
                ui.horizontal(|ui| {
                    if ui.button("Nguồn và giới hạn").clicked() {
                        let _ = audio::open_url_or_path("https://github.com/lnb2x/earfun-studio");
                    }
                    if ui.button("Thoát ứng dụng").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            },
        );
    }

    fn render_confirm_modal(&mut self, ctx: &egui::Context) {
        if self.confirm_modal.is_none() {
            return;
        }
        let mut choice: Option<bool> = None;
        let msg = self.confirm_modal.as_ref().unwrap().message.clone();

        egui::Window::new("Xác nhận thay đổi")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .fixed_size(Vec2::new(420.0, 140.0))
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.label(RichText::new(&msg).size(13.0));
                ui.add_space(16.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if Self::primary_button(ui, "Tiếp tục", true).clicked() {
                            choice = Some(true);
                        }
                        if ui.button("Hủy").clicked() {
                            choice = Some(false);
                        }
                    });
                });
            });

        if let Some(confirmed) = choice {
            let modal = self.confirm_modal.take().unwrap();
            if confirmed {
                (modal.action)(self);
            } else {
                let st = self.state.clone();
                self.sync_state_to_drafts(&st);
            }
        }
    }
}

impl eframe::App for StudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.process_events();

        if self.smoke {
            if self.smoke_timer.elapsed() >= Duration::from_millis(120) {
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
            ctx.request_repaint_after(Duration::from_millis(50));
        } else {
            if self.last_reconnect_check.elapsed() >= Duration::from_secs(20) {
                self.last_reconnect_check = Instant::now();
                if self.settings.data.auto_connect
                    && !self.user_disconnected
                    && !self.is_connected()
                    && self.busy == 0
                    && !self.address.is_empty()
                {
                    let addr = self.address.clone();
                    let core = self.backend.core.clone();
                    self.backend
                        .spawn_task(async move { core.connect(&addr).await });
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

        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .exact_width(220.0)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 16.0,
                right: 8.0,
                top: 16.0,
                bottom: 16.0,
            }))
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        egui::TopBottomPanel::bottom("footer_panel")
            .resizable(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 12.0,
                right: 20.0,
                top: 6.0,
                bottom: 12.0,
            }))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let color = if self.notice_is_error {
                        Color32::from_rgb(170, 75, 72)
                    } else {
                        Color32::from_rgb(104, 103, 118)
                    };
                    ui.label(RichText::new("✓").size(13.0).color(color));
                    ui.add(
                        egui::Label::new(RichText::new(&self.notice).size(11.5).color(color))
                            .selectable(true),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 10.0,
                right: 20.0,
                top: 16.0,
                bottom: 6.0,
            }))
            .show(ctx, |ui| {
                self.render_header_and_summary(ui);
                ui.add_space(10.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.page {
                        Page::Overview => self.render_overview(ui),
                        Page::Equalizer => self.render_equalizer(ui),
                        Page::Gestures => self.render_gestures(ui),
                        Page::Multipoint => self.render_multipoint(ui),
                        Page::Earbuds => self.render_earbuds(ui),
                        Page::Application => self.render_application(ui, ctx),
                    });
            });

        self.render_confirm_modal(ctx);
    }
}

pub fn run_headless_smoke_test() {
    let mut app = StudioApp::new(None, true, false);
    for page in Page::ALL {
        app.page = page;
        println!("UI OK: {}", page.title());
    }
    app.settings.data.eq = app.eq_levels.clone();
    let _ = app.settings.save();
    app.backend.shutdown_blocking();
}
