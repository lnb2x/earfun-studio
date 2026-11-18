#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use earfun_studio::ui::{build_app_icon, run_headless_smoke_test, StudioApp};
use eframe::egui;

fn has_linux_display() -> bool {
    let non_empty = |k: &str| {
        std::env::var(k)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    };
    non_empty("DISPLAY") || non_empty("WAYLAND_DISPLAY")
}

fn main() -> eframe::Result<()> {
    let mut smoke = false;
    let mut connect_last = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--smoke-test" => smoke = true,
            "--connect-last" => connect_last = true,
            "--help" | "-h" => {
                println!(
                    "EarFun Studio {} cho Windows & Ubuntu / TW600",
                    env!("CARGO_PKG_VERSION")
                );
                println!(
                    "  --smoke-test    Kiểm tra khởi tạo 6 trang giao diện, không truy cập Bluetooth"
                );
                println!("  --connect-last  Kết nối thiết bị cuối sau khi mở cửa sổ");
                return Ok(());
            }
            _ => {}
        }
    }

    let force_headless = std::env::var("EARFUN_HEADLESS").as_deref() == Ok("1");
    if smoke && (force_headless || (!cfg!(target_os = "windows") && !has_linux_display())) {
        run_headless_smoke_test();
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EarFun Studio · Air Pro 4")
            .with_app_id("vn.earfun.Studio")
            .with_decorations(true)
            .with_inner_size([1060.0, 760.0])
            .with_min_inner_size([860.0, 620.0])
            .with_icon(build_app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "vn.earfun.Studio",
        options,
        Box::new(move |cc| Ok(Box::new(StudioApp::new(Some(cc), smoke, connect_last)))),
    )
}
