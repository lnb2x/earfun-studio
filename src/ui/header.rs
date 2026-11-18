//! Sidebar navigation, unified top header toolbar, and Overview hero summary cards.

use crate::audio;
use crate::backend::ConnectionStatus;
use crate::ui::icons::{draw_spinner, draw_vector_icon, IconKind};
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::{Page, StudioApp};
use eframe::egui::{
    self, Color32, CursorIcon, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2,
};

impl StudioApp {
    pub(crate) fn render_sidebar(&mut self, ui: &mut egui::Ui) {
        egui::Frame::none()
            .fill(Color32::from_rgba_unmultiplied(255, 255, 255, 240))
            .stroke(Stroke::new(1.0_f32, CARD_BORDER))
            .rounding(Rounding::same(20.0))
            .shadow(card_shadow())
            .inner_margin(egui::Margin::same(13.0))
            .show(ui, |ui| {
                ui.set_min_height(ui.available_height());

                // Brand block
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.add_space(2.0);
                    draw_icon_badge(
                        ui,
                        40.0,
                        Color32::from_rgb(230, 236, 252),
                        IconKind::Eq,
                        PRIMARY_BTN_BG,
                    );
                    ui.add_space(3.0);
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 1.5;
                        ui.add_space(2.0);
                        ui.label(
                            RichText::new("EarFun Studio")
                                .font(font_semibold(15.2))
                                .color(TEXT_PRIMARY),
                        );
                        ui.label(
                            RichText::new("Air Pro 4 · TW600")
                                .size(11.0)
                                .color(TEXT_SUBTITLE),
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
                        Color32::from_rgb(232, 238, 251)
                    } else if hovered {
                        Color32::from_rgb(242, 246, 253)
                    } else {
                        Color32::TRANSPARENT
                    };
                    let stroke = if active {
                        Stroke::new(1.0_f32, Color32::from_rgb(198, 210, 238))
                    } else {
                        Stroke::NONE
                    };
                    ui.painter()
                        .rect(rect, Rounding::same(11.0), bg_fill, stroke);

                    // Left active indicator bar
                    if active {
                        let bar = Rect::from_min_size(
                            Pos2::new(rect.left() + 3.5, rect.center().y - 9.0),
                            Vec2::new(3.2, 18.0),
                        );
                        ui.painter()
                            .rect_filled(bar, Rounding::same(2.0), PRIMARY_BTN_BG);
                    }

                    let icon_color = if active {
                        ACCENT_INDIGO
                    } else if hovered {
                        PRIMARY_BTN_BG
                    } else {
                        ACCENT_BLUE
                    };
                    let text_color = if active {
                        ACCENT_INDIGO
                    } else if hovered {
                        TEXT_PRIMARY
                    } else {
                        Color32::from_rgb(104, 109, 126)
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

                    // Contextual subtle badge on Equalizer (unsent draft) or Multipoint (connected count)
                    if nav_page == Page::Equalizer && self.has_unsent_eq() && self.is_connected() {
                        ui.painter().circle_filled(
                            Pos2::new(rect.right() - 12.0, rect.center().y),
                            3.5,
                            Color32::from_rgb(226, 154, 52),
                        );
                    } else if nav_page == Page::Multipoint && !self.phones.is_empty() {
                        let active_cnt = self.phones.iter().filter(|p| p.connected).count();
                        if active_cnt > 0 {
                            let badge_txt = format!("{}", active_cnt);
                            ui.painter().text(
                                Pos2::new(rect.right() - 12.0, rect.center().y),
                                egui::Align2::CENTER_CENTER,
                                badge_txt,
                                font_semibold(10.5),
                                ACCENT_INDIGO,
                            );
                        }
                    }

                    let shortcut_tip = format!("{} (Alt+{})", nav_page.subtitle(), idx + 1);
                    if resp.on_hover_text(shortcut_tip).clicked() {
                        self.page = nav_page;
                    }
                    ui.add_space(1.5);
                }

                // Bottom connection status & system Bluetooth shortcut
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(2.0);
                    let sys_bt_label = if cfg!(target_os = "windows") {
                        "Cài đặt Bluetooth"
                    } else {
                        "Bluetooth hệ thống"
                    };
                    let btn_w = ui.available_width();
                    let bt_btn = egui::Button::new(
                        RichText::new(sys_bt_label)
                            .font(font_medium(12.0))
                            .color(TEXT_PRIMARY),
                    )
                    .fill(CTRL_BG)
                    .stroke(Stroke::new(1.0_f32, CTRL_BORDER))
                    .rounding(Rounding::same(10.0))
                    .min_size(Vec2::new(btn_w, 30.0));
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

                    ui.add_space(8.0);

                    // Compact BLE connection status card inside sidebar bottom
                    egui::Frame::none()
                        .fill(Color32::from_rgb(246, 248, 253))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(230, 235, 245)))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            let (dot_color, status_str) = match self.connection {
                                ConnectionStatus::Connected => {
                                    (Color32::from_rgb(46, 168, 108), "Đã kết nối BLE")
                                }
                                ConnectionStatus::Connecting => (ACCENT_BLUE, "Đang kết nối…"),
                                ConnectionStatus::Disconnected => {
                                    (Color32::from_rgb(172, 179, 196), "Chưa kết nối")
                                }
                            };
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                let (dot_r, _) =
                                    ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                                ui.painter().circle_filled(dot_r.center(), 3.8, dot_color);
                                ui.label(
                                    RichText::new(status_str)
                                        .font(font_medium(11.5))
                                        .color(TEXT_PRIMARY),
                                );
                            });
                            let sub_info = if let Some(fw) = self.state.version.as_deref() {
                                format!("FW {}", fw)
                            } else {
                                concat!("EarFun Studio ", env!("CARGO_PKG_VERSION")).to_string()
                            };
                            ui.label(RichText::new(sub_info).size(10.5).color(TEXT_MUTED));
                        });
                });
            });
    }

    /// Unified top header with page title on left and responsive connection/battery toolbar on right.
    pub(crate) fn render_top_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(
                    RichText::new(self.page.title())
                        .font(font_semibold(23.5))
                        .color(TEXT_PRIMARY),
                );
                ui.label(
                    RichText::new(self.page.subtitle())
                        .size(12.0)
                        .color(TEXT_SUBTITLE),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 7.0;
                let is_busy = self.busy > 0;

                if self.is_connected() {
                    if secondary_button(ui, "Ngắt kết nối", !is_busy).clicked() {
                        self.toggle_connect();
                    }
                    if secondary_button(ui, "Đọc lại", !is_busy)
                        .on_hover_text("Đọc lại trạng thái trực tiếp từ tai nghe (F5)")
                        .clicked()
                    {
                        self.trigger_manual_query();
                    }

                    // On sub-pages, display compact inline battery & codec pills right in the header!
                    if self.page != Page::Overview {
                        if let Some(codec) = self.state.codec.as_deref() {
                            status_pill(ui, codec, INFO_BG, INFO_BORDER, ACCENT_INDIGO);
                        }
                        let bat_summary = format!(
                            "L {} · R {} · Hộp {}",
                            Self::fmt_pct(self.state.bat_l),
                            Self::fmt_pct(self.state.bat_r),
                            Self::fmt_pct(self.state.bat_box),
                        );
                        status_pill(ui, &bat_summary, Color32::WHITE, CARD_BORDER, TEXT_SUBTITLE);
                    }
                } else {
                    let can_connect = !is_busy && !self.devices.is_empty();
                    let conn_label = if self.connection == ConnectionStatus::Connecting {
                        "Đang kết nối…"
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
                        self.toggle_connect();
                    }

                    if secondary_button(ui, "Quét BLE", !is_busy)
                        .on_hover_text("Quét tìm tai nghe EarFun Air Pro 4 trong 5 giây")
                        .clicked()
                    {
                        let core = self.backend.core.clone();
                        self.backend.spawn_task(async move { core.scan().await });
                    }

                    let picker_w = (ui.available_width() * 0.45).clamp(145.0, 210.0);
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
                                RichText::new(selected_label).size(12.5).color(TEXT_PRIMARY),
                            )
                            .show_ui(ui, |ui| {
                                if self.devices.is_empty() {
                                    ui.label(
                                        RichText::new(
                                            "Chưa tìm thấy tai nghe. Hãy ghép đôi và bấm Quét BLE.",
                                        )
                                        .size(12.0)
                                        .color(TEXT_SUBTITLE),
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
                    draw_spinner(ui, 16.0, ACCENT_BLUE);
                }
            });
        });
    }

    /// Hero summary strip shown on Overview: Device Status Card + 3 Battery Cards.
    /// Dynamically scales down cleanly to 860px window width with zero horizontal overflow.
    pub(crate) fn render_hero_summary(&mut self, ui: &mut egui::Ui) {
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
                .add(card_shadow().as_shape(dev_rect, Rounding::same(18.0)));
            ui.painter().rect(
                dev_rect,
                Rounding::same(18.0),
                Color32::WHITE,
                Stroke::new(1.0_f32, CARD_BORDER),
            );

            let mut dev_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(dev_rect.shrink(14.0))
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
            );
            dev_ui.horizontal(|ui| {
                draw_icon_badge(
                    ui,
                    38.0,
                    Color32::from_rgb(236, 241, 253),
                    IconKind::Earbud,
                    ACCENT_BLUE,
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.label(
                        RichText::new("EarFun Air Pro 4")
                            .font(font_semibold(15.0))
                            .color(TEXT_PRIMARY),
                    );
                    let (badge_str, badge_color) = match self.connection {
                        ConnectionStatus::Connected => ("Đã kết nối", SUCCESS_TEXT),
                        ConnectionStatus::Connecting => ("Đang kết nối…", ACCENT_INDIGO),
                        ConnectionStatus::Disconnected => ("Chưa kết nối", TEXT_SUBTITLE),
                    };
                    ui.label(
                        RichText::new(badge_str)
                            .font(font_medium(11.5))
                            .color(badge_color),
                    );
                });
            });

            dev_ui.add_space(7.0);
            dev_ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let codec = self.state.codec.as_deref().unwrap_or("Codec —");
                status_pill(ui, codec, INFO_BG, INFO_BORDER, ACCENT_INDIGO);

                if let Some(dual) = self.state.dual {
                    let lbl = if dual {
                        "Đa điểm: Bật"
                    } else {
                        "Đa điểm: Tắt"
                    };
                    status_pill(
                        ui,
                        lbl,
                        Color32::from_rgb(245, 247, 252),
                        CTRL_BORDER,
                        TEXT_SUBTITLE,
                    );
                }
                if let Some(true) = self.state.game {
                    status_pill(ui, "Game Mode", WARN_BG, WARN_BORDER, WARN_TEXT);
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
                ui.painter()
                    .add(card_shadow().as_shape(bat_rect, Rounding::same(18.0)));
                ui.painter().rect(
                    bat_rect,
                    Rounding::same(18.0),
                    Color32::WHITE,
                    Stroke::new(1.0_f32, CARD_BORDER),
                );

                let inner = bat_rect.shrink(14.0);
                let icon_r = Rect::from_min_size(inner.left_top(), Vec2::new(15.0, 15.0));
                draw_vector_icon(ui.painter(), icon_r, icon_kind, ACCENT_BLUE);
                ui.painter().text(
                    Pos2::new(inner.left() + 21.0, inner.top() + 7.5),
                    egui::Align2::LEFT_CENTER,
                    title,
                    font_medium(11.8),
                    TEXT_SUBTITLE,
                );

                let pct_text = Self::fmt_pct(val);
                let pct_color = match val {
                    Some(v) if v <= 15 => DANGER_TEXT,
                    Some(_) => TEXT_PRIMARY,
                    None => TEXT_MUTED,
                };
                ui.painter().text(
                    Pos2::new(inner.left(), inner.center().y + 3.0),
                    egui::Align2::LEFT_CENTER,
                    pct_text,
                    font_semibold(23.5),
                    pct_color,
                );

                let bar_rect = Rect::from_min_max(
                    Pos2::new(inner.left(), inner.bottom() - 5.5),
                    Pos2::new(inner.right(), inner.bottom()),
                );
                let (frac, bar_col) = match val {
                    Some(v) => ((v.min(100) as f32) / 100.0, battery_bar_color(v)),
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
