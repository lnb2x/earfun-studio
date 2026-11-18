//! EarFun Studio light lavender-indigo design tokens, typography, and global egui styling.

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, FontId, Rounding, Stroke, Vec2,
};

const FONT_REGULAR: &[u8] = include_bytes!("../../assets/fonts/BeVietnamPro-Regular.ttf");
const FONT_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/BeVietnamPro-Medium.ttf");
const FONT_SEMIBOLD: &[u8] = include_bytes!("../../assets/fonts/BeVietnamPro-SemiBold.ttf");
const FONT_BOLD: &[u8] = include_bytes!("../../assets/fonts/BeVietnamPro-Bold.ttf");

// Signature background gradient stops
pub const BG_TOP_LEFT: Color32 = Color32::from_rgb(223, 232, 245); // #dfe8f5
pub const BG_MID: Color32 = Color32::from_rgb(229, 237, 247); // #e5edf7
pub const BG_BOT_RIGHT: Color32 = Color32::from_rgb(234, 230, 246); // #eae6f6

// Text hierarchy
pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(30, 33, 42); // Crisp charcoal-navy
pub const TEXT_SUBTITLE: Color32 = Color32::from_rgb(100, 105, 122); // Balanced slate
pub const TEXT_MUTED: Color32 = Color32::from_rgb(142, 148, 166);

// Brand & accent colors
pub const ACCENT_BLUE: Color32 = Color32::from_rgb(96, 128, 208); // #6080d0
pub const ACCENT_INDIGO: Color32 = Color32::from_rgb(66, 91, 152); // #425b98
pub const PRIMARY_BTN_BG: Color32 = Color32::from_rgb(82, 112, 186); // #5270ba
pub const PRIMARY_BTN_HOVER: Color32 = Color32::from_rgb(70, 99, 172); // #4663ac

// Surfaces & borders
pub const CARD_BORDER: Color32 = Color32::from_rgb(233, 238, 247);
pub const ROW_DIVIDER: Color32 = Color32::from_rgb(239, 243, 250);
pub const CTRL_BG: Color32 = Color32::from_rgb(244, 246, 252);
pub const CTRL_BG_HOVER: Color32 = Color32::from_rgb(234, 239, 251);
pub const CTRL_BORDER: Color32 = Color32::from_rgb(222, 229, 241);

// Semantic status colors
pub const SUCCESS_BG: Color32 = Color32::from_rgb(234, 247, 240);
pub const SUCCESS_BORDER: Color32 = Color32::from_rgb(192, 229, 209);
pub const SUCCESS_TEXT: Color32 = Color32::from_rgb(38, 115, 77);

pub const WARN_BG: Color32 = Color32::from_rgb(254, 247, 234);
pub const WARN_BORDER: Color32 = Color32::from_rgb(243, 218, 176);
pub const WARN_TEXT: Color32 = Color32::from_rgb(156, 98, 28);

pub const DANGER_BG: Color32 = Color32::from_rgb(255, 243, 242);
pub const DANGER_BORDER: Color32 = Color32::from_rgb(241, 210, 207);
pub const DANGER_TEXT: Color32 = Color32::from_rgb(172, 68, 64);

pub const INFO_BG: Color32 = Color32::from_rgb(235, 241, 253);
pub const INFO_BORDER: Color32 = Color32::from_rgb(198, 211, 242);

#[inline]
pub fn font_regular(size: f32) -> FontId {
    FontId::new(size, FontFamily::Proportional)
}

#[inline]
pub fn font_medium(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Medium".into()))
}

#[inline]
pub fn font_semibold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("SemiBold".into()))
}

#[inline]
pub fn font_bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("Bold".into()))
}

pub fn card_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: Vec2::new(0.0, 4.0),
        blur: 14.0,
        spread: 0.0,
        color: Color32::from_rgba_unmultiplied(105, 124, 166, 17),
    }
}

pub fn modal_shadow() -> egui::epaint::Shadow {
    egui::epaint::Shadow {
        offset: Vec2::new(0.0, 10.0),
        blur: 28.0,
        spread: 2.0,
        color: Color32::from_rgba_unmultiplied(35, 48, 82, 42),
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

pub fn configure_style(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Light);
    let mut visuals = egui::Visuals::light();
    visuals.dark_mode = false;
    visuals.panel_fill = BG_MID;
    visuals.window_fill = Color32::WHITE;
    visuals.window_stroke = Stroke::new(1.0_f32, Color32::from_rgb(218, 226, 240));
    visuals.window_rounding = Rounding::same(18.0);
    visuals.window_shadow = modal_shadow();
    visuals.popup_shadow = card_shadow();
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
    visuals.widgets.hovered.bg_fill = CTRL_BG_HOVER;
    visuals.widgets.hovered.weak_bg_fill = CTRL_BG_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(152, 168, 216));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.4_f32, TEXT_PRIMARY);
    visuals.widgets.hovered.rounding = Rounding::same(10.0);

    // Active / pressed controls
    visuals.widgets.active.bg_fill = Color32::from_rgb(225, 233, 251);
    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(225, 233, 251);
    visuals.widgets.active.bg_stroke = Stroke::new(1.2_f32, ACCENT_BLUE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.5_f32, ACCENT_INDIGO);
    visuals.widgets.active.rounding = Rounding::same(10.0);

    // Open ComboBox / menu state
    visuals.widgets.open.bg_fill = Color32::from_rgb(230, 237, 252);
    visuals.widgets.open.weak_bg_fill = Color32::from_rgb(230, 237, 252);
    visuals.widgets.open.bg_stroke = Stroke::new(1.2_f32, ACCENT_BLUE);
    visuals.widgets.open.fg_stroke = Stroke::new(1.4_f32, TEXT_PRIMARY);
    visuals.widgets.open.rounding = Rounding::same(10.0);

    visuals.selection.bg_fill = Color32::from_rgb(226, 234, 252);
    visuals.selection.stroke = Stroke::new(1.2_f32, ACCENT_INDIGO);
    visuals.slider_trailing_fill = true;

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(9.0, 9.0);
    style.spacing.button_padding = Vec2::new(12.0, 7.0);
    style.spacing.combo_height = 240.0;
    style.spacing.interact_size = Vec2::new(38.0, 30.0);
    style.spacing.scroll.floating = true;
    style.spacing.scroll.bar_width = 6.0;
    ctx.set_style(style);
}

/// Paint the signature `linear-gradient(135deg, #dfe8f5 0%, #e5edf7 45%, #eae6f6 100%)` background.
pub fn paint_window_background(ctx: &egui::Context) {
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
