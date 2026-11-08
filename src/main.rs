#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use earfun_studio::ui::{build_app_icon, run_headless_smoke_test, StudioApp};
use eframe::egui;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut smoke = false;
    let mut connect_last = false;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--smoke-test" => smoke = true,
            "--connect-last" => connect_last = true,
            "--help" | "-h" => {
                println!(
                    "EarFun Studio {} cho Windows & Ubuntu / TW600",
                    env!("CARGO_PKG_VERSION")
                );
                println!("  --smoke-test    Kiểm tra khởi tạo 6 trang giao diện, không truy cập Bluetooth");
                println!("  --connect-last  Kết nối thiết bị cuối sau khi mở cửa sổ");
                return Ok(());
            }
            _ => {}
        }
        i += 1;
    }

    if smoke && std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        run_headless_smoke_test();
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EarFun Studio · Air Pro 4")
            .with_inner_size([1380.0, 860.0])
            .with_min_inner_size([960.0, 660.0])
            .with_icon(build_app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "vn.earfun.Studio",
        options,
        Box::new(move |cc| Ok(Box::new(StudioApp::new(Some(cc), smoke, connect_last)))),
    )
}
