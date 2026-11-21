//! Touch Gestures page: TW600-verified operation choices per gesture & firmware version,
//! instant BLE confirmation, and per-earbud touch lock toggles.

use crate::protocol as p;
use crate::ui::icons::IconKind;
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::StudioApp;
use eframe::egui::{self, RichText, Rounding, Stroke};

impl StudioApp {
    pub(crate) fn render_gestures(&mut self, ui: &mut egui::Ui) {
        let pal = ui_palette(ui);
        let enabled = self.is_enabled();
        let fw_version = self.state.version.clone();
        let mut save_req: Option<(String, String, String)> = None;
        let mut lock_req: Option<(bool, bool)> = None;

        ui.columns(2, |cols| {
            for (col_idx, (side, title, sub_hint)) in [
                ("L", "Tai trái (L)", "Chọn thao tác chạm bên trái"),
                ("R", "Tai phải (R)", "Chọn thao tác chạm bên phải"),
            ]
            .into_iter()
            .enumerate()
            {
                let both_known = self.state.dis_l.is_some() && self.state.dis_r.is_some();
                let mut locked = if side == "L" {
                    self.state.dis_l.unwrap_or(false)
                } else {
                    self.state.dis_r.unwrap_or(false)
                };

                draw_card(&mut cols[col_idx], None, None, |ui| {
                    setting_row(
                        ui,
                        38.0,
                        |ui| {
                            if locked {
                                status_pill(
                                    ui,
                                    "Đã khóa",
                                    pal.warn_bg,
                                    pal.warn_border,
                                    pal.warn_text,
                                );
                            }
                        },
                        |ui| {
                            draw_icon_badge(
                                ui,
                                36.0,
                                pal.badge_icon_bg,
                                if locked {
                                    IconKind::Lock
                                } else {
                                    IconKind::Earbud
                                },
                                if locked { pal.warn_text } else { pal.accent },
                            );
                            let rem = ui.available_width().max(60.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(rem);
                                ui.spacing_mut().item_spacing.y = 1.5;
                                ui.label(
                                    RichText::new(title)
                                        .font(font_semibold(15.2))
                                        .color(pal.text_primary),
                                );
                                ui.label(
                                    RichText::new(sub_hint).size(11.4).color(pal.text_subtitle),
                                );
                            });
                        },
                    );

                    ui.add_space(10.0);

                    for (g_idx, (gesture, _)) in p::GESTURES.iter().enumerate() {
                        if g_idx > 0 {
                            draw_row_divider(ui);
                        }
                        let field = format!("{}.{}", side, gesture);
                        let current_idx = *self.draft_keys.get(&field).unwrap_or(&0);
                        let selected_label = current_idx
                            .checked_sub(1)
                            .and_then(|i| p::OPERATIONS.get(i))
                            .map(|(lbl, _)| *lbl)
                            .unwrap_or("—");

                        let tap_badge = match *gesture {
                            "Chạm một lần" => "1×",
                            "Chạm hai lần" => "2×",
                            "Chạm ba lần" => "3×",
                            _ => "Giữ",
                        };

                        let allowed_ops = p::allowed_operations(gesture, fw_version.as_deref());
                        let row_fg = if locked {
                            pal.text_muted
                        } else {
                            pal.text_primary
                        };

                        setting_row(
                            ui,
                            33.0,
                            |ui| {
                                let combo_w = (ui.available_width() * 0.50).clamp(106.0, 175.0);
                                ui.add_enabled_ui(enabled && !locked, |ui| {
                                    let mut new_idx = current_idx;
                                    egui::ComboBox::from_id_salt(&field)
                                        .width(combo_w)
                                        .selected_text(
                                            RichText::new(selected_label).size(12.3).color(row_fg),
                                        )
                                        .show_ui(ui, |ui| {
                                            for (op_name, _) in &allowed_ops {
                                                if let Some(full_pos) = p::OPERATIONS
                                                    .iter()
                                                    .position(|(n, _)| n == op_name)
                                                {
                                                    ui.selectable_value(
                                                        &mut new_idx,
                                                        full_pos + 1,
                                                        *op_name,
                                                    );
                                                }
                                            }
                                        });
                                    if new_idx != current_idx {
                                        if let Some((act, _)) = new_idx
                                            .checked_sub(1)
                                            .and_then(|i| p::OPERATIONS.get(i))
                                        {
                                            self.draft_keys.insert(field.clone(), new_idx);
                                            save_req = Some((
                                                side.to_string(),
                                                gesture.to_string(),
                                                (*act).to_string(),
                                            ));
                                        }
                                    }
                                });
                            },
                            |ui| {
                                egui::Frame::none()
                                    .fill(pal.badge_icon_bg)
                                    .stroke(Stroke::new(1.0_f32, pal.ctrl_border))
                                    .rounding(Rounding::same(8.0))
                                    .inner_margin(egui::Margin::symmetric(7.0, 3.0))
                                    .show(ui, |ui| {
                                        ui.set_min_width(24.0);
                                        ui.label(
                                            RichText::new(tap_badge)
                                                .font(font_semibold(11.0))
                                                .color(pal.accent_strong),
                                        );
                                    });
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(*gesture)
                                            .font(font_medium(12.8))
                                            .color(row_fg),
                                    )
                                    .truncate(),
                                );
                            },
                        );
                    }

                    ui.add_space(4.0);
                    draw_row_divider(ui);
                    ui.add_space(2.0);

                    setting_row(
                        ui,
                        36.0,
                        |ui| {
                            let sw_id = format!("sw_lock_{}", side);
                            if toggle_switch(ui, &sw_id, &mut locked, enabled && both_known) {
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
                        |ui| {
                            let rem = ui.available_width().max(60.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(rem);
                                ui.spacing_mut().item_spacing.y = 1.5;
                                ui.label(
                                    RichText::new("Tắt cảm ứng bên này")
                                        .font(font_medium(12.8))
                                        .color(pal.text_primary),
                                );
                                ui.add(
                                    egui::Label::new(
                                        RichText::new("Tránh chạm nhầm khi nằm nghỉ")
                                            .size(11.0)
                                            .color(pal.text_subtitle),
                                    )
                                    .wrap(),
                                );
                            });
                        },
                    );
                });
            }
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Các tùy chọn cảm ứng được lọc chính xác theo khả năng phần cứng TW600 và phiên bản firmware hiện tại.",
            )
            .size(11.4)
            .color(pal.text_muted),
        );

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
}
