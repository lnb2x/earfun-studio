#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use earfun_studio::ui::{run_headless_smoke_test, StudioApp};
use eframe::egui;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut smoke = false;
    let mut connect_last = false;

    for arg in &args[1..] {
        match arg.as_str() {
            "--smoke-test" => smoke = true,
            "--connect-last" => connect_last = true,
            "--help" | "-h" => {
                println!("EarFun Studio 1.3 (Rust) cho Windows & Ubuntu / TW600");
                println!("  --smoke-test    Kiểm tra khởi tạo 6 trang giao diện, không truy cập Bluetooth");
                println!("  --connect-last  Kết nối thiết bị cuối sau khi mở cửa sổ");
                return Ok(());
            }
            _ => {}
        }
    }

    if smoke && std::env::var("DISPLAY").is_err() && std::env::var("WAYLAND_DISPLAY").is_err() {
        run_headless_smoke_test();
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("EarFun Studio · Air Pro 4")
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([920.0, 640.0]),
        ..Default::default()
    };

    eframe::run_native(
        "vn.earfun.Studio",
        options,
        Box::new(move |cc| Ok(Box::new(StudioApp::new(Some(cc), smoke, connect_last)))),
    )
}
