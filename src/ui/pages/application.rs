//! Application settings: Light/Dark/System theme switcher, auto-reconnect policy,
//! keyboard shortcuts, and privacy-redacted virtualized BLE diagnostic log.

use crate::audio;
use crate::protocol as p;
use crate::settings::{self, ThemeMode};
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::{NoticeTone, StudioApp};
use eframe::egui::{self, RichText, Rounding, Stroke};

fn contains_case_insensitive(haystack: &str, needle_lower: &str) -> bool {
    if needle_lower.is_empty() {
        return true;
    }
    if haystack.is_ascii() && needle_lower.is_ascii() {
        let nb = needle_lower.as_bytes();
        haystack
            .as_bytes()
            .windows(nb.len())
            .any(|w| w.eq_ignore_ascii_case(nb))
    } else {
        haystack.to_lowercase().contains(needle_lower)
    }
}

impl StudioApp {
    pub(crate) fn render_application(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let pal = ui_palette(ui);
        let enabled = self.is_enabled();
        let mut chosen_theme: Option<ThemeMode> = None;
        let mut auto_changed: Option<bool> = None;

        draw_card(ui, Some("Tùy chọn ứng dụng"), None, |ui| {
            // 1. Theme selector (Light / Dark / System)
            let cur_theme = self.settings.data.theme;
            setting_row(
                ui,
                38.0,
                |ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    for mode in [ThemeMode::System, ThemeMode::Dark, ThemeMode::Light] {
                        let active = cur_theme == mode;
                        if chip_button(ui, mode.label(), active, true).clicked() && !active {
                            chosen_theme = Some(mode);
                        }
                    }
                },
                |ui| {
                    let rem = ui.available_width().max(80.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(rem);
                        ui.spacing_mut().item_spacing.y = 1.5;
                        ui.label(
                            RichText::new("Chế độ giao diện (Theme)")
                                .font(font_semibold(13.5))
                                .color(pal.text_primary),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    "Chọn giao diện Sáng, Tối hoặc tự động đồng bộ theo hệ điều hành (Alt+T).",
                                )
                                .size(11.4)
                                .color(pal.text_subtitle),
                            )
                            .wrap(),
                        );
                    });
                },
            );

            draw_row_divider(ui);

            // 2. Auto-reconnect toggle
            let mut auto = self.settings.data.auto_connect;
            setting_row(
                ui,
                40.0,
                |ui| {
                    if toggle_switch(ui, "sw_auto_connect", &mut auto, true) {
                        auto_changed = Some(auto);
                    }
                },
                |ui| {
                    let rem = ui.available_width().max(80.0);
                    ui.vertical(|ui| {
                        ui.set_max_width(rem);
                        ui.spacing_mut().item_spacing.y = 1.5;
                        ui.label(
                            RichText::new("Tự động kết nối lại khi đang mở ứng dụng")
                                .font(font_semibold(13.5))
                                .color(pal.text_primary),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    "Tự động quét và kết nối lại mỗi 8 giây khi tai nghe lấy ra khỏi hộp. Đóng cửa sổ sẽ giải phóng kênh BLE ngay.",
                                )
                                .size(11.4)
                                .color(pal.text_subtitle),
                            )
                            .wrap(),
                        );
                    });
                },
            );

            draw_row_divider(ui);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.label(
                    RichText::new("Phím tắt nhanh:")
                        .font(font_medium(12.0))
                        .color(pal.text_subtitle),
                );
                for badge in [
                    "Alt + 1..6: Chuyển trang",
                    "Alt + T: Đổi giao diện Sáng/Tối",
                    "F5: Đọc lại tai nghe",
                    "Enter / Esc: Xác nhận / Hủy hộp thoại",
                ] {
                    status_pill(
                        ui,
                        badge,
                        pal.card_subtle_bg,
                        pal.ctrl_border,
                        pal.text_subtitle,
                    );
                }
            });
        });

        if let Some(mode) = chosen_theme {
            self.set_theme_mode(ctx, mode);
            self.set_notice(
                format!("Đã áp dụng chế độ giao diện: {}.", mode.label()),
                NoticeTone::Info,
            );
        }
        if let Some(auto) = auto_changed {
            self.settings.data.auto_connect = auto;
            self.save_settings();
        }

        ui.add_space(10.0);

        // Live BLE Diagnostic Log with full-width container, zero-alloc ASCII search, and MAC-redacted export
        let mut copy_logs = false;
        let mut clear_logs = false;
        let mut export_logs = false;
        let mut refresh_state = false;

        draw_card(ui, None, None, |ui| {
            let fw = self.state.version.as_deref().unwrap_or("—");
            let codec = self.state.codec.as_deref().unwrap_or("—");
            draw_card_header_with_right(
                ui,
                "Nhật ký chẩn đoán Bluetooth LE",
                Some("Lưu cục bộ trong phiên • Tự động ẩn địa chỉ MAC khi sao chép hoặc xuất tệp."),
                |ui| {
                    status_pill(
                        ui,
                        &format!("FW {} · {}", fw, codec),
                        pal.info_bg,
                        pal.info_border,
                        pal.info_text,
                    );
                },
            );

            setting_row(
                ui,
                32.0,
                |ui| {
                    if secondary_button(ui, "Đọc lại trạng thái", enabled).clicked() {
                        refresh_state = true;
                    }
                },
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        styled_text_input(
                            ui,
                            &mut self.log_filter,
                            "Lọc nhật ký (VD: >>, <<, 0e01)…",
                            195.0,
                        );
                        let has_logs = !self.logs.is_empty();
                        if secondary_button(ui, "Sao chép", has_logs).clicked() {
                            copy_logs = true;
                        }
                        if secondary_button(ui, "Xóa màn hình", has_logs).clicked() {
                            clear_logs = true;
                        }
                        if secondary_button(ui, "Xuất tệp…", has_logs).clicked() {
                            export_logs = true;
                        }
                    });
                },
            );

            ui.add_space(8.0);

            let filter_q = self.log_filter.trim().to_lowercase();
            let filtered: Vec<&str> = if filter_q.is_empty() {
                self.logs.iter().map(String::as_str).collect()
            } else {
                self.logs
                    .iter()
                    .map(String::as_str)
                    .filter(|line| contains_case_insensitive(line, &filter_q))
                    .collect()
            };

            egui::Frame::none()
                .fill(pal.input_bg)
                .stroke(Stroke::new(1.0_f32, pal.ctrl_border))
                .rounding(Rounding::same(10.0))
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    let row_height = 16.5;
                    if self.logs.is_empty() {
                        ui.set_min_height(125.0);
                        ui.label(
                            RichText::new("Chưa có gói tin Bluetooth LE nào trong phiên này.")
                                .monospace()
                                .size(11.2)
                                .color(pal.text_muted),
                        );
                    } else if filtered.is_empty() {
                        ui.set_min_height(125.0);
                        ui.label(
                            RichText::new("Không có dòng nhật ký nào khớp bộ lọc.")
                                .size(11.2)
                                .color(pal.text_muted),
                        );
                    } else {
                        egui::ScrollArea::vertical()
                            .id_salt("ble_log_scroll")
                            .min_scrolled_height(130.0)
                            .max_height(165.0)
                            .stick_to_bottom(true)
                            .show_rows(ui, row_height, filtered.len(), |ui, row_range| {
                                ui.set_width(ui.available_width());
                                for idx in row_range {
                                    let line = filtered[idx];
                                    let col = if line.contains(" ! ") || line.contains(" ? ") {
                                        pal.danger_text
                                    } else if line.contains(" >> ") {
                                        pal.accent_strong
                                    } else {
                                        pal.text_subtitle
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

        if copy_logs {
            let redacted = self
                .logs
                .iter()
                .map(|l| p::redact_sensitive_identifiers(l))
                .collect::<Vec<_>>()
                .join("\n");
            ctx.copy_text(redacted);
            self.set_notice(
                "Đã sao chép nhật ký vào bộ nhớ tạm (đã ẩn địa chỉ MAC).",
                NoticeTone::Success,
            );
        }
        if clear_logs {
            self.logs.clear();
            self.set_notice("Đã làm sạch khung hiển thị nhật ký.", NoticeTone::Info);
        }
        if export_logs {
            if let Some(path) = rfd::FileDialog::new()
                .set_title("Lưu nhật ký Bluetooth")
                .set_file_name("earfun-bluetooth.log")
                .save_file()
            {
                let mut redacted = self
                    .logs
                    .iter()
                    .map(|l| p::redact_sensitive_identifiers(l))
                    .collect::<Vec<_>>()
                    .join("\n");
                redacted.push('\n');
                match settings::write_atomic(&path, redacted.as_bytes()) {
                    Ok(()) => self.set_notice(
                        "Đã xuất tệp nhật ký (đã ẩn địa chỉ MAC).",
                        NoticeTone::Success,
                    ),
                    Err(e) => self.show_notice(format!("Không ghi được nhật ký: {}", e), true),
                }
            }
        }
        if refresh_state {
            self.trigger_manual_query();
        }

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
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Các tính năng đám mây, cập nhật firmware OTA, cài gói giọng nói mới và bài đo thính lực cá nhân hóa (Adaptive EQ / Auracast) không được giả lập trên máy tính để đảm bảo an toàn phần cứng; vui lòng dùng EarFun Audio chính hãng khi cần.",
                        )
                        .size(11.6)
                        .color(pal.text_subtitle),
                    )
                    .wrap(),
                );
                ui.add_space(8.0);
                ui.horizontal_wrapped(|ui| {
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
