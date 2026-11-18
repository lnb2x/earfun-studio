//! Earbuds settings page: 1-click prompt volume & mic chips, balance slider with center snap,
//! Bluetooth LE Audio & codec switches with live "Đang phát" indicator, and 10s Find My countdown.

use crate::ui::icons::{draw_vector_icon, IconKind};
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::StudioApp;
use crate::workflows;
use eframe::egui::{self, RichText, Sense, Vec2};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const PROMPT_CHIPS: &[(&str, u8, u8)] = &[
    ("Tắt", 0, 4),
    ("25%", 25, 3),
    ("50%", 50, 2),
    ("75%", 75, 1),
    ("100%", 100, 0),
];

const MIC_CHIPS: &[(&str, &str, u8)] = &[
    ("Tự động", "AUTO", 0),
    ("Tai trái", "L", 1),
    ("Tai phải", "R", 2),
];

const HEARING_LIMIT_OPTS: &[(&str, u8)] = &[
    ("Tắt giới hạn", 0),
    ("Tối đa 90 dB", 1),
    ("Tối đa 80 dB", 2),
    ("Tối đa 70 dB", 3),
    ("Tối đa 60 dB", 4),
    ("Tối đa 50 dB", 5),
];

impl StudioApp {
    pub(crate) fn render_earbuds(&mut self, ui: &mut egui::Ui) {
        let enabled = self.is_enabled();
        let mut trigger_le_audio: Option<bool> = None;
        let mut trigger_codec: Option<(&'static str, bool)> = None;
        let mut find_req: Option<u8> = None;
        let mut reset_req = false;

        draw_card(ui, Some("Âm thanh & Cuộc gọi"), None, |ui| {
            // 1. Âm lượng thông báo (1-click chips)
            let prompt_avail = enabled && self.state.prompt.is_some();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Âm lượng giọng thông báo")
                        .font(font_medium(13.2))
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    for &(lbl, pct, wire_idx) in PROMPT_CHIPS.iter().rev() {
                        let active = self.state.prompt == Some(pct);
                        if chip_button(ui, lbl, active, prompt_avail).clicked() && !active {
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_u8_value(56, 57, wire_idx, "prompt", pct).await
                            });
                        }
                    }
                });
            });

            draw_row_divider(ui);

            // 2. Micro cuộc gọi (1-click chips)
            let mic_avail = enabled && self.state.mic.is_some();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Micro đàm thoại")
                        .font(font_medium(13.2))
                        .color(TEXT_PRIMARY),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 5.0;
                    for &(lbl, key, wire_idx) in MIC_CHIPS.iter().rev() {
                        let active = self.state.mic.as_deref() == Some(key);
                        if chip_button(ui, lbl, active, mic_avail).clicked() && !active {
                            let wanted = key.to_string();
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_string_value(48, 49, wire_idx, "mic", &wanted)
                                    .await
                            });
                        }
                    }
                });
            });

            draw_row_divider(ui);

            // 3. Giới hạn âm lượng bảo vệ thính giác (direct apply on ComboBox select)
            let hl_avail = enabled && self.state.hearing_limit.is_some();
            let cur_hl = self.state.hearing_limit;
            let hl_label = cur_hl
                .and_then(|v| HEARING_LIMIT_OPTS.iter().find(|(_, code)| *code == v))
                .map(|(lbl, _)| *lbl)
                .unwrap_or("—");

            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.5;
                    ui.label(
                        RichText::new("Giới hạn âm lượng bảo vệ tai")
                            .font(font_medium(13.2))
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new("Giới hạn trần âm lượng đầu ra tối đa trên tai nghe")
                            .size(11.2)
                            .color(TEXT_SUBTITLE),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(hl_avail, |ui| {
                        let mut selected = cur_hl.unwrap_or(255);
                        egui::ComboBox::from_id_salt("choice_hl")
                            .width(165.0)
                            .selected_text(RichText::new(hl_label).size(12.5).color(TEXT_PRIMARY))
                            .show_ui(ui, |ui| {
                                for &(lbl, code) in HEARING_LIMIT_OPTS {
                                    ui.selectable_value(&mut selected, code, lbl);
                                }
                            });
                        if selected <= 5 && Some(selected) != cur_hl {
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_u8_value(83, 84, selected, "hearing_limit", selected)
                                    .await
                            });
                        }
                    });
                });
            });

            draw_row_divider(ui);

            // 4. Cân bằng trái / phải
            let bal_avail = enabled && self.state.balance.is_some();
            let clamped_bal = self.draft_balance.min(100);
            let bal_desc = if clamped_bal == 50 {
                "Cân bằng ở giữa (50)".to_string()
            } else if clamped_bal < 50 {
                format!("Nghiêng trái +{}%", 50 - clamped_bal)
            } else {
                format!("Nghiêng phải +{}%", clamped_bal - 50)
            };
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.5;
                    ui.label(
                        RichText::new("Cân bằng âm lượng Trái / Phải")
                            .font(font_medium(13.2))
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(RichText::new(bal_desc).size(11.2).color(TEXT_SUBTITLE));
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let can_apply = bal_avail && Some(self.draft_balance) != self.state.balance;
                    if action_button(ui, "Áp dụng", can_apply).clicked() {
                        let v = self.draft_balance;
                        let core = self.backend.core.clone();
                        self.backend.spawn_task(async move {
                            core.set_u8_value(85, 86, v, "balance", v).await
                        });
                    }
                    if self.draft_balance != 50
                        && secondary_button(ui, "Giữa", bal_avail)
                            .on_hover_text("Đưa về vị trí cân bằng 50")
                            .clicked()
                    {
                        self.draft_balance = 50;
                        self.balance_dirty = true;
                    }
                    ui.add_enabled_ui(bal_avail, |ui| {
                        ui.label(RichText::new("R").size(11.0).color(TEXT_MUTED));
                        ui.spacing_mut().slider_width = 135.0;
                        if ui
                            .add(
                                egui::Slider::new(&mut self.draft_balance, 0..=100)
                                    .show_value(false),
                            )
                            .changed()
                        {
                            self.balance_dirty = true;
                        }
                        ui.label(RichText::new("L").size(11.0).color(TEXT_MUTED));
                    });
                });
            });

            draw_row_divider(ui);

            // 5. Gói giọng thông báo
            let lang_text = match self.state.voice_language.as_deref() {
                Some("EN") => "English",
                Some("ZH") => "中文",
                Some("DE") => "Deutsch",
                Some("JA") => "日本語",
                _ => "Mặc định (EarFun)",
            };
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 1.5;
                    ui.label(
                        RichText::new("Ngôn ngữ giọng thông báo")
                            .font(font_medium(13.2))
                            .color(TEXT_PRIMARY),
                    );
                    ui.label(
                        RichText::new(
                            "Nạp gói ngôn ngữ mới qua ứng dụng EarFun Audio trên di động",
                        )
                        .size(11.2)
                        .color(TEXT_SUBTITLE),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    status_pill(ui, lang_text, INFO_BG, INFO_BORDER, ACCENT_INDIGO);
                });
            });
        });

        ui.add_space(10.0);

        // Bluetooth Mode & Codecs Card
        draw_card(
            ui,
            Some("Kết nối Bluetooth & Codec"),
            Some("Thay đổi chế độ Bluetooth hoặc codec sẽ tự khởi động lại tai nghe và xác nhận lại."),
            |ui| {
                let le_avail = enabled && self.state.le_audio.is_some();
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.5;
                        ui.label(
                            RichText::new("Chế độ kết nối Bluetooth")
                                .font(font_medium(13.2))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("LE Audio (LC3) hoặc Bluetooth thông thường kèm Google Fast Pair")
                                .size(11.2)
                                .color(TEXT_SUBTITLE),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 5.0;
                        let is_le = self.state.le_audio == Some(true);
                        let is_std = self.state.le_audio == Some(false);
                        if chip_button(ui, "LE Audio", is_le, le_avail).clicked() && !is_le {
                            trigger_le_audio = Some(true);
                        }
                        if chip_button(ui, "Thông thường · Fast Pair", is_std, le_avail).clicked()
                            && !is_std
                        {
                            trigger_le_audio = Some(false);
                        }
                    });
                });

                let codecs_known = ["aptx", "adaptive", "lossless", "ldac", "dual"]
                    .iter()
                    .all(|k| self.state.get_bool(k).is_some());
                let active_codec_str = self.state.codec.as_deref().unwrap_or("");

                for (key, label, desc, match_name) in [
                    (
                        "ldac",
                        "LDAC",
                        "Truyền tải Hi-Res tối đa 990 kbps • Sẽ tắt kết nối Đa điểm",
                        "LDAC",
                    ),
                    (
                        "aptx",
                        "Qualcomm aptX",
                        "Âm thanh độ nét cao cổ điển trên chip Snapdragon/Qualcomm",
                        "aptX",
                    ),
                    (
                        "adaptive",
                        "aptX Adaptive",
                        "Tự động tối ưu giữa chất lượng âm thanh và độ ổn định sóng",
                        "aptX Adaptive",
                    ),
                    (
                        "lossless",
                        "aptX Lossless",
                        "Truyền tải không suy hao chuẩn CD 16-bit/44.1kHz",
                        "aptX Lossless",
                    ),
                ] {
                    draw_row_divider(ui);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 1.5;
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(label)
                                        .font(font_medium(13.2))
                                        .color(TEXT_PRIMARY),
                                );
                                if active_codec_str.eq_ignore_ascii_case(match_name) {
                                    status_pill(
                                        ui,
                                        "Đang phát",
                                        SUCCESS_BG,
                                        SUCCESS_BORDER,
                                        SUCCESS_TEXT,
                                    );
                                }
                            });
                            ui.label(RichText::new(desc).size(11.2).color(TEXT_SUBTITLE));
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let mut val = self.state.get_bool(key).unwrap_or(false);
                            let sw_id = format!("sw_codec_{}", key);
                            if toggle_switch(ui, &sw_id, &mut val, enabled && codecs_known) {
                                trigger_codec = Some((key, val));
                            }
                        });
                    });
                }
            },
        );

        ui.add_space(10.0);

        // Two-column bottom row: Find My Earbuds (with live 10s countdown!) & Factory Reset
        ui.columns(2, |cols| {
            draw_card(
                &mut cols[0],
                Some("Tìm tai nghe"),
                Some("Tháo tai nghe khỏi tai trước khi phát. Âm báo tự ngắt sau 10 giây."),
                |ui| {
                    let remaining_secs = self.find_until.and_then(|until| {
                        let now = Instant::now();
                        if until > now {
                            Some((until - now).as_secs_f32())
                        } else {
                            None
                        }
                    });
                    if remaining_secs.is_none() && self.find_until.is_some() {
                        self.find_until = None;
                    }

                    ui.horizontal_wrapped(|ui| {
                        if secondary_button(ui, "Phát tai trái", enabled).clicked() {
                            find_req = Some(1);
                        }
                        if secondary_button(ui, "Phát tai phải", enabled).clicked() {
                            find_req = Some(2);
                        }
                        if remaining_secs.is_some() {
                            if danger_button(ui, "Dừng ngay", self.is_connected()).clicked() {
                                self.find_until = None;
                                let core = self.backend.core.clone();
                                self.backend.spawn_task(async move { core.find(0).await });
                            }
                        } else if secondary_button(ui, "Dừng tìm", enabled).clicked() {
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move { core.find(0).await });
                        }
                    });

                    if let Some(rem) = remaining_secs {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            let (ir, _) = ui.allocate_exact_size(Vec2::splat(15.0), Sense::hover());
                            draw_vector_icon(ui.painter(), ir, IconKind::SoundWave, ACCENT_INDIGO);
                            ui.label(
                                RichText::new(format!(
                                    "Đang phát tiếng tìm • Tự dừng sau {:.0} giây…",
                                    rem.ceil()
                                ))
                                .font(font_medium(11.5))
                                .color(ACCENT_INDIGO),
                            );
                        });
                        ui.add_space(4.0);
                        let (bar_r, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 5.0),
                            Sense::hover(),
                        );
                        draw_progress_bar(ui, bar_r, rem / 10.0, ACCENT_BLUE);
                        ui.ctx().request_repaint_after(Duration::from_millis(100));
                    }
                },
            );

            draw_card(
                &mut cols[1],
                Some("Cài đặt nâng cao"),
                Some("Xóa toàn bộ thiết lập cá nhân và danh sách thiết bị đã ghép trên tai nghe."),
                |ui| {
                    if danger_button(ui, "Khôi phục cài đặt gốc…", enabled).clicked() {
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
                "Hãy đảm bảo bạn ĐÃ THÁO tai nghe ra khỏi tai để bảo vệ thính giác. Bắt đầu phát âm tìm trong 10 giây?",
                move |app| {
                    let core = app.backend.core.clone();
                    app.backend
                        .spawn_task(async move { core.find(mask).await });
                },
            );
        }
        if reset_req {
            self.ask_confirm(
                "Khôi phục cài đặt gốc sẽ xóa mọi thiết lập và kết nối đã ghép trên tai nghe. Bạn sẽ cần ghép đôi lại Bluetooth. Tiếp tục?",
                |app| {
                    let core = app.backend.core.clone();
                    app.backend
                        .spawn_task(async move { core.direct(54, &[]).await });
                },
            );
        }
    }
}
