//! Multipoint Dual-Device page: proactive LDAC conflict awareness and paired host management.

use crate::ui::icons::{draw_vector_icon, IconKind};
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::StudioApp;
use crate::workflows;
use eframe::egui::{self, RichText, Rounding, Sense, Stroke, Vec2};

impl StudioApp {
    pub(crate) fn render_multipoint(&mut self, ui: &mut egui::Ui) {
        let pal = ui_palette(ui);
        let enabled = self.is_enabled();
        let mut trigger_dual: Option<bool> = None;
        let mut pair_new = false;
        let mut refresh_list = false;

        draw_card(
            ui,
            Some("Kết nối đa điểm (Multipoint)"),
            Some("Giữ kết nối đồng thời và tự động chuyển âm thanh giữa tối đa 2 thiết bị."),
            |ui| {
                let mut dual_on = self.state.dual.unwrap_or(false);
                let avail = enabled && self.state.dual.is_some();
                setting_row(
                    ui,
                    40.0,
                    |ui| {
                        if toggle_switch(ui, "sw_dual", &mut dual_on, avail) {
                            trigger_dual = Some(dual_on);
                        }
                    },
                    |ui| {
                        draw_icon_badge(ui, 36.0, pal.badge_icon_bg, IconKind::Devices, pal.accent);
                        let rem = ui.available_width().max(80.0);
                        ui.vertical(|ui| {
                            ui.set_max_width(rem);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new("Cho phép kết nối hai thiết bị cùng lúc")
                                    .font(font_semibold(13.6))
                                    .color(pal.text_primary),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(
                                        "Ví dụ: vừa kết nối máy tính vừa nhận cuộc gọi trên điện thoại",
                                    )
                                    .size(11.4)
                                    .color(pal.text_subtitle),
                                )
                                .wrap(),
                            );
                        });
                    },
                );

                // Proactive notice when LDAC is enabled
                if self.state.ldac == Some(true) && self.state.dual == Some(false) {
                    ui.add_space(8.0);
                    egui::Frame::none()
                        .fill(pal.warn_bg)
                        .stroke(Stroke::new(1.0_f32, pal.warn_border))
                        .rounding(Rounding::same(10.0))
                        .inner_margin(egui::Margin::symmetric(12.0, 8.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.spacing_mut().interact_size.y = 16.0;
                                let (ir, _) =
                                    ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
                                draw_vector_icon(ui.painter(), ir, IconKind::Alert, pal.warn_text);
                                ui.add(
                                    egui::Label::new(
                                        RichText::new(
                                            "Codec LDAC đang bật. Khi bạn bật Đa điểm, ứng dụng sẽ hỏi xác nhận để tự động tắt LDAC trước.",
                                        )
                                        .size(11.5)
                                        .color(pal.warn_text),
                                    )
                                    .wrap(),
                                );
                            });
                        });
                }

                ui.add_space(6.0);
                draw_row_divider(ui);
                ui.add_space(4.0);

                ui.horizontal_wrapped(|ui| {
                    if secondary_button(ui, "Làm mới danh sách", enabled).clicked() {
                        refresh_list = true;
                    }
                    if primary_button(ui, "Mở ghép thiết bị mới", enabled)
                        .on_hover_text(
                            "Đưa tai nghe vào chế độ ghép đôi để kết nối thêm thiết bị thứ hai",
                        )
                        .clicked()
                    {
                        pair_new = true;
                    }
                });
            },
        );

        ui.add_space(10.0);

        let mut phone_cmd: Option<(u8, u8)> = None;
        draw_card(ui, None, None, |ui| {
            let active_cnt = self.phones.iter().filter(|p| p.connected).count();
            draw_card_header_with_right(
                ui,
                "Thiết bị đã ghép trên tai nghe",
                Some("Danh sách đọc trực tiếp từ bộ nhớ của EarFun Air Pro 4."),
                |ui| {
                    if !self.phones.is_empty() {
                        status_pill(
                            ui,
                            &format!("{}/2 đang kết nối", active_cnt),
                            pal.info_bg,
                            pal.info_border,
                            pal.info_text,
                        );
                    }
                },
            );

            if self.phones.is_empty() {
                egui::Frame::none()
                    .fill(pal.card_subtle_bg)
                    .stroke(Stroke::new(1.0_f32, pal.card_border))
                    .rounding(Rounding::same(12.0))
                    .inner_margin(egui::Margin::symmetric(16.0, 18.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.vertical_centered(|ui| {
                            let hint = if self.is_connected() {
                                "Chưa nhận được danh sách thiết bị. Bấm “Làm mới danh sách” ở trên."
                            } else {
                                "Kết nối tai nghe để xem và quản lý các thiết bị đã ghép."
                            };
                            ui.label(RichText::new(hint).size(12.2).color(pal.text_subtitle));
                        });
                    });
            } else {
                for (idx, phone) in self.phones.iter().enumerate() {
                    if idx > 0 {
                        draw_row_divider(ui);
                    }
                    let name = if phone.name.is_empty() {
                        "Thiết bị Bluetooth"
                    } else {
                        &phone.name
                    };
                    let mac_fmt = Self::format_mac_tail(&phone.mac);
                    let cmd = if phone.connected { 42u8 } else { 41u8 };
                    let btn_label = if phone.connected {
                        "Ngắt kết nối"
                    } else {
                        "Kết nối"
                    };

                    setting_row(
                        ui,
                        40.0,
                        |ui| {
                            if secondary_button(ui, btn_label, enabled).clicked() {
                                phone_cmd = Some((cmd, phone.num));
                            }
                            if phone.connected {
                                status_pill(
                                    ui,
                                    "Đang kết nối",
                                    pal.success_bg,
                                    pal.success_border,
                                    pal.success_text,
                                );
                            } else {
                                status_pill(
                                    ui,
                                    "Đã lưu",
                                    pal.card_subtle_bg,
                                    pal.ctrl_border,
                                    pal.text_subtitle,
                                );
                            }
                        },
                        |ui| {
                            draw_icon_badge(
                                ui,
                                34.0,
                                if phone.connected {
                                    pal.success_bg
                                } else {
                                    pal.badge_icon_bg
                                },
                                IconKind::Devices,
                                if phone.connected {
                                    pal.success_text
                                } else {
                                    pal.text_muted
                                },
                            );
                            let rem = ui.available_width().max(60.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(rem);
                                ui.spacing_mut().item_spacing.y = 2.0;
                                ui.label(
                                    RichText::new(name)
                                        .font(font_semibold(13.4))
                                        .color(pal.text_primary),
                                );
                                ui.label(
                                    RichText::new(format!("MAC ••:{}", mac_fmt))
                                        .size(11.0)
                                        .color(pal.text_muted),
                                );
                            });
                        },
                    );
                }
            }
        });

        if refresh_list {
            let core = self.backend.core.clone();
            self.backend
                .spawn_task(async move { core.phones_action(44, None).await });
        }
        if let Some(on) = trigger_dual {
            let plan_res = workflows::dual(&self.state, on);
            self.execute_plan(plan_res);
        }
        if pair_new {
            self.ask_confirm(
                "Đưa tai nghe vào chế độ ghép đôi Bluetooth mới? Tiếp tục?",
                |app| {
                    let core = app.backend.core.clone();
                    app.backend
                        .spawn_task(async move { core.phones_action(43, None).await });
                },
            );
        }
        if let Some((cmd, num)) = phone_cmd {
            self.ask_confirm(
                "Thay đổi kết nối thiết bị này có thể tạm ngắt âm thanh hiện tại. Tiếp tục?",
                move |app| {
                    let core = app.backend.core.clone();
                    app.backend
                        .spawn_task(async move { core.phones_action(cmd, Some(num)).await });
                },
            );
        }
    }

    fn format_mac_tail(raw: &str) -> String {
        if raw.len() == 6 && raw.is_ascii() {
            format!("{}:{}:{}", &raw[0..2], &raw[2..4], &raw[4..6])
        } else {
            raw.to_string()
        }
    }
}
