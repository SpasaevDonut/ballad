//! LeftBar rendering + input geometry — port of `LeftBar.qml` and
//! `LeftBarWidgets/WorkspacePanel.qml`.
//!
//! Layout model (QML ColumnLayout): fixed top section (power, workspace
//! dial), a window list that takes the remaining space, and a bottom section
//! (network … clock) anchored above the 16 px bottom margin. Draw calls and
//! pointer hit-testing share [`geometry()`], so they can never disagree.

use chrono::{DateTime, Local};
use tiny_skia::{Color, FillRule, PathBuilder, PixmapMut, Transform};

use crate::niri::{self, Group, State as NiriState};
use crate::sysinfo::Snapshot;
use crate::theme::Theme;
use crate::ui::draw::{self, alpha};
use crate::ui::text::{HAlign, Style, Text};

const ROMAN: [&str; 10] = ["Ⅰ", "Ⅱ", "Ⅲ", "Ⅳ", "Ⅴ", "Ⅵ", "Ⅶ", "Ⅷ", "Ⅸ", "Ⅹ"];

const WS_TOP: f32 = 78.0; // 20 margin + 36 power + 18 separator + 4 grid margin
const WS_END: f32 = 208.0; // WS_TOP + 126 grid + 4 bottom margin
const SEP_H: f32 = 18.0;
/// Bottom section: sep + network(108) + sep + volume(92) + sep + battery(92)
/// + sep + updates(32) + sep + layout(24) + sep + clock(40) = 496.
const BOTTOM_H: f32 = 496.0;
/// Extra height of the updates widget when its count is visible, matching the
/// QML ColumnLayout (icon 20 + spacing 2 + count line + 4 px margins).
const UPDATES_COUNT_H: f32 = 12.0;

fn updates_height(updates_visible: bool) -> f32 {
    32.0 + if updates_visible { UPDATES_COUNT_H } else { 0.0 }
}
const LABEL_W: f32 = 10.0;

// ---------------------------------------------------------------------------
// Vertical anchor points (shared by popups)
// ---------------------------------------------------------------------------

/// Widget centers used to vertically anchor popups, matching the QML
/// `anchorItem` centres. Includes pointer-independent positions only.
#[derive(Debug, Clone, Copy)]
pub struct Anchors {
    pub bottom_y: f32,
    pub power_center: f32,
    pub net_center: f32,
    pub volume_center: f32,
    pub battery_center: f32,
    pub updates_center: f32,
    /// Top of the tray column (before its 4 px top margin); 0 when hidden.
    pub tray_top: f32,
    pub clock_center: f32,
    pub clock_bottom: f32,
}

/// Tray height, matching `TrayWidget.qml`: 4 px top + 4 px bottom margin,
/// 28 px per icon, 6 px spacing. Empty tray collapses to zero.
pub fn tray_height(count: usize) -> f32 {
    if count == 0 {
        return 0.0;
    }
    let n = count as f32;
    8.0 + n * 28.0 + (n - 1.0) * 6.0
}

/// Total bottom-section height including the optional tray block and its
/// trailing separator (the one above the keyboard-layout badge).
fn bottom_height(tray_count: usize, updates_visible: bool) -> f32 {
    let th = tray_height(tray_count);
    BOTTOM_H - 32.0
        + updates_height(updates_visible)
        + if th > 0.0 { th + SEP_H } else { 0.0 }
}

pub fn anchors(height: f32, tray_count: usize, updates_visible: bool) -> Anchors {
    let bottom_y = (height - 16.0 - bottom_height(tray_count, updates_visible)).max(WS_END);
    let net_y = bottom_y + SEP_H;
    let vol_y = net_y + 108.0 + SEP_H;
    let batt_y = vol_y + 92.0 + SEP_H;
    let upd_y = batt_y + 92.0 + SEP_H;
    let tray_top = upd_y + updates_height(updates_visible) + SEP_H;
    let after_tray = tray_top + if tray_count > 0 { tray_height(tray_count) + SEP_H } else { 0.0 };
    let clock_y = after_tray + 24.0 + SEP_H;
    Anchors {
        bottom_y,
        power_center: 20.0 + 18.0,
        net_center: net_y + 54.0,
        volume_center: vol_y + 46.0,
        battery_center: batt_y + 46.0,
        updates_center: upd_y + 16.0,
        tray_top,
        clock_center: clock_y + 20.0,
        clock_bottom: clock_y + 40.0,
    }
}

/// Live data fed to the bar beyond the system snapshot.
#[derive(Debug, Clone, Copy)]
pub struct BarData {
    pub audio: crate::audio::AudioState,
    pub updates: usize,
    pub wifi_enabled: bool,
    pub wifi_connected: bool,
    pub wifi_signal: u8,
    /// Hovered widget (drives QML hover colours/cursor).
    pub hover: Option<WidgetHit>,
}

impl Default for BarData {
    fn default() -> Self {
        Self {
            audio: crate::audio::AudioState::default(),
            updates: 0,
            wifi_enabled: true,
            wifi_connected: false,
            wifi_signal: 0,
            hover: None,
        }
    }
}

/// QML `updateWifiIcon()`.
fn wifi_icon(d: &BarData) -> &'static str {
    if !d.wifi_enabled {
        "󰤮"
    } else if !d.wifi_connected {
        "󰤭"
    } else if d.wifi_signal >= 75 {
        "󰤨"
    } else if d.wifi_signal >= 50 {
        "󰤥"
    } else if d.wifi_signal >= 25 {
        "󰤢"
    } else {
        "󰤟"
    }
}

/// Hit regions for bar widgets beyond the workspace dial / window list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WidgetHit {
    Power,
    Network,
    Volume,
    Battery,
    Updates,
    Tray(usize),
    Clock,
}

pub fn widget_hit(
    height: f32,
    y: f32,
    tray_count: usize,
    updates_visible: bool,
) -> Option<WidgetHit> {
    let a = anchors(height, tray_count, updates_visible);
    let half = updates_height(updates_visible) / 2.0;
    let spans = [
        (WidgetHit::Power, 20.0, 56.0),
        (WidgetHit::Network, a.net_center - 54.0, a.net_center + 54.0),
        (WidgetHit::Volume, a.volume_center - 46.0, a.volume_center + 46.0),
        (WidgetHit::Battery, a.battery_center - 46.0, a.battery_center + 46.0),
        (WidgetHit::Updates, a.updates_center - half, a.updates_center + half),
        (WidgetHit::Clock, a.clock_center - 20.0, a.clock_center + 20.0),
    ];
    if let Some(hit) = spans
        .iter()
        .find(|(_, top, bottom)| y >= *top && y <= *bottom)
        .map(|(hit, _, _)| *hit)
    {
        return Some(hit);
    }
    // Tray medallions: 28 px rows with 6 px spacing after the 4 px margin.
    for i in 0..tray_count {
        let top = a.tray_top + 4.0 + i as f32 * 34.0;
        if y >= top && y <= top + 28.0 {
            return Some(WidgetHit::Tray(i));
        }
    }
    None
}


// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub struct GroupGeom {
    pub rect: (f32, f32, f32, f32),
    /// Diamond centers, one per window (same order as `Group::windows`).
    pub dots: Vec<(f32, f32)>,
    pub target: u64,
}

#[derive(Debug)]
pub struct Geom {
    pub win_top: f32,
    pub bottom_y: f32,
    pub groups: Vec<GroupGeom>,
}

pub fn geometry(
    height: f32,
    groups: &[Group],
    tray_count: usize,
    updates_visible: bool,
) -> Geom {
    let win_top = if groups.is_empty() { WS_END } else { WS_END + SEP_H };
    let bottom_y = (height - 16.0 - bottom_height(tray_count, updates_visible)).max(win_top);
    let mut out = Vec::new();
    let mut y = win_top;
    for g in groups {
        let label_h = g.name.chars().count() as f32 * 9.5;
        let n = g.windows.len() as f32;
        let dots_h = n * 9.0 - 5.0;
        let row_h = label_h.max(dots_h);
        let group_h = row_h + 16.0;
        if y + group_h > bottom_y {
            break; // ListView clips at the bottom section.
        }
        let row_x = (50.0 - (LABEL_W + 20.0)) / 2.0;
        let dot_x = row_x + LABEL_W + 18.0;
        let dots = (0..g.windows.len())
            .map(|i| (dot_x, y + 8.0 + (row_h - dots_h) / 2.0 + i as f32 * 9.0 + 2.0))
            .collect();
        out.push(GroupGeom {
            rect: (0.0, y, 50.0, group_h),
            dots,
            target: g.target_window().unwrap_or(0),
        });
        y += group_h + 8.0;
    }
    Geom { win_top, bottom_y, groups: out }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Workspace(usize),
    Window(u64),
    WindowDot { group: usize, dot: usize },
}

pub fn hit_test(geom: &Geom, x: f32, y: f32) -> Option<Hit> {
    // Workspace dial slots.
    if (78.0..208.0).contains(&y) {
        for i in 0..10 {
            let sx = 6.0 + (i % 2) as f32 * 21.0;
            let sy = WS_TOP + (i / 2) as f32 * 26.0;
            if x >= sx && x <= sx + 17.0 && y >= sy && y <= sy + 22.0 {
                return Some(Hit::Workspace(i));
            }
        }
    }
    // Window groups: individual diamonds first, then the whole group row.
    for (gi, gg) in geom.groups.iter().enumerate() {
        let (_, gy, _, gh) = gg.rect;
        if y < gy || y > gy + gh {
            continue;
        }
        for (di, &(dx, dy)) in gg.dots.iter().enumerate() {
            if (x - dx).abs() <= 6.0 && (y - dy).abs() <= 6.0 {
                return Some(Hit::WindowDot { group: gi, dot: di });
            }
        }
        return Some(Hit::Window(gg.target));
    }
    None
}

/// Resolves a `Hit` to a concrete window id.
pub fn resolve_window(hit: Hit, groups: &[Group]) -> Option<u64> {
    match hit {
        Hit::WindowDot { group, dot } => groups.get(group)?.windows.get(dot).map(|w| w.id),
        Hit::Window(id) => Some(id),
        Hit::Workspace(_) => None,
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
pub fn draw(
    pm: &mut PixmapMut,
    theme: &Theme,
    text: &mut Text,
    snap: &Snapshot,
    niri_state: &NiriState,
    data: &BarData,
    tray: &[crate::tray::TrayItem],
    tray_hover: Option<usize>,
    now: &DateTime<Local>,
) {
    let bw = theme.bar_width as f32;
    let h = pm.height() as f32;
    let slots = niri::slots(&niri_state.workspaces, niri_state.active_workspace);
    let groups = niri::groups(&niri_state.windows, niri_state.active_workspace);
    let geom = geometry(h, &groups, tray.len(), data.updates > 0);

    // --- Background, bookbinding border, vignettes ---
    draw::rect(pm, 0.0, 0.0, bw, h, theme.bg0);
    draw::rect(pm, bw - 2.0, 0.0, 1.0, h, alpha(theme.gold, 0.45));
    draw::rect(pm, bw - 1.0, 0.0, 1.0, h, alpha(theme.bg2, 0.8));
    draw::v_gradient(pm, 0.0, 0.0, bw, 70.0, theme.bar_vignette, Color::TRANSPARENT);
    draw::v_gradient(pm, 0.0, h - 70.0, bw, 70.0, Color::TRANSPARENT, theme.bar_vignette);

    // --- Victorian pediment / tailpiece ---
    text.draw(pm, "7", &Style::new(theme.font_victorian, 15.0), 0.0, 2.0, bw, HAlign::Center, alpha(theme.gold, 0.75));
    text.draw(pm, "A", &Style::new(theme.font_victorian, 13.0), 0.0, h - 18.0, bw, HAlign::Center, alpha(theme.gold, 0.75));

    // --- Right-hand drop shadow gradient ---
    draw::h_gradient(pm, bw, 0.0, theme.bar_shadow as f32, h, theme.shadow, Color::TRANSPARENT);

    // --- PowerButton rosette (QML swaps ")" for "*" and turns red on hover) ---
    let power_hover = data.hover == Some(WidgetHit::Power);
    let count = if power_hover { "*" } else { ")" };
    let rosette = Style::new(theme.font_bodoni, if power_hover { 33.0 } else { 31.0 });
    let (_, rh) = text.measure(count, &rosette);
    text.draw(
        pm,
        count,
        &rosette,
        0.0,
        20.0 + (36.0 - rh) / 2.0,
        bw,
        HAlign::Center,
        if power_hover { theme.red } else { theme.gold },
    );
    separator(pm, theme, text, 56.0, bw);

    // --- Workspace dial ---
    for (i, slot) in slots.iter().enumerate() {
        let cx = 6.0 + (i % 2) as f32 * 21.0;
        let cy = WS_TOP + (i / 2) as f32 * 26.0;
        let (bg, border, fg) = if slot.focused {
            (Some(alpha(theme.gold_bright, 0.19)), Some(theme.gold), theme.gold)
        } else if slot.exists {
            (Some(alpha(theme.gold, 0.08)), Some(alpha(theme.bg2, 0.5)), theme.fg)
        } else {
            (None, None, theme.bg2)
        };
        if let Some(bg) = bg {
            draw::rounded_rect(pm, cx, cy, 17.0, 22.0, 3.0, bg);
        }
        if let Some(bc) = border {
            draw::border(pm, cx, cy, 17.0, 22.0, 3.0, 1.0, bc);
        }
        // Cinzel Decorative lacks U+2160 and cosmic-text's fallback is a
        // heavier serif than Qt's; EB Garamond has the numeral glyphs and
        // matches the QML fallback closely.
        let st = Style::new(theme.font_widget, 11.0);
        let st = if slot.focused { st.bold() } else { st };
        let (_, nh) = text.measure(ROMAN[i], &st);
        text.draw(pm, ROMAN[i], &st, cx, cy + (22.0 - nh) / 2.0, 17.0, HAlign::Center, fg);
        if slot.exists && !slot.focused {
            draw::rounded_rect(pm, cx + 7.0, cy + 22.0 - 4.0, 3.0, 3.0, 1.5, theme.red_dim);
        }
    }

    // --- Window groups ---
    if !groups.is_empty() {
        separator(pm, theme, text, WS_END, bw);
    }
    for (g, gg) in groups.iter().zip(geom.groups.iter()) {
        let (_, gy, _, _) = gg.rect;
        let item_color = if g.focused { theme.gold } else { theme.fg };
        let label_h = g.name.chars().count() as f32 * 9.5;
        let n = g.windows.len() as f32;
        let dots_h = n * 9.0 - 5.0;
        let row_h = label_h.max(dots_h);
        let row_x = (50.0 - (LABEL_W + 20.0)) / 2.0;
        let content_y = gy + 8.0;

        // Bracket (1 px caps + 3 px spine).
        let by = content_y + (row_h - label_h) / 2.0 - 2.0;
        draw::rect(pm, row_x, by, 4.0, 1.0, item_color);
        draw::rect(pm, row_x, by, 3.0, label_h + 4.0, item_color);
        draw::rect(pm, row_x, by + label_h + 3.0, 4.0, 1.0, item_color);

        // Vertical app label.
        let label = g.name.chars().map(|c| c.to_string()).collect::<Vec<_>>().join("\n");
        let st = Style::new(theme.font_title, 10.0).bold().line_height(9.5);
        text.draw(pm, &label, &st, row_x + 10.0, content_y + (row_h - label_h) / 2.0, LABEL_W, HAlign::Center, item_color);

        // Diamonds, one per window.
        for (w, &(dx, dy)) in g.windows.iter().zip(gg.dots.iter()) {
            diamond(pm, dx, dy, 2.83, if w.is_focused { theme.gold } else { theme.red_dim });
        }
    }

    // --- Bottom section (anchored above the 16 px bottom margin) ---
    let mut y = geom.bottom_y;
    y += separator(pm, theme, text, y, bw);

    // Network.
    y += 2.0;
    let net_icon = wifi_icon(data);
    let icon_color = if !data.wifi_enabled {
        theme.bg2
    } else if data.wifi_connected {
        theme.gold
    } else {
        theme.bg2
    };
    text.draw(pm, net_icon, &Style::new(theme.font_icons, 18.0), 0.0, y, bw, HAlign::Center, icon_color);
    let dl_lit = (snap.net.dl_ratio * 8.0).round() as u32;
    let ul_lit = (snap.net.ul_ratio * 8.0).round() as u32;
    draw::led_col(pm, (bw - 26.0) / 2.0, y + 24.0, 8, dl_lit, 10.0, 3.0, 2.0, theme.gold, theme.bg2);
    draw::led_col(pm, (bw - 26.0) / 2.0 + 16.0, y + 24.0, 8, ul_lit, 10.0, 3.0, 2.0, theme.fg, theme.bg2);
    let speed = Style::new(theme.font_widget, 11.0);
    text.draw(pm, &snap.net.download_str, &speed, 0.0, y + 66.0, bw, HAlign::Center, theme.gold);
    text.draw(pm, &snap.net.upload_str, &speed, 0.0, y + 79.0, bw, HAlign::Center, theme.fg);
    y += 100.0 + 6.0;

    y += separator(pm, theme, text, y, bw);

    // Volume (live via wpctl poll).
    y += 6.0;
    let muted = data.audio.muted;
    let vol_icon = crate::audio::icon(data.audio);
    text.draw(
        pm,
        vol_icon,
        &Style::new(theme.font_icons, 18.0),
        0.0,
        y,
        bw,
        HAlign::Center,
        if muted { theme.bg2 } else { theme.gold },
    );
    let vol_lit = if muted {
        0
    } else {
        (data.audio.volume * 8.0).round() as u32
    };
    draw::led_col(pm, (bw - 14.0) / 2.0, y + 24.0, 8, vol_lit, 14.0, 3.0, 2.0, theme.gold, theme.bg2);
    let vol_pct = format!("{}%", (data.audio.volume * 100.0).round() as u32);
    text.draw(
        pm,
        &vol_pct,
        &Style::new(theme.font_widget, 12.0).bold(),
        0.0,
        y + 66.0,
        bw,
        HAlign::Center,
        if muted { theme.bg2 } else { theme.fg },
    );
    y += 80.0 + 6.0;

    y += separator(pm, theme, text, y, bw);

    // Battery.
    y += 6.0;
    let charging = snap.battery.status == "Charging";
    let low = snap.battery.percent <= 20.0;
    let batt_color = if charging { theme.gold_bright } else if low { theme.red } else { theme.gold };
    let batt_text_color = if charging { theme.gold_bright } else if low { theme.red } else { theme.fg };
    text.draw(pm, if charging { "⚡" } else { "󰁹" }, &Style::new(theme.font_icons, 18.0), 0.0, y, bw, HAlign::Center, theme.gold);
    let lit = (snap.battery.percent / 100.0 * 8.0).round() as u32;
    draw::led_col(pm, (bw - 14.0) / 2.0, y + 24.0, 8, lit, 14.0, 3.0, 2.0, batt_color, theme.bg2);
    let pct = format!("{}%", snap.battery.percent.round() as u32);
    text.draw(pm, &pct, &Style::new(theme.font_widget, 12.0).bold(), 0.0, y + 66.0, bw, HAlign::Center, batt_text_color);
    y += 80.0 + 6.0;

    y += separator(pm, theme, text, y, bw);

    // Updates (count hidden while zero).
    y += 4.0;
    let updates_hover = data.hover == Some(WidgetHit::Updates);
    text.draw(
        pm,
        "󰮯",
        &Style::new(theme.font_icons, 18.0),
        0.0,
        y,
        bw,
        HAlign::Center,
        if updates_hover { theme.red } else { theme.gold },
    );
    if data.updates > 0 {
        let label = if data.updates > 999 {
            "999+".to_string()
        } else {
            data.updates.to_string()
        };
        text.draw(
            pm,
            &label,
            &Style::new(theme.font_widget, 11.0).bold(),
            0.0,
            y + 24.0,
            bw,
            HAlign::Center,
            if updates_hover { theme.red } else { theme.fg },
        );
    }
    y += 24.0 + 4.0;
    if data.updates > 0 {
        y += UPDATES_COUNT_H;
    }

    y += separator(pm, theme, text, y, bw);

    // --- Tray (StatusNotifier medallions) ---
    if !tray.is_empty() {
        let cx = bw / 2.0;
        for (i, item) in tray.iter().enumerate() {
            let top = y + 4.0 + i as f32 * 34.0;
            let hovered = tray_hover == Some(i);
            // Warm engraved shadow, 1 px below the plate.
            let r = if hovered { 14.0 } else { 13.0 };
            draw::circle(pm, cx, top + 15.0, r, Color::from_rgba8(0x3a, 0x2a, 0x20, 53));
            // Victorian medallion plate (1.08x on hover, like QML).
            draw::rounded_rect(
                pm,
                cx - r,
                top + 14.0 - r,
                r * 2.0,
                r * 2.0,
                r,
                if hovered { theme.red } else { theme.fg },
            );
            draw::border(
                pm,
                cx - r,
                top + 14.0 - r,
                r * 2.0,
                r * 2.0,
                r,
                1.0,
                if hovered { theme.gold_bright } else { theme.gold },
            );
            if let Some(icon) = &item.icon {
                let x = (cx - icon.width() as f32 / 2.0).round() as i32;
                let iy = (top + 14.0 - icon.height() as f32 / 2.0).round() as i32;
                pm.draw_pixmap(
                    x,
                    iy,
                    tiny_skia::Pixmap::as_ref(&**icon),
                    &tiny_skia::PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
            }
        }
        y += tray_height(tray.len());
        y += separator(pm, theme, text, y, bw);
    }

    // Keyboard layout badge.
    y += 3.0;
    let badge = niri::layout_badge(&niri_state.layouts);
    // QML LayoutWidget: bold Cinzel in textColor (fg), not gold.
    text.draw(
        pm,
        &badge,
        &Style::new(theme.font_title, 11.0).black(),
        0.0,
        y,
        bw,
        HAlign::Center,
        theme.fg,
    );
    y += 18.0 + 3.0;

    y += separator(pm, theme, text, y, bw);

    // Vertical clock.
    let clock_style = Style::new(theme.font_title, 13.0).bold().line_height(13.0 * 0.92);
    let clock = now.format("%H\n%M").to_string();
    let (_, ch) = text.measure(&clock, &clock_style);
    text.draw(
        pm,
        &clock,
        &clock_style,
        0.0,
        y + 2.0 + (38.0 - ch) / 2.0,
        bw,
        HAlign::Center,
        if data.hover == Some(WidgetHit::Clock) { theme.gold } else { theme.fg },
    );
}

fn diamond(pm: &mut PixmapMut, cx: f32, cy: f32, r: f32, color: Color) {
    let mut pb = PathBuilder::new();
    pb.move_to(cx, cy - r);
    pb.line_to(cx + r, cy);
    pb.line_to(cx, cy + r);
    pb.line_to(cx - r, cy);
    pb.close();
    if let Some(path) = pb.finish() {
        pm.fill_path(&path, &draw::paint(color), FillRule::Winding, Transform::identity(), None);
    }
}

/// Bodoni "·" divider: 14 px content + 2 px margins, glyph lifted by 5 px.
fn separator(pm: &mut PixmapMut, theme: &Theme, text: &mut Text, y: f32, bw: f32) -> f32 {
    let st = Style::new(theme.font_bodoni, 14.0);
    let (_, sh) = text.measure("·", &st);
    text.draw(pm, "·", &st, 0.0, y + 2.0 + (14.0 - sh) / 2.0 - 5.0, bw, HAlign::Center, theme.gold);
    SEP_H
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::niri::Window;

    fn group(name: &str, ids: &[u64]) -> Group {
        Group {
            name: name.into(),
            windows: ids
                .iter()
                .map(|&id| Window {
                    id,
                    app_id: Some("foot".into()),
                    title: None,
                    workspace_id: Some(1),
                    is_focused: false,
                })
                .collect(),
            focused: false,
        }
    }

    #[test]
    fn bottom_section_is_anchored() {
        let g = geometry(1080.0, &[], 0, false);
        assert_eq!(g.win_top, WS_END);
        assert_eq!(g.bottom_y, 1080.0 - 16.0 - 496.0);
        assert!(g.groups.is_empty());
    }

    #[test]
    fn group_hit_testing() {
        let groups = vec![group("TERM", &[10, 11]), group("FFOX", &[20])];
        let geom = geometry(1080.0, &groups, 0, false);
        assert_eq!(geom.win_top, WS_END + SEP_H);
        assert_eq!(geom.groups.len(), 2);

        // First group starts at win_top; its row is 50 wide.
        let (x, y, w, h) = geom.groups[0].rect;
        assert_eq!((x, w), (0.0, 50.0));
        assert_eq!(y, geom.win_top);
        assert!(h > 16.0);

        // Clicking a diamond selects that specific window.
        let (dx, dy) = geom.groups[0].dots[1];
        let hit = hit_test(&geom, dx, dy).unwrap();
        assert_eq!(resolve_window(hit, &groups), Some(11));

        // Clicking the row (not a diamond) selects the group target.
        let hit = hit_test(&geom, 2.0, y + 2.0).unwrap();
        assert_eq!(resolve_window(hit, &groups), Some(10));

        // Second group is reachable too.
        let (_, gy2, _, gh2) = geom.groups[1].rect;
        assert!(gy2 > y + h);
        let hit = hit_test(&geom, 25.0, gy2 + gh2 / 2.0);
        assert!(hit.is_some());
    }

    #[test]
    fn workspace_slot_hit_testing() {
        let geom = geometry(1080.0, &[], 0, false);
        // Slot 0 top-left at (6, 78); slot 1 at (27, 78); slot 2 at (6, 104).
        assert_eq!(hit_test(&geom, 10.0, 80.0), Some(Hit::Workspace(0)));
        assert_eq!(hit_test(&geom, 30.0, 80.0), Some(Hit::Workspace(1)));
        assert_eq!(hit_test(&geom, 10.0, 106.0), Some(Hit::Workspace(2)));
        assert_eq!(hit_test(&geom, 45.0, 80.0), None);
        assert_eq!(hit_test(&geom, 10.0, 20.0), None);
    }

    #[test]
    fn tray_occupies_its_own_stack_slot() {
        assert_eq!(tray_height(0), 0.0);
        assert_eq!(tray_height(1), 8.0 + 28.0);
        assert_eq!(tray_height(2), 8.0 + 28.0 * 2.0 + 6.0);

        let h = 1080.0;
        let no_tray = anchors(h, 0, false);
        let with_tray = anchors(h, 2, false);
        // The tray pushes everything below it up by its height + one separator.
        let shift = tray_height(2) + SEP_H;
        // The clock stays bottom-anchored; the widgets above the tray move up.
        assert_eq!(with_tray.clock_center, no_tray.clock_center);
        assert_eq!(with_tray.clock_bottom, no_tray.clock_bottom);
        assert_eq!(with_tray.updates_center, no_tray.updates_center - shift);
        assert_eq!(with_tray.net_center, no_tray.net_center - shift);

        // First medallion hit box.
        assert_eq!(
            widget_hit(h, with_tray.tray_top + 4.0 + 2.0, 2, false),
            Some(WidgetHit::Tray(0))
        );
        // Second medallion, 34 px lower.
        assert_eq!(
            widget_hit(h, with_tray.tray_top + 4.0 + 34.0 + 2.0, 2, false),
            Some(WidgetHit::Tray(1))
        );
    }

    #[test]
    fn groups_clip_above_bottom_section() {
        // A tiny bar must not draw groups over the bottom widgets.
        let groups: Vec<Group> = (0..30).map(|i| group("TERM", &[i])).collect();
        let geom = geometry(400.0, &groups, 0, false);
        for gg in &geom.groups {
            assert!(gg.rect.1 + gg.rect.3 <= geom.bottom_y + 0.01);
        }
    }
}
