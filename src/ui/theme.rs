//! EarFun Studio Design System: semantic Light & Dark palettes, cached typography,
//! spacing tokens, elevation shadows, and global `egui` visual configuration.

use crate::settings::ThemeMode;
use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Rounding, Stroke, Vec2,
};
use std::sync::LazyLock;

const FONT_REGULAR: &[u8] = include_bytes!("../../assets/fonts/BeVietnamPro-Regular.ttf");
const FONT_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/BeVietnamPro-Medium.ttf");
const FONT_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/BeVietnamPro-SemiBold.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../assets/fonts/BeVietnamPro-Bold.ttf");

static FAMILY_MEDIUM: LazyLock<FontFamily> = LazyLock::new(|| FontFamily::Name("Medium".into()));
static FAMILY_SEMIBOLD: LazyLock<FontFamily> =
    LazyLock::new(|| FontFamily::Name("SemiBold".into()));
static FAMILY_BOLD: LazyLock<FontFamily> = LazyLock::new(|| FontFamily::Name("Bold".into()));

// Layout & radius scale tokens
pub const RADIUS_CARD: f32 = 18.0;
pub const RADIUS_CTRL: f32 = 10.0;
pub const RADIUS_PILL: f32 = 16.0;
pub const CTRL_HEIGHT: f32 = 31.0;

// Legacy light-mode constants kept for external compatibility
pub const BG_MID: Color32 = Color32::from_rgb(233, 239, 248);
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(26, 31, 44);
pub const TEXT_SUBTITLE: Color32 = Color32::from_rgb(94, 102, 122);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(134, 142, 162);
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(76, 108, 212);
pub const ACCENT_INDIGO: Color32 = Color32::from_rgb(54, 80, 168);
pub const PRIMARY_BTN_BG: Color32 = Color32::from_rgb(68, 98, 198);
pub const PRIMARY_BTN_HOVER: Color32 = Color32::from_rgb(56, 84, 182);
pub const CARD_BORDER: Color32 = Color32::from_rgb(225, 232, 244);
pub const ROW_DIVIDER: Color32 = Color32::from_rgb(236, 240, 248);
pub const CTRL_BG: Color32 = Color32::from_rgb(243, 246, 252);
pub const CTRL_BG_HOVER: Color32 = Color32::from_rgb(232, 238, 250);
pub const CTRL_BORDER: Color32 = Color32::from_rgb(218, 226, 240);
pub const SUCCESS_BG: Color32 = Color32::from_rgb(232, 247, 239);
pub const SUCCESS_BORDER: Color32 = Color32::from_rgb(186, 227, 205);
pub const SUCCESS_TEXT: Color32 = Color32::from_rgb(32, 112, 74);
pub const WARN_BG: Color32 = Color32::from_rgb(254, 246, 231);
pub const WARN_BORDER: Color32 = Color32::from_rgb(241, 214, 168);
pub const WARN_TEXT: Color32 = Color32::from_rgb(152, 92, 22);
pub const DANGER_BG: Color32 = Color32::from_rgb(254, 241, 240);
pub const DANGER_BORDER: Color32 = Color32::from_rgb(240, 204, 201);
pub const DANGER_TEXT: Color32 = Color32::from_rgb(172, 58, 54);
pub const INFO_BG: Color32 = Color32::from_rgb(234, 240, 253);
pub const INFO_BORDER: Color32 = Color32::from_rgb(196, 210, 244);

/// Semantic color palette adapting cleanly between Light and Dark desktop themes.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub is_dark: bool,
    pub bg_top_left: Color32,
    pub bg_mid: Color32,
    pub bg_bot_right: Color32,
    pub sidebar_bg: Color32,
    pub footer_bg: Color32,
    pub card_bg: Color32,
    pub card_subtle_bg: Color32,
    pub card_border: Color32,
    pub row_divider: Color32,
    pub text_primary: Color32,
    pub text_subtitle: Color32,
    pub text_muted: Color32,
    pub accent: Color32,
    pub accent_strong: Color32,
    pub primary_bg: Color32,
    pub primary_hover: Color32,
    pub primary_disabled_bg: Color32,
    pub primary_disabled_fg: Color32,
    pub ctrl_bg: Color32,
    pub ctrl_hover: Color32,
    pub ctrl_border: Color32,
    pub input_bg: Color32,
    pub badge_icon_bg: Color32,
    pub nav_active_bg: Color32,
    pub nav_active_border: Color32,
    pub nav_hover_bg: Color32,
    pub chip_active_bg: Color32,
    pub track_bg: Color32,
    pub success_bg: Color32,
    pub success_border: Color32,
    pub success_text: Color32,
    pub warn_bg: Color32,
    pub warn_border: Color32,
    pub warn_text: Color32,
    pub danger_bg: Color32,
    pub danger_border: Color32,
    pub danger_text: Color32,
    pub info_bg: Color32,
    pub info_border: Color32,
    pub info_text: Color32,
    pub chart_bg: Color32,
    pub chart_border: Color32,
    pub chart_grid: Color32,
    pub chart_zero: Color32,
    pub chart_fill_top: Color32,
    pub chart_fill_zero: Color32,
}

impl Palette {
    pub fn light() -> Self {
        Self {
            is_dark: false,
            bg_top_left: Color32::from_rgb(228, 235, 247),
            bg_mid: BG_MID,
            bg_bot_right: Color32::from_rgb(236, 233, 247),
            sidebar_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 244),
            footer_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 212),
            card_bg: Color32::WHITE,
            card_subtle_bg: Color32::from_rgb(246, 248, 253),
            card_border: CARD_BORDER,
            row_divider: ROW_DIVIDER,
            text_primary: TEXT_PRIMARY,
            text_subtitle: TEXT_SUBTITLE,
            text_muted: TEXT_MUTED,
            accent: ACCENT_BLUE,
            accent_strong: ACCENT_INDIGO,
            primary_bg: PRIMARY_BTN_BG,
            primary_hover: PRIMARY_BTN_HOVER,
            primary_disabled_bg: Color32::from_rgb(228, 234, 245),
            primary_disabled_fg: Color32::from_rgb(122, 132, 156),
            ctrl_bg: CTRL_BG,
            ctrl_hover: CTRL_BG_HOVER,
            ctrl_border: CTRL_BORDER,
            input_bg: Color32::from_rgb(248, 250, 254),
            badge_icon_bg: Color32::from_rgb(235, 241, 253),
            nav_active_bg: Color32::from_rgb(231, 238, 253),
            nav_active_border: Color32::from_rgb(194, 208, 242),
            nav_hover_bg: Color32::from_rgb(242, 246, 253),
            chip_active_bg: Color32::from_rgb(228, 236, 253),
            track_bg: Color32::from_rgb(226, 232, 244),
            success_bg: SUCCESS_BG,
            success_border: SUCCESS_BORDER,
            success_text: SUCCESS_TEXT,
            warn_bg: WARN_BG,
            warn_border: WARN_BORDER,
            warn_text: WARN_TEXT,
            danger_bg: DANGER_BG,
            danger_border: DANGER_BORDER,
            danger_text: DANGER_TEXT,
            info_bg: INFO_BG,
            info_border: INFO_BORDER,
            info_text: ACCENT_INDIGO,
            chart_bg: Color32::from_rgb(247, 250, 255),
            chart_border: Color32::from_rgb(226, 234, 247),
            chart_grid: Color32::from_rgb(232, 238, 249),
            chart_zero: Color32::from_rgb(198, 212, 238),
            chart_fill_top: Color32::from_rgba_unmultiplied(76, 108, 212, 38),
            chart_fill_zero: Color32::from_rgba_unmultiplied(76, 108, 212, 6),
        }
    }

    pub fn dark() -> Self {
        Self {
            is_dark: true,
            bg_top_left: Color32::from_rgb(16, 20, 28),
            bg_mid: Color32::from_rgb(19, 23, 33),
            bg_bot_right: Color32::from_rgb(22, 22, 34),
            sidebar_bg: Color32::from_rgba_unmultiplied(26, 31, 44, 246),
            footer_bg: Color32::from_rgba_unmultiplied(26, 31, 44, 230),
            card_bg: Color32::from_rgb(28, 34, 48),
            card_subtle_bg: Color32::from_rgb(23, 28, 40),
            card_border: Color32::from_rgb(44, 53, 73),
            row_divider: Color32::from_rgb(38, 46, 64),
            text_primary: Color32::from_rgb(240, 244, 252),
            text_subtitle: Color32::from_rgb(158, 169, 192),
            text_muted: Color32::from_rgb(114, 125, 148),
            accent: Color32::from_rgb(112, 146, 255),
            accent_strong: Color32::from_rgb(148, 176, 255),
            primary_bg: Color32::from_rgb(74, 108, 224),
            primary_hover: Color32::from_rgb(90, 124, 240),
            primary_disabled_bg: Color32::from_rgb(36, 44, 62),
            primary_disabled_fg: Color32::from_rgb(112, 122, 146),
            ctrl_bg: Color32::from_rgb(37, 45, 62),
            ctrl_hover: Color32::from_rgb(46, 56, 78),
            ctrl_border: Color32::from_rgb(56, 67, 92),
            input_bg: Color32::from_rgb(22, 27, 39),
            badge_icon_bg: Color32::from_rgb(36, 46, 70),
            nav_active_bg: Color32::from_rgb(38, 50, 78),
            nav_active_border: Color32::from_rgb(72, 96, 152),
            nav_hover_bg: Color32::from_rgb(33, 41, 58),
            chip_active_bg: Color32::from_rgb(40, 54, 88),
            track_bg: Color32::from_rgb(42, 50, 68),
            success_bg: Color32::from_rgb(22, 50, 38),
            success_border: Color32::from_rgb(42, 96, 70),
            success_text: Color32::from_rgb(110, 222, 164),
            warn_bg: Color32::from_rgb(56, 42, 20),
            warn_border: Color32::from_rgb(112, 82, 34),
            warn_text: Color32::from_rgb(246, 192, 106),
            danger_bg: Color32::from_rgb(58, 26, 28),
            danger_border: Color32::from_rgb(118, 48, 52),
            danger_text: Color32::from_rgb(248, 132, 128),
            info_bg: Color32::from_rgb(28, 40, 68),
            info_border: Color32::from_rgb(56, 80, 136),
            info_text: Color32::from_rgb(154, 182, 255),
            chart_bg: Color32::from_rgb(22, 27, 39),
            chart_border: Color32::from_rgb(42, 52, 72),
            chart_grid: Color32::from_rgb(33, 40, 56),
            chart_zero: Color32::from_rgb(62, 78, 112),
            chart_fill_top: Color32::from_rgba_unmultiplied(112, 146, 255, 48),
            chart_fill_zero: Color32::from_rgba_unmultiplied(112, 146, 255, 8),
        }
    }
}

#[inline]
pub fn palette(ctx: &egui::Context) -> Palette {
    if ctx.style().visuals.dark_mode {
        Palette::dark()
    } else {
        Palette::light()
    }
}

#[inline]
pub fn ui_palette(ui: &egui::Ui) -> Palette {
    if ui.visuals().dark_mode {
        Palette::dark()
    } else {
        Palette::light()
    }
}

#[inline]
pub fn font_regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

#[inline]
pub fn font_medium(size: f32) -> FontId {
    FontId::new(size, FAMILY_MEDIUM.clone())
}

#[inline]
pub fn font_semibold(size: f32) -> FontId {
    FontId::new(size, FAMILY_SEMIBOLD.clone())
}

#[inline]
pub fn font_bold(size: f32) -> FontId {
    FontId::new(size, FAMILY_BOLD.clone())
}

pub fn card_shadow_for(is_dark: bool) -> egui::epaint::Shadow {
    if is_dark {
        egui::epaint::Shadow {
            offset: Vec2::new(0.0, 5.0),
            blur: 18.0,
            spread: 0.0,
            color: Color32::from_rgba_unmultiplied(0, 0, 0, 72),
        }
    } else {
        egui::epaint::Shadow {
            offset: Vec2::new(0.0, 4.0),
            blur: 14.0,
            spread: 0.0,
            color: Color32::from_rgba_unmultiplied(88, 108, 152, 16),
        }
    }
}

pub fn card_shadow() -> egui::epaint::Shadow {
    card_shadow_for(false)
}

pub fn modal_shadow(is_dark: bool) -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: Vec2::new(0.0, 12.0),
        blur: 32.0,
        spread: 2.0,
        color: if is_dark {
            Color32::from_rgba_unmultiplied(0, 0, 0, 140)
        } else {
            Color32::from_rgba_unmultiplied(28, 40, 72, 46)
        },
    }
}

pub fn configure_fonts(ctx: &egui::Context) {
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
    fonts.font_data.insert(
        "BeVietnamPro-Bold".to_owned(),
        FontData::from_static(FONT_BOLD),
    );

    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "BeVietnamPro-Regular".to_owned());

    let fallback: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();

    for (family_name, primary_font) in [
        ("Medium", "BeVietnamPro-Medium"),
        ("SemiBold", "BeVietnamPro-SemiBold"),
        ("Bold", "BeVietnamPro-Bold"),
    ] {
        let mut list = vec![primary_font.to_owned()];
        list.extend(fallback.iter().cloned());
        fonts
            .families
            .insert(FontFamily::Name(family_name.into()), list);
    }

    ctx.set_fonts(fonts);
}

pub fn resolve_dark_mode(ctx: &egui::Context, mode: ThemeMode) -> bool {
    match mode {
        ThemeMode::Light => false,
        ThemeMode::Dark => true,
        ThemeMode::System => {
            matches!(ctx.input(|i| i.raw.system_theme), Some(egui::Theme::Dark))
        }
    }
}

pub fn apply_theme(ctx: &egui::Context, mode: ThemeMode) {
    let is_dark = resolve_dark_mode(ctx, mode);
    let p = if is_dark {
        Palette::dark()
    } else {
        Palette::light()
    };

    ctx.set_theme(if is_dark {
        egui::ThemePreference::Dark
    } else {
        egui::ThemePreference::Light
    });

    let mut visuals = if is_dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.dark_mode = is_dark;
    visuals.panel_fill = p.bg_mid;
    visuals.window_fill = p.card_bg;
    visuals.extreme_bg_color = p.input_bg;
    visuals.faint_bg_color = p.card_subtle_bg;
    visuals.window_stroke = Stroke::new(1.0_f32, p.card_border);
    visuals.window_rounding = Rounding::same(RADIUS_CARD);
    visuals.window_shadow = modal_shadow(is_dark);
    visuals.popup_shadow = card_shadow_for(is_dark);
    visuals.override_text_color = Some(p.text_primary);

    visuals.widgets.noninteractive.bg_fill = p.card_subtle_bg;
    visuals.widgets.noninteractive.weak_bg_fill = p.card_subtle_bg;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, p.ctrl_border);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, p.text_primary);
    visuals.widgets.noninteractive.rounding = Rounding::same(RADIUS_CTRL);

    visuals.widgets.inactive.bg_fill = p.ctrl_bg;
    visuals.widgets.inactive.weak_bg_fill = p.ctrl_bg;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, p.ctrl_border);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.2_f32, p.text_primary);
    visuals.widgets.inactive.rounding = Rounding::same(RADIUS_CTRL);

    visuals.widgets.hovered.bg_fill = p.ctrl_hover;
    visuals.widgets.hovered.weak_bg_fill = p.ctrl_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.1_f32, p.accent);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.4_f32, p.text_primary);
    visuals.widgets.hovered.rounding = Rounding::same(RADIUS_CTRL);

    visuals.widgets.active.bg_fill = p.chip_active_bg;
    visuals.widgets.active.weak_bg_fill = p.chip_active_bg;
    visuals.widgets.active.bg_stroke = Stroke::new(1.3_f32, p.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, p.accent_strong);
    visuals.widgets.active.rounding = Rounding::same(RADIUS_CTRL);

    visuals.widgets.open.bg_fill = p.chip_active_bg;
    visuals.widgets.open.weak_bg_fill = p.chip_active_bg;
    visuals.widgets.open.bg_stroke = Stroke::new(1.2_f32, p.accent);
    visuals.widgets.open.fg_stroke = Stroke::new(1.4_f32, p.text_primary);
    visuals.widgets.open.rounding = Rounding::same(RADIUS_CTRL);

    visuals.selection.bg_fill = p.chip_active_bg;
    visuals.selection.stroke = Stroke::new(1.2_f32, p.accent_strong);
    visuals.slider_trailing_fill = true;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(9.0, 9.0);
    style.spacing.button_padding = Vec2::new(12.0, 6.5);
    style.spacing.combo_height = 250.0;
    style.spacing.interact_size = Vec2::new(38.0, CTRL_HEIGHT);
    style.spacing.scroll.floating = true;
    style.spacing.scroll.bar_width = 6.0;
    ctx.set_style(style);
}

pub fn configure_style(ctx: &egui::Context) {
    apply_theme(ctx, ThemeMode::Light);
}

/// Paint the subtle diagonal ambient background gradient for the active theme.
pub fn paint_window_background(ctx: &egui::Context) {
    let p = palette(ctx);
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    let mut mesh = egui::epaint::Mesh::default();
    mesh.vertices.push(egui::epaint::Vertex {
        pos: screen.left_top(),
        uv: egui::epaint::WHITE_UV,
        color: p.bg_top_left,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: screen.right_top(),
        uv: egui::epaint::WHITE_UV,
        color: p.bg_mid,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: screen.right_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: p.bg_bot_right,
    });
    mesh.vertices.push(egui::epaint::Vertex {
        pos: screen.left_bottom(),
        uv: egui::epaint::WHITE_UV,
        color: p.bg_mid,
    });
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}
