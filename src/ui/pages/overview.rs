//! Overview page: quick connection onboarding, 1-click noise control + sub-modes,
//! Game Mode & In-Ear Detection cards, and OS audio integration.

use crate::audio;
use crate::ui::icons::{draw_vector_icon, IconKind};
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::StudioApp;
use crate::workflows;
use eframe::egui::{
    self, Color32, CursorIcon, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2,
};

const ANC_SUB_MODES: &[(&str, &str, u8)] = &[
    ("Mạnh", "DEFAULT", 2),
    ("Cân bằng", "BALANCE", 3),
    ("Thích ứng môi trường", "ADAPTIVE", 4),
    ("Thích ứng tai", "EAR_ADAPTIVE", 0),
    ("Giảm tiếng gió", "WIND", 1),
];

const AMBIENT_SUB_MODES: &[(&str, &str, u8)] = &[
    ("Mặc định", "DEFAULT", 0),
    ("Tự nhiên (Toàn dải)", "TOTAL", 1),
];

impl StudioApp {
    pub(crate) fn render_overview(&mut self, ui: &mut egui::Ui) {
        self.render_hero_summary(ui);
        ui.add_space(10.0);

        // Quick onboarding banner when disconnected so first-time users know exact steps
        if !self.is_connected() {
            egui::Frame::none()
                .fill(Color32::from_rgb(240, 244, 253))
                .stroke(Stroke::new(1.0_f32, INFO_BORDER))
                .rounding(Rounding::same(14.0))
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let (ir, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                        draw_vector_icon(ui.painter(), ir, IconKind::Bluetooth, ACCENT_INDIGO);
                        ui.label(
                            RichText::new(
                                "Bắt đầu nhanh: Mở hộp sạc & lấy tai nghe ra → Ghép đôi Bluetooth trên máy → Bấm Quét BLE rồi Kết nối ở góc trên phải.",
                            )
                            .size(12.0)
                            .color(ACCENT_INDIGO),
                        );
                    });
                });
            ui.add_space(10.0);
        }

        let enabled = self.is_enabled();
        let current_noise = self.state.noise.clone();

        // Noise Control Card with 1-click mode cards + contextual 1-click sub-mode chips!
        draw_card(ui, Some("Kiểm soát tiếng ồn"), None, |ui| {
            let active_group = Self::noise_group(current_noise.as_deref());
            ui.columns(3, |cols| {
                let modes = [
                    (
                        "normal",
                        "Bình thường",
                        "Âm thanh nguyên bản",
                        0u8,
                        IconKind::Normal,
                    ),
                    ("anc", "Chống ồn", "Khử ồn chủ động ANC", 1u8, IconKind::Anc),
                    (
                        "ambient",
                        "Xuyên âm",
                        "Nghe âm thanh xung quanh",
                        2u8,
                        IconKind::Ambient,
                    ),
                ];
                for (i, (key, title, desc, code, icon_kind)) in modes.into_iter().enumerate() {
                    let is_active = active_group == Some(key);
                    let w = cols[i].available_width();
                    let desired = Vec2::new(w, 60.0);
                    let sense = if enabled {
                        Sense::click()
                    } else {
                        Sense::hover()
                    };
                    let (rect, mut resp) = cols[i].allocate_exact_size(desired, sense);
                    if enabled && resp.hovered() {
                        cols[i].ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                    if !enabled {
                        resp = resp.on_hover_text("Kết nối tai nghe để chuyển chế độ nghe");
                    }

                    let fill = if is_active {
                        Color32::from_rgb(230, 237, 253)
                    } else if resp.hovered() && enabled {
                        Color32::from_rgb(240, 244, 253)
                    } else {
                        Color32::from_rgb(247, 249, 253)
                    };
                    let stroke = if is_active {
                        Stroke::new(1.5_f32, ACCENT_BLUE)
                    } else {
                        Stroke::new(1.0_f32, Color32::from_rgb(228, 233, 244))
                    };
                    cols[i]
                        .painter()
                        .rect(rect, Rounding::same(13.0), fill, stroke);

                    let icon_bg = if is_active {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(237, 242, 252)
                    };
                    let badge_rect = Rect::from_min_size(
                        Pos2::new(rect.left() + 11.0, rect.center().y - 17.0),
                        Vec2::new(34.0, 34.0),
                    );
                    cols[i]
                        .painter()
                        .rect_filled(badge_rect, Rounding::same(10.0), icon_bg);
                    draw_vector_icon(
                        cols[i].painter(),
                        badge_rect.shrink(6.5),
                        icon_kind,
                        if is_active {
                            PRIMARY_BTN_BG
                        } else {
                            ACCENT_BLUE
                        },
                    );

                    let title_color = if is_active {
                        ACCENT_INDIGO
                    } else if enabled {
                        TEXT_PRIMARY
                    } else {
                        Color32::from_rgb(96, 102, 118)
                    };
                    cols[i].painter().text(
                        Pos2::new(rect.left() + 54.0, rect.center().y - 7.5),
                        egui::Align2::LEFT_CENTER,
                        title,
                        if is_active {
                            font_semibold(13.5)
                        } else {
                            font_medium(13.5)
                        },
                        title_color,
                    );
                    cols[i].painter().text(
                        Pos2::new(rect.left() + 54.0, rect.center().y + 9.0),
                        egui::Align2::LEFT_CENTER,
                        desc,
                        font_regular(11.0),
                        TEXT_SUBTITLE,
                    );

                    if enabled && resp.clicked() && !is_active {
                        let core = self.backend.core.clone();
                        let k_owned = key.to_string();
                        self.backend.spawn_task(async move {
                            core.set_string_value(20, 21, code, "noise", &k_owned).await
                        });
                    }
                }
            });

            // Contextual 1-click Sub-Mode Chips (zero extra "Áp dụng" clicks!)
            ui.add_space(12.0);
            draw_row_divider(ui);
            ui.add_space(4.0);

            if active_group == Some("ambient") {
                let amb_avail = enabled && self.state.amb.is_some();
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Chế độ xuyên âm:")
                            .font(font_medium(12.5))
                            .color(TEXT_SUBTITLE),
                    );
                    for &(label, key, wire_val) in AMBIENT_SUB_MODES {
                        let active = self.state.amb.as_deref() == Some(key);
                        if chip_button(ui, label, active, amb_avail).clicked() && !active {
                            let wanted = key.to_string();
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_string_value(60, 61, wire_val, "amb", &wanted)
                                    .await
                            });
                        }
                    }
                });
            } else {
                // Show ANC sub-modes prominently when in ANC, or as quick-preset chips otherwise
                let sub_avail = enabled && self.state.sub.is_some();
                ui.horizontal_wrapped(|ui| {
                    let lbl = if active_group == Some("anc") {
                        "Mức chống ồn:"
                    } else {
                        "Mức chống ồn (áp dụng khi bật ANC):"
                    };
                    ui.label(
                        RichText::new(lbl)
                            .font(font_medium(12.5))
                            .color(TEXT_SUBTITLE),
                    );
                    for &(label, key, wire_val) in ANC_SUB_MODES {
                        let active = self.state.sub.as_deref() == Some(key);
                        if chip_button(ui, label, active, sub_avail).clicked() && !active {
                            let wanted = key.to_string();
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_string_value(58, 59, wire_val, "sub", &wanted)
                                    .await
                            });
                        }
                    }
                });

                if active_group != Some("anc") {
                    ui.add_space(4.0);
                    let amb_avail = enabled && self.state.amb.is_some();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Kiểu xuyên âm:")
                                .font(font_medium(12.5))
                                .color(TEXT_SUBTITLE),
                        );
                        for &(label, key, wire_val) in AMBIENT_SUB_MODES {
                            let active = self.state.amb.as_deref() == Some(key);
                            if chip_button(ui, label, active, amb_avail).clicked() && !active {
                                let wanted = key.to_string();
                                let core = self.backend.core.clone();
                                self.backend.spawn_task(async move {
                                    core.set_string_value(60, 61, wire_val, "amb", &wanted)
                                        .await
                                });
                            }
                        }
                    });
                }
            }
        });

        ui.add_space(10.0);

        // Two quick toggle cards side by side with custom vector icon badges
        let mut trigger_ear: Option<bool> = None;
        ui.columns(2, |cols| {
            draw_card(&mut cols[0], None, None, |ui| {
                ui.horizontal(|ui| {
                    draw_icon_badge(
                        ui,
                        36.0,
                        Color32::from_rgb(236, 241, 253),
                        IconKind::Game,
                        ACCENT_BLUE,
                    );
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            RichText::new("Chế độ chơi game")
                                .font(font_semibold(13.8))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("Giảm độ trễ âm thanh (<50ms)")
                                .size(11.5)
                                .color(TEXT_SUBTITLE),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut game_on = self.state.game.unwrap_or(false);
                        let avail = enabled && self.state.game.is_some();
                        if toggle_switch(ui, "sw_game", &mut game_on, avail) {
                            let core = self.backend.core.clone();
                            self.backend.spawn_task(async move {
                                core.set_bool_value(18, 19, u8::from(game_on), "game", game_on)
                                    .await
                            });
                        }
                    });
                });
            });

            draw_card(&mut cols[1], None, None, |ui| {
                ui.horizontal(|ui| {
                    draw_icon_badge(
                        ui,
                        36.0,
                        Color32::from_rgb(236, 241, 253),
                        IconKind::Ear,
                        ACCENT_BLUE,
                    );
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            RichText::new("Nhận biết đeo tai")
                                .font(font_semibold(13.8))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("Tự dừng/phát khi tháo hoặc đeo tai")
                                .size(11.5)
                                .color(TEXT_SUBTITLE),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let mut ear_on = self.state.ear.unwrap_or(false);
                        let avail = enabled && self.state.ear.is_some();
                        if toggle_switch(ui, "sw_ear", &mut ear_on, avail) {
                            trigger_ear = Some(ear_on);
                        }
                    });
                });
            });
        });
        if let Some(on) = trigger_ear {
            let plan_res = workflows::restart_setting(&self.state, "ear", on);
            self.execute_plan(plan_res);
        }

        ui.add_space(10.0);

        // OS Sound & Volume Card
        draw_card(ui, None, None, |ui| {
            ui.horizontal(|ui| {
                draw_icon_badge(
                    ui,
                    36.0,
                    Color32::from_rgb(236, 241, 253),
                    IconKind::Volume,
                    ACCENT_BLUE,
                );
                if cfg!(target_os = "windows") {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            RichText::new("Âm thanh đầu ra Windows")
                                .font(font_semibold(13.8))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new(
                                "Điều chỉnh trực tiếp trên khay âm lượng Windows hoặc thao tác chạm trên tai nghe.",
                            )
                            .size(11.5)
                            .color(TEXT_SUBTITLE),
                        );
                    });
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if secondary_button(ui, "Cài đặt âm thanh", true).clicked() {
                            if let Err(e) = audio::open_sound_settings() {
                                self.show_notice(e, true);
                            }
                        }
                        if secondary_button(ui, "Bộ trộn âm lượng", true).clicked() {
                            if let Err(e) = audio::open_url_or_path("ms-settings:apps-volume") {
                                self.show_notice(e, true);
                            }
                        }
                    });
                } else {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 2.0;
                        ui.label(
                            RichText::new("Âm lượng PipeWire (Ubuntu)")
                                .font(font_semibold(13.8))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new(&self.volume_label)
                                .size(11.5)
                                .color(TEXT_SUBTITLE),
                        );
                    });
                    ui.add_space(4.0);
                    if secondary_button(ui, "Đọc", enabled)
                        .on_hover_text("Đọc mức âm lượng sink Bluetooth của tai nghe")
                        .clicked()
                    {
                        self.read_or_set_volume(None);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if secondary_button(ui, "Cài đặt âm thanh", true).clicked() {
                            if let Err(e) = audio::open_sound_settings() {
                                self.show_notice(e, true);
                            }
                        }
                        if primary_button(ui, "Áp dụng", enabled).clicked() {
                            let v = self.volume_value;
                            self.read_or_set_volume(Some(v));
                        }
                        ui.add_enabled_ui(enabled, |ui| {
                            ui.spacing_mut().slider_width =
                                (ui.available_width() - 16.0).max(90.0);
                            ui.add(
                                egui::Slider::new(&mut self.volume_value, 0..=100)
                                    .suffix("%")
                                    .show_value(true),
                            );
                        });
                    });
                }
            });
        });
    }

    /// Normalize firmware `noise` values (`normal`, `anc`, `adaptive`, `wind`, `ambient`) to 3 UI groups.
    pub fn noise_group(noise: Option<&str>) -> Option<&'static str> {
        match noise {
            Some("normal") => Some("normal"),
            Some("anc" | "adaptive" | "wind") => Some("anc"),
            Some("ambient") => Some("ambient"),
            _ => None,
        }
    }

    pub(crate) fn read_or_set_volume(&mut self, value: Option<u8>) {
        match audio::volume(&self.address, value) {
            Ok(result) => {
                if let Some(second) = result.split_whitespace().nth(1) {
                    if let Ok(frac) = second.parse::<f64>() {
                        self.volume_value = (frac * 100.0).round().clamp(0.0, 100.0) as u8;
                    }
                }
                self.volume_label = format!("{}% • Đầu ra EarFun", self.volume_value);
                self.show_notice(result, false);
            }
            Err(e) => {
                self.show_notice(e, true);
            }
        }
    }
}
