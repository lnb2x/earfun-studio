//! Application settings, keyboard shortcuts reference, and virtualized BLE diagnostic log.

use crate::audio;
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::StudioApp;
use eframe::egui::{self, Color32, RichText, Rounding, Stroke};

impl StudioApp {
    pub(crate) fn render_application(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let enabled = self.is_enabled();

        draw_card(ui, Some("Tùy chọn ứng dụng"), None, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("Tự động kết nối lại khi đang mở ứng dụng")
                            .font(font_semibold(13.5))
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(
                            "Tự động quét và kết nối lại mỗi 8 giây khi tai nghe lấy ra khỏi hộp. Đóng cửa sổ sẽ giải phóng kênh BLE ngay.",
                        )
                        .size(11.5)
                        .color(TEXT_SUBTITLE),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let mut auto = self.settings.data.auto_connect;
                    if toggle_switch(ui, "sw_auto_connect", &mut auto, true) {
                        self.settings.data.auto_connect = auto;
                        self.save_settings();
                    }
                });
            });

            draw_row_divider(ui);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    RichText::new("Phím tắt nhanh:")
                        .font(font_medium(12.0))
                        .color(TEXT_SUBTITLE),
                );
                for badge in [
                    "Alt + 1..6: Chuyển trang",
                    "F5: Đọc lại tai nghe",
                    "Enter / Esc: Xác nhận / Hủy hộp thoại",
                ] {
                    status_pill(
                        ui,
                        badge,
                        Color32::from_rgb(245, 247, 252),
                        CTRL_BORDER,
                        TEXT_SUBTITLE,
                    );
                }
            });
        });

        ui.add_space(10.0);

        // Live BLE Diagnostic Log with virtualized row rendering, search filter, Copy, Clear, Export
        draw_card(ui, None, None, |ui| {
            let fw = self.state.version.as_deref().unwrap_or("—");
            let codec = self.state.codec.as_deref().unwrap_or("—");
            draw_card_header_with_right(
                ui,
                "Nhật ký chẩn đoán Bluetooth LE",
                Some("Lưu cục bộ trong phiên làm việc • Không gửi dữ liệu ra ngoài."),
                |ui| {
                    status_pill(
                        ui,
                        &format!("FW {} · {}", fw, codec),
                        INFO_BG,
                        INFO_BORDER,
                        ACCENT_INDIGO,
                    );
                },
            );

            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut self.log_filter)
                        .hint_text("Lọc nhật ký (VD: >>, <<, 0e01)…")
                        .desired_width(210.0),
                );
                let has_logs = !self.logs.is_empty();
                if secondary_button(ui, "Sao chép", has_logs).clicked() {
                    let joined: String = self.logs.iter().cloned().collect::<Vec<_>>().join("\n");
                    ctx.copy_text(joined);
                    self.show_notice("Đã sao chép nhật ký vào bộ nhớ tạm.", false);
                }
                if secondary_button(ui, "Xóa màn hình", has_logs).clicked() {
                    self.logs.clear();
                    self.show_notice("Đã làm sạch khung hiển thị nhật ký.", false);
                }
                if secondary_button(ui, "Xuất tệp…", has_logs).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_title("Lưu nhật ký Bluetooth")
                        .set_file_name("earfun-bluetooth.log")
                        .save_file()
                    {
                        let joined: String =
                            self.logs.iter().cloned().collect::<Vec<_>>().join("\n");
                        match std::fs::write(&path, joined) {
                            Ok(()) => self.show_notice("Đã xuất tệp nhật ký.", false),
                            Err(e) => self.show_notice(e.to_string(), true),
                        }
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if secondary_button(ui, "Đọc lại trạng thái", enabled).clicked() {
                        self.trigger_manual_query();
                    }
                });
            });

            ui.add_space(6.0);

            let filter_q = self.log_filter.trim().to_ascii_lowercase();
            let filtered: Vec<&str> = if filter_q.is_empty() {
                self.logs.iter().map(String::as_str).collect()
            } else {
                self.logs
                    .iter()
                    .map(String::as_str)
                    .filter(|line| line.to_lowercase().contains(&filter_q))
                    .collect()
            };

            egui::Frame::none()
                .fill(Color32::from_rgb(247, 249, 254))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(222, 229, 241)))
                .rounding(Rounding::same(10.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    let row_height = 16.0;
                    if self.logs.is_empty() {
                        ui.set_min_height(130.0);
                        ui.label(
                            RichText::new("Chưa có gói tin Bluetooth LE nào trong phiên này.")
                                .monospace()
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                    } else if filtered.is_empty() {
                        ui.set_min_height(130.0);
                        ui.label(
                            RichText::new("Không có dòng nhật ký nào khớp bộ lọc.")
                                .size(11.0)
                                .color(TEXT_MUTED),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .min_scrolled_height(135.0)
                            .max_height(165.0)
                            .stick_to_bottom(true)
                            .show_rows(ui, row_height, filtered.len(), |ui, row_range| {
                                ui.set_width(ui.available_width());
                                for idx in row_range {
                                    let line = filtered[idx];
                                    let col = if line.contains(" ! ") || line.contains(" ? ") {
                                        DANGER_TEXT
                                    } else if line.contains(" >> ") {
                                        ACCENT_INDIGO
                                    } else {
                                        TEXT_SUBTITLE
                                    };
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(line).monospace().size(11.0).color(col),
                                        )
                                        .selectable(true),
                                    );
                                }
                            });
                    }
                });
        });

        ui.add_space(10.0);

        draw_card(
            ui,
            Some(concat!(
                "Giới thiệu EarFun Studio ",
                env!("CARGO_PKG_VERSION"),
                " (Rust)"
            )),
            Some(
                "Ứng dụng điều khiển độc lập cho Windows & Ubuntu Linux (eframe/egui + btleplug).",
            ),
            |ui| {
                ui.label(
                    RichText::new(
                        "Các tính năng đám mây, cập nhật firmware OTA, gói giọng nói mới và đo thính lực cá nhân hóa hiện vẫn thực hiện qua ứng dụng EarFun Audio trên điện thoại.",
                    )
                    .size(11.8)
                    .color(TEXT_SUBTITLE),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if secondary_button(ui, "Mã nguồn trên GitHub", true).clicked() {
                        if let Err(e) =
                            audio::open_url_or_path("https://github.com/lnb2x/earfun-studio")
                        {
                            self.show_notice(e, true);
                        }
                    }
                    if secondary_button(ui, "Thoát ứng dụng", true).clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            },
        );
    }
}
