//! Interactive 10-band Equalizer page: draggable Bezier curve nodes, category filters,
//! quick-audition chips, vertical sliders, and custom preset management.

use crate::backend::BackendEvent;
use crate::presets;
use crate::protocol as p;
use crate::ui::theme::*;
use crate::ui::widgets::*;
use crate::ui::{PresetCategory, StudioApp};
use eframe::egui::{self, Color32, CursorIcon, Pos2, RichText, Rounding, Sense, Stroke, Vec2};

const QUICK_PRESETS: &[(&str, &str)] = &[
    ("Mặc định (0 dB)", "earfun:0"),
    ("Bass +2", "earfun:1"),
    ("Rock", "earfun:5"),
    ("Pop", "earfun:6"),
    ("Giọng hát", "earfun:29"),
    ("Classical", "earfun:9"),
    ("Fernanda Ấm áp", "fernanda:201"),
];

impl StudioApp {
    pub(crate) fn render_equalizer(&mut self, ui: &mut egui::Ui) {
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

        draw_card(ui, None, None, |ui| {
            // Card header with live sync badge + quick reset
            draw_card_header_with_right(
                ui,
                "Bộ cân bằng âm 10 dải (Equalizer)",
                Some("Kéo trực tiếp các điểm trên đồ thị hoặc thanh trượt • Nhấp đúp một dải để đưa về 0 dB"),
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

            // Category Filter Chips + Searchable Preset Dropdown
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                let custom_cnt = self.settings.data.presets.len();
                let cats = [
                    (PresetCategory::All, "Tất cả".to_string()),
                    (PresetCategory::EarFun, "EarFun (30)".to_string()),
                    (PresetCategory::Fernanda, "Fernanda (2)".to_string()),
                    (PresetCategory::Custom, format!("Cá nhân ({})", custom_cnt)),
                ];
                for (cat, lbl) in cats {
                    if chip_button(ui, &lbl, self.eq_category == cat, true).clicked() {
                        self.eq_category = cat;
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let current_label = self
                        .preset_options
                        .get(self.selected_preset_idx)
                        .map(|o| o.label.clone())
                        .unwrap_or_else(|| "Tùy chỉnh (bản nháp)".to_string());

                    let combo_w = (ui.available_width()).clamp(180.0, 260.0);
                    let prev_idx = self.selected_preset_idx;
                    egui::ComboBox::from_id_salt("preset_picker")
                        .width(combo_w)
                        .selected_text(RichText::new(current_label).size(12.5).color(TEXT_PRIMARY))
                        .show_ui(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.preset_search)
                                    .hint_text("Tìm nhanh mẫu EQ…"),
                            );
                            ui.add_space(4.0);
                            let q = self.preset_search.trim().to_lowercase();
                            for (i, opt) in self.preset_options.iter().enumerate() {
                                if !Self::matches_category(&opt.key, self.eq_category) {
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
                });
            });

            // Quick-pick favorite preset chips for 1-click auditioning
            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                ui.label(RichText::new("Chọn nhanh:").size(11.5).color(TEXT_MUTED));
                let cur_key = self
                    .preset_options
                    .get(self.selected_preset_idx)
                    .map(|o| o.key.as_str())
                    .unwrap_or("");
                for &(lbl, pkey) in QUICK_PRESETS {
                    let active = cur_key == pkey;
                    if chip_button(ui, lbl, active, true).clicked() {
                        selected_key_to_load = Some(pkey.to_string());
                    }
                }
            });

            ui.add_space(8.0);
            if Self::draw_interactive_eq_curve(ui, &mut self.eq_levels) {
                sliders_changed = true;
            }
            ui.add_space(8.0);

            // 10 Vertical Band Faders
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
                                TEXT_MUTED
                            } else {
                                ACCENT_INDIGO
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
                            }

                            let slider = egui::Slider::new(&mut self.eq_levels[i], -10..=10)
                                .vertical()
                                .show_value(false);
                            let sl_resp = ui.add_sized([22.0, 122.0], slider);
                            if sl_resp.double_clicked() {
                                self.eq_levels[i] = 0;
                                sliders_changed = true;
                            } else if sl_resp.changed() {
                                sliders_changed = true;
                            }

                            let freq_label = Self::format_freq(freq);
                            ui.label(RichText::new(freq_label).size(11.0).color(TEXT_SUBTITLE));
                        });
                    }
                });
            }

            ui.add_space(10.0);
            draw_row_divider(ui);
            ui.add_space(4.0);

            // Custom preset management bar
            let is_custom_selected = self
                .preset_options
                .get(self.selected_preset_idx)
                .map(|o| o.key.starts_with("custom:"))
                .unwrap_or(false);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 6.0;
                ui.add(
                    egui::TextEdit::singleline(&mut self.preset_name_input)
                        .hint_text("Tên mẫu cá nhân…")
                        .desired_width(165.0),
                );
                let has_name = !self.preset_name_input.trim().is_empty();
                if secondary_button(ui, "Lưu mẫu", has_name)
                    .on_hover_text("Lưu đường cong hiện tại thành mẫu EQ cá nhân trên máy")
                    .clicked()
                {
                    save_clicked = true;
                }
                if secondary_button(ui, "Đổi tên", is_custom_selected && has_name)
                    .on_hover_text("Đổi tên mẫu cá nhân đang chọn sang tên mới trong ô bên trái")
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if secondary_button(ui, "Xuất JSON…", true).clicked() {
                        export_clicked = true;
                    }
                    if secondary_button(ui, "Nhập JSON…", true).clicked() {
                        import_clicked = true;
                    }
                });
            });
        });

        ui.add_space(6.0);
        ui.label(
            RichText::new(
                "Lưu ý: Giao thức TW600 hỗ trợ ghi trực tiếp 10 dải EQ xuống DSP tai nghe nhưng không hỗ trợ đọc ngược đường cong từ tai nghe.",
            )
            .size(11.2)
            .color(TEXT_MUTED),
        );

        if sliders_changed {
            self.sync_eq_picker();
        }
        if reset_clicked {
            self.set_eq(vec![0; 10], Some("earfun:0"));
        }
        if let Some(key) = selected_key_to_load {
            if let Some(opt) = self.preset_options.iter().find(|o| o.key == key).cloned() {
                if let Some(levels) = opt.levels {
                    self.set_eq(levels, Some(&opt.key));
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
        if self.eq_sending_levels.is_some() {
            status_pill(ui, "Đang gửi EQ…", INFO_BG, INFO_BORDER, ACCENT_INDIGO);
        } else if self.eq_sent_levels.as_ref() == Some(&self.eq_levels) {
            status_pill(
                ui,
                "Đã đồng bộ tai nghe",
                SUCCESS_BG,
                SUCCESS_BORDER,
                SUCCESS_TEXT,
            );
        } else if self.is_connected() {
            status_pill(ui, "Bản nháp chưa gửi", WARN_BG, WARN_BORDER, WARN_TEXT);
        } else {
            status_pill(
                ui,
                "Bản nháp trên máy",
                Color32::from_rgb(245, 247, 252),
                CTRL_BORDER,
                TEXT_SUBTITLE,
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

    /// Interactive EQ curve: supports dragging any of the 10 band nodes directly on the canvas!
    pub(crate) fn draw_interactive_eq_curve(ui: &mut egui::Ui, levels: &mut [i32]) -> bool {
        let desired = Vec2::new(ui.available_width(), 126.0);
        let (rect, resp) = ui.allocate_exact_size(desired, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let mut changed = false;

        painter.rect(
            rect,
            Rounding::same(13.0),
            Color32::from_rgb(247, 250, 255),
            Stroke::new(1.0_f32, Color32::from_rgb(229, 236, 248)),
        );

        let left = rect.left() + 38.0;
        let right = rect.right() - 16.0;
        let top = rect.top() + 14.0;
        let bottom = rect.bottom() - 14.0;
        if levels.len() != 10 || right <= left || bottom <= top {
            return false;
        }
        let y_for_gain = |gain: f64| -> f32 { top + ((6.0 - gain) as f32 / 12.0) * (bottom - top) };
        let zero_y = y_for_gain(0.0);

        // Handle interactive pointer dragging / double-clicking on the 10 bands
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
                }
            }
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
                Color32::from_rgb(124, 131, 152),
            );
            let line_col = if db == 0 {
                Color32::from_rgb(202, 214, 238)
            } else {
                Color32::from_rgb(231, 237, 248)
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
                Color32::from_rgb(205, 218, 245)
            } else {
                Color32::from_rgb(238, 242, 250)
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

        // Translucent mesh fill between curve and 0 dB baseline
        if curve_pts.len() >= 2 {
            let mut mesh = egui::epaint::Mesh::default();
            let fill_col = Color32::from_rgba_unmultiplied(96, 128, 208, 34);
            let zero_col = Color32::from_rgba_unmultiplied(96, 128, 208, 6);
            for pt in &curve_pts {
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: *pt,
                    uv: egui::epaint::WHITE_UV,
                    color: fill_col,
                });
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: Pos2::new(pt.x, zero_y),
                    uv: egui::epaint::WHITE_UV,
                    color: zero_col,
                });
            }
            for idx in 0..(curve_pts.len() as u32 - 1) {
                let base = idx * 2;
                mesh.add_triangle(base, base + 1, base + 2);
                mesh.add_triangle(base + 1, base + 3, base + 2);
            }
            painter.add(egui::Shape::mesh(mesh));

            let stroke = Stroke::new(2.4_f32, ACCENT_BLUE);
            for w in curve_pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }

        // Control handles + hover tooltip
        for (i, &pt) in points.iter().enumerate() {
            let is_hot = active_band == Some(i) && resp.hovered();
            let r = if is_hot { 5.2 } else { 3.8 };
            let fill = if is_hot { ACCENT_BLUE } else { Color32::WHITE };
            let ring = if is_hot { Color32::WHITE } else { ACCENT_BLUE };
            painter.circle(pt, r, fill, Stroke::new(2.0_f32, ring));
        }

        if let Some(idx) = active_band {
            if resp.hovered() || resp.dragged() {
                let freq = Self::format_freq(p::FREQUENCIES[idx]);
                let gain = p::GAINS[(levels[idx].clamp(-10, 10) + 10) as usize];
                resp.on_hover_text(format!(
                    "{} Hz: {:+.1} dB (Kéo để chỉnh • Nhấp đúp về 0 dB)",
                    freq, gain
                ));
            }
        }

        changed
    }

    pub(crate) fn handle_save_preset(&mut self) {
        let name = self.preset_name_input.trim().to_string();
        if name.is_empty() || name.chars().count() > 100 {
            self.show_notice("Nhập tên mẫu cá nhân từ 1 đến 100 ký tự.", true);
            return;
        }
        let levels = self.eq_levels.clone();
        let exists = self.settings.data.presets.contains_key(&name);
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
            let prompt = format!("Ghi đè mẫu EQ cá nhân “{}”?", self.preset_name_input.trim());
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
        let new_name = self.preset_name_input.trim().to_string();
        if new_name.is_empty() || new_name.chars().count() > 100 {
            self.show_notice("Nhập tên mới vào ô Tên mẫu cá nhân (1–100 ký tự).", true);
            return;
        }
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
                .and_then(|s| std::fs::write(&path, format!("{}\n", s)).map_err(|e| e.to_string()))
            {
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
                Ok(meta) if meta.len() > MAX_EQ_BYTES => {
                    self.show_notice("Tệp EQ quá lớn (tối đa 64 KiB).", true);
                    return;
                }
                Err(e) => {
                    self.show_notice(e.to_string(), true);
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
                        .map_err(|e| e.to_string())
                        .and_then(|v| presets::decode_value(&v));
                    match res {
                        Ok((name, levels)) => {
                            self.set_eq(levels, None);
                            self.preset_name_input = name;
                            self.show_notice(
                                "Đã nạp tệp vào bản nháp. Bấm Lưu mẫu hoặc Gửi tới tai nghe để áp dụng.",
                                false,
                            );
                        }
                        Err(e) => self.show_notice(e, true),
                    }
                }
                Err(e) => self.show_notice(e.to_string(), true),
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
