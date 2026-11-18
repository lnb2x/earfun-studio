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
fn test_dark_and_light_themes_render_all_pages_without_overflow() {
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
    let ctx = egui::Context::default();
    configure_fonts(&ctx);
    apply_theme(&ctx, crate::settings::ThemeMode::Light);

    // Run one minimal frame so egui initializes its font atlas
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(860.0, 620.0))),
            ..Default::default()
        },
        |_| {},
    );

    let sample_strings = [
        "Tổng quan Equalizer Cảm ứng Hai thiết bị Tai nghe Ứng dụng",
        "Đang chuẩn bị kết nối • Đã ghép đôi · Chờ kết nối — −6 dB +1.5 dB 1× 2× 3× …",
        "Áp dụng Khôi phục cài đặt gốc Giới hạn âm lượng bảo vệ thính giác",
    ];
    ctx.fonts(|fonts| {
        for text in sample_strings {
            for ch in text.chars() {
                if ch.is_whitespace() {
                    continue;
                }
                assert!(
                    fonts.has_glyph(&font_regular(13.0), ch),
                    "Font atlas missing glyph for {:?} (U+{:04X})",
                    ch,
                    ch as u32
                );
            }
        }
    });
}
