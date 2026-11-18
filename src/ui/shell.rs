//! Window chrome, keyboard shortcuts, confirmation modal, and semantic status footer.

use crate::ui::icons::{draw_spinner, draw_vector_icon, IconKind};
use crate::ui::theme::*;
use crate::ui::widgets::{primary_button, secondary_button};
use crate::ui::{ConfirmModal, NoticeTone, Page, StudioApp};
use eframe::egui::{self, Color32, Key, RichText, Rounding, Sense, Stroke, Vec2};

impl StudioApp {
    pub(crate) fn ask_confirm<F>(&mut self, message: impl Into<String>, action: F)
    where
        F: FnOnce(&mut StudioApp) + Send + 'static,
    {
        if self.confirm_modal.is_some() {
            return;
        }
        self.confirm_modal = Some(ConfirmModal {
            message: message.into(),
            just_opened: true,
            action: Box::new(action),
        });
    }

    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.confirm_modal.is_some() || ctx.wants_keyboard_input() {
            return;
        }
        let mut manual_refresh = false;
        let mut cycle_theme = false;
        ctx.input(|input| {
            if input.modifiers.alt {
                for (idx, key) in [
                    Key::Num1,
                    Key::Num2,
                    Key::Num3,
                    Key::Num4,
                    Key::Num5,
                    Key::Num6,
                ]
                .into_iter()
                .enumerate()
                {
                    if input.key_pressed(key) {
                        self.page = Page::ALL[idx];
                    }
                }
                if input.key_pressed(Key::T) {
                    cycle_theme = true;
                }
            }
            if input.key_pressed(Key::F5) && self.is_enabled() {
                manual_refresh = true;
            }
        });
        if manual_refresh {
            self.trigger_manual_query();
        }
        if cycle_theme {
            self.cycle_theme_mode(ctx);
        }
    }

    pub(crate) fn render_confirm_modal(&mut self, ctx: &egui::Context) {
        let (msg, allow_keys) = match self.confirm_modal.as_mut() {
            Some(m) => {
                let allow = !m.just_opened;
                m.just_opened = false;
                (m.message.clone(), allow)
            }
            None => return,
        };

        let p = palette(ctx);
        let mut choice: Option<bool> = None;

        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("modal_backdrop_scrim"))
            .order(egui::Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let (rect, resp) = ui.allocate_exact_size(screen.size(), Sense::click());
                ui.painter().rect_filled(
                    rect,
                    Rounding::ZERO,
                    Color32::from_rgba_unmultiplied(12, 16, 26, if p.is_dark { 135 } else { 76 }),
                );
                if allow_keys && resp.clicked() {
                    choice = Some(false);
                }
            });

        if allow_keys {
            ctx.input(|i| {
                if i.key_pressed(Key::Escape) {
                    choice = Some(false);
                } else if i.key_pressed(Key::Enter) {
                    choice = Some(true);
                }
            });
        }

        egui::Window::new("Xác nhận thay đổi")
            .order(egui::Order::Foreground)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(430.0)
            .min_width(380.0)
            .show(ctx, |ui| {
                ui.set_max_width(430.0);
                ui.add_space(6.0);
                ui.add(
                    egui::Label::new(RichText::new(&msg).size(13.0).color(p.text_primary)).wrap(),
                );
                ui.add_space(16.0);
                ui.horizontal_wrapped(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if primary_button(ui, "Tiếp tục", true).clicked() {
                            choice = Some(true);
                        }
                        if secondary_button(ui, "Hủy", true).clicked() {
                            choice = Some(false);
                        }
                    });
                });
            });

        if let Some(confirmed) = choice {
            if let Some(modal) = self.confirm_modal.take() {
                if confirmed {
                    (modal.action)(self);
                } else {
                    let st = self.state.clone();
                    self.sync_state_to_drafts(&st);
                }
            }
        }
    }

    pub fn render_panels(&mut self, ctx: &egui::Context) {
        self.handle_shortcuts(ctx);
        let p = palette(ctx);
        let bg_interactive = self.confirm_modal.is_none();

        egui::SidePanel::left("sidebar_panel")
            .resizable(false)
            .exact_width(214.0)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 16.0,
                right: 8.0,
                top: 16.0,
                bottom: 16.0,
            }))
            .show(ctx, |ui| {
                ui.add_enabled_ui(bg_interactive, |ui| {
                    self.render_sidebar(ui);
                });
            });

        egui::TopBottomPanel::bottom("footer_panel")
            .resizable(false)
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 10.0,
                right: 18.0,
                top: 4.0,
                bottom: 12.0,
            }))
            .show(ctx, |ui| {
                let (bg, border, fg, icon, icon_col) = match self.notice_tone {
                    NoticeTone::Error => (
                        p.danger_bg,
                        p.danger_border,
                        p.danger_text,
                        IconKind::Alert,
                        p.danger_text,
                    ),
                    NoticeTone::Warning => (
                        p.warn_bg,
                        p.warn_border,
                        p.warn_text,
                        IconKind::Alert,
                        p.warn_text,
                    ),
                    NoticeTone::Success => (
                        p.footer_bg,
                        p.card_border,
                        p.text_primary,
                        IconKind::Check,
                        p.success_text,
                    ),
                    NoticeTone::Info => (
                        p.footer_bg,
                        p.card_border,
                        p.text_subtitle,
                        IconKind::Info,
                        p.accent,
                    ),
                };
                egui::Frame::none()
                    .fill(bg)
                    .stroke(Stroke::new(1.0_f32, border))
                    .rounding(Rounding::same(11.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 6.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.horizontal_wrapped(|ui| {
                            if self.busy > 0 && self.notice_tone != NoticeTone::Error {
                                draw_spinner(ui, 14.0, p.accent);
                            } else {
                                let (icon_rect, _) =
                                    ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                                draw_vector_icon(ui.painter(), icon_rect, icon, icon_col);
                            }
                            ui.add(
                                egui::Label::new(RichText::new(&self.notice).size(11.5).color(fg))
                                    .selectable(true)
                                    .wrap(),
                            );
                        });
                    });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(egui::Margin {
                left: 10.0,
                right: 18.0,
                top: 16.0,
                bottom: 4.0,
            }))
            .show(ctx, |ui| {
                ui.add_enabled_ui(bg_interactive, |ui| {
                    self.render_top_header(ui);
                    ui.add_space(10.0);
                    egui::ScrollArea::vertical()
                        .id_salt("main_content")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add_space(2.0);
                            match self.page {
                                Page::Overview => self.render_overview(ui),
                                Page::Equalizer => self.render_equalizer(ui),
                                Page::Gestures => self.render_gestures(ui),
                                Page::Multipoint => self.render_multipoint(ui),
                                Page::Earbuds => self.render_earbuds(ui),
                                Page::Application => self.render_application(ui, ctx),
                            }
                            ui.add_space(8.0);
                        });
                });
            });

        self.render_confirm_modal(ctx);
    }
}
