//! Reusable design-system components: overlap-proof rows & headers, cards, switches,
//! chips, semantic badges, inputs, custom studio EQ band faders, and balance slider.

use crate::ui::icons::{draw_vector_icon, IconKind};
use crate::ui::theme::*;
use eframe::egui::{
    self, Color32, CursorIcon, Key, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2,
};

pub fn draw_card<R>(
    ui: &mut egui::Ui,
    title: Option<&str>,
    subtitle: Option<&str>,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let p = ui_palette(ui);
    egui::Frame::none()
        .fill(p.card_bg)
        .stroke(Stroke::new(1.0_f32, p.card_border))
        .rounding(Rounding::same(RADIUS_CARD))
        .shadow(card_shadow_for(p.is_dark))
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if let Some(t) = title {
                ui.label(
                    RichText::new(t)
                        .font(font_semibold(16.0))
                        .color(p.text_primary),
                );
                if let Some(sub) = subtitle {
                    ui.add_space(1.5);
                    ui.add(
                        egui::Label::new(RichText::new(sub).size(12.0).color(p.text_subtitle))
                            .wrap(),
                    );
                }
                ui.add_space(10.0);
            }
            add_contents(ui)
        })
        .inner
}

/// Overlap-proof card header: lays out right-aligned actions first inside a single-row height
/// budget so left title/subtitle wrap strictly inside the remaining horizontal space.
pub fn draw_card_header_with_right(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: Option<&str>,
    right_ui: impl FnOnce(&mut egui::Ui),
) {
    let p = ui_palette(ui);
    let row_h = if subtitle.is_some() { 38.0 } else { 31.0 };
    let avail_w = ui.available_width();
    ui.allocate_ui_with_layout(
        Vec2::new(avail_w, row_h),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            right_ui(ui);
            let rem_w = ui.available_width().max(80.0);
            ui.allocate_ui_with_layout(
                Vec2::new(rem_w, row_h),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_max_width(rem_w);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.add(
                        egui::Label::new(
                            RichText::new(title)
                                .font(font_semibold(16.0))
                                .color(p.text_primary),
                        )
                        .wrap(),
                    );
                    if let Some(sub) = subtitle {
                        ui.add(
                            egui::Label::new(RichText::new(sub).size(11.8).color(p.text_subtitle))
                                .wrap(),
                        );
                    }
                },
            );
        },
    );
    ui.add_space(10.0);
}

/// Overlap-proof horizontal setting row: right controls reserve space first inside a bounded
/// row height; left content wraps cleanly without vertical stretching.
pub fn setting_row(
    ui: &mut egui::Ui,
    min_height: f32,
    right_ui: impl FnOnce(&mut egui::Ui),
    left_ui: impl FnOnce(&mut egui::Ui),
) {
    let avail_w = ui.available_width();
    ui.allocate_ui_with_layout(
        Vec2::new(avail_w, min_height),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            right_ui(ui);
            let rem_w = ui.available_width().max(60.0);
            ui.allocate_ui_with_layout(
                Vec2::new(rem_w, min_height),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.set_max_width(rem_w);
                    left_ui(ui);
                },
            );
        },
    );
}

pub fn draw_row_divider(ui: &mut egui::Ui) {
    let p = ui_palette(ui);
    ui.add_space(3.5);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0_f32, p.row_divider),
    );
    ui.add_space(3.5);
}

/// Smoothly animated pill switch with keyboard accessibility, pointer cursor, and disabled tooltip.
pub fn toggle_switch(ui: &mut egui::Ui, id_salt: &str, on: &mut bool, enabled: bool) -> bool {
    let p = ui_palette(ui);
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

    let off_bg = if p.is_dark {
        if enabled && response.hovered() {
            Color32::from_rgb(64, 76, 102)
        } else if enabled {
            Color32::from_rgb(52, 62, 84)
        } else {
            Color32::from_rgb(38, 45, 62)
        }
    } else if enabled && response.hovered() {
        Color32::from_rgb(198, 208, 228)
    } else if enabled {
        Color32::from_rgb(212, 220, 236)
    } else {
        Color32::from_rgb(228, 233, 243)
    };

    let on_bg = if enabled {
        p.primary_bg
    } else if p.is_dark {
        Color32::from_rgb(52, 72, 128)
    } else {
        Color32::from_rgb(172, 188, 230)
    };

    let bg_fill = lerp_color(off_bg, on_bg, anim_t);
    let border_color = if *on && enabled {
        p.accent
    } else {
        p.ctrl_border
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
    } else if p.is_dark {
        Color32::from_rgb(168, 176, 196)
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
    let p = ui_palette(ui);
    let (fill, border, fg, font) = if active {
        if enabled {
            (
                p.chip_active_bg,
                p.accent,
                p.accent_strong,
                font_semibold(12.0),
            )
        } else {
            (
                p.card_subtle_bg,
                p.ctrl_border,
                p.text_subtitle,
                font_medium(12.0),
            )
        }
    } else if enabled {
        (p.ctrl_bg, p.ctrl_border, p.text_subtitle, font_medium(12.0))
    } else {
        (
            p.card_subtle_bg,
            p.row_divider,
            p.text_muted,
            font_regular(12.0),
        )
    };

    let btn = egui::Button::new(RichText::new(label).font(font).color(fg))
        .fill(fill)
        .stroke(Stroke::new(if active { 1.3_f32 } else { 1.0_f32 }, border))
        .rounding(Rounding::same(RADIUS_PILL))
        .min_size(Vec2::new(0.0, 28.0));

    let resp = ui.add_enabled(enabled, btn);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

pub fn primary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let p = ui_palette(ui);
    ui.scope(|ui| {
        let fg = if enabled {
            let w = &mut ui.visuals_mut().widgets;
            w.inactive.bg_fill = p.primary_bg;
            w.inactive.weak_bg_fill = p.primary_bg;
            w.inactive.bg_stroke = Stroke::new(1.0_f32, p.accent);
            w.hovered.bg_fill = p.primary_hover;
            w.hovered.weak_bg_fill = p.primary_hover;
            w.hovered.bg_stroke = Stroke::new(1.0_f32, p.accent_strong);
            w.active.bg_fill = p.accent_strong;
            w.active.weak_bg_fill = p.accent_strong;
            w.active.bg_stroke = Stroke::new(1.2_f32, p.accent_strong);
            Color32::WHITE
        } else {
            let w = &mut ui.visuals_mut().widgets;
            w.inactive.bg_fill = p.primary_disabled_bg;
            w.inactive.weak_bg_fill = p.primary_disabled_bg;
            w.inactive.bg_stroke = Stroke::new(1.0_f32, p.ctrl_border);
            p.primary_disabled_fg
        };
        let btn = egui::Button::new(RichText::new(text).font(font_medium(12.6)).color(fg))
            .rounding(Rounding::same(RADIUS_CTRL))
            .min_size(Vec2::new(0.0, CTRL_HEIGHT));
        let resp = ui.add_enabled(enabled, btn);
        if enabled && resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        resp
    })
    .inner
}

pub fn action_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let p = ui_palette(ui);
    let (fill, border, fg) = if enabled {
        (p.chip_active_bg, p.accent, p.accent_strong)
    } else {
        (p.card_subtle_bg, p.ctrl_border, p.text_muted)
    };
    let btn = egui::Button::new(RichText::new(text).font(font_medium(12.2)).color(fg))
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border))
        .rounding(Rounding::same(RADIUS_CTRL))
        .min_size(Vec2::new(68.0, CTRL_HEIGHT));
    let resp = ui.add_enabled(enabled, btn);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

pub fn secondary_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let p = ui_palette(ui);
    let (fill, border, fg) = if enabled {
        (p.ctrl_bg, p.ctrl_border, p.text_primary)
    } else {
        (p.card_subtle_bg, p.row_divider, p.text_muted)
    };
    let btn = egui::Button::new(RichText::new(text).font(font_medium(12.4)).color(fg))
        .fill(fill)
        .stroke(Stroke::new(1.0_f32, border))
        .rounding(Rounding::same(RADIUS_CTRL))
        .min_size(Vec2::new(0.0, CTRL_HEIGHT));
    let resp = ui.add_enabled(enabled, btn);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

pub fn danger_button(ui: &mut egui::Ui, text: &str, enabled: bool) -> egui::Response {
    let p = ui_palette(ui);
    let fg = if enabled { p.danger_text } else { p.text_muted };
    let btn = egui::Button::new(RichText::new(text).font(font_medium(12.6)).color(fg))
        .fill(p.danger_bg)
        .stroke(Stroke::new(1.0_f32, p.danger_border))
        .rounding(Rounding::same(RADIUS_CTRL))
        .min_size(Vec2::new(0.0, CTRL_HEIGHT));
    let resp = ui.add_enabled(enabled, btn);
    if enabled && resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
    }
    resp
}

/// Single-line text input vertically aligned to `CTRL_HEIGHT` (31px) buttons.
pub fn styled_text_input(
    ui: &mut egui::Ui,
    value: &mut String,
    hint: &str,
    desired_width: f32,
) -> egui::Response {
    let p = ui_palette(ui);
    egui::Frame::none()
        .fill(p.input_bg)
        .stroke(Stroke::new(1.0_f32, p.ctrl_border))
        .rounding(Rounding::same(RADIUS_CTRL))
        .inner_margin(egui::Margin::symmetric(10.0, 5.5))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(value)
                    .frame(false)
                    .hint_text(RichText::new(hint).size(12.2).color(p.text_muted))
                    .font(font_regular(12.5))
                    .desired_width(desired_width),
            )
        })
        .inner
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
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_string(), font_medium(11.0), fg);
    let pad = Vec2::new(18.0, 7.0);
    let desired = Vec2::new(galley.size().x + pad.x, (galley.size().y + pad.y).max(22.0));
    let (rect, _) = ui.allocate_exact_size(desired, Sense::hover());
    ui.painter()
        .rect(rect, Rounding::same(11.0), bg, Stroke::new(1.0_f32, border));
    ui.painter().galley(
        Pos2::new(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        fg,
    );
}

/// Semantic battery fill color: coral red <= 15%, amber <= 30%, soothing sapphire > 30%.
pub fn battery_bar_color(pct: u8, is_dark: bool) -> Color32 {
    if pct <= 15 {
        if is_dark {
            Color32::from_rgb(242, 106, 102)
        } else {
            Color32::from_rgb(218, 84, 80)
        }
    } else if pct <= 30 {
        if is_dark {
            Color32::from_rgb(242, 182, 84)
        } else {
            Color32::from_rgb(224, 154, 52)
        }
    } else if is_dark {
        Color32::from_rgb(106, 152, 248)
    } else {
        Color32::from_rgb(84, 126, 218)
    }
}

pub fn draw_progress_bar(ui: &mut egui::Ui, rect: Rect, fraction: f32, fill_color: Color32) {
    let p = ui_palette(ui);
    let r = rect.height() * 0.5;
    ui.painter()
        .rect_filled(rect, Rounding::same(r), p.track_bg);
    let clamped = fraction.clamp(0.0, 1.0);
    if clamped > 0.0 {
        let fill_w = (rect.width() * clamped).max(4.0).min(rect.width());
        let fill_rect = Rect::from_min_size(rect.min, Vec2::new(fill_w, rect.height()));
        ui.painter()
            .rect_filled(fill_rect, Rounding::same(r), fill_color);
    }
}

/// Custom Apple-inspired vertical EQ band fader (`-10..=10`) with 0 dB center-origin bar,
/// sleek pill thumb, double-click reset, scroll fine-tuning, and keyboard arrow support.
/// Returns `(value_changed, interaction_committed)`.
pub fn vertical_eq_fader(ui: &mut egui::Ui, id_salt: usize, level: &mut i32) -> (bool, bool) {
    let p = ui_palette(ui);
    let desired = Vec2::new(28.0, 124.0);
    let id = ui.make_persistent_id(("eq_fader", id_salt));
    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click_and_drag());
    let mut changed = false;
    let mut committed = false;

    let pad_y = 8.0;
    let top = rect.top() + pad_y;
    let bottom = rect.bottom() - pad_y;
    let span = (bottom - top).max(1.0);

    if resp.double_clicked() {
        if *level != 0 {
            *level = 0;
            changed = true;
        }
        committed = true;
    } else if resp.dragged() || resp.clicked() {
        resp.request_focus();
        if let Some(pos) = resp.interact_pointer_pos() {
            let norm = 1.0 - ((pos.y - top) / span).clamp(0.0, 1.0);
            let new_lv = ((norm * 20.0).round() as i32 - 10).clamp(-10, 10);
            if new_lv != *level {
                *level = new_lv;
                changed = true;
            }
            if resp.clicked() {
                committed = true;
            }
        }
    }

    if resp.drag_stopped() {
        committed = true;
    }

    if resp.hovered() {
        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_y.abs() >= 12.0 {
            let step = if scroll_y > 0.0 { 1 } else { -1 };
            let next = (*level + step).clamp(-10, 10);
            if next != *level {
                *level = next;
                changed = true;
                committed = true;
            }
        }
    }

    let has_focus = ui.memory(|m| m.has_focus(id)) || resp.has_focus();
    if has_focus {
        ui.input(|i| {
            if i.key_pressed(Key::ArrowUp) {
                let next = (*level + 1).min(10);
                if next != *level {
                    *level = next;
                    changed = true;
                    committed = true;
                }
            } else if i.key_pressed(Key::ArrowDown) {
                let next = (*level - 1).max(-10);
                if next != *level {
                    *level = next;
                    changed = true;
                    committed = true;
                }
            } else if (i.key_pressed(Key::Num0) || i.key_pressed(Key::Backspace)) && *level != 0 {
                *level = 0;
                changed = true;
                committed = true;
            }
        });
    }

    let painter = ui.painter();
    let cx = rect.center().x;
    let zero_y = top + span * 0.5;
    let y_val = top + ((10 - (*level).clamp(-10, 10)) as f32 / 20.0) * span;

    // Subtle center 0 dB tick mark
    painter.line_segment(
        [Pos2::new(cx - 7.5, zero_y), Pos2::new(cx + 7.5, zero_y)],
        Stroke::new(1.0_f32, p.chart_zero),
    );

    // Track
    let track_rect = Rect::from_center_size(Pos2::new(cx, rect.center().y), Vec2::new(5.5, span));
    painter.rect_filled(track_rect, Rounding::same(3.0), p.track_bg);

    // Center-origin active fill bar
    if *level != 0 {
        let (y0, y1) = if y_val < zero_y {
            (y_val, zero_y)
        } else {
            (zero_y, y_val)
        };
        let fill_rect = Rect::from_min_max(
            Pos2::new(track_rect.left(), y0),
            Pos2::new(track_rect.right(), y1),
        );
        painter.rect_filled(fill_rect, Rounding::same(3.0), p.accent);
    }

    // Sleek pill thumb
    let hot = resp.hovered() || resp.dragged() || has_focus;
    let thumb_size = if hot {
        Vec2::new(19.0, 10.5)
    } else {
        Vec2::new(17.0, 9.5)
    };
    let thumb_rect = Rect::from_center_size(Pos2::new(cx, y_val), thumb_size);
    let thumb_fill = if *level != 0 || hot {
        p.primary_bg
    } else {
        p.card_bg
    };
    let thumb_border = if hot {
        Stroke::new(2.0_f32, p.accent_strong)
    } else {
        Stroke::new(1.5_f32, p.accent)
    };
    painter.rect(thumb_rect, Rounding::same(4.5), thumb_fill, thumb_border);

    (changed, committed)
}

/// Center-origin horizontal stereo balance slider (`0..=100`, center `50`).
pub fn balance_slider(ui: &mut egui::Ui, value: &mut u8, enabled: bool) -> bool {
    let p = ui_palette(ui);
    let desired = Vec2::new(138.0, 24.0);
    let sense = if enabled {
        Sense::click_and_drag()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(desired, sense);
    let mut changed = false;

    let pad_x = 8.0;
    let left = rect.left() + pad_x;
    let right = rect.right() - pad_x;
    let span = (right - left).max(1.0);

    if enabled {
        if resp.double_clicked() && *value != 50 {
            *value = 50;
            changed = true;
        } else if resp.dragged() || resp.clicked() {
            if let Some(pos) = resp.interact_pointer_pos() {
                let norm = ((pos.x - left) / span).clamp(0.0, 1.0);
                let next = (norm * 100.0).round() as u8;
                if next != *value {
                    *value = next;
                    changed = true;
                }
            }
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
    }

    let painter = ui.painter();
    let cy = rect.center().y;
    let mid_x = left + span * 0.5;
    let x_val = left + ((*value).min(100) as f32 / 100.0) * span;

    painter.line_segment(
        [Pos2::new(mid_x, cy - 6.5), Pos2::new(mid_x, cy + 6.5)],
        Stroke::new(1.0_f32, p.chart_zero),
    );

    let track_rect = Rect::from_center_size(Pos2::new(rect.center().x, cy), Vec2::new(span, 5.5));
    painter.rect_filled(track_rect, Rounding::same(3.0), p.track_bg);

    if *value != 50 {
        let (x0, x1) = if x_val < mid_x {
            (x_val, mid_x)
        } else {
            (mid_x, x_val)
        };
        let fill_rect = Rect::from_min_max(
            Pos2::new(x0, track_rect.top()),
            Pos2::new(x1, track_rect.bottom()),
        );
        painter.rect_filled(
            fill_rect,
            Rounding::same(3.0),
            if enabled { p.accent } else { p.text_muted },
        );
    }

    let thumb_r = if enabled && (resp.hovered() || resp.dragged()) {
        6.5
    } else {
        5.8
    };
    painter.circle(
        Pos2::new(x_val, cy),
        thumb_r,
        if enabled { p.card_bg } else { p.ctrl_bg },
        Stroke::new(1.8_f32, if enabled { p.primary_bg } else { p.ctrl_border }),
    );

    changed
}
