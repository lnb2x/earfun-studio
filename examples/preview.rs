//! Offline screenshots:
//! EFRAME_SCREENSHOT_TO=... cargo run --features ui-capture --example preview -- 1060 760 0 [--dark] [--connected]
use earfun_studio::backend::{BackendEvent, ConnectionPhase, ConnectionStatus};
use earfun_studio::protocol::{DeviceState, PhoneEntry};
use earfun_studio::settings::ThemeMode;
use earfun_studio::ui::{Page, StudioApp};
use eframe::egui;

fn main() -> eframe::Result<()> {
    let cfg_dir = std::env::temp_dir().join("earfun-preview-cfg");
    let _ = std::fs::remove_dir_all(&cfg_dir);
    std::env::set_var("APPDATA", &cfg_dir);
    std::env::set_var("XDG_CONFIG_HOME", &cfg_dir);

    let args: Vec<String> = std::env::args().skip(1).collect();
    let dark = args.iter().any(|a| a == "--dark");
    let connected = args.iter().any(|a| a == "--connected");
    let values: Vec<usize> = args.iter().filter_map(|s| s.parse().ok()).collect();
    let width = *values.first().unwrap_or(&1060) as f32;
    let height = *values.get(1).unwrap_or(&760) as f32;
    let page = *values.get(2).unwrap_or(&0);

    eframe::run_native(
        "EarFun Studio · Offline preview",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_app_id("vn.earfun.Studio")
                .with_decorations(true)
                .with_inner_size([width, height]),
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut app = StudioApp::new(Some(cc), true, false);
            app.set_theme_mode(
                &cc.egui_ctx,
                if dark {
                    ThemeMode::Dark
                } else {
                    ThemeMode::Light
                },
            );
            if connected {
                app.apply_backend_event(BackendEvent::Connection(ConnectionStatus::Connected));
                app.apply_backend_event(BackendEvent::Phase(ConnectionPhase::Ready));
                app.apply_backend_event(BackendEvent::State(DeviceState {
                    version: Some("TW600-3091_20250919_v0.4.1".to_string()),
                    bat_l: Some(92),
                    bat_r: Some(88),
                    bat_box: Some(74),
                    codec: Some("LDAC".to_string()),
                    noise: Some("anc".to_string()),
                    sub: Some("DEFAULT".to_string()),
                    amb: Some("TOTAL".to_string()),
                    game: Some(false),
                    ear: Some(true),
                    dual: Some(false),
                    ldac: Some(true),
                    aptx: Some(true),
                    adaptive: Some(true),
                    lossless: Some(false),
                    le_audio: Some(false),
                    google_fast_pair: Some(true),
                    prompt: Some(75),
                    mic: Some("AUTO".to_string()),
                    hearing_limit: Some(1),
                    balance: Some(50),
                    dis_l: Some(false),
                    dis_r: Some(false),
                    keys: [
                        ("L.Chạm một lần", "Giảm âm lượng"),
                        ("L.Chạm hai lần", "Phát / tạm dừng"),
                        ("L.Chạm ba lần", "Bài trước"),
                        ("L.Nhấn giữ", "Đổi chế độ nghe"),
                        ("R.Chạm một lần", "Tăng âm lượng"),
                        ("R.Chạm hai lần", "Phát / tạm dừng"),
                        ("R.Chạm ba lần", "Bài tiếp"),
                        ("R.Nhấn giữ", "Trợ lý giọng nói"),
                    ]
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
                    ..Default::default()
                }));
                app.apply_backend_event(BackendEvent::Phones(vec![
                    PhoneEntry {
                        num: 0,
                        name: "ThinkPad X1 Carbon".to_string(),
                        mac: "A4B1C2".to_string(),
                        connected: true,
                    },
                    PhoneEntry {
                        num: 1,
                        name: "Pixel 9 Pro".to_string(),
                        mac: "88E9F0".to_string(),
                        connected: false,
                    },
                ]));
                app.apply_backend_event(BackendEvent::Log("11:58:02 >> 00 0a 03 09".to_string()));
                app.apply_backend_event(BackendEvent::Log(
                    "11:58:02 << 00 0a 83 09 00 54 57 36 30 30 2d 33 30 39 31".to_string(),
                ));
                app.apply_backend_event(BackendEvent::Log(
                    "11:58:03 >> 00 0a 03 14 01".to_string(),
                ));
                app.set_eq(vec![3, 4, 0, 0, 0, 0, 2, 3, 3, 3], Some("earfun:6"));
            }
            app.page = Page::ALL[page.min(Page::ALL.len() - 1)];
            app.smoke_timer = std::time::Instant::now() + std::time::Duration::from_secs(3600);
            Ok(Box::new(app))
        }),
    )
}
