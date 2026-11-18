//! Sidebar navigation, unified top header toolbar, and Overview hero summary cards.

use crate::audio;
use crate::backend::{ConnectionPhase, ConnectionStatus};
use crate::ui::icons::{draw_spinner, draw_vector_icon, IconKind};
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::{Page, StudioApp};
use eframe::egui::{
    self, Color32, CursorIcon, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2,
};

impl StudioApp {
    pub(crate) fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        let p = ui_palette(ui);
        let ctx = ui.ctx().clone();
        let mut toggle_theme = false;

        egui::Frame::none()
            .fill(p.sidebar_bg)
            .stroke(Stroke::new(1.0_f32, p.card_border))
            .rounding(Rounding::same(20.0))
            .shadow(card_shadow_for(p.is_dark))
            .inner_margin(egui::Margin::same(13.0))
            .show(ui, |ui| {
                ui.set_min_height(ui.available_height());

                // Brand block
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(2.0);
                    draw_icon_badge(ui, 40.0, p.badge_icon_bg, IconKind::Eq, p.primary_bg);
                    ui.add_space(3.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.5;
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("EarFun Studio")
                                .font(font_semibold(15.2))
                                .color(p.text_primary),
                        );
                        ui.label(
                            RichText::new("Air Pro 4 · TW600")
                                .size(11.0)
                                .color(p.text_subtitle),
                        );
                    });
                });

                ui.add_space(18.0);

                // Navigation items with active left accent bar & subtle status indicators
                for (idx, nav_page) in Page::ALL.into_iter().enumerate() {
                    let active = self.page == nav_page;
                    let desired = Vec2::new(ui.available_width(), 40.0);
                    let (rect, resp) = ui.allocate_exact_size(desired, Sense::click());
                    let hovered = resp.hovered();
                    if hovered {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }

                    let bg_fill = if active {
                        p.nav_active_bg
                    } else if hovered {
                        p.nav_hover_bg
                    } else {
                        Color32::TRANSPARENT
                    };
                    let stroke = if active {
                        Stroke::new(1.0_f32, p.nav_active_border)
                    } else {
                        Stroke::NONE
                    };
                    ui.painter()
                        .rect(rect, Rounding::same(11.0), bg_fill, stroke);

                    if active {
                        let bar = Rect::from_min_size(
                            Pos2::new(rect.left() + 3.5, rect.center().y - 9.0),
                            Vec2::new(3.2, 18.0),
                        );
                        ui.painter()
                            .rect_filled(bar, Rounding::same(2.0), p.primary_bg);
                    }

                    let icon_color = if active {
                        p.accent_strong
                    } else if hovered {
                        p.primary_bg
                    } else {
                        p.accent
                    };
                    let text_color = if active {
                        p.accent_strong
                    } else if hovered {
                        p.text_primary
                    } else {
                        p.text_subtitle
                    };
                    let font = if active {
                        font_semibold(13.2)
                    } else {
                        font_regular(13.2)
                    };

                    let icon_rect = Rect::from_min_size(
                        Pos2::new(rect.left() + 13.0, rect.center().y - 9.0),
                        Vec2::new(18.0, 18.0),
                    );
                    draw_vector_icon(ui.painter(), icon_rect, nav_page.icon(), icon_color);

                    ui.painter().text(
                        Pos2::new(rect.left() + 41.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        nav_page.title(),
                        font,
                        text_color,
                    );

                    if nav_page == Page::Equalizer && self.has_unsent_eq() && self.is_connected() {
                        ui.painter().circle_filled(
                            Pos2::new(rect.right() - 12.0, rect.center().y),
                            3.5,
                            p.warn_text,
                        );
                    } else if nav_page == Page::Multipoint && !self.phones.is_empty() {
                        let active_cnt = self.phones.iter().filter(|ph| ph.connected).count();
                        if active_cnt > 0 {
                            ui.painter().text(
                                Pos2::new(rect.right() - 12.0, rect.center().y),
                                egui::Align2::CENTER_CENTER,
                                format!("{}", active_cnt),
                                font_semibold(10.5),
                                p.accent_strong,
                            );
                        }
                    }

                    let shortcut_tip = format!("{} (Alt+{})", nav_page.subtitle(), idx + 1);
                    if resp.on_hover_text(shortcut_tip).clicked() {
                        self.page = nav_page;
                    }
                    ui.add_space(1.5);
                }

                // Bottom quick actions + exact-height BLE lifecycle status card
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(2.0);
                    let bottom_w = ui.available_width();
                    ui.allocate_ui_with_layout(
                        Vec2::new(bottom_w, 30.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 6.0;
                            let theme_btn_w = 30.0;
                            let bt_btn_w = (bottom_w - theme_btn_w - 6.0).max(96.0);
                            let sys_bt_label = if cfg!(target_os = "windows") {
                                "Cài đặt Bluetooth"
                            } else {
                                "Bluetooth hệ thống"
                            };
                            let bt_btn = egui::Button::new(
                                RichText::new(sys_bt_label)
                                    .font(font_medium(11.6))
                                    .color(p.text_primary),
                            )
                            .fill(p.ctrl_bg)
                            .stroke(Stroke::new(1.0_f32, p.ctrl_border))
                            .rounding(Rounding::same(RADIUS_CTRL))
                            .min_size(Vec2::new(bt_btn_w, 30.0));
                            let bt_resp = ui.add(bt_btn);
                            if bt_resp.hovered() {
                                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            }
                            if bt_resp
                                .on_hover_text("Mở cửa sổ ghép nối Bluetooth của hệ điều hành")
                                .clicked()
                            {
                                if let Err(e) = audio::open_bluetooth_settings() {
                                    self.show_notice(e, true);
                                }
                            }

                            let (th_rect, th_resp) = ui
                                .allocate_exact_size(Vec2::new(theme_btn_w, 30.0), Sense::click());
                            if th_resp.hovered() {
                                ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                            }
                            let th_bg = if th_resp.hovered() {
                                p.ctrl_hover
                            } else {
                                p.ctrl_bg
                            };
                            ui.painter().rect(
                                th_rect,
                                Rounding::same(RADIUS_CTRL),
                                th_bg,
                                Stroke::new(1.0_f32, p.ctrl_border),
                            );
                            let th_icon = if p.is_dark {
                                IconKind::Sun
                            } else {
                                IconKind::Moon
                            };
                            draw_vector_icon(ui.painter(), th_rect.shrink(6.8), th_icon, p.accent);
                            let tip = format!(
                                "Giao diện: {} (Nhấp để chuyển Sáng / Tối / Hệ thống • Alt+T)",
                                self.settings.data.theme.label()
                            );
                            if th_resp.on_hover_text(tip).clicked() {
                                toggle_theme = true;
                            }
                        },
                    );

                    ui.add_space(8.0);

                    // Pixel-exact status card anchored right above bottom buttons
                    let (st_rect, st_resp) =
                        ui.allocate_exact_size(Vec2::new(bottom_w, 50.0), Sense::hover());
                    ui.painter().rect(
                        st_rect,
                        Rounding::same(12.0),
                        p.card_subtle_bg,
                        Stroke::new(1.0_f32, p.card_border),
                    );
                    let dot_color = match self.phase {
                        ConnectionPhase::Ready => p.success_text,
                        ConnectionPhase::PairedIdle => p.warn_text,
                        ConnectionPhase::Disconnected => p.text_muted,
                        _ => p.accent,
                    };
                    let dot_pos = Pos2::new(st_rect.left() + 14.0, st_rect.top() + 16.5);
                    ui.painter().circle_filled(dot_pos, 3.8, dot_color);
                    ui.painter().text(
                        Pos2::new(st_rect.left() + 24.0, st_rect.top() + 16.5),
                        egui::Align2::LEFT_CENTER,
                        self.phase.short_label(),
                        font_medium(11.4),
                        p.text_primary,
                    );
                    let sub_info = if let Some(fw) = self.state.version.as_deref() {
                        let chars: Vec<char> = fw.chars().collect();
                        if chars.len() > 21 {
                            let tail: String = chars[chars.len() - 14..].iter().collect();
                            format!("FW …{}", tail)
                        } else {
                            format!("FW {}", fw)
                        }
                    } else {
                        concat!("EarFun Studio ", env!("CARGO_PKG_VERSION")).to_string()
                    };
                    ui.painter().text(
                        Pos2::new(st_rect.left() + 11.0, st_rect.bottom() - 13.5),
                        egui::Align2::LEFT_CENTER,
                        sub_info,
                        font_regular(10.5),
                        p.text_muted,
                    );
                    if let Some(fw) = self.state.version.as_deref() {
                        st_resp.on_hover_text(format!("Firmware: {}", fw));
                    }
                });
            });

        if toggle_theme {
            self.cycle_theme_mode(&ctx);
        }
    }

    /// Overlap-proof top header: toolbar reserves right width first; page title/subtitle wrap cleanly on left.
    pub(crate) fn render_top_header(&mut self, ui: &mut egui::Ui) {
        let p = ui_palette(ui);
        let mut toggle_conn = false;
        let mut manual_refresh = false;
        let mut scan_ble = false;

        let header_w = ui.available_width();
        ui.allocate_ui_with_layout(
            Vec2::new(header_w, 46.0),
            egui::Layout::right_to_left(egui::Align::Center),
            |ui| {
                ui.spacing_mut().item_spacing.x = 7.0;
                let is_busy = self.busy > 0;

                if self.is_connected() {
                    if secondary_button(ui, "Ngắt kết nối", !is_busy).clicked() {
                        toggle_conn = true;
                    }
                    if secondary_button(ui, "Đọc lại", !is_busy)
                        .on_hover_text("Đọc lại trạng thái trực tiếp từ tai nghe (F5)")
                        .clicked()
                    {
                        manual_refresh = true;
                    }

                    if self.page != Page::Overview {
                        if let Some(codec) = self.state.codec.as_deref() {
                            status_pill(ui, codec, p.info_bg, p.info_border, p.info_text);
                        }
                        let bat_summary = format!(
                            "L {} · R {} · Hộp {}",
                            Self::fmt_pct(self.state.bat_l),
                            Self::fmt_pct(self.state.bat_r),
                            Self::fmt_pct(self.state.bat_box),
                        );
                        status_pill(ui, &bat_summary, p.card_bg, p.card_border, p.text_subtitle);
                    }
                } else {
                    let can_connect = !is_busy && !self.devices.is_empty();
                    let conn_label = if self.connection == ConnectionStatus::Connecting {
                        "Đang nối…"
                    } else {
                        "Kết nối"
                    };
                    if primary_button(ui, conn_label, can_connect)
                        .on_hover_text(if self.devices.is_empty() {
                            "Bấm Quét BLE để tìm tai nghe trước"
                        } else {
                            "Mở phiên điều khiển Bluetooth LE tới tai nghe đã chọn"
                        })
                        .clicked()
                    {
                        toggle_conn = true;
                    }

                    if secondary_button(ui, "Quét BLE", !is_busy)
                        .on_hover_text("Quét tìm tai nghe EarFun Air Pro 4 trong 5 giây")
                        .clicked()
                    {
                        scan_ble = true;
                    }

                    let picker_w = (ui.available_width() * 0.36).clamp(132.0, 195.0);
                    ui.add_enabled_ui(!is_busy, |ui| {
                        let selected_label = if self.devices.is_empty() {
                            "Chưa thấy tai nghe".to_string()
                        } else {
                            self.devices
                                .get(self.selected_device_idx)
                                .map(|(_, name)| name.clone())
                                .unwrap_or_else(|| "Chọn tai nghe".to_string())
                        };
                        egui::ComboBox::from_id_salt("device_picker")
                            .width(picker_w)
                            .selected_text(
                                RichText::new(selected_label)
                                    .size(12.2)
                                    .color(p.text_primary),
                            )
                            .show_ui(ui, |ui| {
                                if self.devices.is_empty() {
                                    ui.label(
                                        RichText::new(
                                            "Chưa tìm thấy tai nghe. Hãy ghép đôi và bấm Quét BLE.",
                                        )
                                        .size(12.0)
                                        .color(p.text_subtitle),
                                    );
                                }
                                for (i, (mac, name)) in self.devices.iter().enumerate() {
                                    let row_txt = format!("{} ({})", name, Self::short_mac(mac));
                                    ui.selectable_value(&mut self.selected_device_idx, i, row_txt);
                                }
                            });
                    });
                }

                if is_busy {
                    draw_spinner(ui, 16.0, p.accent);
                }

                let max_left = ui.available_width().max(110.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(max_left, 46.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_max_width(max_left);
                        ui.spacing_mut().item_spacing.y = 1.5;
                        ui.label(
                            RichText::new(self.page.title())
                                .font(font_semibold(22.5))
                                .color(p.text_primary),
                        );
                        ui.add(
                            egui::Label::new(
                                RichText::new(self.page.subtitle())
                                    .size(11.8)
                                    .color(p.text_subtitle),
                            )
                            .wrap(),
                        );
                    },
                );
            },
        );

        if toggle_conn {
            self.toggle_connect();
        }
        if manual_refresh {
            self.trigger_manual_query();
        }
        if scan_ble {
            let core = self.backend.core.clone();
            self.backend.spawn_task(async move { core.scan().await });
        }
    }

    /// Hero summary strip shown on Overview: Device Status Card + 3 Battery Cards.
    /// Dynamically scales down cleanly to 860px window width with zero horizontal overflow.
    pub(crate) fn render_hero_summary(&mut self, ui: &mut egui::Ui) {
        let p = ui_palette(ui);
        let total_w = ui.available_width();
        let gap = 10.0;
        let col_w = ((total_w - gap * 3.0 - 1.0) / 5.0).max(70.0);
        let device_card_w = col_w * 2.0;
        let card_h = 106.0;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;

            // 1. Device Hero Card (2/5 width)
            let (dev_rect, _) =
                ui.allocate_exact_size(Vec2::new(device_card_w, card_h), Sense::hover());
            ui.painter()
                .add(card_shadow_for(p.is_dark).as_shape(dev_rect, Rounding::same(RADIUS_CARD)));
            ui.painter().rect(
                dev_rect,
                Rounding::same(RADIUS_CARD),
                p.card_bg,
                Stroke::new(1.0_f32, p.card_border),
            );

            let mut dev_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(dev_rect.shrink(14.0))
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            dev_ui.horizontal(|ui| {
                draw_icon_badge(ui, 38.0, p.badge_icon_bg, IconKind::Earbud, p.accent);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("EarFun Air Pro 4")
                            .font(font_semibold(15.0))
                            .color(p.text_primary),
                    );
                    let badge_color = match self.phase {
                        ConnectionPhase::Ready => p.success_text,
                        ConnectionPhase::PairedIdle => p.warn_text,
                        ConnectionPhase::Disconnected => p.text_subtitle,
                        _ => p.accent_strong,
                    };
                    ui.label(
                        RichText::new(self.phase.short_label())
                            .font(font_medium(11.3))
                            .color(badge_color),
                    );
                });
            });

            dev_ui.add_space(7.0);
            dev_ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let codec = self.state.codec.as_deref().unwrap_or("Codec —");
                status_pill(ui, codec, p.info_bg, p.info_border, p.info_text);

                if let Some(dual) = self.state.dual {
                    let lbl = if dual {
                        "Đa điểm: Bật"
                    } else {
                        "Đa điểm: Tắt"
                    };
                    status_pill(ui, lbl, p.card_subtle_bg, p.ctrl_border, p.text_subtitle);
                } else if !self.is_connected() && !self.devices.is_empty() {
                    let lbl = format!("{} đã ghép", self.devices.len());
                    status_pill(ui, &lbl, p.card_subtle_bg, p.ctrl_border, p.text_subtitle);
                }
                if let Some(true) = self.state.game {
                    status_pill(ui, "Game Mode", p.warn_bg, p.warn_border, p.warn_text);
                }
            });

            // 2. Three Battery Cards (Tai trái, Tai phải, Hộp sạc)
            let bat_items = [
                ("Tai trái", self.state.bat_l, IconKind::Earbud),
                ("Tai phải", self.state.bat_r, IconKind::Earbud),
                ("Hộp sạc", self.state.bat_box, IconKind::Case),
            ];
            for (title, val, icon_kind) in bat_items {
                let (bat_rect, bat_resp) =
                    ui.allocate_exact_size(Vec2::new(col_w, card_h), Sense::hover());
                ui.painter().add(
                    card_shadow_for(p.is_dark).as_shape(bat_rect, Rounding::same(RADIUS_CARD)),
                );
                ui.painter().rect(
                    bat_rect,
                    Rounding::same(RADIUS_CARD),
                    p.card_bg,
                    Stroke::new(1.0_f32, p.card_border),
                );

                let inner = bat_rect.shrink(14.0);
                let icon_r = Rect::from_min_size(inner.left_top(), Vec2::new(15.0, 15.0));
                draw_vector_icon(ui.painter(), icon_r, icon_kind, p.accent);
                ui.painter().text(
                    Pos2::new(inner.left() + 21.0, inner.top() + 7.5),
                    egui::Align2::LEFT_CENTER,
                    title,
                    font_medium(11.8),
                    p.text_subtitle,
                );

                let pct_text = Self::fmt_pct(val);
                let pct_color = match val {
                    Some(v) if v <= 15 => p.danger_text,
                    Some(_) => p.text_primary,
                    None => p.text_muted,
                };
                ui.painter().text(
                    Pos2::new(inner.left(), inner.center().y + 3.0),
                    egui::Align2::LEFT_CENTER,
                    pct_text,
                    font_semibold(23.0),
                    pct_color,
                );

                let bar_rect = Rect::from_min_max(
                    Pos2::new(inner.left(), inner.bottom() - 5.5),
                    Pos2::new(inner.right(), inner.bottom()),
                );
                let (frac, bar_col) = match val {
                    Some(v) => ((v.min(100) as f32) / 100.0, battery_bar_color(v, p.is_dark)),
                    None => (0.0, Color32::TRANSPARENT),
                };
                draw_progress_bar(ui, bar_rect, frac, bar_col);

                if let Some(v) = val {
                    let tip = if v <= 15 {
                        format!("{}: {}% (Pin yếu — nên sạc sớm)", title, v)
                    } else {
                        format!("{}: {}%", title, v)
                    };
                    bat_resp.on_hover_text(tip);
                }
            }
        });
    }

    pub(crate) fn fmt_pct(val: Option<u8>) -> String {
        val.map(|v| format!("{}%", v))
            .unwrap_or_else(|| "—".to_string())
    }

    pub(crate) fn short_mac(mac: &str) -> String {
        let chars: Vec<char> = mac.chars().collect();
        if chars.len() > 8 {
            let tail: String = chars[chars.len() - 5..].iter().collect();
            format!("…{}", tail)
        } else {
            mac.to_string()
        }
    }
}
