//! Cross-platform GUI for EarFun Studio (Windows & Ubuntu) built with eframe/egui.
//! Faithfully implements the EarFun Studio light lavender-blue design system from `assets/theme.css`.

use crate::audio;
use crate::backend::{BackendEvent, BackendHandle, ConnectionStatus};
use crate::presets::{self, PresetOption};
use crate::protocol::{self as p, DeviceState, PhoneEntry};
use crate::settings::Settings;
use crate::workflows::{self, Plan};
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Pos2, Rect, RichText, Rounding,
    Sense, Stroke, Vec2,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

const FONT_REGULAR: &[u8] = include_bytes!("../assets/fonts/BeVietnamPro-Regular.ttf");
const FONT_MEDIUM: &[u8] = include_bytes!("../assets/fonts/BeVietnamPro-Medium.ttf");
const FONT_SEMIBOLD: &[u8] = include_bytes!("../assets/fonts/BeVietnamPro-SemiBold.ttf");

// Theme palette matching assets/theme.css
const BG_TOP_LEFT: Color32 = Color32::from_rgb(223, 232, 245); // #dfe8f5
const BG_MID: Color32 = Color32::from_rgb(229, 237, 247); // #e5edf7
const BG_BOT_RIGHT: Color32 = Color32::from_rgb(234, 230, 246); // #eae6f6

const TEXT_PRIMARY: Color32 = Color32::from_rgb(33, 34, 35); // #212223
const TEXT_SUBTITLE: Color32 = Color32::from_rgb(104, 103, 118); // #686776
const TEXT_MUTED: Color32 = Color32::from_rgb(142, 148, 166);
const ACCENT_BLUE: Color32 = Color32::from_rgb(102, 133, 209); // #6685d1
const PRIMARY_BTN_BG: Color32 = Color32::from_rgb(85, 115, 188); // #5573bc
const CARD_BORDER: Color32 = Color32::from_rgb(237, 241, 248); // #edf1f8
const ROW_DIVIDER: Color32 = Color32::from_rgb(241, 244, 250);
const CTRL_BG: Color32 = Color32::from_rgb(244, 246, 252); // #f4f6fc
const CTRL_BORDER: Color32 = Color32::from_rgb(224, 230, 240); // #e0e6f0
const DANGER_BG: Color32 = Color32::from_rgb(255, 244, 243); // #fff4f3
const DANGER_BORDER: Color32 = Color32::from_rgb(241, 217, 214); // #f1d9d6
const DANGER_TEXT: Color32 = Color32::from_rgb(170, 75, 72); // #aa4b48

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Overview,
    Eq,
    Touch,
    Devices,
    Settings,
    App,
    Earbud,
    Case,
    Normal,
    Anc,
    Ambient,
    Check,
    Refresh,
    Bluetooth,
}

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

pub fn build_app_icon() -> egui::IconData {
    let size = 64usize;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 - 31.5;
            let fy = y as f32 - 31.5;
            let dist = (fx.abs().max(fy.abs())) - 18.0;
            let corner_dist =
                ((fx.abs() - 18.0).max(0.0).powi(2) + (fy.abs() - 18.0).max(0.0).powi(2)).sqrt();
            let idx = (y * size + x) * 4;
            if corner_dist <= 12.0 || dist <= 0.0 {
                // Soft blue-indigo rounded square
                rgba[idx] = 85;
                rgba[idx + 1] = 115;
                rgba[idx + 2] = 188;
                rgba[idx + 3] = 255;

                // Three vertical white EQ bars inside
                let bar1 = (x >= 18 && x <= 22) && (y >= 20 && y <= 44);
                let bar2 = (x >= 30 && x <= 34) && (y >= 16 && y <= 48);
                let bar3 = (x >= 42 && x <= 46) && (y >= 22 && y <= 42);
                if bar1 || bar2 || bar3 {
                    rgba[idx] = 255;
                    rgba[idx + 1] = 255;
                    rgba[idx + 2] = 255;
                }
            }
        }
    }
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

/// Draw crisp vector icons matching `assets/icons/*.svg` without relying on emoji fonts.
pub fn draw_vector_icon(painter: &egui::Painter, rect: Rect, kind: IconKind, color: Color32) {
    let stroke = Stroke::new(1.65_f32, color);
    let map = |x: f32, y: f32| -> Pos2 {
        Pos2::new(
            rect.left() + (x / 24.0) * rect.width(),
            rect.top() + (y / 24.0) * rect.height(),
        )
    };
    let s = (rect.width().min(rect.height())) / 24.0;

    match kind {
        IconKind::Overview => {
            // 4 grid squares from overview.svg
            for (x0, y0, x1, y1) in [
                (3.0, 3.0, 10.0, 10.0),
                (14.0, 3.0, 21.0, 10.0),
                (3.0, 14.0, 10.0, 21.0),
                (14.0, 14.0, 21.0, 21.0),
            ] {
                let r = Rect::from_min_max(map(x0, y0), map(x1, y1));
                painter.rect_stroke(r, Rounding::same(1.8 * s), stroke);
            }
        }
        IconKind::Eq => {
            // 3 faders from eq.svg
            painter.line_segment([map(5.0, 3.0), map(5.0, 9.0)], stroke);
            painter.line_segment([map(5.0, 13.0), map(5.0, 21.0)], stroke);
            painter.rect_stroke(
                Rect::from_min_max(map(2.5, 9.0), map(7.5, 13.0)),
                Rounding::same(1.2 * s),
                stroke,
            );

            painter.line_segment([map(12.0, 3.0), map(12.0, 14.0)], stroke);
            painter.line_segment([map(12.0, 18.0), map(12.0, 21.0)], stroke);
            painter.rect_stroke(
                Rect::from_min_max(map(9.5, 14.0), map(14.5, 18.0)),
                Rounding::same(1.2 * s),
                stroke,
            );

            painter.line_segment([map(19.0, 3.0), map(19.0, 6.0)], stroke);
            painter.line_segment([map(19.0, 10.0), map(19.0, 21.0)], stroke);
            painter.rect_stroke(
                Rect::from_min_max(map(16.5, 6.0), map(21.5, 10.0)),
                Rounding::same(1.2 * s),
                stroke,
            );
        }
        IconKind::Touch => {
            // Concentric touch waves + finger tap
            let center = map(11.0, 7.5);
            painter.circle_stroke(center, 3.2 * s, stroke);
            painter.line_segment([map(11.0, 8.0), map(11.0, 19.0)], stroke);
            painter.line_segment([map(11.0, 13.0), map(16.5, 14.5)], stroke);
            painter.line_segment([map(16.5, 14.5), map(16.0, 20.5)], stroke);
            painter.line_segment([map(16.0, 20.5), map(9.0, 20.5)], stroke);
            painter.line_segment([map(9.0, 20.5), map(6.0, 16.0)], stroke);
        }
        IconKind::Devices => {
            // Monitor + phone from devices.svg
            let mon = Rect::from_min_max(map(3.0, 4.0), map(15.0, 15.5));
            painter.rect_stroke(mon, Rounding::same(1.5 * s), stroke);
            painter.line_segment([map(9.0, 15.5), map(9.0, 19.5)], stroke);
            painter.line_segment([map(6.5, 19.5), map(11.5, 19.5)], stroke);

            let phone = Rect::from_min_max(map(16.5, 9.0), map(21.5, 21.0));
            painter.rect_stroke(phone, Rounding::same(1.5 * s), stroke);
            painter.line_segment([map(18.5, 18.8), map(19.5, 18.8)], stroke);
        }
        IconKind::Settings => {
            // Horizontal sliders from settings.svg
            painter.line_segment([map(3.0, 5.0), map(7.0, 5.0)], stroke);
            painter.line_segment([map(11.0, 5.0), map(21.0, 5.0)], stroke);
            painter.line_segment([map(7.0, 2.5), map(7.0, 7.5)], stroke);

            painter.line_segment([map(3.0, 12.0), map(15.0, 12.0)], stroke);
            painter.line_segment([map(19.0, 12.0), map(21.0, 12.0)], stroke);
            painter.line_segment([map(15.0, 9.5), map(15.0, 14.5)], stroke);

            painter.line_segment([map(3.0, 19.0), map(9.0, 19.0)], stroke);
            painter.line_segment([map(13.0, 19.0), map(21.0, 19.0)], stroke);
            painter.line_segment([map(9.0, 16.5), map(9.0, 21.5)], stroke);
        }
        IconKind::App => {
            // Window layout from app.svg
            let win = Rect::from_min_max(map(3.0, 3.0), map(21.0, 21.0));
            painter.rect_stroke(win, Rounding::same(2.2 * s), stroke);
            painter.line_segment([map(3.0, 8.0), map(21.0, 8.0)], stroke);
            painter.line_segment([map(8.5, 8.0), map(8.5, 21.0)], stroke);
        }
        IconKind::Earbud => {
            // Earbud head + stem from earbud.svg
            painter.circle_stroke(map(12.5, 7.5), 4.6 * s, stroke);
            painter.circle_stroke(map(8.2, 8.2), 2.3 * s, stroke);
            let stem = Rect::from_min_max(map(10.0, 11.5), map(14.0, 21.5));
            painter.rect_stroke(stem, Rounding::same(2.0 * s), stroke);
        }
        IconKind::Case => {
            // Charging case from case.svg
            let r = Rect::from_min_max(map(2.5, 6.0), map(21.5, 20.5));
            painter.rect_stroke(r, Rounding::same(3.2 * s), stroke);
            painter.line_segment([map(2.5, 11.2), map(21.5, 11.2)], stroke);
            painter.line_segment([map(10.0, 15.8), map(14.0, 15.8)], stroke);
        }
        IconKind::Normal => {
            // Dual smooth sound waves from normal.svg
            for y_base in [8.5_f32, 15.5_f32] {
                let mut prev = map(3.0, y_base);
                for step in 1..=24 {
                    let t = step as f32 / 24.0;
                    let x = 3.0 + t * 18.0;
                    let y = y_base - (t * std::f32::consts::TAU * 1.5).sin() * 2.5;
                    let cur = map(x, y);
                    painter.line_segment([prev, cur], stroke);
                    prev = cur;
                }
            }
        }
        IconKind::Anc => {
            // Shield + wave from anc.svg
            let pts = [
                map(12.0, 2.5),
                map(20.5, 6.0),
                map(19.0, 15.0),
                map(12.0, 21.5),
                map(5.0, 15.0),
                map(3.5, 6.0),
                map(12.0, 2.5),
            ];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            let wave = [
                map(7.5, 12.0),
                map(9.8, 12.0),
                map(11.0, 8.2),
                map(13.0, 15.8),
                map(14.2, 12.0),
                map(16.5, 12.0),
            ];
            for w in wave.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        IconKind::Ambient => {
            // Sound wave + ear contour from ambient.svg
            painter.circle_stroke(map(12.0, 11.5), 4.8 * s, stroke);
            painter.circle_stroke(map(12.0, 11.5), 1.8 * s, stroke);
            painter.line_segment([map(10.0, 16.2), map(9.0, 20.5)], stroke);
            painter.line_segment([map(9.0, 20.5), map(12.5, 20.5)], stroke);
        }
        IconKind::Check => {
            painter.line_segment([map(4.5, 12.5), map(9.5, 17.5)], stroke);
            painter.line_segment([map(9.5, 17.5), map(20.0, 6.0)], stroke);
        }
        IconKind::Refresh => {
            painter.circle_stroke(map(12.0, 12.0), 7.2 * s, stroke);
            painter.line_segment([map(16.0, 5.5), map(19.5, 5.5)], stroke);
            painter.line_segment([map(19.5, 5.5), map(19.5, 9.0)], stroke);
        }
        IconKind::Bluetooth => {
            let pts = [
                map(7.5, 6.5),
                map(17.0, 16.0),
                map(12.0, 21.0),
                map(12.0, 3.0),
                map(17.0, 8.0),
                map(7.5, 17.5),
            ];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
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

        let mut medium_list = vec!["BeVietnamPro-Medium".to_owned()];
        if let Some(prop) = fonts.families.get(&FontFamily::Proportional) {
            medium_list.extend(prop.iter().cloned());
        }
        fonts
            .families
            .insert(FontFamily::Name("Medium".into()), medium_list);

        let mut semibold_list = vec!["BeVietnamPro-SemiBold".to_owned()];
        if let Some(prop) = fonts.families.get(&FontFamily::Proportional) {
            semibold_list.extend(prop.iter().cloned());
        }
        fonts
            .families
            .insert(FontFamily::Name("SemiBold".into()), semibold_list);

        ctx.set_fonts(fonts);
    }

    fn configure_style(ctx: &egui::Context) {
        ctx.set_theme(egui::ThemePreference::Light);
        let mut visuals = egui::Visuals::light();
        visuals.dark_mode = false;
        visuals.panel_fill = BG_MID;
        visuals.window_fill = Color32::WHITE;
        visuals.window_stroke = Stroke::new(1.0_f32, Color32::from_rgb(220, 227, 240));
        visuals.window_rounding = Rounding::same(16.0);
        visuals.override_text_color = Some(TEXT_PRIMARY);

        // Non-interactive frames & borders
        visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(247, 249, 253);
        visuals.widgets.noninteractive.weak_bg_fill = Color32::from_rgb(247, 249, 253);
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, CTRL_BORDER);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_PRIMARY);
        visuals.widgets.noninteractive.rounding = Rounding::same(10.0);

        // Inactive controls (buttons, dropdowns, sliders)
        visuals.widgets.inactive.bg_fill = CTRL_BG;
        visuals.widgets.inactive.weak_bg_fill = CTRL_BG;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, CTRL_BORDER);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.2_f32, TEXT_PRIMARY);
        visuals.widgets.inactive.rounding = Rounding::same(10.0);

        // Hovered controls
        visuals.widgets.hovered.bg_fill = Color32::from_rgb(235, 240, 251);
        visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(235, 240, 251);
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(158, 163, 215));
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.4_f32, TEXT_PRIMARY);
        visuals.widgets.hovered.rounding = Rounding::same(10.0);

        // Active / pressed controls
        visuals.widgets.active.bg_fill = Color32::from_rgb(228, 234, 251);
        visuals.widgets.active.weak_bg_fill = Color32::from_rgb(228, 234, 251);
        visuals.widgets.active.bg_stroke = Stroke::new(1.2_f32, ACCENT_BLUE);
        visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, Color32::from_rgb(64, 90, 154));
        visuals.widgets.active.rounding = Rounding::same(10.0);

        // Open ComboBox / menu state
        visuals.widgets.open.bg_fill = Color32::from_rgb(232, 238, 252);
        visuals.widgets.open.weak_bg_fill = Color32::from_rgb(232, 238, 252);
        visuals.widgets.open.bg_stroke = Stroke::new(1.2_f32, ACCENT_BLUE);
        visuals.widgets.open.fg_stroke = Stroke::new(1.4_f32, TEXT_PRIMARY);
        visuals.widgets.open.rounding = Rounding::same(10.0);

        visuals.selection.bg_fill = Color32::from_rgb(228, 234, 251);
        visuals.selection.stroke = Stroke::new(1.2_f32, Color32::from_rgb(64, 90, 154));
        visuals.slider_trailing_fill = true;

        ctx.set_visuals(visuals);

        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(13.0, 8.0);
        style.spacing.combo_height = 34.0;
        style.spacing.interact_size = Vec2::new(40.0, 32.0);
        ctx.set_style(style);
    }

    /// Paint the signature `linear-gradient(135deg, #dfe8f5 0%, #e5edf7 45%, #eae6f6 100%)` background.
    fn paint_window_background(ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        let mut mesh = egui::epaint::Mesh::default();
        mesh.vertices.push(egui::epaint::Vertex {
            pos: screen.left_top(),
            uv: egui::epaint::WHITE_UV,
            color: BG_TOP_LEFT,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: screen.right_top(),
            uv: egui::epaint::WHITE_UV,
            color: BG_MID,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: screen.right_bottom(),
            uv: egui::epaint::WHITE_UV,
            color: BG_BOT_RIGHT,
        });
        mesh.vertices.push(egui::epaint::Vertex {
            pos: screen.left_bottom(),
            uv: egui::epaint::WHITE_UV,
            color: BG_MID,
        });
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        painter.add(egui::Shape::mesh(mesh));
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
        if self.smoke {
            return;
        }
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
                    if msg == "Đã đọc trạng thái trực tiếp từ tai nghe." && self.busy == 0
                    {
                        continue;
                    }
                    self.show_notice(msg, false);
                }
                BackendEvent::Error(msg) => {
                    self.eq_sending_levels = None;
                    self.show_notice(msg, true);
                }
                BackendEvent::EqSent { generation, levels } => {
                    if generation == self.connection_generation && self.is_connected() {
                        self.eq_sending_levels = None;
                        self.eq_sent_levels = Some(levels);
                    }
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
            if self.preset_options[self.selected_preset_idx]
                .levels
                .as_ref()
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

    fn card_shadow() -> egui::epaint::Shadow {
        egui::epaint::Shadow {
            offset: Vec2::new(0.0, 4.0),
            blur: 14.0,
            spread: 0.0,
            color: Color32::from_rgba_unmultiplied(111, 130, 169, 18),
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
            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
            .rounding(Rounding::same(20.0))
            .shadow(Self::card_shadow())
            .inner_margin(egui::Margin::same(20.0))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                if let Some(t) = title {
                    ui.label(
                        RichText::new(t)
                            .font(FontId::new(17.0, FontFamily::Name("SemiBold".into())))
                            .color(TEXT_PRIMARY),
                    );
                    if let Some(sub) = subtitle {
                        ui.add_space(2.0);
                        ui.label(RichText::new(sub).size(12.0).color(TEXT_SUBTITLE));
                    }
                    ui.add_space(10.0);
                }
                add_contents(ui)
            })
            .inner
    }

    fn draw_row_divider(ui: &mut egui::Ui) {
        ui.add_space(4.0);
        let (rect, _) =
            ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
        ui.painter().line_segment(
            [rect.left_center(), rect.right_center()],
            Stroke::new(1.0_f32, ROW_DIVIDER),
        );
        ui.add_space(4.0);
    }

    /// Pill toggle switch matching `switch` in `assets/theme.css` (`38x21px`, `#d9dfeb` -> `#6685d1`).
    fn toggle_switch(ui: &mut egui::Ui, on: &mut bool, enabled: bool) -> bool {
        let desired_size = Vec2::new(40.0, 22.0);
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, mut response) = ui.allocate_exact_size(desired_size, sense);
        let mut changed = false;
        if enabled && response.clicked() {
            *on = !*on;
            changed = true;
            response.mark_changed();
        }
        if !enabled {
            response = response.on_hover_text("Kết nối tai nghe để thay đổi thiết lập này");
        }

        let painter = ui.painter();
        let radius = rect.height() * 0.5;
        let (bg_fill, border_color) = if *on {
            if enabled {
                (ACCENT_BLUE, PRIMARY_BTN_BG)
            } else {
                (
                    Color32::from_rgb(174, 190, 230),
                    Color32::from_rgb(160, 178, 220),
                )
            }
        } else if enabled {
            if response.hovered() {
                (
                    Color32::from_rgb(206, 214, 230),
                    Color32::from_rgb(192, 201, 219),
                )
            } else {
                (
                    Color32::from_rgb(217, 223, 235), // #d9dfeb
                    Color32::from_rgb(205, 213, 227), // #d2d9e6
                )
            }
        } else {
            (
                Color32::from_rgb(228, 233, 243),
                Color32::from_rgb(214, 221, 234),
            )
        };

        painter.rect(
            rect,
            Rounding::same(radius),
            bg_fill,
            Stroke::new(1.0_f32, border_color),
        );

        let thumb_r = radius - 2.5;
        let thumb_x = if *on {
            rect.right() - radius
        } else {
            rect.left() + radius
        };
        let thumb_color = if enabled {
            Color32::WHITE
        } else {
            Color32::from_rgb(249, 250, 253)
        };
        painter.circle_filled(Pos2::new(thumb_x, rect.center().y), thumb_r, thumb_color);

        changed
    }

    fn primary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
        let (fill, border, fg) = if enabled {
            (PRIMARY_BTN_BG, ACCENT_BLUE, Color32::WHITE)
        } else {
            (
                Color32::from_rgb(205, 214, 235),
                Color32::from_rgb(194, 205, 230),
                Color32::WHITE,
            )
        };
        let btn = egui::Button::new(
            RichText::new(text)
                .font(FontId::new(13.0, FontFamily::Name("Medium".into())))
                .color(fg),
        )
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border))
        .rounding(Rounding::same(11.0))
        .min_size(Vec2::new(0.0, 32.0));
        let resp = ui.add_enabled(enabled, btn);
        if resp.hovered() && enabled {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        resp
    }

    fn action_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
        let (fill, border, fg) = if enabled {
            (
                Color32::from_rgb(233, 238, 252),
                Color32::from_rgb(168, 182, 224),
                Color32::from_rgb(56, 80, 142),
            )
        } else {
            (
                Color32::from_rgb(245, 247, 252),
                Color32::from_rgb(228, 233, 243),
                TEXT_MUTED,
            )
        };
        let btn = egui::Button::new(
            RichText::new(text)
                .font(FontId::new(12.5, FontFamily::Name("Medium".into())))
                .color(fg),
        )
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border))
        .rounding(Rounding::same(10.0))
        .min_size(Vec2::new(74.0, 32.0));
        ui.add_enabled(enabled, btn)
    }

    fn secondary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
        let (fill, border, fg) = if enabled {
            (CTRL_BG, CTRL_BORDER, TEXT_PRIMARY)
        } else {
            (
                Color32::from_rgb(246, 248, 252),
                Color32::from_rgb(230, 235, 244),
                TEXT_MUTED,
            )
        };
        let btn = egui::Button::new(
            RichText::new(text)
                .font(FontId::new(12.5, FontFamily::Name("Medium".into())))
                .color(fg),
        )
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border))
        .rounding(Rounding::same(10.0))
        .min_size(Vec2::new(0.0, 32.0));
        ui.add_enabled(enabled, btn)
    }

    fn danger_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
        let fg = if enabled {
            DANGER_TEXT
        } else {
            Color32::from_rgb(196, 146, 144)
        };
        let btn = egui::Button::new(
            RichText::new(text)
                .font(FontId::new(13.0, FontFamily::Name("Medium".into())))
                .color(fg),
        )
        .fill(DANGER_BG)
        .stroke(Stroke::new(1.0_f32, DANGER_BORDER))
        .rounding(Rounding::same(11.0))
        .min_size(Vec2::new(0.0, 34.0));
        ui.add_enabled(enabled, btn)
    }

    fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 238))
            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
            .rounding(Rounding::same(22.0))
            .shadow(Self::card_shadow())
            .inner_margin(egui::Margin::same(14.0))
            .show(ui, |ui| {
                ui.set_min_height(ui.available_height());

                // Brand block matching `.brand-block` & `.brand-icon` in theme.css
                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(4.0);
                    let (badge_rect, _) =
                        ui.allocate_exact_size(Vec2::new(42.0, 42.0), Sense::hover());
                    ui.painter().rect_filled(
                        badge_rect,
                        Rounding::same(13.0),
                        Color32::from_rgb(230, 236, 251), // #e6ecfb
                    );
                    let icon_rect = badge_rect.shrink(9.0);
                    draw_vector_icon(ui.painter(), icon_rect, IconKind::Eq, PRIMARY_BTN_BG);

                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new("EarFun Studio")
                                .font(FontId::new(15.5, FontFamily::Name("SemiBold".into())))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(RichText::new("Air Pro 4").size(11.5).color(TEXT_SUBTITLE));
                    });
                });

                ui.add_space(22.0);

                // Navigation buttons with custom vector icons
                for nav_page in Page::ALL {
                    let active = self.page == nav_page;
                    let desired = Vec2::new(ui.available_width(), 42.0);
                    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
                    let hovered = resp.hovered();

                    let bg_fill = if active {
                        Color32::from_rgb(232, 237, 249) // #e8edf9
                    } else if hovered {
                        Color32::from_rgb(242, 245, 252) // #f2f5fc
                    } else {
                        Color32::TRANSPARENT
                    };
                    let stroke = if active {
                        Stroke::new(1.0_f32, Color32::from_rgb(198, 209, 236)) // #c6d1ec
                    } else {
                        Stroke::NONE
                    };
                    ui.painter()
                        .rect(rect, Rounding::same(12.0), bg_fill, stroke);

                    let icon_color = if active {
                        Color32::from_rgb(66, 91, 152) // #425b98
                    } else {
                        ACCENT_BLUE
                    };
                    let text_color = if active {
                        Color32::from_rgb(66, 91, 152)
                    } else if hovered {
                        TEXT_PRIMARY
                    } else {
                        Color32::from_rgb(110, 114, 130) // #6e7282
                    };
                    let font = if active {
                        FontId::new(13.5, FontFamily::Name("SemiBold".into()))
                    } else {
                        FontId::new(13.5, FontFamily::Proportional)
                    };

                    let icon_rect = Rect::from_min_size(
                        Pos2::new(rect.left() + 13.0, rect.center().y - 9.5),
                        Vec2::new(19.0, 19.0),
                    );
                    draw_vector_icon(ui.painter(), icon_rect, nav_page.icon(), icon_color);

                    ui.painter().text(
                        Pos2::new(rect.left() + 43.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        nav_page.title(),
                        font,
                        text_color,
                    );

                    if resp.on_hover_text(nav_page.subtitle()).clicked() {
                        self.page = nav_page;
                    }
                    ui.add_space(2.0);
                }

                // Bottom status & system Bluetooth shortcut
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(2.0);
                    ui.label(
                        RichText::new(concat!("EarFun Studio ", env!("CARGO_PKG_VERSION")))
                            .size(10.5)
                            .color(TEXT_MUTED),
                    );
                    let status_text = match self.connection {
                        ConnectionStatus::Connected => "Bluetooth LE • Đã kết nối",
                        ConnectionStatus::Connecting => "Bluetooth LE • Đang kết nối…",
                        ConnectionStatus::Disconnected => "Bluetooth LE • Chưa kết nối",
                    };
                    ui.label(RichText::new(status_text).size(11.0).color(TEXT_SUBTITLE));
                    ui.add_space(4.0);

                    let sys_bt_label = if cfg!(target_os = "windows") {
                        "Bluetooth Windows"
                    } else {
                        "Bluetooth Ubuntu"
                    };
                    if Self::secondary_button(ui, sys_bt_label, true).clicked() {
                        if let Err(e) = audio::open_bluetooth_settings() {
                            self.show_notice(e, true);
                        }
                    }
                });
            });
    }

    fn render_header_and_summary(&mut self, ui: &mut egui::Ui) {
        ui.label(
            RichText::new(self.page.title())
                .font(FontId::new(26.0, FontFamily::Name("SemiBold".into())))
                .color(TEXT_PRIMARY),
        );
        ui.label(
            RichText::new(self.page.subtitle())
                .size(12.5)
                .color(TEXT_SUBTITLE),
        );
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            let available = self.busy == 0 && self.connection == ConnectionStatus::Disconnected;
            ui.add_enabled_ui(available, |ui| {
                let label = self
                    .devices
                    .get(self.selected_device_idx)
                    .map(|(_, name)| name.as_str())
                    .unwrap_or("Chọn tai nghe");
                egui::ComboBox::from_id_salt("device_picker")
                    .width(222.0)
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        for (i, (_, name)) in self.devices.iter().enumerate() {
                            ui.selectable_value(&mut self.selected_device_idx, i, name);
                        }
                    });
            });
            if Self::secondary_button(ui, "Tìm thiết bị", available).clicked() {
                let core = self.backend.core.clone();
                self.backend.spawn_task(async move { core.scan().await });
            }
            if self.connection == ConnectionStatus::Connecting {
                ui.spinner();
            }
        });
        ui.add_space(10.0);

        // 5-column homogeneous summary strip:
        // Columns 0..2 (2/5 width) = Device card; Columns 2, 3, 4 (1/5 width each) = 3 Battery cards.
        // All 4 cards locked to exact 106px height!
        let total_w = ui.available_width();
        let gap = 12.0;
        let col_w = ((total_w - gap * 3.0) / 5.0).max(70.0);
        let device_card_w = col_w * 2.0;
        let card_h = 132.0;

        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = gap;

            // 1. Device Card (2 columns wide)
            let (dev_rect, _) =
                ui.allocate_exact_size(Vec2::new(device_card_w, card_h), Sense::hover());
            ui.painter()
                .add(Self::card_shadow().as_shape(dev_rect, Rounding::same(20.0)));
            ui.painter().rect(
                dev_rect,
                Rounding::same(20.0),
                Color32::WHITE,
                Stroke::new(1.0_f32, CARD_BORDER),
            );

            let mut dev_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(dev_rect.shrink(15.0))
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            dev_ui.set_clip_rect(dev_rect.shrink(2.0));
            dev_ui.horizontal_wrapped(|ui| {
                let (icon_box, _) = ui.allocate_exact_size(Vec2::new(40.0, 40.0), Sense::hover());
                ui.painter().rect_filled(
                    icon_box,
                    Rounding::same(12.0),
                    Color32::from_rgb(237, 241, 253), // #edf1fd
                );
                draw_vector_icon(
                    ui.painter(),
                    icon_box.shrink(8.0),
                    IconKind::Earbud,
                    ACCENT_BLUE,
                );

                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("EarFun Air Pro 4")
                            .font(FontId::new(15.5, FontFamily::Name("SemiBold".into())))
                            .color(TEXT_PRIMARY),
                    );
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        let badge_str = match self.connection {
                            ConnectionStatus::Connected => "Đã kết nối",
                            ConnectionStatus::Connecting => "Đang kết nối…",
                            ConnectionStatus::Disconnected => "Chưa kết nối",
                        };
                        let badge_color = if self.is_connected() {
                            Color32::from_rgb(56, 128, 96)
                        } else {
                            TEXT_SUBTITLE
                        };
                        ui.label(RichText::new(badge_str).size(11.5).color(badge_color));
                        ui.label(RichText::new("•").size(11.5).color(TEXT_MUTED));
                        let codec_str = self.state.codec.as_deref().unwrap_or("—");
                        ui.label(
                            RichText::new(codec_str)
                                .font(FontId::new(11.5, FontFamily::Name("Medium".into())))
                                .color(Color32::from_rgb(80, 107, 170)),
                        );
                    });
                });
            });

            dev_ui.add_space(6.0);
            dev_ui.horizontal_wrapped(|ui| {
                let btn_label = match self.connection {
                    ConnectionStatus::Connected => "Ngắt kết nối",
                    ConnectionStatus::Connecting => "Đang kết nối…",
                    ConnectionStatus::Disconnected => "Kết nối",
                };
                if Self::primary_button(ui, btn_label, self.busy == 0).clicked() {
                    self.toggle_connect();
                }
                if self.is_connected()
                    && Self::secondary_button(ui, "Đọc lại", self.busy == 0).clicked()
                {
                    let core = self.backend.core.clone();
                    self.backend.spawn_task(async move { core.query().await });
                }
            });

            // 2. Three Battery Cards (Tai trái, Tai phải, Hộp sạc)
            let bat_items = [
                ("Tai trái", self.state.bat_l, IconKind::Earbud),
                ("Tai phải", self.state.bat_r, IconKind::Earbud),
                ("Hộp sạc", self.state.bat_box, IconKind::Case),
            ];
            for (title, val, icon_kind) in bat_items {
                let (bat_rect, _) =
                    ui.allocate_exact_size(Vec2::new(col_w, card_h), Sense::hover());
                ui.painter()
                    .add(Self::card_shadow().as_shape(bat_rect, Rounding::same(20.0)));
                ui.painter().rect(
                    bat_rect,
                    Rounding::same(20.0),
                    Color32::WHITE,
                    Stroke::new(1.0_f32, CARD_BORDER),
                );

                let inner = bat_rect.shrink(16.0);
                let icon_r = Rect::from_min_size(inner.left_top(), Vec2::new(16.0, 16.0));
                draw_vector_icon(ui.painter(), icon_r, icon_kind, ACCENT_BLUE);
                ui.painter().text(
                    Pos2::new(inner.left() + 23.0, inner.top() + 8.0),
                    egui::Align2::LEFT_CENTER,
                    title,
                    FontId::new(12.0, FontFamily::Proportional),
                    TEXT_SUBTITLE,
                );

                let pct_text = val
                    .map(|v| format!("{}%", v))
                    .unwrap_or_else(|| "—".to_string());
                ui.painter().text(
                    Pos2::new(inner.left(), inner.center().y + 3.0),
                    egui::Align2::LEFT_CENTER,
                    pct_text,
                    FontId::new(25.0, FontFamily::Name("SemiBold".into())),
                    TEXT_PRIMARY,
                );

                // Sleek battery progress bar at bottom of card
                let bar_rect = Rect::from_min_max(
                    Pos2::new(inner.left(), inner.bottom() - 6.0),
                    Pos2::new(inner.right(), inner.bottom()),
                );
                ui.painter().rect_filled(
                    bar_rect,
                    Rounding::same(3.0),
                    Color32::from_rgb(235, 238, 246), // #ebeef6
                );
                if let Some(v) = val {
                    let frac = (v.min(100) as f32) / 100.0;
                    let fill_w = (bar_rect.width() * frac).max(if v > 0 { 4.0 } else { 0.0 });
                    let fill_rect =
                        Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_rect.height()));
                    ui.painter().rect_filled(
                        fill_rect,
                        Rounding::same(3.0),
                        Color32::from_rgb(142, 172, 210), // #8eacd2
                    );
                }
            }
        });
    }

    fn render_overview(&mut self, ui: &mut egui::Ui) {
        let enabled = self.is_enabled();
        let current_noise = self.state.noise.clone();

        Self::draw_card(ui, Some("Kiểm soát tiếng ồn"), None, |ui| {
            // 3 Mode cards with custom vector icons (Normal, ANC, Ambient)
            ui.columns(3, |cols| {
                let modes = [
                    (
                        "normal",
                        "Bình thường",
                        "Âm thanh nguyên bản",
                        0u8,
                        IconKind::Normal,
                    ),
                    (
                        "anc",
                        "Chống ồn",
                        "Tập trung vào âm nhạc",
                        1u8,
                        IconKind::Anc,
                    ),
                    (
                        "ambient",
                        "Xuyên âm",
                        "Nghe môi trường xung quanh",
                        2u8,
                        IconKind::Ambient,
                    ),
                ];
                for (i, (key, title, desc, code, icon_kind)) in modes.into_iter().enumerate() {
                    let is_active = current_noise.as_deref() == Some(key);
                    let w = cols[i].available_width();
                    let desired = Vec2::new(w, 60.0);
                    let sense = if enabled {
                        Sense::click()
                    } else {
                        Sense::hover()
                    };
                    let (rect, resp) = cols[i].allocate_exact_size(desired, sense);
                    let fill = if is_active {
                        Color32::from_rgb(233, 238, 252) // #e9eefc
                    } else if resp.hovered() && enabled {
                        Color32::from_rgb(240, 244, 252)
                    } else {
                        Color32::from_rgb(247, 249, 252) // #f7f9fc
                    };
                    let stroke = if is_active {
                        Stroke::new(1.5_f32, Color32::from_rgb(140, 156, 210)) // #8c9cd2
                    } else {
                        Stroke::new(1.0_f32, Color32::from_rgb(231, 235, 243)) // #e7ebf3
                    };
                    cols[i]
                        .painter()
                        .rect(rect, Rounding::same(13.0), fill, stroke);

                    let icon_rect = Rect::from_min_size(
                        Pos2::new(rect.left() + 14.0, rect.center().y - 12.0),
                        Vec2::new(24.0, 24.0),
                    );
                    draw_vector_icon(cols[i].painter(), icon_rect, icon_kind, ACCENT_BLUE);

                    let title_color = if enabled || is_active {
                        TEXT_PRIMARY
                    } else {
                        Color32::from_rgb(90, 95, 110)
                    };
                    cols[i].painter().text(
                        Pos2::new(rect.left() + 48.0, rect.center().y - 8.0),
                        egui::Align2::LEFT_CENTER,
                        title,
                        FontId::new(13.5, FontFamily::Name("Medium".into())),
                        title_color,
                    );
                    cols[i].painter().text(
                        Pos2::new(rect.left() + 48.0, rect.center().y + 9.0),
                        egui::Align2::LEFT_CENTER,
                        desc,
                        FontId::new(11.0, FontFamily::Proportional),
                        TEXT_SUBTITLE,
                    );

                    if enabled && resp.clicked() {
                        let core = self.backend.core.clone();
                        let k_owned = key.to_string();
                        self.backend.spawn_task(async move {
                            core.set_string_value(20, 21, code, "noise", &k_owned).await
                        });
                    }
                }
            });

            ui.add_space(12.0);

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
                        .font(FontId::new(13.0, FontFamily::Name("Medium".into())))
                        .color(TEXT_PRIMARY),
                );
                cols[0].add_space(4.0);
                cols[0].horizontal(|ui| {
                    ui.add_enabled_ui(sub_avail, |ui| {
                        let prev = self.draft_sub;
                        egui::ComboBox::from_id_salt("choice_sub")
                            .width((ui.available_width() - 100.0).clamp(110.0, 220.0))
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
                    if Self::action_button(ui, "Áp dụng", can_apply).clicked() {
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
                        .font(FontId::new(13.0, FontFamily::Name("Medium".into())))
                        .color(TEXT_PRIMARY),
                );
                cols[1].add_space(4.0);
                cols[1].horizontal(|ui| {
                    ui.add_enabled_ui(amb_avail, |ui| {
                        let prev = self.draft_amb;
                        egui::ComboBox::from_id_salt("choice_amb")
                            .width((ui.available_width() - 100.0).clamp(110.0, 220.0))
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
                    if Self::action_button(ui, "Áp dụng", can_apply).clicked() {
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

        ui.add_space(12.0);

        // Two quick toggle cards side by side
        let mut trigger_ear: Option<bool> = None;
        ui.columns(2, |cols| {
            Self::draw_card(&mut cols[0], None, None, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        ui.label(
                            RichText::new("Chế độ chơi game")
                                .font(FontId::new(14.0, FontFamily::Name("Medium".into())))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("Giảm độ trễ khi xem và chơi")
                                .size(11.5)
                                .color(TEXT_SUBTITLE),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut game_on = self.state.game.unwrap_or(false);
                        let avail = enabled && self.state.game.is_some();
                        if Self::toggle_switch(ui, &mut game_on, avail) {
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_bool_value(18, 19, u8::from(game_on), "game", game_on)
                                    .await
                            });
                        }
                    });
                });
            });

            Self::draw_card(&mut cols[1], None, None, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 3.0;
                        ui.label(
                            RichText::new("Nhận biết đeo tai")
                                .font(FontId::new(14.0, FontFamily::Name("Medium".into())))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("Phản hồi khi đeo hoặc tháo tai nghe")
                                .size(11.5)
                                .color(TEXT_SUBTITLE),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut ear_on = self.state.ear.unwrap_or(false);
                        let avail = enabled && self.state.ear.is_some();
                        if Self::toggle_switch(ui, &mut ear_on, avail) {
                            trigger_ear = Some(ear_on);
                        }
                    });
                });
            });
        });
        if let Some(on) = trigger_ear {
            let plan_res = workflows::restart_setting(&self.state, "ear", on);
            self.execute_plan(plan_res);
        }

        ui.add_space(12.0);

        // Keep labels, slider and actions on separate rows. A nested RTL layout
        // used to paint the slider's value over the Read button.
        Self::draw_card(ui, None, None, |ui| {
            let title = if cfg!(target_os = "windows") {
                "Âm lượng Windows"
            } else {
                "Âm lượng Ubuntu"
            };
            ui.label(RichText::new(title).size(14.0).color(TEXT_PRIMARY));
            if cfg!(target_os = "windows") {
                ui.label(
                    "Điều chỉnh âm lượng đầu ra EarFun bằng Windows hoặc nút cảm ứng tai nghe.",
                );
            } else {
                ui.label(
                    RichText::new(&self.volume_label)
                        .size(11.5)
                        .color(TEXT_SUBTITLE),
                );
                ui.spacing_mut().slider_width = (ui.available_width() - 65.0).max(100.0);
                ui.add_enabled(
                    enabled,
                    egui::Slider::new(&mut self.volume_value, 0..=100).suffix("%"),
                );
            }
            ui.horizontal_wrapped(|ui| {
                if !cfg!(target_os = "windows") {
                    if Self::secondary_button(ui, "Đọc", enabled).clicked() {
                        self.read_or_set_volume(None);
                    }
                    if Self::primary_button(ui, "Áp dụng", enabled).clicked() {
                        self.read_or_set_volume(Some(self.volume_value));
                    }
                }
                if Self::secondary_button(ui, "Cài đặt âm thanh", true).clicked() {
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
                ui.horizontal_wrapped(|ui| {
                    let current_label = self
                        .preset_options
                        .get(self.selected_preset_idx)
                        .map(|o| o.label.clone())
                        .unwrap_or_else(|| "Tùy chỉnh (bản nháp)".to_string());

                    let prev_idx = self.selected_preset_idx;
                    egui::ComboBox::from_id_salt("preset_picker")
                        .width(270.0)
                        .selected_text(current_label)
                        .show_ui(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.preset_search)
                                    .hint_text("Tìm mẫu EQ…"),
                            );
                            ui.add_space(4.0);
                            let q = self.preset_search.trim().to_lowercase();
                            for (i, opt) in self.preset_options.iter().enumerate() {
                                if q.is_empty() || opt.label.to_lowercase().contains(&q) {
                                    ui.selectable_value(
                                        &mut self.selected_preset_idx,
                                        i,
                                        &opt.label,
                                    );
                                }
                            }
                        });
                    if self.selected_preset_idx != prev_idx {
                        load_clicked = true;
                    }
                    if Self::secondary_button(ui, "Nạp bản chỉnh", true).clicked() {
                        load_clicked = true;
                    }
                    if Self::secondary_button(ui, "Đặt lại", true).clicked() {
                        reset_clicked = true;
                    }
                });

                ui.add_space(8.0);
                Self::draw_eq_curve(ui, &self.eq_levels);
                ui.add_space(8.0);

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
                                    .font(FontId::new(11.0, FontFamily::Name("Medium".into())))
                                    .color(Color32::from_rgb(80, 107, 170)),
                            );
                            let slider = egui::Slider::new(&mut self.eq_levels[i], -10..=10)
                                .vertical()
                                .show_value(false);
                            if ui.add_sized([24.0, 138.0], slider).changed() {
                                sliders_changed = true;
                            }
                            let freq_label = if freq >= 1000.0 {
                                format!("{}k", (freq / 1000.0) as u32)
                            } else if (freq - 31.5).abs() < 0.1 {
                                "31.5".to_string()
                            } else {
                                format!("{}", freq as u32)
                            };
                            ui.label(RichText::new(freq_label).size(11.0).color(TEXT_SUBTITLE));
                        });
                    }
                });

                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut self.preset_name_input)
                            .hint_text("Tên bản chỉnh")
                            .desired_width(200.0),
                    );
                    if Self::secondary_button(ui, "Lưu trên máy", true).clicked() {
                        save_clicked = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if Self::primary_button(ui, "Gửi tới tai nghe", enabled).clicked() {
                            apply_clicked = true;
                        }
                    });
                });

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    if Self::secondary_button(ui, "Đổi tên", true).clicked() {
                        rename_clicked = true;
                    }
                    if Self::secondary_button(ui, "Xóa bản riêng", true).clicked() {
                        delete_clicked = true;
                    }
                    if Self::secondary_button(ui, "Nhập từ tệp…", true).clicked() {
                        import_clicked = true;
                    }
                    if Self::secondary_button(ui, "Xuất bản nháp…", true).clicked() {
                        export_clicked = true;
                    }
                });

                ui.add_space(4.0);
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
                        .color(TEXT_SUBTITLE),
                );
            },
        );

        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Đường cong hiển thị bản chỉnh trên máy. Chưa hỗ trợ đọc lại EQ từ tai nghe.",
            )
            .size(11.5)
            .color(TEXT_SUBTITLE),
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
        let desired = Vec2::new(ui.available_width(), 112.0);
        let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
        let painter = ui.painter_at(rect);

        painter.rect(
            rect,
            Rounding::same(13.0),
            Color32::from_rgb(248, 250, 254), // #f8fafe
            Stroke::new(1.0_f32, Color32::from_rgb(233, 238, 248)),
        );

        let left = rect.left() + 38.0;
        let right = rect.right() - 18.0;
        let top = rect.top() + 13.0;
        let bottom = rect.bottom() - 13.0;
        let y_for_gain = |gain: f64| -> f32 { top + ((6.0 - gain) as f32 / 12.0) * (bottom - top) };

        for db in [6, 3, 0, -3, -6] {
            let yy = y_for_gain(db as f64);
            let label = if db > 0 {
                format!("+{}", db)
            } else {
                db.to_string()
            };
            painter.text(
                Pos2::new(rect.left() + 10.0, yy),
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

        let stroke = Stroke::new(2.5_f32, ACCENT_BLUE);
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
            painter.circle(pt, 3.5, Color32::WHITE, Stroke::new(1.8_f32, ACCENT_BLUE));
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
            app.settings
                .data
                .presets
                .insert(name.clone(), levels.clone());
            app.settings.data.eq = levels;
            app.save_settings();
            app.refresh_presets(Some(&format!("custom:{}", name)));
            app.show_notice("Đã lưu bản chỉnh trên máy.", false);
        };
        if exists {
            let prompt = format!(
                "Ghi đè bản chỉnh riêng “{}”?",
                self.preset_name_input.trim()
            );
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
        let generation = self.connection_generation;
        self.backend.spawn_task(async move {
            core.set_eq(&levels).await?;
            core.emit_event(BackendEvent::EqSent { generation, levels });
            Ok(())
        });
    }

    fn render_gestures(&mut self, ui: &mut egui::Ui) {
        let enabled = self.is_enabled();
        let mut save_req: Option<(String, String, String)> = None;
        let mut lock_req: Option<(bool, bool)> = None;

        ui.columns(2, |cols| {
            for (col_idx, (side, title)) in [("L", "Tai trái"), ("R", "Tai phải")]
                .into_iter()
                .enumerate()
            {
                Self::draw_card(
                    &mut cols[col_idx],
                    Some(title),
                    Some("Chọn chức năng cho từng thao tác."),
                    |ui| {
                        for (g_idx, (gesture, _)) in p::GESTURES.iter().enumerate() {
                            if g_idx > 0 {
                                ui.add_space(4.0);
                            }
                            let field = format!("{}.{}", side, gesture);
                            let current_idx = *self.draft_keys.get(&field).unwrap_or(&0);
                            let selected_label = if current_idx == 0 {
                                "—"
                            } else {
                                p::OPERATIONS[current_idx - 1].0
                            };

                            ui.label(
                                RichText::new(*gesture)
                                    .font(FontId::new(13.0, FontFamily::Name("Medium".into())))
                                    .color(TEXT_PRIMARY),
                            );
                            ui.horizontal_wrapped(|ui| {
                                let combo_w = (ui.available_width() - 88.0).clamp(100.0, 250.0);
                                ui.add_enabled_ui(enabled, |ui| {
                                    let mut new_idx = current_idx;
                                    egui::ComboBox::from_id_salt(&field)
                                        .width(combo_w)
                                        .selected_text(selected_label)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut new_idx, 0, "—");
                                            for (i, (op_name, _)) in
                                                p::OPERATIONS.iter().enumerate()
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
                                if Self::action_button(ui, "Lưu", can_save).clicked() {
                                    if let Some(act) = wanted {
                                        save_req = Some((
                                            side.to_string(),
                                            gesture.to_string(),
                                            act.to_string(),
                                        ));
                                    }
                                }
                            });
                        }

                        Self::draw_row_divider(ui);
                        let both_known = self.state.dis_l.is_some() && self.state.dis_r.is_some();
                        let mut locked = if side == "L" {
                            self.state.dis_l.unwrap_or(false)
                        } else {
                            self.state.dis_r.unwrap_or(false)
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("Tắt cảm ứng bên này")
                                    .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                                    .color(TEXT_PRIMARY),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if Self::toggle_switch(ui, &mut locked, enabled && both_known) {
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
                        });
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
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Cho phép kết nối hai thiết bị")
                            .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut dual_on = self.state.dual.unwrap_or(false);
                        let avail = enabled && self.state.dual.is_some();
                        if Self::toggle_switch(ui, &mut dual_on, avail) {
                            trigger_dual = Some(dual_on);
                        }
                    });
                });

                Self::draw_row_divider(ui);
                ui.horizontal_wrapped(|ui| {
                    if Self::secondary_button(ui, "Làm mới danh sách", enabled).clicked() {
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

        ui.add_space(12.0);

        let mut phone_cmd: Option<(u8, u8)> = None;
        Self::draw_card(ui, Some("Thiết bị đã lưu"), None, |ui| {
            if self.phones.is_empty() {
                ui.label(
                    RichText::new("Chưa có dữ liệu thiết bị.")
                        .size(12.5)
                        .color(TEXT_SUBTITLE),
                );
            } else {
                for (idx, phone) in self.phones.iter().enumerate() {
                    if idx > 0 {
                        Self::draw_row_divider(ui);
                    }
                    ui.horizontal_wrapped(|ui| {
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
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new(format!("{} · {}", name, phone.mac))
                                    .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                                    .color(TEXT_PRIMARY),
                            );
                            ui.label(RichText::new(status).size(11.5).color(TEXT_SUBTITLE));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let cmd = if phone.connected { 42u8 } else { 41u8 };
                            let label = if phone.connected {
                                "Ngắt"
                            } else {
                                "Kết nối"
                            };
                            if Self::secondary_button(ui, label, enabled).clicked() {
                                phone_cmd = Some((cmd, phone.num));
                            }
                        });
                    });
                }
            }
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Danh sách do tai nghe cung cấp. Ngắt thiết bị có thể làm gián đoạn âm thanh.",
            )
            .size(11.5)
            .color(TEXT_SUBTITLE),
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
            // Row 1: Âm lượng thông báo
            let prompt_opts = [
                ("—", None),
                ("100%", Some((100u8, 0u8))),
                ("75%", Some((75u8, 1u8))),
                ("50%", Some((50u8, 2u8))),
                ("25%", Some((25u8, 3u8))),
                ("Tắt", Some((0u8, 4u8))),
            ];
            let prompt_avail = enabled && self.state.prompt.is_some();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Âm lượng thông báo")
                        .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wanted = prompt_opts[self.draft_prompt].1;
                    let can_apply = prompt_avail
                        && wanted.is_some()
                        && wanted.map(|(v, _)| v) != self.state.prompt;
                    if Self::action_button(ui, "Áp dụng", can_apply).clicked() {
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
                            .width(200.0)
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

            Self::draw_row_divider(ui);

            // Row 2: Gói giọng thông báo
            let lang_text = match self.state.voice_language.as_deref() {
                Some("EN") => "English",
                Some("ZH") => "中文",
                Some("DE") => "Deutsch",
                Some("JA") => "日本語",
                _ => "EarFun Audio",
            };
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("Gói giọng thông báo")
                            .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(
                            "Cài bằng ứng dụng trên điện thoại; bản desktop chưa hỗ trợ.",
                        )
                        .size(11.5)
                        .color(TEXT_SUBTITLE),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(lang_text)
                            .font(FontId::new(12.5, FontFamily::Name("Medium".into())))
                            .color(Color32::from_rgb(80, 107, 170)),
                    );
                });
            });

            Self::draw_row_divider(ui);

            // Row 3: Micro cuộc gọi
            let mic_opts = [
                ("—", None),
                ("Tự động", Some(("AUTO", 0u8))),
                ("Tai trái", Some(("L", 1u8))),
                ("Tai phải", Some(("R", 2u8))),
            ];
            let mic_avail = enabled && self.state.mic.is_some();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Micro cuộc gọi")
                        .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wanted = mic_opts[self.draft_mic].1;
                    let can_apply = mic_avail
                        && wanted.is_some()
                        && wanted.map(|(k, _)| k) != self.state.mic.as_deref();
                    if Self::action_button(ui, "Áp dụng", can_apply).clicked() {
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
                            .width(200.0)
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

            Self::draw_row_divider(ui);

            // Row 4: Giới hạn âm lượng
            let hl_opts = ["—", "Tắt", "90 dB", "80 dB", "70 dB", "60 dB", "50 dB"];
            let hl_avail = enabled && self.state.hearing_limit.is_some();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Giới hạn âm lượng")
                        .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let wanted = if self.draft_hearing_limit >= 1 {
                        Some((self.draft_hearing_limit - 1) as u8)
                    } else {
                        None
                    };
                    let can_apply =
                        hl_avail && wanted.is_some() && wanted != self.state.hearing_limit;
                    if Self::action_button(ui, "Áp dụng", can_apply).clicked() {
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
                            .width(200.0)
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

            Self::draw_row_divider(ui);

            // Row 5: Cân bằng trái/phải
            let bal_avail = enabled && self.state.balance.is_some();
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("Cân bằng trái/phải (50 = giữa)")
                        .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_apply = bal_avail && Some(self.draft_balance) != self.state.balance;
                    if Self::action_button(ui, "Áp dụng", can_apply).clicked() {
                        let v = self.draft_balance;
                        let core = self.backend.core.clone();
                        self.backend.spawn_task(async move {
                            core.set_u8_value(85, 86, v, "balance", v).await
                        });
                    }
                    ui.add_enabled_ui(bal_avail, |ui| {
                        ui.spacing_mut().slider_width = 160.0;
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

        ui.add_space(12.0);

        Self::draw_card(
            ui,
            Some("Kết nối Bluetooth"),
            Some("Codec cần được cả hệ điều hành và bộ phát hỗ trợ."),
            |ui| {
                let le_opts = ["—", "Thông thường · Google Fast Pair", "LE Audio"];
                let le_avail = enabled && self.state.le_audio.is_some();
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Chế độ Bluetooth")
                            .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                            .color(TEXT_PRIMARY),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let wanted = match self.draft_le_audio {
                            1 => Some(false),
                            2 => Some(true),
                            _ => None,
                        };
                        let can_apply =
                            le_avail && wanted.is_some() && wanted != self.state.le_audio;
                        if Self::action_button(ui, "Áp dụng", can_apply).clicked() {
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
                    Self::draw_row_divider(ui);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new(label)
                                .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                                .color(TEXT_PRIMARY),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut val = self.state.get_bool(key).unwrap_or(false);
                            if Self::toggle_switch(ui, &mut val, enabled && codecs_known) {
                                trigger_codec = Some((key, val));
                            }
                        });
                    });
                }
            },
        );

        ui.add_space(12.0);

        // Two-column bottom row for Tìm tai nghe & Cài đặt nâng cao
        ui.columns(2, |cols| {
            Self::draw_card(
                &mut cols[0],
                Some("Tìm tai nghe"),
                Some("Tháo tai nghe khỏi tai. Giữ kết nối để gửi lệnh dừng sau 10 giây."),
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if Self::secondary_button(ui, "Phát tiếng tai trái", enabled).clicked()
                        {
                            find_req = Some(1);
                        }
                        if Self::secondary_button(ui, "Phát tiếng tai phải", enabled).clicked()
                        {
                            find_req = Some(2);
                        }
                        if Self::secondary_button(ui, "Dừng tìm", enabled).clicked() {
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move { core.find(0).await });
                        }
                    });
                },
            );

            Self::draw_card(
                &mut cols[1],
                Some("Cài đặt nâng cao"),
                Some("Khôi phục sẽ xóa thiết lập và các thiết bị đã ghép trên tai nghe."),
                |ui| {
                    if Self::danger_button(ui, "Khôi phục cài đặt gốc…", enabled).clicked()
                    {
                        reset_req = true;
                    }
                },
            );
        });

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
                    app.backend.spawn_task(async move { core.find(mask).await });
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
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("Tự kết nối khi mở ứng dụng")
                            .font(FontId::new(13.5, FontFamily::Name("Medium".into())))
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(
                            "Tự tìm tai nghe và thử lại khi lấy tai nghe ra khỏi hộp. Đóng cửa sổ để thoát.",
                        )
                        .size(11.5)
                        .color(TEXT_SUBTITLE),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut auto = self.settings.data.auto_connect;
                    if Self::toggle_switch(ui, &mut auto, true) {
                        self.settings.data.auto_connect = auto;
                        self.save_settings();
                    }
                });
            });
        });

        ui.add_space(12.0);

        Self::draw_card(
            ui,
            Some("Thông tin & nhật ký"),
            Some("Chỉ lưu trên máy. Nhật ký có thể chứa tên và địa chỉ thiết bị Bluetooth."),
            |ui| {
                let fw = self.state.version.as_deref().unwrap_or("—");
                let codec = self.state.codec.as_deref().unwrap_or("—");
                ui.label(
                    RichText::new(format!("Firmware {}  /  Codec {}", fw, codec))
                        .font(FontId::new(12.5, FontFamily::Name("Medium".into())))
                        .color(Color32::from_rgb(80, 107, 170)),
                );
                ui.add_space(4.0);

                egui::Frame::none()
                    .fill(Color32::from_rgb(248, 250, 254))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(222, 228, 239)))
                    .rounding(Rounding::same(10.0))
                    .inner_margin(egui::Margin::same(10.0))
                    .show(ui, |ui| {
                        egui::ScrollArea::vertical()
                            .min_scrolled_height(160.0)
                            .max_height(190.0)
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                let log_str = if self.logs.is_empty() {
                                    "Chưa có sự kiện Bluetooth LE nào trong phiên này.".to_string()
                                } else {
                                    self.logs.join("\n")
                                };
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(log_str)
                                            .monospace()
                                            .size(11.0)
                                            .color(TEXT_SUBTITLE),
                                    )
                                    .selectable(true),
                                );
                            });
                    });

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if Self::secondary_button(ui, "Xuất nhật ký…", true).clicked() {
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
                    if Self::secondary_button(ui, "Đọc lại trạng thái", enabled).clicked() {
                        let core = self.backend.core.clone();
                        self.backend.spawn_task(async move { core.query().await });
                    }
                });
            },
        );

        ui.add_space(12.0);

        Self::draw_card(
            ui,
            Some(concat!("EarFun Studio ", env!("CARGO_PKG_VERSION"))),
            Some(
                "Ứng dụng độc lập cho Windows và Ubuntu. Viết bằng Rust (eframe/egui + btleplug).",
            ),
            |ui| {
                ui.label(
                    RichText::new(
                        "EQ cá nhân hóa theo thính lực, firmware/gói giọng nói, tham gia Auracast, AI/dịch thuật và hồ sơ đám mây hiện vẫn cần EarFun Audio chính hãng.",
                    )
                    .size(12.0)
                    .color(TEXT_SUBTITLE),
                );
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    if Self::secondary_button(ui, "Nguồn và giới hạn", true).clicked() {
                        let _ = audio::open_url_or_path("https://github.com/lnb2x/earfun-studio");
                    }
                    if Self::secondary_button(ui, "Thoát ứng dụng", true).clicked() {
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
                ui.label(RichText::new(&msg).size(13.0).color(TEXT_PRIMARY));
                ui.add_space(16.0);
                ui.horizontal_wrapped(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if Self::primary_button(ui, "Tiếp tục", true).clicked() {
                            choice = Some(true);
                        }
                        if Self::secondary_button(ui, "Hủy", true).clicked() {
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
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [229.0 / 255.0, 237.0 / 255.0, 247.0 / 255.0, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Prevent OS dark mode from ever overriding our light lavender theme
        if ctx.style().visuals.dark_mode || ctx.style().visuals.panel_fill != BG_MID {
            Self::configure_style(ctx);
        }
        Self::paint_window_background(ctx);

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

impl StudioApp {
    pub fn render_panels(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .exact_width(226.0)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 18.0,
                right: 10.0,
                top: 18.0,
                bottom: 18.0,
            }))
            .show(ctx, |ui| {
                self.render_sidebar(ui);
            });

        egui::TopBottomPanel::bottom("footer_panel")
            .resizable(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 12.0,
                right: 22.0,
                top: 6.0,
                bottom: 14.0,
            }))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    let color = if self.notice_is_error {
                        DANGER_TEXT
                    } else {
                        TEXT_SUBTITLE
                    };
                    let (icon_rect, _) =
                        ui.allocate_exact_size(Vec2::new(15.0, 15.0), Sense::hover());
                    draw_vector_icon(
                        ui.painter(),
                        icon_rect,
                        IconKind::Check,
                        if self.notice_is_error {
                            DANGER_TEXT
                        } else {
                            ACCENT_BLUE
                        },
                    );
                    ui.add(
                        egui::Label::new(RichText::new(&self.notice).size(11.5).color(color))
                            .selectable(true),
                    );
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 12.0,
                right: 22.0,
                top: 18.0,
                bottom: 6.0,
            }))
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("main_content")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        self.render_header_and_summary(ui);
                        ui.add_space(16.0);
                        match self.page {
                            Page::Overview => self.render_overview(ui),
                            Page::Equalizer => self.render_equalizer(ui),
                            Page::Gestures => self.render_gestures(ui),
                            Page::Multipoint => self.render_multipoint(ui),
                            Page::Earbuds => self.render_earbuds(ui),
                            Page::Application => self.render_application(ui, ctx),
                        }
                        ui.add_space(10.0);
                    });
            });

        self.render_confirm_modal(ctx);
    }
}

pub fn run_headless_smoke_test() {
    let ctx = egui::Context::default();
    StudioApp::configure_fonts(&ctx);
    StudioApp::configure_style(&ctx);
    let mut app = StudioApp::new(None, true, false);
    for page in Page::ALL {
        app.page = page;
        let raw_input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1380.0, 860.0))),
            ..Default::default()
        };
        let _ = ctx.run(raw_input, |ctx| {
            StudioApp::paint_window_background(ctx);
            app.render_panels(ctx);
        });
        assert!(
            !ctx.style().visuals.dark_mode,
            "Theme must remain Light mode"
        );
        println!("UI OK: {}", page.title());
    }
    app.settings.data.eq = app.eq_levels.clone();
    app.backend.shutdown_blocking();
}
