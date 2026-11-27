//! Interactive 10-band Equalizer page: draggable Bezier curve nodes, category filters,
//! quick-audition chips, custom center-origin studio faders, and atomic preset management.

use crate::backend::BackendEvent;
use crate::presets;
use crate::protocol as p;
use crate::settings;
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::{NoticeTone, PresetCategory, StudioApp};
use eframe::egui::{self, Color32, CursorIcon, Pos2, RichText, Rounding, Sense, Stroke, Vec2};

const QUICK_PRESETS: &[(&str, &str, &str)] = &[
    ("Mặc định (0 dB)", "Mặc định", "earfun:0"),
    ("Bass +2", "Bass +2", "earfun:1"),
    ("Rock", "Rock", "earfun:5"),
    ("Pop", "Pop", "earfun:6"),
    ("Giọng hát", "Giọng hát", "earfun:29"),
    ("Classical", "Classical", "earfun:9"),
    ("Fernanda Ấm áp", "Ấm áp", "fernanda:201"),
];

impl StudioApp {
    pub(crate) fn render_equalizer(&mut self, ui: &mut egui::Ui) {
        let pal = ui_palette(ui);
        let enabled = self.is_enabled();
        let mut reset_clicked = false;
        let mut selected_key_to_load: Option<String> = None;
        let mut save_clicked = false;
        let mut apply_clicked = false;
        let mut rename_clicked = false;
        let mut delete_clicked = false;
        let mut import_clicked = false;
        let mut export_clicked = false;
        let mut sliders_changed = false;
        let mut draft_committed = false;

        draw_card(ui, None, None, |ui| {
            let is_compact = ui.available_width() < 660.0;
            // Overlap-proof card header with live sync badge + quick reset
            if is_compact {
                draw_card_header_with_right(ui, "Equalizer 10 dải", None, |ui| {
                    if primary_button(ui, "Gửi tới tai nghe", enabled)
                        .on_hover_text("Ghi 10 dải EQ hiện tại xuống tai nghe qua Bluetooth LE")
                        .clicked()
                    {
                        apply_clicked = true;
                    }
                    let is_flat = self.eq_levels.iter().all(|&v| v == 0);
                    if secondary_button(ui, "Đặt lại 0 dB", !is_flat).clicked() {
                        reset_clicked = true;
                    }
                    self.render_eq_sync_badge(ui);
                });
                ui.add_space(-6.0);
                ui.add(
                    egui::Label::new(
                        RichText::new(
                            "Kéo điểm trên đồ thị hoặc thanh gạt • Nhấp đúp hoặc phím 0 để về 0 dB",
                        )
                        .size(11.5)
                        .color(pal.text_subtitle),
                    )
                    .wrap(),
                );
                ui.add_space(8.0);
            } else {
                draw_card_header_with_right(
                    ui,
                    "Bộ cân bằng âm 10 dải (Equalizer)",
                    Some("Kéo điểm trên đồ thị hoặc thanh gạt • Nhấp đúp hoặc nhấn phím 0 để đưa dải về 0 dB"),
                    |ui| {
                        if primary_button(ui, "Gửi tới tai nghe", enabled)
                            .on_hover_text("Ghi 10 dải EQ hiện tại xuống tai nghe qua Bluetooth LE")
                            .clicked()
                        {
                            apply_clicked = true;
                        }
                        let is_flat = self.eq_levels.iter().all(|&v| v == 0);
                        if secondary_button(ui, "Đặt lại 0 dB", !is_flat).clicked() {
                            reset_clicked = true;
                        }
                        self.render_eq_sync_badge(ui);
                    },
                );
            }

            // Category Filter Chips + Searchable Preset Dropdown
            let cur_cat = self.eq_category;
            let custom_cnt = self.settings.data.presets.len();
            let mut next_cat: Option<PresetCategory> = None;
            setting_row(
                ui,
                32.0,
                |ui| {
                    let current_label = self
                        .preset_options
                        .get(self.selected_preset_idx)
                        .map(|o| o.label.clone())
                        .unwrap_or_else(|| "Tùy chỉnh (bản nháp)".to_string());

                    let combo_w = if is_compact {
                        148.0
                    } else {
                        (ui.available_width() * 0.42).clamp(170.0, 245.0)
                    };
                    let prev_idx = self.selected_preset_idx;
                    egui::ComboBox::from_id_salt("preset_picker")
                        .width(combo_w)
                        .selected_text(
                            RichText::new(current_label)
                                .size(12.3)
                                .color(pal.text_primary),
                        )
                        .show_ui(ui, |ui| {
                            styled_text_input(
                                ui,
                                &mut self.preset_search,
                                "Tìm nhanh mẫu EQ…",
                                210.0,
                            );
                            ui.add_space(4.0);
                            let q = self.preset_search.trim().to_lowercase();
                            for (i, opt) in self.preset_options.iter().enumerate() {
                                if !Self::matches_category(&opt.key, cur_cat) {
                                    continue;
                                }
                                if q.is_empty() || opt.label.to_lowercase().contains(&q) {
                                    ui.selectable_value(
                                        &mut self.selected_preset_idx,
                                        i,
                                        &opt.label,
                                    );
                                }
                            }
                        });
                    if self.selected_preset_idx != prev_idx {
                        if let Some(opt) = self.preset_options.get(self.selected_preset_idx) {
                            selected_key_to_load = Some(opt.key.clone());
                        }
                    }
                },
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = if is_compact { 4.0 } else { 6.0 };
                        let cats = [
                            (PresetCategory::All, "Tất cả".to_string()),
                            (PresetCategory::EarFun, "EarFun (30)".to_string()),
                            (PresetCategory::Fernanda, "Fernanda (2)".to_string()),
                            (PresetCategory::Custom, format!("Cá nhân ({})", custom_cnt)),
                        ];
                        for (cat, lbl) in cats {
                            if chip_button(ui, &lbl, cur_cat == cat, true).clicked() {
                                next_cat = Some(cat);
                            }
                        }
                    });
                },
            );
            if let Some(cat) = next_cat {
                self.eq_category = cat;
            }

            // Quick-pick favorite preset chips for 1-click auditioning
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = if is_compact { 4.0 } else { 5.0 };
                ui.label(
                    RichText::new("Chọn nhanh:")
                        .size(11.5)
                        .color(pal.text_muted),
                );
                let cur_key = self
                    .preset_options
                    .get(self.selected_preset_idx)
                    .map(|o| o.key.as_str())
                    .unwrap_or("");
                for &(full_lbl, short_lbl, pkey) in QUICK_PRESETS {
                    let active = cur_key == pkey;
                    let lbl = if is_compact { short_lbl } else { full_lbl };
                    let mut resp = chip_button(ui, lbl, active, true);
                    if is_compact && short_lbl != full_lbl {
                        resp = resp.on_hover_text(full_lbl);
                    }
                    if resp.clicked() {
                        selected_key_to_load = Some(pkey.to_string());
                    }
                }
            });

            ui.add_space(8.0);
            let (curve_changed, curve_committed) =
                Self::draw_interactive_eq_curve(ui, &mut self.eq_levels);
            if curve_changed {
                sliders_changed = true;
            }
            if curve_committed {
                draft_committed = true;
            }
            ui.add_space(8.0);

            // 10 Custom Studio Vertical Band Faders
            if self.eq_levels.len() == 10 {
                ui.columns(10, |cols| {
                    for (i, &freq) in p::FREQUENCIES.iter().enumerate() {
                        cols[i].vertical_centered(|ui| {
                            ui.spacing_mut().item_spacing.y = 4.0;
                            let gain_idx = (self.eq_levels[i].clamp(-10, 10) + 10) as usize;
                            let gain = p::GAINS.get(gain_idx).copied().unwrap_or(0.0);
                            let gain_str = if gain.abs() < 1e-6 {
                                "0 dB".to_string()
                            } else {
                                format!("{:+.1}", gain).trim_end_matches(".0").to_string() + " dB"
                            };
                            let gain_col = if self.eq_levels[i] == 0 {
                                pal.text_muted
                            } else {
                                pal.accent_strong
                            };
                            let lbl_resp = ui.add(
                                egui::Label::new(
                                    RichText::new(gain_str)
                                        .font(font_medium(10.8))
                                        .color(gain_col),
                                )
                                .sense(Sense::click()),
                            );
                            if lbl_resp
                                .on_hover_text("Nhấp để đưa dải này về 0 dB")
                                .clicked()
                                && self.eq_levels[i] != 0
                            {
                                self.eq_levels[i] = 0;
                                sliders_changed = true;
                                draft_committed = true;
                            }

                            let (f_changed, f_committed) =
                                vertical_eq_fader(ui, i, &mut self.eq_levels[i]);
                            if f_changed {
                                sliders_changed = true;
                            }
                            if f_committed {
                                draft_committed = true;
                            }

                            let freq_label = Self::format_freq(freq);
                            ui.label(
                                RichText::new(freq_label)
                                    .size(11.0)
                                    .color(pal.text_subtitle),
                            );
                        });
                    }
                });
            }

            ui.add_space(10.0);
            draw_row_divider(ui);
            ui.add_space(4.0);

            // Custom preset management bar with aligned 31px text input
            let is_custom_selected = self
                .preset_options
                .get(self.selected_preset_idx)
                .map(|o| o.key.starts_with("custom:"))
                .unwrap_or(false);

            setting_row(
                ui,
                32.0,
                |ui| {
                    if secondary_button(ui, "Xuất JSON…", true).clicked() {
                        export_clicked = true;
                    }
                    if secondary_button(ui, "Nhập JSON…", true).clicked() {
                        import_clicked = true;
                    }
                },
                |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        styled_text_input(
                            ui,
                            &mut self.preset_name_input,
                            "Tên mẫu cá nhân…",
                            155.0,
                        );
                        let has_name = !self.preset_name_input.trim().is_empty();
                        if secondary_button(ui, "Lưu mẫu", has_name)
                            .on_hover_text("Lưu đường cong hiện tại thành mẫu EQ cá nhân trên máy")
                            .clicked()
                        {
                            save_clicked = true;
                        }
                        if secondary_button(ui, "Đổi tên", is_custom_selected && has_name)
                            .on_hover_text(
                                "Đổi tên mẫu cá nhân đang chọn sang tên mới trong ô bên trái",
                            )
                            .clicked()
                        {
                            rename_clicked = true;
                        }
                        if secondary_button(ui, "Xóa mẫu", is_custom_selected)
                            .on_hover_text("Xóa mẫu EQ cá nhân đang chọn khỏi máy")
                            .clicked()
                        {
                            delete_clicked = true;
                        }
                    });
                },
            );
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Lưu ý: Giao thức TW600 hỗ trợ ghi trực tiếp 10 dải EQ xuống DSP tai nghe nhưng không hỗ trợ đọc ngược đường cong từ tai nghe.",
            )
            .size(11.2)
            .color(pal.text_muted),
        );

        if sliders_changed {
            self.sync_eq_picker();
        }
        if draft_committed {
            self.persist_draft_eq();
        }
        if reset_clicked {
            self.set_eq(vec![0; 10], Some("earfun:0"));
            self.persist_draft_eq();
        }
        if let Some(key) = selected_key_to_load {
            if let Some(opt) = self.preset_options.iter().find(|o| o.key == key).cloned() {
                if let Some(levels) = opt.levels {
                    self.set_eq(levels, Some(&opt.key));
                    self.persist_draft_eq();
                    if let Some(custom_name) = opt.key.strip_prefix("custom:") {
                        self.preset_name_input = custom_name.to_string();
                    }
                }
            }
        }
        if save_clicked {
            self.handle_save_preset();
        }
        if rename_clicked {
            self.handle_rename_preset();
        }
        if delete_clicked {
            self.handle_delete_preset();
        }
        if import_clicked {
            self.handle_import_preset();
        }
        if export_clicked {
            self.handle_export_preset();
        }
        if apply_clicked {
            self.handle_apply_eq();
        }
    }

    fn render_eq_sync_badge(&self, ui: &mut egui::Ui) {
        let p = ui_palette(ui);
        if self.eq_sending_levels.is_some() {
            status_pill(ui, "Đang gửi EQ…", p.info_bg, p.info_border, p.info_text);
        } else if self.eq_sent_levels.as_ref() == Some(&self.eq_levels) {
            status_pill(
                ui,
                "Đã đồng bộ tai nghe",
                p.success_bg,
                p.success_border,
                p.success_text,
            );
        } else if self.is_connected() {
            status_pill(
                ui,
                "Bản nháp chưa gửi",
                p.warn_bg,
                p.warn_border,
                p.warn_text,
            );
        } else {
            status_pill(
                ui,
                "Bản nháp trên máy",
                p.card_subtle_bg,
                p.ctrl_border,
                p.text_subtitle,
            );
        }
    }

    pub fn matches_category(key: &str, category: PresetCategory) -> bool {
        match category {
            PresetCategory::All => true,
            PresetCategory::EarFun => key == "draft" || key.starts_with("earfun:"),
            PresetCategory::Fernanda => key == "draft" || key.starts_with("fernanda:"),
            PresetCategory::Custom => key == "draft" || key.starts_with("custom:"),
        }
    }

    fn format_freq(freq: f64) -> String {
        if freq >= 1000.0 {
            format!("{}k", (freq / 1000.0) as u32)
        } else if (freq - 31.5).abs() < 0.1 {
            "31.5".to_string()
        } else {
            format!("{}", freq as u32)
        }
    }

    pub(crate) fn format_freq_unit(freq: f64) -> String {
        if freq >= 1000.0 {
            format!("{} kHz", (freq / 1000.0) as u32)
        } else if (freq - 31.5).abs() < 0.1 {
            "31.5 Hz".to_string()
        } else {
            format!("{} Hz", freq as u32)
        }
    }

    /// Interactive EQ curve: supports dragging any of the 10 band nodes directly on the canvas.
    /// Returns `(levels_changed, interaction_committed)`.
    pub(crate) fn draw_interactive_eq_curve(ui: &mut egui::Ui, levels: &mut [i32]) -> (bool, bool) {
        let pal = ui_palette(ui);
        let desired = Vec2::new(ui.available_width(), 126.0);
        let (rect, resp) = ui.allocate_exact_size(desired, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let mut changed = false;
        let mut committed = false;

        painter.rect(
            rect,
            Rounding::same(13.0),
            pal.chart_bg,
            Stroke::new(1.0_f32, pal.chart_border),
        );

        let left = rect.left() + 38.0;
        let right = rect.right() - 16.0;
        let top = rect.top() + 14.0;
        let bottom = rect.bottom() - 14.0;
        if levels.len() != 10 || right <= left || bottom <= top {
            return (false, false);
        }
        let y_for_gain = |gain: f64| -> f32 { top + ((6.0 - gain) as f32 / 12.0) * (bottom - top) };
        let zero_y = y_for_gain(0.0);

        let mut active_band: Option<usize> = None;
        if let Some(pos) = resp.interact_pointer_pos().or_else(|| resp.hover_pos()) {
            if pos.x >= left - 12.0 && pos.x <= right + 12.0 {
                let rel = ((pos.x - left) / (right - left)).clamp(0.0, 0.9999);
                let idx = (rel * 10.0).floor() as usize;
                active_band = Some(idx);

                if resp.double_clicked() {
                    if levels[idx] != 0 {
                        levels[idx] = 0;
                        changed = true;
                    }
                    committed = true;
                } else if resp.dragged() || resp.clicked() {
                    let target_gain =
                        (6.0 - ((pos.y - top) / (bottom - top)).clamp(0.0, 1.0) * 12.0) as f64;
                    let best_step = p::GAINS
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            ((*a - target_gain).abs())
                                .partial_cmp(&((*b - target_gain).abs()))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .map(|(step_i, _)| (step_i as i32) - 10)
                        .unwrap_or(0);
                    if levels[idx] != best_step {
                        levels[idx] = best_step;
                        changed = true;
                    }
                    if resp.clicked() {
                        committed = true;
                    }
                }
            }
        }
        if resp.drag_stopped() {
            committed = true;
        }
        if resp.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }

        // Horizontal dB grid lines
        for db in [6, 3, 0, -3, -6] {
            let yy = y_for_gain(db as f64);
            let label = if db > 0 {
                format!("+{}", db)
            } else {
                db.to_string()
            };
            painter.text(
                Pos2::new(rect.left() + 10.0, yy),
                egui::Align2::LEFT_CENTER,
                label,
                font_regular(10.0),
                pal.text_muted,
            );
            let line_col = if db == 0 {
                pal.chart_zero
            } else {
                pal.chart_grid
            };
            painter.line_segment(
                [Pos2::new(left, yy), Pos2::new(right, yy)],
                Stroke::new(if db == 0 { 1.2_f32 } else { 1.0_f32 }, line_col),
            );
        }

        let mut points = Vec::with_capacity(10);
        for (i, &lv) in levels.iter().enumerate() {
            let gain = p::GAINS[(lv.clamp(-10, 10) + 10) as usize];
            let x = left + (right - left) * ((i as f32 + 0.5) / 10.0);
            let y = y_for_gain(gain);
            let vline_col = if active_band == Some(i) {
                pal.chart_zero
            } else {
                pal.chart_grid
            };
            painter.line_segment(
                [Pos2::new(x, top), Pos2::new(x, bottom)],
                Stroke::new(1.0_f32, vline_col),
            );
            points.push(Pos2::new(x, y));
        }

        // Smooth cubic curve + translucent area fill to 0 dB line
        let mut curve_pts = Vec::with_capacity(110);
        if let Some(&first) = points.first() {
            curve_pts.push(first);
        }
        for i in 0usize..9usize {
            let p0 = points[i.saturating_sub(1)];
            let p1 = points[i];
            let p2 = points[i + 1];
            let p3 = points[(i + 2).min(9)];

            let c1 = Pos2::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
            let c2 = Pos2::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);

            for step in 1..=12 {
                let t = step as f32 / 12.0;
                let inv = 1.0 - t;
                let x = inv * inv * inv * p1.x
                    + 3.0 * inv * inv * t * c1.x
                    + 3.0 * inv * t * t * c2.x
                    + t * t * t * p2.x;
                let y = inv * inv * inv * p1.y
                    + 3.0 * inv * inv * t * c1.y
                    + 3.0 * inv * t * t * c2.y
                    + t * t * t * p2.y;
                curve_pts.push(Pos2::new(x, y.clamp(top, bottom)));
            }
        }

        if curve_pts.len() >= 2 {
            let mut mesh = egui::epaint::Mesh::default();
            for pt in &curve_pts {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: *pt,
                    uv: egui::epaint::WHITE_UV,
                    color: pal.chart_fill_top,
                });
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: Pos2::new(pt.x, zero_y),
                    uv: egui::epaint::WHITE_UV,
                    color: pal.chart_fill_zero,
                });
            }
            for idx in 0..(curve_pts.len() as u32 - 1) {
                let base = idx * 2;
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base + 1, base + 3, base + 2);
            }
            painter.add(egui::Shape::mesh(mesh));

            let stroke = Stroke::new(2.3_f32, pal.accent);
            for w in curve_pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }

        for (i, &pt) in points.iter().enumerate() {
            let is_hot = active_band == Some(i) && resp.hovered();
            let r = if is_hot { 5.2 } else { 3.9 };
            let fill = if is_hot { pal.accent } else { pal.card_bg };
            let ring = if is_hot { Color32::WHITE } else { pal.accent };
            painter.circle(pt, r, fill, Stroke::new(2.0_f32, ring));
        }

        if let Some(idx) = active_band {
            if resp.hovered() || resp.dragged() {
                let freq = Self::format_freq_unit(p::FREQUENCIES[idx]);
                let gain = p::GAINS[(levels[idx].clamp(-10, 10) + 10) as usize];
                resp.on_hover_text(format!(
                    "{}: {:+.1} dB (Kéo để chỉnh • Nhấp đúp về 0 dB)",
                    freq, gain
                ));
            }
        }

        (changed, committed)
    }

    pub(crate) fn handle_save_preset(&mut self) {
        let name = match presets::sanitize_preset_name(&self.preset_name_input) {
            Ok(clean) => clean,
            Err(e) => {
                self.show_notice(e, true);
                return;
            }
        };
        let levels = self.eq_levels.clone();
        let exists = self.settings.data.presets.contains_key(&name);
        let prompt_name = name.clone();
        let do_save = move |app: &mut StudioApp| {
            app.settings
                .data
                .presets
                .insert(name.clone(), levels.clone());
            app.settings.data.eq = levels;
            app.save_settings();
            app.refresh_presets(Some(&format!("custom:{}", name)));
            app.show_notice("Đã lưu mẫu EQ cá nhân trên máy.", false);
        };
        if exists {
            let prompt = format!("Ghi đè mẫu EQ cá nhân “{}”?", prompt_name);
            self.ask_confirm(prompt, do_save);
        } else {
            do_save(self);
        }
    }

    fn selected_custom_name(&mut self) -> Option<String> {
        let key = self
            .preset_options
            .get(self.selected_preset_idx)
            .map(|o| o.key.as_str())
            .unwrap_or("");
        if let Some(custom) = key.strip_prefix("custom:") {
            Some(custom.to_string())
        } else {
            self.show_notice("Hãy chọn một mẫu trong nhóm Cá nhân trước.", true);
            None
        }
    }

    pub(crate) fn handle_rename_preset(&mut self) {
        let Some(old) = self.selected_custom_name() else {
            return;
        };
        let new_name = match presets::sanitize_preset_name(&self.preset_name_input) {
            Ok(clean) => clean,
            Err(e) => {
                self.show_notice(e, true);
                return;
            }
        };
        if new_name != old && self.settings.data.presets.contains_key(&new_name) {
            self.show_notice("Tên mới đã tồn tại. Hãy chọn tên khác.", true);
            return;
        }
        if let Some(levels) = self.settings.data.presets.remove(&old) {
            self.settings.data.presets.insert(new_name.clone(), levels);
            self.save_settings();
            self.refresh_presets(Some(&format!("custom:{}", new_name)));
            self.show_notice("Đã đổi tên mẫu EQ cá nhân.", false);
        }
    }

    pub(crate) fn handle_delete_preset(&mut self) {
        let Some(name) = self.selected_custom_name() else {
            return;
        };
        let prompt = format!("Xóa mẫu EQ cá nhân “{}” khỏi máy?", name);
        self.ask_confirm(prompt, move |app| {
            app.settings.data.presets.remove(&name);
            app.save_settings();
            app.refresh_presets(None);
            app.show_notice(
                "Đã xóa mẫu cá nhân; đường cong đang chỉnh vẫn được giữ.",
                false,
            );
        });
    }

    pub(crate) fn handle_export_preset(&mut self) {
        let fallback_name = self
            .preset_options
            .get(self.selected_preset_idx)
            .map(|o| o.label.clone())
            .unwrap_or_else(|| "Bản chỉnh EQ".to_string());
        let name = if self.preset_name_input.trim().is_empty() {
            fallback_name
        } else {
            self.preset_name_input.trim().to_string()
        };
        let doc = match presets::encode(&name, &self.eq_levels) {
            Ok(d) => d,
            Err(e) => {
                self.show_notice(e, true);
                return;
            }
        };
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Xuất bản nháp EQ")
            .set_file_name("earfun-eq.json")
            .add_filter("Bản chỉnh EQ (*.json)", &["json"])
            .save_file()
        {
            match serde_json::to_string_pretty(&doc)
                .map_err(|e| e.to_string())
                .and_then(|s| {
                    settings::write_atomic(&path, format!("{}\n", s).as_bytes())
                        .map_err(|e| format!("Không ghi được tệp EQ: {}", e))
                }) {
                Ok(()) => self.show_notice("Đã xuất mẫu EQ ra tệp JSON.", false),
                Err(e) => self.show_notice(e, true),
            }
        }
    }

    pub(crate) fn handle_import_preset(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Nhập bản chỉnh EQ EarFun Studio")
            .add_filter("Bản chỉnh EQ (*.json)", &["json"])
            .pick_file()
        {
            const MAX_EQ_BYTES: u64 = 65_536;
            match std::fs::metadata(&path) {
                Ok(meta) if !meta.is_file() => {
                    self.show_notice("Đường dẫn đã chọn không phải tệp hợp lệ.", true);
                    return;
                }
                Ok(meta) if meta.len() > MAX_EQ_BYTES => {
                    self.show_notice("Tệp EQ quá lớn (tối đa 64 KiB).", true);
                    return;
                }
                Err(e) => {
                    self.show_notice(format!("Không đọc được tệp EQ: {}", e), true);
                    return;
                }
                _ => {}
            }
            let read_res = std::fs::File::open(&path).and_then(|file| {
                use std::io::Read;
                let mut buf = Vec::new();
                file.take(MAX_EQ_BYTES + 1).read_to_end(&mut buf)?;
                Ok(buf)
            });
            match read_res {
                Ok(bytes) => {
                    if bytes.len() as u64 > MAX_EQ_BYTES {
                        self.show_notice("Tệp EQ quá lớn (tối đa 64 KiB).", true);
                        return;
                    }
                    let res = serde_json::from_slice::<serde_json::Value>(&bytes)
                        .map_err(|_| "Tệp JSON không đúng cú pháp.".to_string())
                        .and_then(|v| presets::decode_value(&v));
                    match res {
                        Ok((name, levels)) => {
                            self.set_eq(levels, None);
                            self.persist_draft_eq();
                            self.preset_name_input = name;
                            self.set_notice(
                                "Đã nạp tệp vào bản nháp. Bấm Lưu mẫu hoặc Gửi tới tai nghe để áp dụng.",
                                NoticeTone::Success,
                            );
                        }
                        Err(e) => self.show_notice(e, true),
                    }
                }
                Err(e) => self.show_notice(format!("Không đọc được tệp EQ: {}", e), true),
            }
        }
    }

    pub(crate) fn handle_apply_eq(&mut self) {
        let levels = self.eq_levels.clone();
        self.settings.data.eq = levels.clone();
        self.save_settings();
        self.eq_sending_levels = Some(levels.clone());
        let core = self.backend.core.clone();
        let generation = self.connection_generation;
        self.backend.spawn_task(async move {
            core.set_eq(&levels).await?;
            core.emit_event(BackendEvent::EqSent { generation, levels });
            Ok(())
        });
    }
}
