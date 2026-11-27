//! Unit and headless viewport tests for `earfun_studio::ui`.

use super::*;
use eframe::egui::{Color32, Key};

#[test]
fn test_headless_ui_renders_all_pages_and_compact_viewports() {
    let ctx = egui::Context::default();
    configure_fonts(&ctx);
    configure_style(&ctx);
    let mut app = StudioApp::new(None, true, false);

    app.connection = ConnectionStatus::Connected;
    app.state.version = Some("TW600-0.4.9".to_string());
    app.state.bat_l = Some(85);
    app.state.bat_r = Some(22);
    app.state.bat_box = Some(10);
    app.state.codec = Some("LDAC".to_string());
    app.state.noise = Some("adaptive".to_string());
    app.state.sub = Some("ADAPTIVE".to_string());
    app.state.amb = Some("TOTAL".to_string());
    app.state.game = Some(true);
    app.state.ear = Some(true);
    app.state.dual = Some(false);
    app.state.ldac = Some(true);
    app.state.aptx = Some(true);
    app.state.adaptive = Some(true);
    app.state.lossless = Some(false);
    app.state.le_audio = Some(false);
    app.state.prompt = Some(75);
    app.state.mic = Some("AUTO".to_string());
    app.state.hearing_limit = Some(1);
    app.state.balance = Some(255); // Exercise > 100 clamping safety
    let st_clone = app.state.clone();
    app.sync_state_to_drafts(&st_clone);
    assert_eq!(app.draft_balance, 100);

    app.state.dis_l = Some(true);
    app.state.dis_r = Some(false);
    app.find_until = Some(Instant::now() + Duration::from_secs(7));
    app.phones = vec![
        PhoneEntry {
            num: 0,
            name: "Laptop".to_string(),
            mac: "AABBCC".to_string(),
            connected: true,
        },
        PhoneEntry {
            num: 1,
            name: "Pixel".to_string(),
            mac: "112233".to_string(),
            connected: false,
        },
    ];
    app.logs.push_back("12:00:00 >> 00 0a 03 14 01".to_string());
    app.logs.push_back("12:00:01 ! Lỗi thử nghiệm".to_string());

    for size in [Vec2::new(860.0, 620.0), Vec2::new(1060.0, 760.0)] {
        for page in Page::ALL {
            app.page = page;
            let raw_input = egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                ..Default::default()
            };
            let _ = ctx.run(raw_input, |ctx| {
                paint_window_background(ctx);
                app.render_panels(ctx);
            });
            assert!(!ctx.style().visuals.dark_mode);
        }
    }
    app.backend.shutdown_blocking();
}

#[test]
fn summary_stays_on_one_row_at_fractional_display_sizes() {
    for width in [860.0, 960.0, 1060.0, 1280.0, 1380.0, 1706.6666] {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        configure_style(&ctx);
        let mut app = StudioApp::new(None, true, false);
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(width, 760.0))),
                ..Default::default()
            },
            |ctx| app.render_panels(ctx),
        );
        let cards: Vec<Rect> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::Shape::Rect(rect) = &shape.shape {
                    if rect.fill == Color32::WHITE && (rect.rect.height() - 106.0).abs() < 0.01 {
                        return Some(rect.rect);
                    }
                }
                None
            })
            .collect();
        assert_eq!(cards.len(), 4, "Four summary cards at width {width}");
        assert!(
            cards
                .iter()
                .all(|card| (card.top() - cards[0].top()).abs() < 0.01),
            "A battery card wrapped at display width {width}"
        );
        assert!(
            cards.iter().all(|card| card.right() <= width),
            "Summary overflow at display width {width}"
        );
        app.backend.shutdown_blocking();
    }
}

#[test]
fn test_modal_does_not_auto_confirm_on_same_frame_enter_press() {
    let ctx = egui::Context::default();
    configure_fonts(&ctx);
    configure_style(&ctx);
    let mut app = StudioApp::new(None, true, false);
    let confirmed_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag_clone = confirmed_flag.clone();

    app.ask_confirm("Xác nhận khôi phục?", move |_| {
        flag_clone.store(true, std::sync::atomic::Ordering::SeqCst);
    });

    let enter_event = egui::Event::Key {
        key: Key::Enter,
        physical_key: Some(Key::Enter),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let raw_frame1 = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1060.0, 760.0))),
        events: vec![enter_event.clone()],
        ..Default::default()
    };
    let _ = ctx.run(raw_frame1, |ctx| {
        app.render_panels(ctx);
    });
    assert!(
        !confirmed_flag.load(std::sync::atomic::Ordering::SeqCst),
        "Modal must ignore Enter on the frame it opens"
    );
    assert!(app.confirm_modal.is_some());

    let raw_frame2 = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1060.0, 760.0))),
        events: vec![enter_event],
        ..Default::default()
    };
    let _ = ctx.run(raw_frame2, |ctx| {
        app.render_panels(ctx);
    });
    assert!(confirmed_flag.load(std::sync::atomic::Ordering::SeqCst));
    assert!(app.confirm_modal.is_none());

    app.backend.shutdown_blocking();
}

#[test]
fn test_eq_sending_and_error_recovery_syncs_state_accurately() {
    let mut app = StudioApp::new(None, true, false);
    app.connection = ConnectionStatus::Connected;
    app.set_eq(vec![4, 5, 2, 0, 0, 1, 2, 3, 3, 2], Some("earfun:5"));

    assert!(app.has_unsent_eq());
    app.eq_sending_levels = Some(app.eq_levels.clone());
    app.draft_keys.insert("L.Chạm một lần".to_string(), 5);
    app.find_until = Some(Instant::now() + Duration::from_secs(9));

    app.apply_backend_event(BackendEvent::Error("Mất kết nối BLE".to_string()));
    assert!(app.eq_sending_levels.is_none());
    assert!(app.eq_sent_levels.is_none());
    assert!(app.find_until.is_none());
    assert_eq!(app.draft_keys.get("L.Chạm một lần"), Some(&0));
    assert!(app.notice_is_error);

    app.apply_backend_event(BackendEvent::Notice(
        "Đang phát tiếng tìm; tự dừng sau 10 giây.".to_string(),
    ));
    assert!(app.find_until.is_some());

    // Stale generation EqSent is ignored
    app.eq_sending_levels = Some(app.eq_levels.clone());
    app.apply_backend_event(BackendEvent::EqSent {
        generation: app.connection_generation + 99,
        levels: app.eq_levels.clone(),
    });
    assert!(app.has_unsent_eq());

    // Matching generation EqSent marks exact sent levels
    let cur_gen = app.connection_generation;
    let sent_lv = app.eq_levels.clone();
    app.apply_backend_event(BackendEvent::EqSent {
        generation: cur_gen,
        levels: sent_lv,
    });
    assert!(app.eq_sending_levels.is_none());
    assert!(!app.has_unsent_eq());

    app.apply_backend_event(BackendEvent::Connection(ConnectionStatus::Disconnected));
    assert!(app.eq_sent_levels.is_none());
    assert!(app.find_until.is_none());
    app.backend.shutdown_blocking();
}

#[test]
fn test_noise_group_utf8_mac_and_preset_category_filters() {
    assert_eq!(StudioApp::noise_group(Some("normal")), Some("normal"));
    assert_eq!(StudioApp::noise_group(Some("anc")), Some("anc"));
    assert_eq!(StudioApp::noise_group(Some("adaptive")), Some("anc"));
    assert_eq!(StudioApp::noise_group(Some("wind")), Some("anc"));
    assert_eq!(StudioApp::noise_group(Some("ambient")), Some("ambient"));
    assert_eq!(StudioApp::noise_group(None), None);

    assert_eq!(StudioApp::short_mac("Tai nghe Tiếng Việt"), "… Việt");
    assert_eq!(StudioApp::short_mac("AA:BB:CC:DD:EE:FF"), "…EE:FF");

    assert!(StudioApp::matches_category(
        "earfun:5",
        PresetCategory::EarFun
    ));
    assert!(!StudioApp::matches_category(
        "fernanda:201",
        PresetCategory::EarFun
    ));
    assert!(StudioApp::matches_category(
        "fernanda:201",
        PresetCategory::Fernanda
    ));
    assert!(StudioApp::matches_category(
        "custom:MyEQ",
        PresetCategory::Custom
    ));
    assert!(StudioApp::matches_category("draft", PresetCategory::Custom));
}

#[test]
fn test_dark_and_light_themes_render_all_pages_in_both_themes() {
    use crate::settings::ThemeMode;

    let ctx = egui::Context::default();
    configure_fonts(&ctx);
    let mut app = StudioApp::new(None, true, false);
    app.connection = ConnectionStatus::Connected;
    app.phase = crate::backend::ConnectionPhase::Ready;
    app.state.version = Some("TW600-tiếng-việt_20250919_v0.4.1".to_string());
    app.state.bat_l = Some(92);
    app.state.bat_r = Some(88);
    app.state.bat_box = Some(64);
    app.state.noise = Some("anc".to_string());
    app.state.sub = Some("DEFAULT".to_string());

    for (mode, expect_dark) in [(ThemeMode::Light, false), (ThemeMode::Dark, true)] {
        app.settings.data.theme = mode;
        apply_theme(&ctx, mode);
        for size in [Vec2::new(860.0, 620.0), Vec2::new(1060.0, 760.0)] {
            for page in Page::ALL {
                app.page = page;
                let raw_input = egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                    ..Default::default()
                };
                let _ = ctx.run(raw_input, |ctx| {
                    paint_window_background(ctx);
                    app.render_panels(ctx);
                });
                assert_eq!(ctx.style().visuals.dark_mode, expect_dark);
            }
        }
    }
    app.backend.shutdown_blocking();
}

#[test]
fn test_font_glyph_coverage_has_zero_tofu_characters() {
    const REGULAR_TTF: &[u8] = include_bytes!("../../assets/fonts/BeVietnamPro-Regular.ttf");
    let face = ttf_parser::Face::parse(REGULAR_TTF, 0).expect("Valid BeVietnamPro-Regular TTF");

    fn collect_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).expect("read src/ui dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if path.is_dir() {
                collect_rs_files(&path, out);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    let ui_dir = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui"));
    let mut files = Vec::new();
    collect_rs_files(ui_dir, &mut files);
    assert!(!files.is_empty());

    let mut checked_non_ascii = 0usize;
    for file in files {
        let content = std::fs::read_to_string(&file).expect("read ui source file");
        for (line_idx, line) in content.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for ch in line.chars() {
                if ch.is_ascii() || ch.is_whitespace() {
                    continue;
                }
                checked_non_ascii += 1;
                assert!(
                    face.glyph_index(ch).is_some(),
                    "BeVietnamPro-Regular missing glyph for {:?} (U+{:04X}) at {}:{}",
                    ch,
                    ch as u32,
                    file.display(),
                    line_idx + 1
                );
            }
        }
    }
    assert!(
        checked_non_ascii > 100,
        "Expected UI sources to contain Vietnamese glyphs"
    );
}

#[test]
fn shortcuts_work_with_non_text_focus_and_pause_in_text_edit() {
    let ctx = egui::Context::default();
    configure_fonts(&ctx);
    configure_style(&ctx);
    let mut app = StudioApp::new(None, true, false);
    assert_eq!(app.page, Page::ALL[0]);

    // Establish focus on an arbitrary non-text widget (e.g., an EQ fader or button)
    let non_text_id = egui::Id::new("non_text_widget_focus");
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1060.0, 760.0))),
            ..Default::default()
        },
        |ctx| {
            ctx.memory_mut(|m| m.request_focus(non_text_id));
            app.render_panels(ctx);
        },
    );
    assert!(ctx.wants_keyboard_input());

    let alt_2 = egui::Event::Key {
        key: Key::Num2,
        physical_key: Some(Key::Num2),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::ALT,
    };
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1060.0, 760.0))),
            modifiers: egui::Modifiers::ALT,
            events: vec![alt_2],
            ..Default::default()
        },
        |ctx| {
            app.render_panels(ctx);
        },
    );
    assert_eq!(
        app.page,
        Page::ALL[1],
        "Alt+2 must still switch pages when a non-text widget is focused"
    );

    // Focus a TextEdit and verify Alt+3 is suppressed while typing
    let text_id = egui::Id::new("focused_text_input");
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1060.0, 760.0))),
            ..Default::default()
        },
        |ctx| {
            egui::TextEdit::store_state(ctx, text_id, egui::text_edit::TextEditState::default());
            ctx.memory_mut(|m| m.request_focus(text_id));
            app.render_panels(ctx);
        },
    );
    assert_eq!(ctx.memory(|m| m.focused()), Some(text_id));
    assert!(egui::TextEdit::load_state(&ctx, text_id).is_some());

    let alt_3 = egui::Event::Key {
        key: Key::Num3,
        physical_key: Some(Key::Num3),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::ALT,
    };
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1060.0, 760.0))),
            modifiers: egui::Modifiers::ALT,
            events: vec![alt_3],
            ..Default::default()
        },
        |ctx| {
            app.render_panels(ctx);
        },
    );
    assert_eq!(
        app.page,
        Page::ALL[1],
        "Alt+3 must be ignored while a TextEdit has focus"
    );

    app.backend.shutdown_blocking();
}

#[test]
fn reconnect_backoff_doubles_to_cap_and_resets_on_disconnect_event() {
    assert_eq!(
        next_reconnect_backoff(Duration::from_secs(8)),
        Duration::from_secs(16)
    );
    assert_eq!(
        next_reconnect_backoff(Duration::from_secs(16)),
        Duration::from_secs(32)
    );
    assert_eq!(
        next_reconnect_backoff(Duration::from_secs(32)),
        Duration::from_secs(60)
    );
    assert_eq!(
        next_reconnect_backoff(Duration::from_secs(60)),
        Duration::from_secs(60)
    );

    assert!(!reconnect_due(
        Duration::from_secs(7),
        Duration::from_secs(8),
        0
    ));
    assert!(reconnect_due(
        Duration::from_secs(8),
        Duration::from_secs(8),
        0
    ));
    assert!(!reconnect_due(
        Duration::from_secs(20),
        Duration::from_secs(8),
        1
    ));

    let mut app = StudioApp::new(None, true, false);
    app.last_reconnect_check = Instant::now() - Duration::from_secs(30);
    app.auto_attempt_in_flight = true;
    app.apply_backend_event(BackendEvent::Connection(ConnectionStatus::Disconnected));
    assert!(app.last_reconnect_check.elapsed() < Duration::from_secs(2));
    assert!(!app.auto_attempt_in_flight);
    assert_eq!(app.reconnect_backoff, Duration::from_secs(16));

    app.apply_backend_event(BackendEvent::Connection(ConnectionStatus::Connected));
    assert_eq!(app.reconnect_backoff, Duration::from_secs(8));
    app.backend.shutdown_blocking();
}

#[test]
fn test_format_fw_short_and_freq_unit_labels() {
    assert_eq!(
        StudioApp::format_fw_short("TW600-3091_20250919_v0.4.1"),
        "FW TW600 · v0.4.1"
    );
    assert_eq!(StudioApp::format_fw_short("TW600-0.4.9"), "FW TW600-0.4.9");
    assert_eq!(
        StudioApp::format_fw_short("VERYLONGFIRMWARESTRING123456"),
        "FW …RESTRING123456"
    );

    assert_eq!(StudioApp::format_freq_unit(31.5), "31.5 Hz");
    assert_eq!(StudioApp::format_freq_unit(63.0), "63 Hz");
    assert_eq!(StudioApp::format_freq_unit(1000.0), "1 kHz");
    assert_eq!(StudioApp::format_freq_unit(16000.0), "16 kHz");
}

#[test]
fn test_toggle_switch_supports_keyboard_space_and_enter() {
    let ctx = egui::Context::default();
    configure_fonts(&ctx);
    configure_style(&ctx);

    let mut state = false;
    let mut sw_id = None;

    // First frame: render switch and capture its persistent id
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0))),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                sw_id = Some(ui.make_persistent_id("kb_test_switch"));
                crate::ui::widgets::toggle_switch(ui, "kb_test_switch", &mut state, true);
            });
        },
    );
    let id = sw_id.expect("switch id");

    // Second frame: focus switch and press Space -> turns on
    let space_ev = egui::Event::Key {
        key: Key::Space,
        physical_key: Some(Key::Space),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let mut changed = false;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0))),
            events: vec![space_ev],
            ..Default::default()
        },
        |ctx| {
            ctx.memory_mut(|m| m.request_focus(id));
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = crate::ui::widgets::toggle_switch(ui, "kb_test_switch", &mut state, true);
            });
        },
    );
    assert!(changed);
    assert!(state);

    // Third frame: press Enter -> turns off
    let enter_ev = egui::Event::Key {
        key: Key::Enter,
        physical_key: Some(Key::Enter),
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(400.0, 200.0))),
            events: vec![enter_ev],
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                changed = crate::ui::widgets::toggle_switch(ui, "kb_test_switch", &mut state, true);
            });
        },
    );
    assert!(changed);
    assert!(!state);
}
