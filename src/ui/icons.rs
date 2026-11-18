//! Crisp custom vector icons and anti-aliased app icon without relying on OS emoji fonts.

use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Overview,
    Eq,
    Touch,
    Devices,
    Settings,
    App,
    Earbud,
    Case,
    Normal,
    Anc,
    Ambient,
    Check,
    Refresh,
    Bluetooth,
    Game,
    Volume,
    Ear,
    Search,
    Alert,
    Info,
    Lock,
    SoundWave,
    Sun,
    Moon,
}

pub fn build_app_icon() -> egui::IconData {
    let size = 64usize;
    let mut rgba = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 - 31.5;
            let fy = y as f32 - 31.5;
            let dx = (fx.abs() - 18.0).max(0.0);
            let dy = (fy.abs() - 18.0).max(0.0);
            let sdf = (dx * dx + dy * dy).sqrt() - 12.0;

            if sdf < 0.8 {
                let alpha = (0.8 - sdf).clamp(0.0, 1.0);
                // Gradient from #5878c4 top-left to #4864ae bottom-right
                let t = ((x + y) as f32) / (2.0 * size as f32);
                let mut r = (92.0 * (1.0 - t) + 70.0 * t) as u8;
                let mut g = (122.0 * (1.0 - t) + 98.0 * t) as u8;
                let mut b = (200.0 * (1.0 - t) + 174.0 * t) as u8;

                // Crisp white EQ bars with rounded caps
                let bar1 = (18..=22).contains(&x) && (21..=43).contains(&y);
                let bar2 = (30..=34).contains(&x) && (15..=49).contains(&y);
                let bar3 = (42..=46).contains(&x) && (23..=41).contains(&y);
                if bar1 || bar2 || bar3 {
                    r = 255;
                    g = 255;
                    b = 255;
                }

                let idx = (y * size + x) * 4;
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = (alpha * 255.0) as u8;
            }
        }
    }
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}

/// Draw crisp vector icons matching `assets/icons/*.svg`.
pub fn draw_vector_icon(painter: &egui::Painter, rect: Rect, kind: IconKind, color: Color32) {
    let s = (rect.width().min(rect.height())) / 24.0;
    let stroke = Stroke::new((1.65 * s).clamp(1.35, 2.1), color);
    let map = |x: f32, y: f32| -> Pos2 {
        Pos2::new(
            rect.left() + (x / 24.0) * rect.width(),
            rect.top() + (y / 24.0) * rect.height(),
        )
    };

    match kind {
        IconKind::Overview => {
            for (x0, y0, x1, y1) in [
                (3.0, 3.0, 10.0, 10.0),
                (14.0, 3.0, 21.0, 10.0),
                (3.0, 14.0, 10.0, 21.0),
                (14.0, 14.0, 21.0, 21.0),
            ] {
                let r = Rect::from_min_max(map(x0, y0), map(x1, y1));
                painter.rect_stroke(r, Rounding::same(1.8 * s), stroke);
            }
        }
        IconKind::Eq => {
            painter.line_segment([map(5.0, 3.0), map(5.0, 9.0)], stroke);
            painter.line_segment([map(5.0, 13.0), map(5.0, 21.0)], stroke);
            painter.rect_stroke(
                Rect::from_min_max(map(2.5, 9.0), map(7.5, 13.0)),
                Rounding::same(1.2 * s),
                stroke,
            );

            painter.line_segment([map(12.0, 3.0), map(12.0, 14.0)], stroke);
            painter.line_segment([map(12.0, 18.0), map(12.0, 21.0)], stroke);
            painter.rect_stroke(
                Rect::from_min_max(map(9.5, 14.0), map(14.5, 18.0)),
                Rounding::same(1.2 * s),
                stroke,
            );

            painter.line_segment([map(19.0, 3.0), map(19.0, 6.0)], stroke);
            painter.line_segment([map(19.0, 10.0), map(19.0, 21.0)], stroke);
            painter.rect_stroke(
                Rect::from_min_max(map(16.5, 6.0), map(21.5, 10.0)),
                Rounding::same(1.2 * s),
                stroke,
            );
        }
        IconKind::Touch => {
            let center = map(11.0, 7.5);
            painter.circle_stroke(center, 3.2 * s, stroke);
            painter.line_segment([map(11.0, 8.0), map(11.0, 19.0)], stroke);
            painter.line_segment([map(11.0, 13.0), map(16.5, 14.5)], stroke);
            painter.line_segment([map(16.5, 14.5), map(16.0, 20.5)], stroke);
            painter.line_segment([map(16.0, 20.5), map(9.0, 20.5)], stroke);
            painter.line_segment([map(9.0, 20.5), map(6.0, 16.0)], stroke);
        }
        IconKind::Devices => {
            let mon = Rect::from_min_max(map(3.0, 4.0), map(15.0, 15.5));
            painter.rect_stroke(mon, Rounding::same(1.5 * s), stroke);
            painter.line_segment([map(9.0, 15.5), map(9.0, 19.5)], stroke);
            painter.line_segment([map(6.5, 19.5), map(11.5, 19.5)], stroke);

            let phone = Rect::from_min_max(map(16.5, 9.0), map(21.5, 21.0));
            painter.rect_stroke(phone, Rounding::same(1.5 * s), stroke);
            painter.line_segment([map(18.5, 18.8), map(19.5, 18.8)], stroke);
        }
        IconKind::Settings => {
            painter.line_segment([map(3.0, 5.0), map(7.0, 5.0)], stroke);
            painter.line_segment([map(11.0, 5.0), map(21.0, 5.0)], stroke);
            painter.line_segment([map(7.0, 2.5), map(7.0, 7.5)], stroke);

            painter.line_segment([map(3.0, 12.0), map(15.0, 12.0)], stroke);
            painter.line_segment([map(19.0, 12.0), map(21.0, 12.0)], stroke);
            painter.line_segment([map(15.0, 9.5), map(15.0, 14.5)], stroke);

            painter.line_segment([map(3.0, 19.0), map(9.0, 19.0)], stroke);
            painter.line_segment([map(13.0, 19.0), map(21.0, 19.0)], stroke);
            painter.line_segment([map(9.0, 16.5), map(9.0, 21.5)], stroke);
        }
        IconKind::App => {
            let win = Rect::from_min_max(map(3.0, 3.0), map(21.0, 21.0));
            painter.rect_stroke(win, Rounding::same(2.2 * s), stroke);
            painter.line_segment([map(3.0, 8.0), map(21.0, 8.0)], stroke);
            painter.line_segment([map(8.5, 8.0), map(8.5, 21.0)], stroke);
        }
        IconKind::Earbud => {
            painter.circle_stroke(map(12.5, 7.5), 4.6 * s, stroke);
            painter.circle_stroke(map(8.2, 8.2), 2.3 * s, stroke);
            let stem = Rect::from_min_max(map(10.0, 11.5), map(14.0, 21.5));
            painter.rect_stroke(stem, Rounding::same(2.0 * s), stroke);
        }
        IconKind::Case => {
            let r = Rect::from_min_max(map(2.5, 6.0), map(21.5, 20.5));
            painter.rect_stroke(r, Rounding::same(3.2 * s), stroke);
            painter.line_segment([map(2.5, 11.2), map(21.5, 11.2)], stroke);
            painter.line_segment([map(10.0, 15.8), map(14.0, 15.8)], stroke);
        }
        IconKind::Normal => {
            for y_base in [8.5_f32, 15.5_f32] {
                let mut prev = map(3.0, y_base);
                for step in 1..=24 {
                    let t = step as f32 / 24.0;
                    let x = 3.0 + t * 18.0;
                    let y = y_base - (t * std::f32::consts::TAU * 1.5).sin() * 2.5;
                    let cur = map(x, y);
                    painter.line_segment([prev, cur], stroke);
                    prev = cur;
                }
            }
        }
        IconKind::Anc => {
            let pts = [
                map(12.0, 2.5),
                map(20.5, 6.0),
                map(19.0, 15.0),
                map(12.0, 21.5),
                map(5.0, 15.0),
                map(3.5, 6.0),
                map(12.0, 2.5),
            ];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            let wave = [
                map(7.5, 12.0),
                map(9.8, 12.0),
                map(11.0, 8.2),
                map(13.0, 15.8),
                map(14.2, 12.0),
                map(16.5, 12.0),
            ];
            for w in wave.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        IconKind::Ambient => {
            painter.circle_stroke(map(12.0, 11.5), 4.8 * s, stroke);
            painter.circle_stroke(map(12.0, 11.5), 1.8 * s, stroke);
            painter.line_segment([map(10.0, 16.2), map(9.0, 20.5)], stroke);
            painter.line_segment([map(9.0, 20.5), map(12.5, 20.5)], stroke);
        }
        IconKind::Check => {
            painter.line_segment([map(4.5, 12.5), map(9.5, 17.5)], stroke);
            painter.line_segment([map(9.5, 17.5), map(20.0, 6.5)], stroke);
        }
        IconKind::Refresh => {
            // Open circular arrow
            let center = map(12.0, 12.0);
            let r = 7.2 * s;
            let mut prev = None;
            for step in 0..=20 {
                let a = -0.6 + (step as f32 / 20.0) * 5.0;
                let pt = Pos2::new(center.x + r * a.cos(), center.y + r * a.sin());
                if let Some(p0) = prev {
                    painter.line_segment([p0, pt], stroke);
                }
                prev = Some(pt);
            }
            painter.line_segment([map(15.2, 5.2), map(19.2, 5.5)], stroke);
            painter.line_segment([map(19.2, 5.5), map(18.8, 9.5)], stroke);
        }
        IconKind::Bluetooth => {
            let pts = [
                map(7.5, 6.5),
                map(17.0, 16.0),
                map(12.0, 21.0),
                map(12.0, 3.0),
                map(17.0, 8.0),
                map(7.5, 17.5),
            ];
            for w in pts.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
        }
        IconKind::Game => {
            // Gamepad outline + D-pad + buttons
            let body = Rect::from_min_max(map(2.5, 6.5), map(21.5, 17.5));
            painter.rect_stroke(body, Rounding::same(4.5 * s), stroke);
            painter.line_segment([map(8.0, 9.8), map(8.0, 14.2)], stroke);
            painter.line_segment([map(5.8, 12.0), map(10.2, 12.0)], stroke);
            painter.circle_filled(map(15.5, 10.8), 1.15 * s, color);
            painter.circle_filled(map(18.0, 13.2), 1.15 * s, color);
        }
        IconKind::Volume => {
            let poly = [
                map(3.5, 9.5),
                map(7.5, 9.5),
                map(12.5, 5.0),
                map(12.5, 19.0),
                map(7.5, 14.5),
                map(3.5, 14.5),
                map(3.5, 9.5),
            ];
            for w in poly.windows(2) {
                painter.line_segment([w[0], w[1]], stroke);
            }
            for (r_wave, span) in [(4.2_f32, 0.75_f32), (7.2_f32, 0.85_f32)] {
                let c = map(12.5, 12.0);
                let mut prev = None;
                for step in 0..=10 {
                    let a = -span + (step as f32 / 10.0) * (2.0 * span);
                    let pt = Pos2::new(c.x + r_wave * s * a.cos(), c.y + r_wave * s * a.sin());
                    if let Some(p0) = prev {
                        painter.line_segment([p0, pt], stroke);
                    }
                    prev = Some(pt);
                }
            }
        }
        IconKind::Ear => {
            // Ear contour + sensor dot
            let c = map(11.5, 10.5);
            let mut prev = None;
            for step in 0..=16 {
                let a = -2.4 + (step as f32 / 16.0) * 3.8;
                let pt = Pos2::new(c.x + 6.0 * s * a.cos(), c.y + 6.0 * s * a.sin());
                if let Some(p0) = prev {
                    painter.line_segment([p0, pt], stroke);
                }
                prev = Some(pt);
            }
            painter.line_segment([map(13.2, 16.0), map(10.5, 19.8)], stroke);
            painter.line_segment([map(10.5, 19.8), map(8.0, 18.8)], stroke);
            painter.circle_filled(map(11.5, 10.8), 1.6 * s, color);
        }
        IconKind::Search => {
            painter.circle_stroke(map(10.5, 10.5), 6.2 * s, stroke);
            painter.line_segment([map(15.2, 15.2), map(20.5, 20.5)], stroke);
        }
        IconKind::Alert => {
            painter.circle_stroke(map(12.0, 12.0), 8.5 * s, stroke);
            painter.line_segment([map(12.0, 7.5), map(12.0, 13.2)], stroke);
            painter.circle_filled(map(12.0, 16.5), 1.15 * s, color);
        }
        IconKind::Info => {
            painter.circle_stroke(map(12.0, 12.0), 8.5 * s, stroke);
            painter.circle_filled(map(12.0, 7.8), 1.15 * s, color);
            painter.line_segment([map(12.0, 11.0), map(12.0, 16.8)], stroke);
        }
        IconKind::Lock => {
            let body = Rect::from_min_max(map(5.0, 11.0), map(19.0, 20.5));
            painter.rect_stroke(body, Rounding::same(2.2 * s), stroke);
            let c = map(12.0, 9.0);
            let mut prev = None;
            for step in 0..=12 {
                let a = std::f32::consts::PI + (step as f32 / 12.0) * std::f32::consts::PI;
                let pt = Pos2::new(c.x + 4.0 * s * a.cos(), c.y + 4.0 * s * a.sin());
                if let Some(p0) = prev {
                    painter.line_segment([p0, pt], stroke);
                }
                prev = Some(pt);
            }
            painter.line_segment([map(8.0, 9.0), map(8.0, 11.0)], stroke);
            painter.line_segment([map(16.0, 9.0), map(16.0, 11.0)], stroke);
        }
        IconKind::SoundWave => {
            for (x, h) in [
                (4.5, 3.5),
                (8.2, 6.5),
                (12.0, 8.5),
                (15.8, 6.5),
                (19.5, 3.5),
            ] {
                painter.line_segment([map(x, 12.0 - h), map(x, 12.0 + h)], stroke);
            }
        }
        IconKind::Sun => {
            let c = map(12.0, 12.0);
            painter.circle_stroke(c, 4.2 * s, stroke);
            for ray in 0..8 {
                let a = (ray as f32) * (std::f32::consts::TAU / 8.0);
                let p0 = Pos2::new(c.x + 6.6 * s * a.cos(), c.y + 6.6 * s * a.sin());
                let p1 = Pos2::new(c.x + 9.2 * s * a.cos(), c.y + 9.2 * s * a.sin());
                painter.line_segment([p0, p1], stroke);
            }
        }
        IconKind::Moon => {
            let c = map(12.5, 12.0);
            let r = 7.4 * s;
            let mut prev = None;
            for step in 0..=18 {
                let a = 0.65 + (step as f32 / 18.0) * 4.4;
                let pt = Pos2::new(c.x + r * a.cos(), c.y + r * a.sin());
                if let Some(p0) = prev {
                    painter.line_segment([p0, pt], stroke);
                }
                prev = Some(pt);
            }
            let c2 = map(15.6, 9.2);
            let r2 = 6.0 * s;
            let mut prev2 = None;
            for step in 0..=14 {
                let a = 1.45 + (step as f32 / 14.0) * 2.55;
                let pt = Pos2::new(c2.x + r2 * a.cos(), c2.y + r2 * a.sin());
                if let Some(p0) = prev2 {
                    painter.line_segment([p0, pt], stroke);
                }
                prev2 = Some(pt);
            }
        }
    }
}

/// Draw a smooth animated spinner arc for active BLE scans / operations.
pub fn draw_spinner(ui: &mut egui::Ui, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(size), egui::Sense::hover());
    let t = ui.input(|i| i.time) as f32;
    let center = rect.center();
    let radius = size * 0.38;
    let start = t * 4.8;
    let sweep = 4.2_f32;
    let stroke = Stroke::new((size * 0.12).clamp(1.5, 2.4), color);
    let mut prev = None;
    for step in 0..=16 {
        let a = start + (step as f32 / 16.0) * sweep;
        let pt = Pos2::new(center.x + radius * a.cos(), center.y + radius * a.sin());
        if let Some(p0) = prev {
            ui.painter().line_segment([p0, pt], stroke);
        }
        prev = Some(pt);
    }
    ui.ctx().request_repaint();
}
