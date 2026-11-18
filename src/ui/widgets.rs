//! Reusable UI components: cards, animated switches, chips, badges, and buttons.

use crate::ui::icons::{draw_vector_icon, IconKind};
use crate::ui::theme::*;
use eframe::egui::{
    self, Color32, CursorIcon, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2,
};

pub fn draw_card<R>(
    ui: &mut egui::Ui,
    title: Option<&str>,
    subtitle: Option<&str>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    egui::Frame::none()
        .fill(Color32::WHITE)
        .stroke(Stroke::new(1.0_f32, CARD_BORDER))
        .rounding(Rounding::same(18.0))
        .shadow(card_shadow())
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(t) = title {
                ui.label(
                    RichText::new(t)
                        .font(font_semibold(16.0))
                        .color(TEXT_PRIMARY),
                );
                if let Some(sub) = subtitle {
                    ui.add_space(1.0);
                    ui.label(RichText::new(sub).size(12.0).color(TEXT_SUBTITLE));
                }
                ui.add_space(10.0);
            }
            add_contents(ui)
        })
        .inner
}

pub fn draw_card_header_with_right(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: Option<&str>,
    right_ui: impl FnOnce(&mut egui::Ui),
) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.label(
                RichText::new(title)
                    .font(font_semibold(16.0))
                    .color(TEXT_PRIMARY),
            );
            if let Some(sub) = subtitle {
                ui.label(RichText::new(sub).size(12.0).color(TEXT_SUBTITLE));
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), right_ui);
    });
    ui.add_space(10.0);
}

pub fn draw_row_divider(ui: &mut egui::Ui) {
    ui.add_space(3.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0_f32, ROW_DIVIDER),
    );
    ui.add_space(3.0);
}

/// Smoothly animated pill switch with pointer cursor and clear disabled tooltip.
pub fn toggle_switch(ui: &mut egui::Ui, id_salt: &str, on: &mut bool, enabled: bool) -> bool {
    let desired_size = Vec2::new(40.0, 22.0);
    let id = ui.make_persistent_id(id_salt);
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
    if enabled && response.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    if !enabled {
        response = response.on_hover_text("Kết nối tai nghe để thay đổi thiết lập này");
    }

    let anim_t = ui.ctx().animate_bool_responsive(id, *on);
    let painter = ui.painter();
    let radius = rect.height() * 0.5;

    let off_bg = if enabled && response.hovered() {
        Color32::from_rgb(202, 211, 229)
    } else if enabled {
        Color32::from_rgb(215, 222, 236)
    } else {
        Color32::from_rgb(228, 233, 243)
    };
    let on_bg = if enabled {
        ACCENT_BLUE
    } else {
        Color32::from_rgb(174, 190, 230)
    };

    let bg_fill = lerp_color(off_bg, on_bg, anim_t);
    let border_color = if *on && enabled {
        PRIMARY_BTN_BG
    } else {
        Color32::from_rgb(200, 209, 225)
    };

    painter.rect(
        rect,
        Rounding::same(radius),
        bg_fill,
        Stroke::new(1.0_f32, border_color),
    );

    let thumb_r = radius - 2.5;
    let min_x = rect.left() + radius;
    let max_x = rect.right() - radius;
    let thumb_x = egui::lerp(min_x..=max_x, anim_t);
    let thumb_color = if enabled {
        Color32::WHITE
    } else {
        Color32::from_rgb(248, 250, 253)
    };
    painter.circle_filled(Pos2::new(thumb_x, rect.center().y), thumb_r, thumb_color);

    changed
}

fn lerp_color(a: Color32, b: Color32, t: f32) -> Color32 {
    let inv = 1.0 - t;
    Color32::from_rgba_unmultiplied(
        (a.r() as f32 * inv + b.r() as f32 * t).round() as u8,
        (a.g() as f32 * inv + b.g() as f32 * t).round() as u8,
        (a.b() as f32 * inv + b.b() as f32 * t).round() as u8,
        255,
    )
}

/// Interactive pill chip for 1-click sub-modes, preset categories, and segmented options.
pub fn chip_button(ui: &mut egui::Ui, label: &str, active: bool, enabled: bool) -> egui::Response {
    let (fill, border, fg, font) = if active {
        if enabled {
            (
                Color32::from_rgb(228, 235, 252),
                ACCENT_BLUE,
                ACCENT_INDIGO,
                font_semibold(12.0),
            )
        } else {
            (
                Color32::from_rgb(236, 240, 250),
                Color32::from_rgb(185, 198, 232),
                Color32::from_rgb(104, 120, 162),
                font_medium(12.0),
            )
        }
    } else if enabled {
        (
            Color32::from_rgb(246, 248, 253),
            CTRL_BORDER,
            TEXT_SUBTITLE,
            font_medium(12.0),
        )
    } else {
        (
            Color32::from_rgb(248, 249, 252),
            Color32::from_rgb(233, 237, 245),
            TEXT_MUTED,
            font_regular(12.0),
        )
    };

    let btn = egui::Button::new(RichText::new(label).font(font).color(fg))
        .fill(fill)
        .stroke(Stroke::new(if active { 1.3_f32 } else { 1.0_f32 }, border))
        .rounding(Rounding::same(16.0))
        .min_size(Vec2::new(0.0, 28.0));

    let resp = ui.add_enabled(enabled, btn);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

pub fn primary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    ui.scope(|ui| {
        if enabled {
            let w = &mut ui.visuals_mut().widgets;
            w.inactive.bg_fill = PRIMARY_BTN_BG;
            w.inactive.weak_bg_fill = PRIMARY_BTN_BG;
            w.inactive.bg_stroke = Stroke::new(1.0_f32, ACCENT_BLUE);
            w.hovered.bg_fill = PRIMARY_BTN_HOVER;
            w.hovered.weak_bg_fill = PRIMARY_BTN_HOVER;
            w.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT_INDIGO);
            w.active.bg_fill = ACCENT_INDIGO;
            w.active.weak_bg_fill = ACCENT_INDIGO;
            w.active.bg_stroke = Stroke::new(1.2_f32, ACCENT_INDIGO);
        } else {
            let disabled_bg = Color32::from_rgb(202, 212, 234);
            let w = &mut ui.visuals_mut().widgets;
            w.inactive.bg_fill = disabled_bg;
            w.inactive.weak_bg_fill = disabled_bg;
            w.inactive.bg_stroke = Stroke::new(1.0_f32, Color32::from_rgb(192, 204, 230));
        }
        let btn = egui::Button::new(
            RichText::new(text)
                .font(font_medium(12.8))
                .color(Color32::WHITE),
        )
        .rounding(Rounding::same(10.0))
        .min_size(Vec2::new(0.0, 31.0));
        let resp = ui.add_enabled(enabled, btn);
        if enabled && resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        resp
    })
    .inner
}

pub fn action_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let (fill, border, fg) = if enabled {
        (
            Color32::from_rgb(232, 238, 252),
            Color32::from_rgb(164, 180, 224),
            ACCENT_INDIGO,
        )
    } else {
        (
            Color32::from_rgb(245, 247, 252),
            Color32::from_rgb(228, 233, 243),
            TEXT_MUTED,
        )
    };
    let btn = egui::Button::new(RichText::new(text).font(font_medium(12.2)).color(fg))
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border))
        .rounding(Rounding::same(10.0))
        .min_size(Vec2::new(68.0, 30.0));
    let resp = ui.add_enabled(enabled, btn);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let (fill, border, fg) = if enabled {
        (CTRL_BG, CTRL_BORDER, TEXT_PRIMARY)
    } else {
        (
            Color32::from_rgb(246, 248, 252),
            Color32::from_rgb(230, 235, 244),
            TEXT_MUTED,
        )
    };
    let btn = egui::Button::new(RichText::new(text).font(font_medium(12.4)).color(fg))
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border))
        .rounding(Rounding::same(10.0))
        .min_size(Vec2::new(0.0, 31.0));
    let resp = ui.add_enabled(enabled, btn);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

pub fn danger_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let fg = if enabled {
        DANGER_TEXT
    } else {
        Color32::from_rgb(196, 146, 144)
    };
    let btn = egui::Button::new(RichText::new(text).font(font_medium(12.8)).color(fg))
        .fill(DANGER_BG)
        .stroke(Stroke::new(1.0_f32, DANGER_BORDER))
        .rounding(Rounding::same(10.0))
        .min_size(Vec2::new(0.0, 32.0));
    let resp = ui.add_enabled(enabled, btn);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

/// Draw a soft tinted rounded square icon badge (`36x36` or custom size).
pub fn draw_icon_badge(
    ui: &mut egui::Ui,
    size: f32,
    bg: Color32,
    icon: IconKind,
    icon_color: Color32,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    ui.painter()
        .rect_filled(rect, Rounding::same(size * 0.3), bg);
    draw_vector_icon(ui.painter(), rect.shrink(size * 0.22), icon, icon_color);
}

/// Compact pill status badge (e.g. "Đã kết nối", "LDAC", "Chưa gửi").
pub fn status_pill(ui: &mut egui::Ui, text: &str, bg: Color32, border: Color32, fg: Color32) {
    egui::Frame::none()
        .fill(bg)
        .stroke(Stroke::new(1.0_f32, border))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::symmetric(9.0, 3.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).font(font_medium(11.0)).color(fg));
        });
}

/// Semantic battery fill color: coral red <= 15%, amber <= 30%, soothing blue-teal > 30%.
pub fn battery_bar_color(pct: u8) -> Color32 {
    if pct <= 15 {
        Color32::from_rgb(218, 92, 88)
    } else if pct <= 30 {
        Color32::from_rgb(224, 158, 62)
    } else {
        Color32::from_rgb(104, 146, 214)
    }
}

pub fn draw_progress_bar(ui: &mut egui::Ui, rect: Rect, fraction: f32, fill_color: Color32) {
    let r = rect.height() * 0.5;
    ui.painter()
        .rect_filled(rect, Rounding::same(r), Color32::from_rgb(234, 238, 247));
    let clamped = fraction.clamp(0.0, 1.0);
    if clamped > 0.0 {
        let fill_w = (rect.width() * clamped).max(4.0).min(rect.width());
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
        ui.painter()
            .rect_filled(fill_rect, Rounding::same(r), fill_color);
    }
}
