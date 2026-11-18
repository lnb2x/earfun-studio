//! Touch Gestures page: clean single-row selectors with instant BLE apply and touch lock badges.

use crate::protocol as p;
use crate::ui::icons::IconKind;
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::StudioApp;
use eframe::egui::{self, Color32, RichText, Rounding, Stroke};

impl StudioApp {
    pub(crate) fn render_gestures(&mut self, ui: &mut egui::Ui) {
        let enabled = self.is_enabled();
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
                    // Header with Earbud icon + Touch Lock pill if locked
                    ui.horizontal(|ui| {
                        draw_icon_badge(
                            ui,
                            36.0,
                            Color32::from_rgb(236, 241, 253),
                            if locked {
                                IconKind::Lock
                            } else {
                                IconKind::Earbud
                            },
                            if locked { WARN_TEXT } else { ACCENT_BLUE },
                        );
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.5;
                            ui.label(
                                RichText::new(title)
                                    .font(font_semibold(15.5))
                                    .color(TEXT_PRIMARY),
                            );
                            ui.label(RichText::new(sub_hint).size(11.5).color(TEXT_SUBTITLE));
                        });
                        if locked {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    status_pill(ui, "Đã khóa", WARN_BG, WARN_BORDER, WARN_TEXT);
                                },
                            );
                        }
                    });

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

                        ui.horizontal(|ui| {
                            egui::Frame::none()
                                .fill(Color32::from_rgb(238, 242, 252))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(216, 225, 244)))
                                .rounding(Rounding::same(8.0))
                                .inner_margin(egui::Margin::symmetric(7.0, 3.0))
                                .show(ui, |ui| {
                                    ui.set_min_width(24.0);
                                    ui.label(
                                        RichText::new(tap_badge)
                                            .font(font_semibold(11.0))
                                            .color(ACCENT_INDIGO),
                                    );
                                });

                            let row_fg = if locked { TEXT_MUTED } else { TEXT_PRIMARY };
                            ui.label(
                                RichText::new(*gesture)
                                    .font(font_medium(13.0))
                                    .color(row_fg),
                            );

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let combo_w = (ui.available_width()).clamp(108.0, 175.0);
                                    ui.add_enabled_ui(enabled && !locked, |ui| {
                                        let mut new_idx = current_idx;
                                        egui::ComboBox::from_id_salt(&field)
                                            .width(combo_w)
                                            .selected_text(
                                                RichText::new(selected_label)
                                                    .size(12.5)
                                                    .color(row_fg),
                                            )
                                            .show_ui(ui, |ui| {
                                                for (i, (op_name, _)) in
                                                    p::OPERATIONS.iter().enumerate()
                                                {
                                                    ui.selectable_value(
                                                        &mut new_idx,
                                                        i + 1,
                                                        *op_name,
                                                    );
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
                            );
                        });
                    }

                    ui.add_space(4.0);
                    draw_row_divider(ui);
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.5;
                            ui.label(
                                RichText::new("Tắt cảm ứng bên này")
                                    .font(font_medium(13.0))
                                    .color(TEXT_PRIMARY),
                            );
                            ui.label(
                                RichText::new("Tránh chạm nhầm khi nằm nghỉ")
                                    .size(11.0)
                                    .color(TEXT_SUBTITLE),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
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
                        });
                    });
                });
            }
        });

        ui.add_space(8.0);
        ui.label(
            RichText::new(
                "Thay đổi chức năng cảm ứng sẽ được gửi và xác nhận trực tiếp với tai nghe ngay khi bạn chọn.",
            )
            .size(11.5)
            .color(TEXT_MUTED),
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
