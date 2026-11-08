//! Offline screenshots: EFRAME_SCREENSHOT_TO=... cargo run --features ui-capture --example preview -- 960 660 0
use earfun_studio::ui::{Page, StudioApp};
use eframe::egui;

fn main() -> eframe::Result<()> {
    let values: Vec<usize> = std::env::args()
        .skip(1)
        .filter_map(|s| s.parse().ok())
        .collect();
    let width = *values.first().unwrap_or(&1380) as f32;
    let height = *values.get(1).unwrap_or(&860) as f32;
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
            app.page = Page::ALL[page.min(Page::ALL.len() - 1)];
            app.smoke_timer = std::time::Instant::now() + std::time::Duration::from_secs(3600);
            Ok(Box::new(app))
        }),
    )
}
