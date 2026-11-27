//! Overview page: quick connection onboarding, 1-click noise control + contextual sub-modes,
//! Game Mode & In-Ear Detection cards, and non-blocking OS audio integration.

use crate::audio;
use crate::ui::icons::{draw_vector_icon, IconKind};
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::StudioApp;
use crate::workflows;
use eframe::egui::{self, CursorIcon, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};

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
        let p = ui_palette(ui);
        self.render_hero_summary(ui);
        ui.add_space(10.0);

        // Quick onboarding banner when disconnected (wrapped cleanly, zero missing `->` glyphs!)
        if !self.is_connected() {
            egui::Frame::none()
                .fill(p.info_bg)
                .stroke(Stroke::new(1.0_f32, p.info_border))
                .rounding(Rounding::same(14.0))
                .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().interact_size.y = 18.0;
                        let (ir, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
                        draw_vector_icon(ui.painter(), ir, IconKind::Bluetooth, p.info_text);
                        ui.add(
                            egui::Label::new(
                                RichText::new(
                                    "Bắt đầu nhanh: (1) Lấy tai nghe khỏi hộp sạc • (2) Ghép đôi Bluetooth trên máy • (3) Bấm Quét BLE rồi Kết nối ở góc trên phải.",
                                )
                                .size(12.0)
                                .color(p.info_text),
                            )
                            .wrap(),
                        );
                    });
                });
            ui.add_space(10.0);
        }

        let enabled = self.is_enabled();
        let current_noise = self.state.noise.clone();

        // Noise Control Card with 1-click mode cards + clean contextual sub-mode chips
        draw_card(ui, Some("Kiểm soát tiếng ồn"), None, |ui| {
            let active_group = Self::noise_group(current_noise.as_deref());
            ui.columns(3, |cols| {
                let modes = [
                    (
                        "normal",
                        "Bình thường",
                        "Tắt khử ồn & xuyên âm",
                        0u8,
                        IconKind::Normal,
                    ),
                    ("anc", "Chống ồn", "Khử ồn chủ động ANC", 1u8, IconKind::Anc),
                    (
                        "ambient",
                        "Xuyên âm",
                        "Nghe âm xung quanh",
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
                        p.chip_active_bg
                    } else if resp.hovered() && enabled {
                        p.ctrl_hover
                    } else {
                        p.card_subtle_bg
                    };
                    let stroke = if is_active {
                        Stroke::new(1.5_f32, p.accent)
                    } else {
                        Stroke::new(1.0_f32, p.ctrl_border)
                    };
                    let painter = cols[i].painter_at(rect);
                    painter.rect(rect, Rounding::same(13.0), fill, stroke);

                    let icon_bg = if is_active {
                        p.card_bg
                    } else {
                        p.badge_icon_bg
                    };
                    let badge_rect = Rect::from_min_size(
                        Pos2::new(rect.left() + 10.0, rect.center().y - 16.0),
                        Vec2::new(32.0, 32.0),
                    );
                    painter.rect_filled(badge_rect, Rounding::same(9.5), icon_bg);
                    draw_vector_icon(
                        &painter,
                        badge_rect.shrink(6.0),
                        icon_kind,
                        if is_active { p.primary_bg } else { p.accent },
                    );

                    let title_color = if is_active {
                        p.accent_strong
                    } else if enabled {
                        p.text_primary
                    } else {
                        p.text_subtitle
                    };
                    let text_x = rect.left() + 49.0;
                    painter.text(
                        Pos2::new(text_x, rect.center().y - 7.5),
                        egui::Align2::LEFT_CENTER,
                        title,
                        if is_active {
                            font_semibold(13.2)
                        } else {
                            font_medium(13.2)
                        },
                        title_color,
                    );
                    painter.text(
                        Pos2::new(text_x, rect.center().y + 8.5),
                        egui::Align2::LEFT_CENTER,
                        desc,
                        font_regular(10.8),
                        p.text_subtitle,
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

            // Contextual sub-mode strip: only show relevant sub-options for the active mode!
            ui.add_space(10.0);
            draw_row_divider(ui);
            ui.add_space(2.0);

            match active_group {
                Some("anc") => {
                    let sub_avail = enabled && self.state.sub.is_some();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Mức chống ồn:")
                                .font(font_medium(12.3))
                                .color(p.text_subtitle),
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
                }
                Some("ambient") => {
                    let amb_avail = enabled && self.state.amb.is_some();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("Kiểu xuyên âm:")
                                .font(font_medium(12.3))
                                .color(p.text_subtitle),
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
                Some("normal") => {
                    ui.label(
                        RichText::new(
                            "Đang ở chế độ Bình thường (tiết kiệm pin tối đa). Chọn Chống ồn hoặc Xuyên âm để tùy chỉnh mức độ chi tiết.",
                        )
                        .size(11.6)
                        .color(p.text_subtitle),
                    );
                }
                _ => {
                    ui.label(
                        RichText::new(
                            "Hỗ trợ 5 chế độ Chống ồn ANC (Mạnh, Cân bằng, Thích ứng môi trường, Thích ứng tai, Giảm gió) và 2 chế độ Xuyên âm.",
                        )
                        .size(11.6)
                        .color(p.text_muted),
                    );
                }
            }
        });

        ui.add_space(10.0);

        // Two quick toggle cards side by side with uniform height & overlap-proof setting_row
        let mut trigger_ear: Option<bool> = None;
        let toggle_row_h = if ui.available_width() < 700.0 {
            54.0
        } else {
            40.0
        };
        ui.columns(2, |cols| {
            draw_card(&mut cols[0], None, None, |ui| {
                let mut game_on = self.state.game.unwrap_or(false);
                let avail = enabled && self.state.game.is_some();
                let mut toggled = false;
                setting_row(
                    ui,
                    toggle_row_h,
                    |ui| {
                        if toggle_switch(ui, "sw_game", &mut game_on, avail) {
                            toggled = true;
                        }
                    },
                    |ui| {
                        draw_icon_badge(ui, 36.0, p.badge_icon_bg, IconKind::Game, p.accent);
                        let rem = ui.available_width().max(60.0);
                        ui.vertical(|ui| {
                            ui.set_max_width(rem);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new("Chế độ chơi game")
                                    .font(font_semibold(13.6))
                                    .color(p.text_primary),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new("Giảm độ trễ âm thanh (<50ms)")
                                        .size(11.4)
                                        .color(p.text_subtitle),
                                )
                                .wrap(),
                            );
                        });
                    },
                );
                if toggled {
                    let core = self.backend.core.clone();
                    self.backend.spawn_task(async move {
                        core.set_bool_value(18, 19, u8::from(game_on), "game", game_on)
                            .await
                    });
                }
            });

            draw_card(&mut cols[1], None, None, |ui| {
                let mut ear_on = self.state.ear.unwrap_or(false);
                let avail = enabled && self.state.ear.is_some();
                setting_row(
                    ui,
                    toggle_row_h,
                    |ui| {
                        if toggle_switch(ui, "sw_ear", &mut ear_on, avail) {
                            trigger_ear = Some(ear_on);
                        }
                    },
                    |ui| {
                        draw_icon_badge(ui, 36.0, p.badge_icon_bg, IconKind::Ear, p.accent);
                        let rem = ui.available_width().max(60.0);
                        ui.vertical(|ui| {
                            ui.set_max_width(rem);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new("Nhận biết đeo tai")
                                    .font(font_semibold(13.6))
                                    .color(p.text_primary),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new("Tự dừng/phát khi tháo hoặc đeo tai")
                                        .size(11.4)
                                        .color(p.text_subtitle),
                                )
                                .wrap(),
                            );
                        });
                    },
                );
            });
        });
        if let Some(on) = trigger_ear {
            let plan_res = workflows::restart_setting(&self.state, "ear", on);
            self.execute_plan(plan_res);
        }

        ui.add_space(10.0);

        // OS Sound & Volume Card (overlap-proof + non-blocking async PipeWire volume!)
        let mut open_sound = false;
        let mut open_mixer = false;
        let mut read_vol = false;
        let mut apply_vol: Option<u8> = None;

        draw_card(ui, None, None, |ui| {
            if cfg!(target_os = "windows") {
                setting_row(
                    ui,
                    42.0,
                    |ui| {
                        if secondary_button(ui, "Cài đặt âm thanh", true).clicked() {
                            open_sound = true;
                        }
                        if secondary_button(ui, "Bộ trộn âm lượng", true).clicked() {
                            open_mixer = true;
                        }
                    },
                    |ui| {
                        draw_icon_badge(ui, 36.0, p.badge_icon_bg, IconKind::Volume, p.accent);
                        let rem = ui.available_width().max(80.0);
                        ui.vertical(|ui| {
                            ui.set_max_width(rem);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new("Âm thanh đầu ra Windows")
                                    .font(font_semibold(13.6))
                                    .color(p.text_primary),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(
                                        "Điều chỉnh trên khay âm lượng Windows hoặc thao tác chạm trên tai nghe.",
                                    )
                                    .size(11.4)
                                    .color(p.text_subtitle),
                                )
                                .wrap(),
                            );
                        });
                    },
                );
            } else {
                setting_row(
                    ui,
                    42.0,
                    |ui| {
                        if secondary_button(ui, "Cài đặt", true)
                            .on_hover_text("Mở Cài đặt âm thanh hệ thống")
                            .clicked()
                        {
                            open_sound = true;
                        }
                        if primary_button(ui, "Áp dụng", enabled).clicked() {
                            apply_vol = Some(self.volume_value);
                        }
                        if secondary_button(ui, "Đọc", enabled)
                            .on_hover_text("Đọc mức âm lượng sink Bluetooth của tai nghe")
                            .clicked()
                        {
                            read_vol = true;
                        }
                        ui.add_enabled_ui(enabled, |ui| {
                            ui.spacing_mut().slider_width = 115.0;
                            ui.add(
                                egui::Slider::new(&mut self.volume_value, 0..=100)
                                    .suffix("%")
                                    .show_value(true),
                            );
                        });
                    },
                    |ui| {
                        draw_icon_badge(ui, 36.0, p.badge_icon_bg, IconKind::Volume, p.accent);
                        let rem = ui.available_width().max(80.0);
                        ui.vertical(|ui| {
                            ui.set_max_width(rem);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            ui.label(
                                RichText::new("Âm lượng PipeWire (Ubuntu)")
                                    .font(font_semibold(13.6))
                                    .color(p.text_primary),
                            );
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&self.volume_label)
                                        .size(11.4)
                                        .color(p.text_subtitle),
                                )
                                .wrap(),
                            );
                        });
                    },
                );
            }
        });

        if open_sound {
            if let Err(e) = audio::open_sound_settings() {
                self.show_notice(e, true);
            }
        }
        if open_mixer {
            if let Err(e) = audio::open_url_or_path("ms-settings:apps-volume") {
                self.show_notice(e, true);
            }
        }
        if read_vol {
            self.backend.spawn_volume_task(self.address.clone(), None);
        }
        if let Some(v) = apply_vol {
            self.backend
                .spawn_volume_task(self.address.clone(), Some(v));
        }
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
}
