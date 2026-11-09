//! DesktopWidgets rendering — background layer, big clock + date and the
//! 2×4 grid of discrete LED monitors (CPU, NETWORK, MEMORY, DISK I/O,
//! STORAGE, THERMAL, SYS TEMP, COOLING). All metrics are live from `Sampler`.

use chrono::{DateTime, Local};
use tiny_skia::{Color, PixmapMut};

use crate::sysinfo::Snapshot;
use crate::theme::Theme;
use crate::ui::draw::{self, alpha};
use crate::ui::text::{HAlign, Style, Text};

struct Part<'a> {
    text: String,
    family: &'a str,
    size: f32,
    color: Color,
}

pub fn draw(pm: &mut PixmapMut, theme: &Theme, text: &mut Text, snap: &Snapshot, now: &DateTime<Local>) {
    let x0 = (theme.bar_width + 120) as f32;
    let content_w = 36.0 + 12.0 + 165.0 + 44.0 + 36.0 + 12.0 + 165.0; // two tiles + gap = 470
    let grid_h = 4.0 * 36.0 + 3.0 * 20.0;
    let total_h = 88.0 + 4.0 + 31.2 + 12.0 + 26.4 + 12.0 + grid_h;
    let mut y = ((pm.height() as f32 - total_h) / 2.0).max(0.0);

    // --- Big clock (Cinzel 76, warm shadow handled in M5 polish) ---
    let clock_style = Style::new(theme.font_title, 76.0).bold();
    text.draw(pm, &now.format("%H:%M").to_string(), &clock_style, x0, y - 16.0, content_w, HAlign::Center, theme.fg);
    y += 88.0 + 4.0;

    // --- Date (Berkshire Swash) ---
    text.draw(pm, &now.format("%A, %-d %B").to_string(), &Style::new(theme.font_accent, 26.0), x0, y, content_w, HAlign::Center, theme.red);
    y += 31.2 + 12.0;

    // --- Bodoni ornament divider ---
    text.draw(pm, "t", &Style::new(theme.font_bodoni, 22.0), x0, y, content_w, HAlign::Center, alpha(theme.gold, 0.85));
    y += 26.4 + 12.0;

    let cpu_lit = (snap.cpu_percent as f32 / 100.0 * 24.0).round() as u32;
    let net_lit = (snap.net.dl_ratio.max(snap.net.ul_ratio) as f32 * 24.0).round() as u32;
    let disk_lit = (snap.disk.read_ratio.max(snap.disk.write_ratio) as f32 * 24.0).round() as u32;
    let storage_lit = (snap.storage_percent as f32 / 100.0 * 24.0).round() as u32;
    let temp1 = snap.temp1.unwrap_or(0);
    let temp2 = snap.temp2.unwrap_or(0);
    let fan_ratio = snap.fan1.map_or(0.0, |(_, r)| r);

    let tiles: [(&str, &str, Vec<Part>, u32); 8] = [
        ("󰘚", "PROCESSOR", vec![Part { text: format!("{}%", snap.cpu_percent), family: theme.font_widget, size: 14.0, color: theme.fg }], cpu_lit),
        ("󰛳", "NETWORK", vec![
            Part { text: "↓".into(), family: theme.font_widget, size: 11.0, color: theme.red },
            Part { text: snap.net.download_str.clone(), family: theme.font_widget, size: 13.0, color: theme.fg },
            Part { text: "↑".into(), family: theme.font_widget, size: 11.0, color: theme.red },
            Part { text: snap.net.upload_str.clone(), family: theme.font_widget, size: 13.0, color: theme.fg },
        ], net_lit),
        ("󰍛", "MEMORY", vec![Part { text: format!("{}%", (snap.ram_ratio * 100.0).round() as u32), family: theme.font_widget, size: 14.0, color: theme.fg }], snap.ram_blocks),
        ("󰆼", "DISK I/O", vec![
            Part { text: "R".into(), family: theme.font_title, size: 10.0, color: theme.red },
            Part { text: snap.disk.read_str.clone(), family: theme.font_widget, size: 13.0, color: theme.fg },
            Part { text: "W".into(), family: theme.font_title, size: 10.0, color: theme.red },
            Part { text: snap.disk.write_str.clone(), family: theme.font_widget, size: 13.0, color: theme.fg },
        ], disk_lit),
        ("󰋊", "STORAGE", vec![Part { text: format!("{}%", snap.storage_percent), family: theme.font_widget, size: 14.0, color: theme.fg }], storage_lit),
        ("󰔏", "THERMAL", vec![Part { text: format!("{temp1} °C"), family: theme.font_widget, size: 14.0, color: theme.fg }], (temp1 as f32 / 100.0 * 24.0).round() as u32),
        ("󰔏", "SYS TEMP", vec![Part { text: format!("{temp2} °C"), family: theme.font_widget, size: 14.0, color: theme.fg }], (temp2 as f32 / 100.0 * 24.0).round() as u32),
        ("󰈐", "COOLING", vec![Part { text: format!("{}%", (fan_ratio * 100.0).round() as u32), family: theme.font_widget, size: 14.0, color: theme.fg }], (fan_ratio as f32 * 24.0).round() as u32),
    ];

    let tile_w = 36.0 + 12.0 + 165.0;
    for (i, (icon, title, parts, lit)) in tiles.iter().enumerate() {
        let tx = x0 + (i % 2) as f32 * (tile_w + 44.0);
        let ty = y + (i / 2) as f32 * 56.0;
        tile(pm, theme, text, tx, ty, icon, title, parts, *lit);
    }
}

#[allow(clippy::too_many_arguments)]
fn tile(
    pm: &mut PixmapMut,
    theme: &Theme,
    text: &mut Text,
    x: f32,
    y: f32,
    icon: &str,
    title: &str,
    parts: &[Part],
    lit: u32,
) {
    // Medallion: dark circle with a thin gold rim + centered Nerd glyph.
    draw::circle(pm, x + 18.0, y + 18.0, 18.0, theme.fg);
    draw::border(pm, x + 0.75, y + 0.75, 34.5, 34.5, 17.25, 1.5, theme.gold);
    let icon_style = Style::new(theme.font_icons, 18.0);
    let (_, ih) = text.measure(icon, &icon_style);
    text.draw(pm, icon, &icon_style, x, y + (36.0 - ih) / 2.0, 36.0, HAlign::Center, theme.gold_bright);

    // Title + value segments.
    let mut cx = x + 48.0;
    let title_style = Style::new(theme.font_title, 11.0).bold();
    text.draw(pm, title, &title_style, cx, y, 200.0, HAlign::Left, theme.red);
    let (tw, _) = text.measure(title, &title_style);
    cx += tw + 8.0;
    for part in parts {
        let st = Style::new(part.family, part.size).bold();
        text.draw(pm, &part.text, &st, cx, y + (14.0 - part.size).max(0.0) * 0.5, 200.0, HAlign::Left, part.color);
        let (pw, _) = text.measure(&part.text, &st);
        cx += pw + 4.0;
    }

    // 24-block LED meter.
    draw::led_row(pm, x + 48.0, y + 20.0, 24, lit, 4.0, 9.0, 3.0, theme.tick_active, theme.tick_inactive, Some(theme.tick_active_border));
}
