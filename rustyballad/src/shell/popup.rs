//! Popup rendering + hit-testing.
//!
//! niri cannot give layer surfaces exclusive keyboard grabs, so the whole
//! popup system is one full-screen overlay surface that is only mapped while
//! a popup is open. Geometry mirrors the QML popups: overlay-style popups
//! (Wi-Fi, Updates) place their card at x=10; anchored ones (Power, Battery,
//! Calendar, Mixer) sit at x=60 next to the bar; the window menu appears at
//! the pointer. Every draw call and hit test shares the same geometry
//! functions.

use chrono::{DateTime, Datelike, Local};
use tiny_skia::{Color, Pixmap, PixmapMut, PixmapPaint, Rect, Transform};

use crate::audio::{AudioState, Stream};
use crate::net::WifiNetwork;
use crate::niri::{State as NiriState, Workspace};
use crate::sysinfo::Snapshot;
use crate::theme::Theme;
use crate::tray::MenuEntry;
use crate::ui::draw::{self, alpha};
use crate::ui::text::{HAlign, Style, Text};
use crate::updates::Update;

use super::bar::{anchors, Anchors};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Power,
    Wifi,
    Battery,
    Updates,
    Calendar,
    WindowMenu,
    Mixer,
    TrayMenu,
}

#[derive(Debug, Clone)]
pub struct WifiUi {
    pub networks: Vec<WifiNetwork>,
    pub scanning: bool,
    pub enabled: bool,
    pub expanded: Option<String>,
    pub password: String,
    pub show_password: bool,
    pub status: String,
    pub error: bool,
    pub connecting: bool,
    pub connecting_ssid: String,
    pub scroll: usize,
}

impl Default for WifiUi {
    fn default() -> Self {
        Self {
            networks: Vec::new(),
            scanning: false,
            enabled: true,
            expanded: None,
            password: String::new(),
            show_password: false,
            status: String::new(),
            error: false,
            connecting: false,
            connecting_ssid: String::new(),
            scroll: 0,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct UpdatesUi {
    pub list: Vec<Update>,
    pub checking: bool,
    pub last_check: String,
    pub filter: String,
    pub scroll: usize,
    pub filter_focused: bool,
}

impl UpdatesUi {
    pub fn filtered(&self) -> Vec<&Update> {
        let q = self.filter.trim().to_lowercase();
        self.list
            .iter()
            .filter(|u| q.is_empty() || u.name.to_lowercase().contains(&q))
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct PowerUi {
    pub mode: u8,
    pub status: String,
    pub error: bool,
}

impl Default for PowerUi {
    fn default() -> Self {
        Self {
            mode: 1,
            status: String::new(),
            error: false,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct MenuUi {
    pub window: u64,
    pub scroll: usize,
}

#[derive(Debug, Clone, Default)]
pub struct MixerUi {
    pub streams: Vec<Stream>,
    pub scroll: usize,
    pub open_external: bool,
}

#[derive(Debug, Clone, Default)]
pub struct TrayMenuUi {
    pub tray_index: usize,
    pub entries: Vec<MenuEntry>,
    pub scroll: usize,
}

#[derive(Debug, Default)]
pub struct PopupState {
    pub kind: Option<Kind>,
    pub power: PowerUi,
    pub wifi: WifiUi,
    pub updates: UpdatesUi,
    pub menu: MenuUi,
    pub mixer: MixerUi,
    pub tray: TrayMenuUi,
    /// Screen-space pointer position (overlay-local).
    pub pointer: (f32, f32),
    pub hover: Option<Hit>,
    pub hover_row: Option<usize>,
}

impl PopupState {
    pub fn is_open(&self) -> bool {
        self.kind.is_some()
    }
}

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct Geometry {
    pub screen: (f32, f32),
    pub anchors: Anchors,
}

/// Menu row height (separators are shorter).
pub const TRAY_ROW_H: f32 = 24.0;
pub const TRAY_SEP_H: f32 = 9.0;
const TRAY_MENU_W: f32 = 220.0;

pub fn tray_menu_height(entries: &[MenuEntry]) -> f32 {
    let rows: f32 = entries
        .iter()
        .map(|e| if e.separator { TRAY_SEP_H } else { TRAY_ROW_H })
        .sum();
    16.0 + rows
}

impl Geometry {
    pub fn new(width: f32, height: f32, tray_count: usize, updates_visible: bool) -> Self {
        Self {
            screen: (width, height),
            anchors: anchors(height, tray_count, updates_visible),
        }
    }

    pub fn card(
        &self,
        kind: Kind,
        pointer: (f32, f32),
        n_workspaces: usize,
        tray_entries: &[MenuEntry],
    ) -> Rect {
        let a = self.anchors;
        let (w, h) = self.screen;
        match kind {
            Kind::Power => Rect::from_xywh(10.0, 8.0, 280.0, 270.0).unwrap(),
            Kind::Wifi => {
                let y = (a.net_center - 210.0).clamp(10.0, (h - 430.0).max(10.0));
                Rect::from_xywh(10.0, y, 280.0, 420.0).unwrap()
            }
            Kind::Battery => {
                let y = (a.battery_center - 130.0).clamp(10.0, (h - 270.0).max(10.0));
                Rect::from_xywh(10.0, y, 280.0, 260.0).unwrap()
            }
            Kind::Updates => {
                let y = (a.updates_center - 215.0).clamp(10.0, (h - 440.0).max(10.0));
                Rect::from_xywh(10.0, y, 300.0, 430.0).unwrap()
            }
            Kind::Calendar => {
                // QML anchors the card bottom 10 px above the screen bottom.
                let y = (h - 10.0 - 410.0).clamp(10.0, (h - 420.0).max(10.0));
                Rect::from_xywh(10.0, y, 280.0, 410.0).unwrap()
            }
            Kind::Mixer => {
                let y = (a.volume_center - 210.0).clamp(10.0, (h - 430.0).max(10.0));
                Rect::from_xywh(10.0, y, 280.0, 420.0).unwrap()
            }
            Kind::WindowMenu => {
                let h_menu = 12.0 + 4.0 * 26.0 + 3.0 + 1.0 + 3.0 + 13.0 + 22.0 * n_workspaces as f32 + 12.0;
                let x = (pointer.0 + 10.0).clamp(0.0, (w - 190.0).max(0.0));
                let y = pointer.1.clamp(0.0, (h - h_menu - 4.0).max(0.0));
                Rect::from_xywh(x, y, 190.0, h_menu).unwrap()
            }
            Kind::TrayMenu => {
                let menu_h = tray_menu_height(tray_entries).min(h - 20.0);
                // Opens beside the medallion, growing up from the pointer row.
                let y = (pointer.1 - 14.0).clamp(10.0, (h - menu_h - 10.0).max(10.0));
                Rect::from_xywh(10.0, y, TRAY_MENU_W, menu_h).unwrap()
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Hit targets
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hit {
    /// Click on the click-catcher outside the card → dismiss.
    Outside,
    PowerButton(u8),
    PowerProfile(u8),
    PowerSlider { x: f32 },
    WifiToggle,
    WifiScan,
    WifiEnable,
    WifiRow(usize),
    WifiDisconnect,
    WifiConnect,
    WifiCancel,
    WifiReveal,
    UpdatesRefresh,
    UpdatesUpgrade,
    MenuAction(u8),
    MenuWorkspace(usize),
    TrayMenuRow(usize),
    MixerOpenExternal,
    MixerMasterMute,
    MixerMasterSeek { x: f32 },
    MixerMute(usize),
    MixerSeek { row: usize, x: f32 },
    WheelUp,
    WheelDown,
}

fn in_rect(r: Rect, x: f32, y: f32) -> bool {
    x >= r.x() && x <= r.x() + r.width() && y >= r.y() && y <= r.y() + r.height()
}

/// Computes the hit for a pointer location, given the current popup.
pub fn hit(state: &PopupState, geom: &Geometry, x: f32, y: f32, n_workspaces: usize) -> Hit {
    let Some(kind) = state.kind else {
        return Hit::Outside;
    };
    let card = geom.card(kind, state.pointer, n_workspaces, &state.tray.entries);
    if !in_rect(card, x, y) {
        return Hit::Outside;
    }
    let (cx, cy) = (card.x(), card.y());
    match kind {
        Kind::Power => {
            // Three action squares, drawn at cy + 146 (see `power`).
            let btn_y = cy + 146.0;
            for i in 0..3u8 {
                let bx = cx + 16.0 + i as f32 * (248.0 + 6.0) / 3.0;
                let bw = (248.0 - 12.0) / 3.0;
                if x >= bx && x <= bx + bw && y >= btn_y && y <= btn_y + 42.0 {
                    return Hit::PowerButton(i);
                }
            }
            Hit::Outside
        }
        Kind::Battery => {
            // Buttons are drawn at cy + 140 (see `battery`).
            let row_y = cy + 140.0;
            for i in 0..3u8 {
                let bw = (248.0 - 8.0) / 3.0;
                let bx = cx + 16.0 + i as f32 * (bw + 4.0);
                if x >= bx && x <= bx + bw && y >= row_y && y <= row_y + 26.0 {
                    return Hit::PowerProfile(i);
                }
            }
            // 3-step profile slider (track at cy + 116, handle ±6/12 px).
            let slider_y = cy + 116.0;
            if x >= cx + 16.0 && x <= cx + 264.0 && y >= slider_y - 8.0 && y <= slider_y + 16.0 {
                return Hit::PowerSlider { x };
            }
            Hit::Outside
        }
        Kind::Wifi => {
            let inner_x = cx + 16.0;
            let inner_w = card.width() - 32.0;
            // Subheader row (STATUS: ON ... scan icon, ON/OFF button).
            let sub_y = cy + 54.0;
            if x >= inner_x + inner_w - 36.0 && y >= sub_y - 1.0 && y <= sub_y + 17.0 {
                return Hit::WifiToggle;
            }
            if x >= inner_x + inner_w - 72.0
                && x <= inner_x + inner_w - 36.0
                && y >= sub_y - 4.0
                && y <= sub_y + 20.0
            {
                return Hit::WifiScan;
            }
            // List rows start after the status strip (when present).
            let mut y0 = cy + 80.0;
            if !state.wifi.status.is_empty() || state.wifi.connecting {
                y0 += 30.0;
            }
            if !state.wifi.enabled {
                if x >= cx + 80.0 && x <= cx + 200.0 && y >= cy + 180.0 && y <= cy + 206.0 {
                    return Hit::WifiEnable;
                }
                return Hit::Outside;
            }
            let mut ry = y0 + 4.0;
            for (i, n) in state
                .wifi
                .networks
                .iter()
                .skip(state.wifi.scroll)
                .enumerate()
            {
                let expanded = state.wifi.expanded.as_deref() == Some(n.ssid.as_str());
                let row_h = if expanded { 116.0 } else { 34.0 };
                if y >= ry && y <= ry + row_h {
                    if expanded {
                        if y >= ry + 85.0 && y <= ry + 107.0 {
                            if x >= inner_x && x <= inner_x + inner_w - 64.0 {
                                return Hit::WifiConnect;
                            }
                            if x >= inner_x + inner_w - 58.0 {
                                return Hit::WifiCancel;
                            }
                        }
                        if y >= ry + 54.0
                            && y <= ry + 80.0
                            && x >= inner_x + inner_w - 30.0
                        {
                            return Hit::WifiReveal;
                        }
                    }
                    if n.in_use && x >= inner_x + inner_w - 76.0 {
                        return Hit::WifiDisconnect;
                    }
                    return Hit::WifiRow(state.wifi.scroll + i);
                }
                ry += row_h + 2.0;
            }
            Hit::Outside
        }
        Kind::Updates => {
            let inner_y = cy + 16.0;
            // Refresh icon on the subheader row.
            if y >= inner_y + 20.0 + 18.0 - 4.0
                && y <= inner_y + 20.0 + 18.0 + 20.0
                && x >= cx + 300.0 - 16.0 - 24.0
            {
                return Hit::UpdatesRefresh;
            }
            // Upgrade button at the bottom.
            if y >= cy + 430.0 - 16.0 - 32.0 && y <= cy + 430.0 - 16.0 {
                return Hit::UpdatesUpgrade;
            }
            Hit::Outside
        }
        Kind::WindowMenu => {
            let inner_x = cx + 6.0;
            let mut ry = cy + 6.0;
            for i in 0..4u8 {
                if y >= ry && y <= ry + 26.0 {
                    return Hit::MenuAction(i);
                }
                ry += 26.0 + 1.0;
            }
            ry += 3.0 + 1.0 + 3.0;
            // Section label.
            ry += 13.0;
            let n = n_workspaces;
            let rows: Vec<usize> = (0..n).skip(state.menu.scroll).collect();
            for (i, _) in rows.iter().enumerate() {
                if y >= ry && y <= ry + 22.0 {
                    return Hit::MenuWorkspace(state.menu.scroll + i);
                }
                ry += 22.0 + 1.0;
                // inner_x used below for consistency
                let _ = inner_x;
            }
            Hit::Outside
        }
        Kind::Mixer => {
            // Bottom external-mixer button.
            if y >= cy + 420.0 - 16.0 - 28.0 {
                return Hit::MixerOpenExternal;
            }
            // Master row: mute button on the right, meter below it.
            let master_y = cy + 54.0;
            if x >= cx + card.width() - 32.0 - 28.0
                && x <= cx + card.width() - 32.0 - 4.0
                && y >= master_y - 1.0
                && y <= master_y + 23.0
            {
                return Hit::MixerMasterMute;
            }
            if y >= master_y + 20.0 && y <= master_y + 42.0 {
                return Hit::MixerMasterSeek { x };
            }
            let mut ry = cy + 16.0 + 20.0 + 18.0 + 20.0 + 18.0;
            for (i, _s) in state.mixer.streams.iter().skip(state.mixer.scroll).enumerate() {
                if y >= ry && y <= ry + 54.0 {
                    let row = state.mixer.scroll + i;
                    // Mute button on the right.
                    if x >= cx + 16.0 + (card.width() - 32.0) - 30.0
                        && y >= ry + 18.0
                        && y <= ry + 42.0
                    {
                        return Hit::MixerMute(row);
                    }
                    // Slider zone.
                    if y >= ry + 20.0 && y <= ry + 38.0 {
                        return Hit::MixerSeek { row, x };
                    }
                }
                ry += 60.0;
            }
            Hit::Outside
        }
        Kind::Calendar => Hit::Outside,
        Kind::TrayMenu => {
            let mut ry = cy + 8.0;
            for (i, e) in state.tray.entries.iter().enumerate() {
                let rh = if e.separator { TRAY_SEP_H } else { TRAY_ROW_H };
                if y >= ry && y <= ry + rh {
                    return Hit::TrayMenuRow(i);
                }
                ry += rh;
                if ry > cy + card.height() - 8.0 {
                    break;
                }
            }
            Hit::Outside
        }
    }
}

// ---------------------------------------------------------------------------
// Drawing
// ---------------------------------------------------------------------------

/// Card chrome shared by every popup: parchment fill, 1 px frame, Victorian
/// corner ornaments, header and filigree divider. Returns the content top.
/// Solid card background, border and corner filigree (no header).
fn card_frame(pm: &mut PixmapMut, theme: &Theme, text: &mut Text, card: Rect) {
    draw::rect(pm, card.x(), card.y(), card.width(), card.height(), theme.bg0);
    draw::border(
        pm,
        card.x(),
        card.y(),
        card.width(),
        card.height(),
        0.0,
        1.0,
        theme.bg2,
    );
    // Corner filigree ('z' from Victorian Free Ornaments), inset 4 px and
    // rotated per corner exactly like the QML Items (270/0/180/90).
    let tile = corner_tile(theme, text);
    // Rotation happens around the centre of QML's 16x16 corner Item (4 px
    // inset), so the ornament points out of its own corner.
    let corners = [
        (card.x() + 12.0, card.y() + 12.0, 3usize),
        (card.x() + card.width() - 12.0, card.y() + 12.0, 0),
        (card.x() + 12.0, card.y() + card.height() - 12.0, 2),
        (card.x() + card.width() - 12.0, card.y() + card.height() - 12.0, 1),
    ];
    for (cx, cy, turns) in corners {
        let rotated = rotate_tile(&tile, turns);
        pm.draw_pixmap(
            (cx - 16.0) as i32,
            (cy - 16.0) as i32,
            rotated.as_ref(),
            &PixmapPaint::default(),
            Transform::identity(),
            None,
        );
    }
}

/// 32x32 tile centred on QML's 16x16 corner Item, with the glyph anchored
/// right/top inside it (the unrotated top-right appearance).
fn corner_tile(theme: &Theme, text: &mut Text) -> Pixmap {
    let mut tile = Pixmap::new(32, 32).expect("corner tile");
    text.draw(
        &mut tile.as_mut(),
        "z",
        &Style::new(theme.font_victorian, 16.0),
        8.0,
        7.0,
        16.0,
        HAlign::Center,
        alpha(theme.gold, 0.65),
    );
    tile
}

/// Rotate a square premultiplied-RGBA pixmap by 0/90/180/270 degrees clockwise.
fn rotate_tile(src: &Pixmap, turns: usize) -> Pixmap {
    let n = src.width();
    let mut dst = Pixmap::new(n, n).expect("rotated tile");
    let s = src.data();
    let d = dst.data_mut();
    for y in 0..n {
        for x in 0..n {
            let (sx, sy) = match turns % 4 {
                1 => (y, n - 1 - x),
                2 => (n - 1 - x, n - 1 - y),
                3 => (n - 1 - y, x),
                _ => (x, y),
            };
            let si = ((sy * n + sx) * 4) as usize;
            let di = ((y * n + x) * 4) as usize;
            d[di..di + 4].copy_from_slice(&s[si..si + 4]);
        }
    }
    dst
}

fn card_chrome(
    pm: &mut PixmapMut,
    theme: &Theme,
    text: &mut Text,
    card: Rect,
    title: &str,
) -> f32 {
    card_frame(pm, theme, text, card);
    let inner_x = card.x() + 16.0;
    let inner_w = card.width() - 32.0;
    let mut y = card.y() + 16.0;
    text.draw(
        pm,
        title,
        &Style::new(theme.font_title, 13.0).bold(),
        inner_x,
        y,
        inner_w,
        HAlign::Center,
        theme.fg,
    );
    y += 20.0;

    // Filigree divider: line · t · line.
    let mid = inner_x + inner_w / 2.0;
    text.draw(
        pm,
        "t",
        &Style::new(theme.font_bodoni, 14.0),
        mid - 10.0,
        y - 3.0,
        20.0,
        HAlign::Center,
        theme.gold,
    );
    draw::rect(pm, inner_x, y + 8.0, inner_w / 2.0 - 14.0, 1.0, alpha(theme.bg2, 0.7));
    draw::rect(pm, mid + 14.0, y + 8.0, inner_w / 2.0 - 14.0, 1.0, alpha(theme.bg2, 0.7));
    y + 18.0
}

fn label(
    text: &mut Text,
    pm: &mut PixmapMut,
    theme: &Theme,
    s: &str,
    x: f32,
    y: f32,
    w: f32,
    size: f32,
    color: Color,
) {
    text.draw(
        pm,
        s,
        &Style::new(theme.font_title, size).bold(),
        x,
        y,
        w,
        HAlign::Left,
        color,
    );
}

fn value(
    text: &mut Text,
    pm: &mut PixmapMut,
    theme: &Theme,
    s: &str,
    x: f32,
    y: f32,
    w: f32,
    size: f32,
    color: Color,
    align: HAlign,
) {
    text.draw(
        pm,
        s,
        &Style::new(theme.font_widget, size).bold(),
        x,
        y,
        w,
        align,
        color,
    );
}

/// Truncates with an ellipsis to fit `max_w`, like QML `elide: Text.ElideRight`.
fn elide(text: &mut Text, s: &str, style: &Style, max_w: f32) -> String {
    if text.measure(s, style).0 <= max_w {
        return s.to_string();
    }
    let mut out = String::new();
    for ch in s.chars() {
        let mut trial = out.clone();
        trial.push(ch);
        trial.push('…');
        if text.measure(&trial, style).0 > max_w && !out.is_empty() {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub fn draw(
    pm: &mut PixmapMut,
    theme: &Theme,
    text: &mut Text,
    state: &PopupState,
    snap: &Snapshot,
    niri: &NiriState,
    audio: AudioState,
    tray_count: usize,
    updates_visible: bool,
    now: &DateTime<Local>,
) {
    let Some(kind) = state.kind else { return };
    let geom = Geometry::new(
        pm.width() as f32,
        pm.height() as f32,
        tray_count,
        updates_visible,
    );
    let n_ws = visible_workspaces(niri).len();
    let card = geom.card(kind, state.pointer, n_ws, &state.tray.entries);
    // Shadow: 12 px offset, radius 14 approximated by three alpha rings.
    for (i, a) in [(14.0f32, 0.10f32), (10.0, 0.14), (6.0, 0.18)] {
        draw::rounded_rect(
            pm,
            card.x() - i + 12.0,
            card.y() - i + 12.0,
            card.width() + i * 2.0,
            card.height() + i * 2.0,
            i,
            alpha(theme.shadow, a),
        );
    }
    match kind {
        Kind::Power => power(pm, theme, text, state, snap, card),
        Kind::Wifi => wifi(pm, theme, text, state, card),
        Kind::Battery => battery(pm, theme, text, state, snap, card),
        Kind::Updates => updates(pm, theme, text, state, card),
        Kind::Calendar => calendar(pm, theme, text, card, now),
        Kind::WindowMenu => window_menu(pm, theme, text, state, niri, card),
        Kind::Mixer => mixer(pm, theme, text, state, audio, card),
        Kind::TrayMenu => tray_menu(pm, theme, text, state, card),
    }
}

/// DBusMenu rows: labels, check marks, submenu arrows, separators.
fn tray_menu(pm: &mut PixmapMut, theme: &Theme, text: &mut Text, state: &PopupState, card: Rect) {
    let x = card.x() + 4.0;
    let w = card.width() - 8.0;
    draw::rect(pm, card.x(), card.y(), card.width(), card.height(), theme.bg0);
    draw::border(pm, card.x(), card.y(), card.width(), card.height(), 0.0, 1.0, theme.bg2);

    let mut y = card.y() + 8.0;
    for (i, e) in state.tray.entries.iter().enumerate() {
        if y + if e.separator { TRAY_SEP_H } else { TRAY_ROW_H } > card.y() + card.height() - 8.0
        {
            break;
        }
        if e.separator {
            draw::rect(pm, x + 6.0, y + 4.0, w - 12.0, 1.0, alpha(theme.bg2, 0.8));
            y += TRAY_SEP_H;
            continue;
        }
        let hovered = state.hover == Some(Hit::TrayMenuRow(i)) && e.enabled;
        if hovered {
            draw::rounded_rect(pm, x, y, w, TRAY_ROW_H, 2.0, alpha(theme.gold, 0.14));
        }
        let indent = 8.0 + e.depth as f32 * 10.0;
        if e.checked {
            text.draw(
                pm,
                "✓",
                &Style::new(theme.font_title, 10.0),
                x + indent - 10.0,
                y + 7.0,
                12.0,
                HAlign::Center,
                theme.gold,
            );
        }
        let color = if !e.enabled {
            alpha(theme.fg, 0.45)
        } else if hovered {
            theme.red
        } else {
            theme.fg
        };
        text.draw(
            pm,
            &e.label,
            &Style::new(theme.font_title, 10.0),
            x + indent,
            y + 7.0,
            w - indent - 16.0,
            HAlign::Left,
            color,
        );
        if e.submenu {
            text.draw(
                pm,
                "›",
                &Style::new(theme.font_widget, 12.0),
                x + w - 16.0,
                y + 5.0,
                12.0,
                HAlign::Center,
                color,
            );
        }
        y += TRAY_ROW_H;
    }
}

fn power(
    pm: &mut PixmapMut,
    theme: &Theme,
    text: &mut Text,
    state: &PopupState,
    snap: &Snapshot,
    card: Rect,
) {
    let mut y = card_chrome(pm, theme, text, card, "SYSTEM POWER");
    let x = card.x() + 16.0;
    let w = card.width() - 32.0;

    for (icon, name, pct) in [
        ("󰘚", "CPU LOAD", snap.cpu_percent as f32 / 100.0),
        ("󰍛", "MEMORY", snap.ram_ratio as f32),
    ] {
        text.draw(
            pm,
            icon,
            &Style::new(theme.font_icons, 15.0),
            x,
            y,
            w,
            HAlign::Left,
            theme.gold,
        );
        label(text, pm, theme, name, x + 24.0, y + 2.0, w - 60.0, 10.0, theme.fg);
        value(
            text,
            pm,
            theme,
            &format!("{}%", (pct * 100.0).round() as u32),
            x,
            y + 1.0,
            w,
            12.0,
            theme.gold,
            HAlign::Right,
        );
        // 18-block bar width; block width + gap 3.
        let block_w = (w - 17.0 * 3.0) / 18.0;
        let lit = (pct * 18.0).round() as u32;
        let by = y + 22.0;
        for i in 0..18u32 {
            let bx = x + i as f32 * (block_w + 3.0);
            draw::rounded_rect(
                pm,
                bx,
                by,
                block_w,
                5.0,
                1.0,
                if i < lit { theme.gold } else { theme.bg2 },
            );
        }
        y += 40.0;
    }

    draw::rect(pm, x, y + 2.0, w, 1.0, alpha(theme.bg2, 0.7));
    y += 12.0;

    let actions = crate::power::SessionAction::all();
    let bw = (w - 12.0) / 3.0;
    for (i, act) in actions.iter().enumerate() {
        let bx = x + i as f32 * (bw + 6.0);
        let hovered = state.hover == Some(Hit::PowerButton(i as u8));
        draw::rect(
            pm,
            bx,
            y,
            bw,
            42.0,
            if hovered { alpha(theme.gold, 0.12) } else { Color::TRANSPARENT },
        );
        draw::border(
            pm,
            bx,
            y,
            bw,
            42.0,
            2.0,
            1.0,
            if hovered { theme.gold_bright } else { theme.bg2 },
        );
        text.draw(
            pm,
            act.icon(),
            &Style::new(theme.font_icons, 15.0),
            bx,
            y + 4.0,
            bw,
            HAlign::Center,
            if hovered { theme.red } else { theme.fg },
        );
    }
    // Action labels centered inside each button.
    for (i, act) in actions.iter().enumerate() {
        let bx = x + i as f32 * (bw + 6.0);
        let hovered = state.hover == Some(Hit::PowerButton(i as u8));
        text.draw(
            pm,
            act.label(),
            &Style::new(theme.font_title, 9.0).bold(),
            bx,
            y + 24.0,
            bw,
            HAlign::Center,
            if hovered { theme.red } else { theme.fg },
        );
    }
}

fn wifi(pm: &mut PixmapMut, theme: &Theme, text: &mut Text, state: &PopupState, card: Rect) {
    let ui = &state.wifi;
    let mut y = card_chrome(pm, theme, text, card, "WI-FI NETWORKS");
    let x = card.x() + 16.0;
    let w = card.width() - 32.0;

    // Subheader: STATUS: ON ... scan icon, ON/OFF button.
    label(text, pm, theme, "STATUS:", x, y, 60.0, 11.0, alpha(theme.fg, 0.75));
    label(
        text,
        pm,
        theme,
        if ui.enabled { "ON" } else { "OFF" },
        x + 54.0,
        y,
        40.0,
        11.0,
        if ui.enabled { theme.gold } else { theme.red },
    );
    text.draw(
        pm,
        "󰑐",
        &Style::new(theme.font_icons, 16.0),
        x + w - 66.0,
        y - 2.0,
        24.0,
        HAlign::Center,
        if ui.scanning { theme.gold_bright } else { theme.fg },
    );
    draw::border(
        pm,
        x + w - 36.0,
        y - 1.0,
        36.0,
        18.0,
        0.0,
        1.0,
        if ui.enabled { theme.gold } else { theme.bg2 },
    );
    text.draw(
        pm,
        if ui.enabled { "ON" } else { "OFF" },
        &Style::new(theme.font_title, 9.0).bold(),
        x + w - 36.0,
        y + 1.0,
        36.0,
        HAlign::Center,
        if ui.enabled { theme.gold } else { theme.fg },
    );
    y += 26.0;

    // Status strip.
    if !ui.status.is_empty() || ui.connecting {
        draw::rect(
            pm,
            x,
            y,
            w,
            24.0,
            if ui.error { Color::from_rgba8(244, 220, 217, 255) } else { theme.bg0 },
        );
        draw::border(
            pm,
            x,
            y,
            w,
            24.0,
            0.0,
            1.0,
            if ui.error { theme.red } else { theme.gold },
        );
        text.draw(
            pm,
            if ui.error {
                "⚠"
            } else if ui.connecting {
                "󰤩"
            } else {
                "✓"
            },
            &Style::new(theme.font_icons, 12.0),
            x + 6.0,
            y + 5.0,
            14.0,
            HAlign::Center,
            if ui.error { theme.red } else { theme.gold },
        );
        text.draw(
            pm,
            &ui.status,
            &Style::new(theme.font_widget, 11.0),
            x + 22.0,
            y + 5.0,
            w - 30.0,
            HAlign::Left,
            if ui.error { theme.red } else { theme.fg },
        );
        y += 30.0;
    }

    if !ui.enabled {
        text.draw(
            pm,
            "󰤮",
            &Style::new(theme.font_icons, 42.0),
            x,
            y + 60.0,
            w,
            HAlign::Center,
            theme.gold,
        );
        text.draw(
            pm,
            "Wi-Fi is disabled",
            &Style::new(theme.font_widget, 13.0).bold(),
            x,
            y + 110.0,
            w,
            HAlign::Center,
            theme.fg,
        );
        draw::rect(pm, card.x() + 80.0, card.y() + 180.0, 120.0, 26.0, theme.gold);
        draw::border(pm, card.x() + 80.0, card.y() + 180.0, 120.0, 26.0, 0.0, 1.0, theme.gold);
        text.draw(
            pm,
            "ENABLE WI-FI",
            &Style::new(theme.font_title, 10.0).bold(),
            card.x() + 80.0,
            card.y() + 187.0,
            120.0,
            HAlign::Center,
            theme.bg0,
        );
        return;
    }

    if ui.networks.is_empty() {
        text.draw(
            pm,
            if ui.scanning { "󰑐" } else { "󰤭" },
            &Style::new(theme.font_icons, 28.0),
            x,
            y + 70.0,
            w,
            HAlign::Center,
            theme.gold,
        );
        text.draw(
            pm,
            if ui.scanning {
                "Scanning..."
            } else {
                "No networks found"
            },
            &Style::new(theme.font_widget, 12.0),
            x,
            y + 106.0,
            w,
            HAlign::Center,
            theme.fg,
        );
        return;
    }

    let bottom = card.y() + card.height() - 16.0;
    let mut ry = y + 4.0;
    for (i, n) in ui.networks.iter().skip(ui.scroll).enumerate() {
        let row = ui.scroll + i;
        let expanded = ui.expanded.as_deref() == Some(n.ssid.as_str());
        let row_h = if expanded { 116.0 } else { 34.0 };
        if ry + row_h > bottom {
            break;
        }
        if expanded {
            draw::rect(pm, x, ry, w, row_h, alpha(theme.bg2, 0.75));
            draw::border(pm, x, ry, w, row_h, 0.0, 1.0, theme.gold);
        } else if state.hover_row == Some(row) {
            draw::rect(pm, x, ry, w, row_h, alpha(theme.bg2, 0.35));
        }
        text.draw(
            pm,
            n.icon(),
            &Style::new(theme.font_icons, 14.0),
            x + 2.0,
            ry + 6.0,
            18.0,
            HAlign::Center,
            if n.in_use { theme.gold } else { theme.fg },
        );
        let name_style = Style::new(theme.font_widget, 13.0).bold();
        let name = elide(text, &n.ssid, &name_style, w - 100.0);
        value(
            text,
            pm,
            theme,
            &name,
            x + 24.0,
            ry + 4.0,
            w - 100.0,
            13.0,
            if n.in_use { theme.gold } else { theme.fg },
            HAlign::Left,
        );
        let subtitle = format!("{} · {}%", n.subtitle(), n.signal);
        text.draw(
            pm,
            &subtitle,
            &Style::new(theme.font_widget, 10.0),
            x + 24.0,
            ry + 20.0,
            w - 100.0,
            HAlign::Left,
            alpha(if n.is_saved { theme.gold } else { theme.fg }, 0.75),
        );
        if n.in_use {
            let bx = x + w - 76.0;
            draw::border(pm, bx, ry + 8.0, 76.0, 18.0, 0.0, 1.0, theme.gold);
            text.draw(
                pm,
                "CONNECTED",
                &Style::new(theme.font_title, 9.0).bold(),
                bx,
                ry + 11.0,
                76.0,
                HAlign::Center,
                theme.gold,
            );
        } else {
            let glyph = if expanded {
                "▲"
            } else if n.is_saved {
                "▶"
            } else if n.secured() {
                "󰌾"
            } else {
                "▶"
            };
            text.draw(
                pm,
                glyph,
                &Style::new(theme.font_icons, 11.0),
                x + w - 22.0,
                ry + 10.0,
                18.0,
                HAlign::Center,
                if expanded { theme.gold_bright } else { theme.gold },
            );
        }

        if expanded {
            draw::rect(pm, x, ry + 36.0, w, 1.0, theme.bg2);
            text.draw(
                pm,
                "Password:",
                &Style::new(theme.font_widget, 11.0),
                x,
                ry + 39.0,
                w,
                HAlign::Left,
                theme.fg,
            );
            draw::border(pm, x, ry + 54.0, w, 26.0, 0.0, 1.0, theme.gold);
            let mut placeholder = Style::new(theme.font_widget, 11.0);
            placeholder.italic = true;
            let filled = Style::new(theme.font_widget, 12.0);
            let masked;
            let shown: &str = if ui.password.is_empty() {
                "Enter password..."
            } else if ui.show_password {
                &ui.password
            } else {
                masked = "•".repeat(ui.password.chars().count());
                &masked
            };
            text.draw(
                pm,
                shown,
                if ui.password.is_empty() { &placeholder } else { &filled },
                x + 8.0,
                ry + 60.0,
                w - 44.0,
                HAlign::Left,
                if ui.password.is_empty() { alpha(theme.gold, 0.7) } else { theme.fg },
            );
            text.draw(
                pm,
                if ui.show_password { "󰈈" } else { "󰈉" },
                &Style::new(theme.font_icons, 13.0),
                x + w - 26.0,
                ry + 58.0,
                18.0,
                HAlign::Center,
                theme.gold,
            );
            let bw = w - 64.0;
            let cy = ry + 85.0;
            draw::rect(pm, x, cy, bw, 22.0, theme.gold);
            text.draw(
                pm,
                if ui.connecting { "CONNECTING..." } else { "CONNECT" },
                &Style::new(theme.font_title, 9.0).bold(),
                x,
                cy + 6.0,
                bw,
                HAlign::Center,
                theme.bg0,
            );
            draw::border(pm, x + w - 58.0, cy, 58.0, 22.0, 0.0, 1.0, theme.gold);
            text.draw(
                pm,
                "CANCEL",
                &Style::new(theme.font_title, 9.0).bold(),
                x + w - 58.0,
                cy + 6.0,
                58.0,
                HAlign::Center,
                theme.gold,
            );
        }
        ry += row_h + 2.0;
    }
}

fn battery(
    pm: &mut PixmapMut,
    theme: &Theme,
    text: &mut Text,
    state: &PopupState,
    snap: &Snapshot,
    card: Rect,
) {
    let mut y = card_chrome(pm, theme, text, card, "POWER PROFILE");
    let x = card.x() + 16.0;
    let w = card.width() - 32.0;
    let charging = snap.battery.status == "Charging";

    text.draw(
        pm,
        if charging { "⚡" } else { "󰁹" },
        &Style::new(theme.font_icons, 16.0),
        x,
        y,
        20.0,
        HAlign::Left,
        theme.gold,
    );
    label(text, pm, theme, "BATTERY", x + 22.0, y + 1.0, 70.0, 11.0, theme.fg);
    value(
        text,
        pm,
        theme,
        &format!("{}%", snap.battery.percent.round() as u32),
        x,
        y,
        w - 80.0,
        13.0,
        theme.gold,
        HAlign::Right,
    );
    text.draw(
        pm,
        &format!("• {}", snap.battery.status.to_uppercase()),
        &Style::new(theme.font_widget, 10.0),
        x + w - 74.0,
        y + 2.0,
        74.0,
        HAlign::Right,
        alpha(theme.fg, 0.7),
    );
    y += 22.0;

    let block_w = (w - 17.0 * 3.0) / 18.0;
    let lit = (snap.battery.percent / 100.0 * 18.0).round() as u32;
    for i in 0..18u32 {
        let bx = x + i as f32 * (block_w + 3.0);
        draw::rect(
            pm,
            bx,
            y,
            block_w,
            6.0,
            if i < lit { theme.gold } else { theme.bg2 },
        );
    }
    y += 16.0;
    draw::rect(pm, x, y, w, 1.0, theme.bg2);
    y += 10.0;

    // LOW / HIGH captions.
    label(
        text,
        pm,
        theme,
        "LOW (POWERSAVE)",
        x,
        y,
        130.0,
        9.0,
        if state.power.mode == 0 { theme.gold } else { alpha(theme.fg, 0.65) },
    );
    label(
        text,
        pm,
        theme,
        "HIGH (PERF)",
        x + w - 90.0,
        y,
        90.0,
        9.0,
        if state.power.mode == 2 { theme.gold } else { alpha(theme.fg, 0.65) },
    );
    y += 14.0;

    // 3-step slider.
    draw::border(pm, x, y, w, 6.0, 0.0, 1.0, theme.gold);
    for i in 0..3 {
        let tick_x = x + i as f32 * (w - 2.0) / 2.0;
        draw::rect(
            pm,
            tick_x,
            y,
            2.0,
            6.0,
            if state.power.mode as i32 >= i { theme.gold } else { theme.bg2 },
        );
    }
    let handle_x = x + state.power.mode as f32 * (w - 16.0) / 2.0;
    draw::rect(pm, handle_x, y - 6.0, 16.0, 18.0, theme.gold);
    draw::border(pm, handle_x, y - 6.0, 16.0, 18.0, 0.0, 1.0, theme.fg);
    y += 24.0;

    let bw = (w - 8.0) / 3.0;
    for i in 0..3u8 {
        let bx = x + i as f32 * (bw + 4.0);
        let active = state.power.mode == i;
        let hovered = state.hover == Some(Hit::PowerProfile(i));
        draw::rect(
            pm,
            bx,
            y,
            bw,
            26.0,
            if active {
                theme.fg
            } else if hovered {
                Color::from_rgba8(0xdf, 0xcb, 0xb0, 255)
            } else {
                Color::TRANSPARENT
            },
        );
        draw::border(
            pm,
            bx,
            y,
            bw,
            26.0,
            0.0,
            1.0,
            if active { theme.gold } else { theme.bg2 },
        );
        text.draw(
            pm,
            ["POWERSAVE", "BALANCED", "PERFORMANCE"][i as usize],
            &Style::new(theme.font_title, 9.0).bold(),
            bx,
            y + 8.0,
            bw,
            HAlign::Center,
            if active { theme.gold_bright } else { theme.fg },
        );
    }
    y += 34.0;

    let footer = if state.power.status.is_empty() {
        crate::power::profile_footer(state.power.mode)
    } else {
        &state.power.status
    };
    text.draw(
        pm,
        footer,
        &Style::new(theme.font_title, 9.0),
        x,
        y,
        w,
        HAlign::Center,
        alpha(if state.power.error { theme.red } else { theme.fg }, 0.65),
    );
}

fn updates(pm: &mut PixmapMut, theme: &Theme, text: &mut Text, state: &PopupState, card: Rect) {
    let ui = &state.updates;
    let mut y = card_chrome(pm, theme, text, card, "PACMAN UPDATES");
    let x = card.x() + 16.0;
    let w = card.width() - 32.0;

    label(text, pm, theme, "STATUS:", x, y, 52.0, 11.0, alpha(theme.fg, 0.75));
    label(
        text,
        pm,
        theme,
        &if ui.list.is_empty() {
            "UP TO DATE".to_string()
        } else {
            format!("{} AVAILABLE", ui.list.len())
        },
        x + 50.0,
        y,
        150.0,
        11.0,
        if ui.list.is_empty() { theme.green } else { theme.gold },
    );
    if !ui.last_check.is_empty() {
        text.draw(
            pm,
            &ui.last_check,
            &Style::new(theme.font_widget, 10.0),
            x + w - 96.0,
            y + 1.0,
            64.0,
            HAlign::Right,
            alpha(theme.fg, 0.6),
        );
    }
    text.draw(
        pm,
        "󰑐",
        &Style::new(theme.font_icons, 15.0),
        x + w - 24.0,
        y - 2.0,
        24.0,
        HAlign::Center,
        if ui.checking { theme.gold_bright } else { theme.fg },
    );
    y += 22.0;

    // Search box.
    draw::rect(pm, x, y, w, 26.0, Color::from_rgba8(229, 211, 186, 255));
    draw::border(pm, x, y, w, 26.0, 2.0, 1.0, term_color(ui, theme));
    text.draw(
        pm,
        "󰍉",
        &Style::new(theme.font_icons, 12.0),
        x + 6.0,
        y + 5.0,
        18.0,
        HAlign::Center,
        if ui.filter_focused { theme.gold } else { alpha(theme.fg, 0.65) },
    );
    let shown = if ui.filter.is_empty() {
        "Filter packages...".to_string()
    } else {
        ui.filter.clone()
    };
    text.draw(
        pm,
        &shown,
        &Style::new(theme.font_widget, 12.0),
        x + 28.0,
        y + 5.0,
        w - 46.0,
        HAlign::Left,
        if ui.filter.is_empty() { alpha(theme.fg, 0.45) } else { theme.fg },
    );
    if !ui.filter.is_empty() {
        text.draw(
            pm,
            "✖",
            &Style::new(theme.font_widget, 10.0),
            x + w - 18.0,
            y + 7.0,
            14.0,
            HAlign::Center,
            alpha(theme.fg, 0.7),
        );
    }
    y += 32.0;

    let list = ui.filtered();
    let bottom = card.y() + card.height() - 16.0 - 32.0 - 6.0;
    if list.is_empty() {
        text.draw(
            pm,
            if ui.checking {
                "󰑐"
            } else if ui.list.is_empty() {
                "󰄬"
            } else {
                "󰍉"
            },
            &Style::new(theme.font_icons, 28.0),
            x,
            y + 60.0,
            w,
            HAlign::Center,
            if ui.list.is_empty() { theme.gold } else { theme.bg2 },
        );
        let msg = if ui.checking {
            "Checking for updates..."
        } else if !ui.filter.is_empty() {
            "No matching packages"
        } else {
            "All packages are up to date"
        };
        text.draw(
            pm,
            msg,
            &Style::new(theme.font_widget, 12.0).bold(),
            x,
            y + 96.0,
            w,
            HAlign::Center,
            theme.fg,
        );
    } else {
        let mut ry = y;
        let mut idx = 0usize;
        for u in list.iter().skip(ui.scroll) {
            let row_h = 40.0;
            if ry + row_h > bottom {
                break;
            }
            if state.hover_row == Some(ui.scroll + idx) {
                draw::rect(pm, x, ry, w, row_h, Color::from_rgba8(238, 219, 194, 255));
                draw::border(pm, x, ry, w, row_h, 2.0, 1.0, theme.gold);
            }
            text.draw(
                pm,
                "󰏔",
                &Style::new(theme.font_icons, 13.0),
                x + 4.0,
                ry + 4.0,
                18.0,
                HAlign::Center,
                alpha(theme.fg, 0.75),
            );
            let pkg_style = Style::new(theme.font_widget, 13.0).bold();
            let pkg = elide(text, &u.name, &pkg_style, w - 84.0);
            value(text, pm, theme, &pkg, x + 24.0, ry + 3.0, w - 80.0, 13.0, theme.fg, HAlign::Left);
            if !u.extra.is_empty() {
                label(
                    text,
                    pm,
                    theme,
                    &u.extra,
                    x + w - 54.0,
                    ry + 5.0,
                    54.0,
                    9.0,
                    theme.gold,
                );
            }
            value(
                text,
                pm,
                theme,
                &u.old_version,
                x + 24.0,
                ry + 20.0,
                90.0,
                11.0,
                alpha(theme.fg, 0.6),
                HAlign::Left,
            );
            text.draw(
                pm,
                "➔",
                &Style::new(theme.font_widget, 9.0),
                x + 116.0,
                ry + 21.0,
                14.0,
                HAlign::Center,
                theme.gold,
            );
            value(
                text,
                pm,
                theme,
                &u.new_version,
                x + 132.0,
                ry + 20.0,
                w - 132.0,
                11.0,
                theme.gold,
                HAlign::Left,
            );
            ry += row_h + 3.0;
            idx += 1;
        }
    }

    // Footer upgrade button.
    let by = card.y() + card.height() - 16.0 - 32.0;
    draw::border(pm, x, by, w, 32.0, 2.0, 1.0, theme.gold);
    text.draw(
        pm,
        "󰮯",
        &Style::new(theme.font_icons, 14.0),
        x,
        by + 8.0,
        32.0,
        HAlign::Center,
        theme.gold,
    );
    label(
        text,
        pm,
        theme,
        "UPGRADE IN TERMINAL",
        x + 30.0,
        by + 10.0,
        w - 30.0,
        10.0,
        theme.fg,
    );
}

fn term_color(ui: &UpdatesUi, theme: &Theme) -> Color {
    if ui.filter_focused {
        theme.gold
    } else {
        theme.bg2
    }
}

fn calendar(pm: &mut PixmapMut, theme: &Theme, text: &mut Text, card: Rect, now: &DateTime<Local>) {
    card_frame(pm, theme, text, card);
    let x = card.x() + 16.0;
    let w = card.width() - 32.0;
    let mut y = card.y() + 16.0;

    text.draw(
        pm,
        &now.format("%H:%M:%S").to_string(),
        &Style::new(theme.font_title, 32.0).bold(),
        x,
        y,
        w,
        HAlign::Center,
        theme.fg,
    );
    y += 40.0;
    text.draw(
        pm,
        &now.format("%Y-%m-%d").to_string(),
        &Style::new(theme.font_widget, 14.0),
        x,
        y,
        w,
        HAlign::Center,
        theme.gold,
    );
    y += 22.0;
    let tz = iana_timezone().unwrap_or_else(|| "LOCAL".into());
    text.draw(
        pm,
        "SYS_TIME",
        &Style::new(theme.font_title, 11.0),
        x,
        y,
        w,
        HAlign::Center,
        theme.red_dim,
    );
    text.draw(
        pm,
        &tz,
        &Style::new(theme.font_widget, 12.0).bold(),
        x,
        y + 12.0,
        w,
        HAlign::Center,
        theme.fg,
    );
    y += 36.0;

    // Divider.
    let mid = x + w / 2.0;
    text.draw(
        pm,
        "t",
        &Style::new(theme.font_bodoni, 14.0),
        mid - 10.0,
        y,
        20.0,
        HAlign::Center,
        theme.gold,
    );
    draw::rect(pm, x, y + 10.0, w / 2.0 - 14.0, 1.0, alpha(theme.bg2, 0.7));
    draw::rect(pm, mid + 14.0, y + 10.0, w / 2.0 - 14.0, 1.0, alpha(theme.bg2, 0.7));
    y += 22.0;

    // Weekday header, Monday first.
    let col_w = w / 7.0;
    for (i, day) in ["MO", "TU", "WE", "TH", "FR", "SA", "SU"].iter().enumerate() {
        text.draw(
            pm,
            day,
            &Style::new(theme.font_title, 12.0).bold(),
            x + i as f32 * col_w,
            y,
            col_w,
            HAlign::Center,
            if i > 4 { theme.red } else { theme.fg },
        );
    }
    y += 20.0;

    let week_day = now.weekday().num_days_from_monday() as i32; // 0 = Monday
    let leading = ((week_day - (now.day() as i32 - 1)) % 7 + 7) % 7;
    let days = days_in_month(now.year(), now.month());
    for d in 1..=days {
        let cell = leading + d as i32 - 1;
        let cx = x + (cell % 7) as f32 * col_w;
        let cy = y + (cell / 7) as f32 * 28.0;
        let today = d == now.day();
        if today {
            draw::rounded_rect(pm, cx + 2.0, cy, 26.0, 26.0, 2.0, theme.gold_bright);
            draw::border(pm, cx + 2.0, cy, 26.0, 26.0, 2.0, 1.0, theme.gold);
        } else {
            draw::border(pm, cx + 2.0, cy, 26.0, 26.0, 2.0, 1.0, theme.bg2);
        }
        let day_style = if today {
            Style::new(theme.font_widget, 13.0).bold()
        } else {
            Style::new(theme.font_widget, 13.0)
        };
        text.draw(
            pm,
            &d.to_string(),
            &day_style,
            cx + 2.0,
            cy + 5.0,
            26.0,
            HAlign::Center,
            theme.fg,
        );
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn iana_timezone() -> Option<String> {
    std::fs::read_link("/etc/localtime")
        .ok()
        .and_then(|p| {
            p.to_string_lossy()
                .split("zoneinfo/")
                .nth(1)
                .map(|s| s.to_string())
        })
        .or_else(|| std::env::var("TZ").ok())
}

fn visible_workspaces(niri: &NiriState) -> Vec<Workspace> {
    let mut list = niri.workspaces.clone();
    list.sort_by_key(|w| w.idx);
    list
}

fn window_menu(
    pm: &mut PixmapMut,
    theme: &Theme,
    text: &mut Text,
    state: &PopupState,
    niri: &NiriState,
    card: Rect,
) {
    let x = card.x() + 6.0;
    let w = card.width() - 12.0;
    draw::rect(pm, card.x(), card.y(), card.width(), card.height(), theme.bg0);
    draw::border(pm, card.x(), card.y(), card.width(), card.height(), 3.0, 1.0, theme.bg2);

    let mut y = card.y() + 6.0;
    let items = [
        ("✕ Close", 0u8),
        ("⛶ Fullscreen", 1),
        ("⇵ Toggle floating", 2),
        ("🔔 Set urgent", 3),
    ];
    for (label_text, idx) in items {
        let hovered = state.hover == Some(Hit::MenuAction(idx));
        if hovered {
            draw::rounded_rect(pm, x, y, w, 26.0, 2.0, alpha(theme.gold, 0.14));
        }
        text.draw(
            pm,
            label_text,
            &Style::new(theme.font_title, 10.0),
            x + 6.0,
            y + 8.0,
            w - 12.0,
            HAlign::Left,
            if hovered { theme.red } else { theme.fg },
        );
        y += 26.0 + 1.0;
    }
    y += 3.0;
    draw::rect(pm, x, y, w, 1.0, alpha(theme.bg2, 0.7));
    y += 4.0;
    label(text, pm, theme, "MOVE TO WORKSPACE", x + 6.0, y, w - 12.0, 9.0, theme.fg);
    y += 13.0;

    let workspaces = visible_workspaces(niri);
    let mut idx = 0usize;
    for ws in workspaces.iter().skip(state.menu.scroll) {
        if y + 22.0 > card.y() + card.height() - 6.0 {
            break;
        }
        let row = state.menu.scroll + idx;
        let hovered = state.hover == Some(Hit::MenuWorkspace(row));
        if hovered {
            draw::rounded_rect(pm, x, y, w, 22.0, 2.0, alpha(theme.gold, 0.14));
        }
        let name = ws
            .name
            .clone()
            .unwrap_or_else(|| format!("WS {}", ws.id));
        text.draw(
            pm,
            &name,
            &Style::new(theme.font_roman, 10.0),
            x + 12.0,
            y + 6.0,
            w - 18.0,
            HAlign::Left,
            if ws.is_focused { theme.gold_bright } else { theme.fg },
        );
        y += 22.0 + 1.0;
        idx += 1;
    }
}

fn mixer(
    pm: &mut PixmapMut,
    theme: &Theme,
    text: &mut Text,
    state: &PopupState,
    audio: AudioState,
    card: Rect,
) {
    let mut y = card_chrome(pm, theme, text, card, "AUDIO MIXER");
    let x = card.x() + 16.0;
    let w = card.width() - 32.0;

    // Master row.
    text.draw(
        pm,
        crate::audio::icon(audio),
        &Style::new(theme.font_icons, 15.0),
        x,
        y,
        24.0,
        HAlign::Center,
        theme.gold,
    );
    value(text, pm, theme, "MASTER", x + 28.0, y, 120.0, 13.0, theme.fg, HAlign::Left);
    value(
        text,
        pm,
        theme,
        &format!("{}%", (audio.volume * 100.0).round() as u32),
        x,
        y,
        w - 34.0,
        11.0,
        theme.fg,
        HAlign::Right,
    );
    // Master mute toggle.
    draw::border(
        pm,
        x + w - 28.0,
        y - 1.0,
        24.0,
        24.0,
        4.0,
        1.0,
        if audio.muted { theme.gold } else { theme.bg2 },
    );
    text.draw(
        pm,
        if audio.muted { "󰝟" } else { "󰕾" },
        &Style::new(theme.font_icons, 12.0),
        x + w - 28.0,
        y + 4.0,
        24.0,
        HAlign::Center,
        if audio.muted { theme.gold } else { theme.fg },
    );
    y += 20.0;

    // Master block meter, 18 blocks, ending before the %/mute column.
    let meter_w = w - 80.0;
    let block_w = (meter_w - 17.0 * 3.0) / 18.0;
    let lit = (audio.volume * 18.0).round() as u32;
    for i in 0..18u32 {
        let bx = x + i as f32 * (block_w + 3.0);
        draw::rect(
            pm,
            bx,
            y + 4.0,
            block_w,
            5.0,
            if i < lit && !audio.muted { theme.gold } else { theme.bg2 },
        );
    }
    y += 18.0;

    let bottom = card.y() + card.height() - 16.0 - 34.0;
    if state.mixer.streams.is_empty() {
        text.draw(
            pm,
            "No application streams",
            &Style::new(theme.font_widget, 12.0),
            x,
            y + 40.0,
            w,
            HAlign::Center,
            alpha(theme.fg, 0.6),
        );
    }
    let mut idx = 0usize;
    for s in state.mixer.streams.iter().skip(state.mixer.scroll) {
        if y + 54.0 > bottom {
            break;
        }
        let _row = state.mixer.scroll + idx;
        value(text, pm, theme, &s.name, x, y, w - 60.0, 13.0, theme.fg, HAlign::Left);
        text.draw(
            pm,
            "APP",
            &Style::new(theme.font_icons, 9.0).bold(),
            x + w - 40.0,
            y,
            40.0,
            HAlign::Right,
            theme.bg2,
        );
        // Slider: filled track + handle.
        let track_y = y + 24.0;
        draw::rect(pm, x, track_y, w - 80.0, 4.0, theme.bg2);
        draw::rect(pm, x, track_y, (w - 80.0) * s.volume, 4.0, theme.gold);
        let hx = x + (w - 80.0) * s.volume;
        draw::rect(pm, hx - 7.0, track_y - 5.0, 14.0, 14.0, if s.muted { theme.red } else { theme.fg });
        draw::border(pm, hx - 7.0, track_y - 5.0, 14.0, 14.0, 2.0, 1.0, theme.gold);
        text.draw(
            pm,
            &format!("{}%", (s.volume * 100.0).round() as u32),
            &Style::new(theme.font_icons, 11.0),
            x + w - 76.0,
            y + 20.0,
            42.0,
            HAlign::Right,
            theme.fg,
        );
        // Mute button.
        draw::border(
            pm,
            x + w - 28.0,
            y + 18.0,
            24.0,
            24.0,
            4.0,
            1.0,
            if s.muted { theme.gold } else { theme.bg2 },
        );
        text.draw(
            pm,
            if s.muted { "󰝟" } else { "󰕾" },
            &Style::new(theme.font_icons, 12.0),
            x + w - 28.0,
            y + 23.0,
            24.0,
            HAlign::Center,
            if s.muted { theme.gold } else { theme.fg },
        );
        y += 60.0;
        idx += 1;
    }

    // External mixer button.
    let by = card.y() + card.height() - 16.0 - 28.0;
    draw::border(pm, x, by, w, 28.0, 2.0, 1.0, theme.gold);
    text.draw(
        pm,
        "OPEN EXTERNAL MIXER",
        &Style::new(theme.font_title, 10.0).bold(),
        x,
        by + 9.0,
        w,
        HAlign::Center,
        theme.gold,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn geom() -> Geometry {
        Geometry::new(1920.0, 1080.0, 0, false)
    }

    #[test]
    fn cards_stay_on_screen() {
        let g = geom();
        for kind in [
            Kind::Power,
            Kind::Wifi,
            Kind::Battery,
            Kind::Updates,
            Kind::Calendar,
            Kind::Mixer,
        ] {
            let r = g.card(kind, (0.0, 0.0), 3, &[]);
            assert!(r.x() >= 0.0 && r.y() >= 0.0, "{kind:?} off-screen: {r:?}");
            assert!(r.x() + r.width() <= 1920.0);
            assert!(r.y() + r.height() <= 1080.0);
        }
    }

    #[test]
    fn outside_click_closes() {
        let mut st = PopupState::default();
        st.kind = Some(Kind::Power);
        assert_eq!(hit(&st, &geom(), 900.0, 900.0, 0), Hit::Outside);
    }

    #[test]
    fn power_buttons_hit() {
        let mut st = PopupState::default();
        st.kind = Some(Kind::Power);
        let g = geom();
        let card = g.card(Kind::Power, (0.0, 0.0), 0, &[]);
        // Third button on the drawn action row.
        let x = card.x() + 16.0 + 2.0 * (248.0 + 6.0) / 3.0 + 5.0;
        let y = card.y() + 146.0 + 5.0;
        assert_eq!(hit(&st, &g, x, y, 0), Hit::PowerButton(2));
    }

    #[test]
    fn menu_actions_hit() {
        let mut st = PopupState::default();
        st.kind = Some(Kind::WindowMenu);
        st.pointer = (60.0, 100.0);
        let g = geom();
        let card = g.card(Kind::WindowMenu, st.pointer, 3, &[]);
        assert_eq!(hit(&st, &g, card.x() + 50.0, card.y() + 10.0, 3), Hit::MenuAction(0));
    }

    #[test]
    fn updates_filter_matches_qml_semantics() {
        let ui = UpdatesUi {
            list: vec![
                Update {
                    name: "linux".into(),
                    old_version: "1".into(),
                    new_version: "2".into(),
                    extra: String::new(),
                },
                Update {
                    name: "mesa".into(),
                    old_version: "1".into(),
                    new_version: "2".into(),
                    extra: String::new(),
                },
            ],
            filter: "LIN".into(),
            ..Default::default()
        };
        let f = ui.filtered();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].name, "linux");
    }

    #[test]
    fn calendar_day_counts() {
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2025, 2), 28);
        assert_eq!(days_in_month(2025, 4), 30);
        assert_eq!(days_in_month(2025, 12), 31);
    }

    fn net(ssid: &str, in_use: bool) -> WifiNetwork {
        WifiNetwork {
            ssid: ssid.into(),
            signal: 80,
            security: "WPA2".into(),
            in_use,
            is_saved: true,
        }
    }

    #[test]
    fn wifi_rows_hit_where_they_draw() {
        let mut st = PopupState::default();
        st.kind = Some(Kind::Wifi);
        st.wifi.enabled = true;
        st.wifi.networks = vec![net("one", false), net("two", true)];
        let g = geom();
        let card = g.card(Kind::Wifi, (0.0, 0.0), 0, &[]);
        let x = card.x() + 40.0;
        // First row starts at card +54 (chrome) +26 (subheader) +4.
        assert_eq!(hit(&st, &g, x, card.y() + 54.0 + 26.0 + 4.0 + 10.0, 0), Hit::WifiRow(0));
        // Second row after 34px + 2px gap.
        assert_eq!(
            hit(&st, &g, x, card.y() + 54.0 + 26.0 + 4.0 + 36.0 + 10.0, 0),
            Hit::WifiRow(1)
        );
    }

    #[test]
    fn wifi_connected_badge_is_disconnect() {
        let mut st = PopupState::default();
        st.kind = Some(Kind::Wifi);
        st.wifi.networks = vec![net("home", true)];
        let g = geom();
        let card = g.card(Kind::Wifi, (0.0, 0.0), 0, &[]);
        let y = card.y() + 54.0 + 26.0 + 4.0 + 10.0;
        assert_eq!(
            hit(&st, &g, card.x() + 16.0 + 248.0 - 40.0, y, 0),
            Hit::WifiDisconnect
        );
    }

    #[test]
    fn mixer_rows_hit_where_they_draw() {
        let mut st = PopupState::default();
        st.kind = Some(Kind::Mixer);
        st.mixer.streams = vec![Stream {
            id: 4,
            name: "app".into(),
            volume: 0.5,
            muted: false,
        }];
        let g = geom();
        let card = g.card(Kind::Mixer, (0.0, 0.0), 0, &[]);
        let row_y = card.y() + 16.0 + 20.0 + 18.0 + 20.0 + 14.0;
        assert_eq!(
            hit(&st, &g, card.x() + 40.0, row_y + 24.0, 0),
            Hit::MixerSeek { row: 0, x: card.x() + 40.0 }
        );
    }

    #[test]
    fn tray_menu_rows_hit_where_they_draw() {
        let mut st = PopupState::default();
        st.kind = Some(Kind::TrayMenu);
        st.pointer = (10.0, 900.0);
        st.tray.entries = vec![
            MenuEntry {
                id: 1,
                label: "Open".into(),
                enabled: true,
                separator: false,
                checked: false,
                submenu: false,
                depth: 0,
            },
            MenuEntry {
                id: 2,
                label: String::new(),
                enabled: true,
                separator: true,
                checked: false,
                submenu: false,
                depth: 0,
            },
            MenuEntry {
                id: 3,
                label: "Quit".into(),
                enabled: true,
                separator: false,
                checked: false,
                submenu: false,
                depth: 0,
            },
        ];
        let g = geom();
        let card = g.card(Kind::TrayMenu, st.pointer, 0, &st.tray.entries);
        assert!(card.height() >= 16.0 + 24.0 * 2.0 + 9.0);
        assert_eq!(
            hit(&st, &g, card.x() + 30.0, card.y() + 8.0 + 4.0, 0),
            Hit::TrayMenuRow(0)
        );
        assert_eq!(
            hit(
                &st,
                &g,
                card.x() + 30.0,
                card.y() + 8.0 + 24.0 + 9.0 + 4.0,
                0
            ),
            Hit::TrayMenuRow(2)
        );
    }
}

#[cfg(test)]
mod battery_hit_tests {
    use super::*;

    #[test]
    fn battery_buttons_and_slider_hit_where_they_draw() {
        let mut st = PopupState::default();
        st.kind = Some(Kind::Battery);
        let g = Geometry::new(1920.0, 1080.0, 0, false);
        let card = g.card(Kind::Battery, (0.0, 0.0), 0, &[]);
        let row_y = card.y() + 140.0;
        assert_eq!(
            hit(&st, &g, card.x() + 16.0 + 10.0, row_y + 10.0, 0),
            Hit::PowerProfile(0)
        );
        assert_eq!(
            hit(&st, &g, card.x() + 16.0 + 84.0 + 10.0, row_y + 10.0, 0),
            Hit::PowerProfile(1)
        );
        assert_eq!(
            hit(&st, &g, card.x() + 16.0 + 168.0 + 10.0, row_y + 10.0, 0),
            Hit::PowerProfile(2)
        );
        let sy = card.y() + 116.0;
        assert!(matches!(
            hit(&st, &g, card.x() + 130.0, sy + 2.0, 0),
            Hit::PowerSlider { .. }
        ));
    }
}

#[cfg(test)]
mod power_hit_tests {
    use super::*;

    #[test]
    fn power_buttons_hit_where_they_draw() {
        let mut st = PopupState::default();
        st.kind = Some(Kind::Power);
        let g = Geometry::new(1920.0, 1080.0, 0, false);
        let card = g.card(Kind::Power, (0.0, 0.0), 0, &[]);
        let by = card.y() + 146.0;
        for i in 0..3 {
            let bw = (248.0 - 12.0) / 3.0;
            let bx = card.x() + 16.0 + i as f32 * (bw + 6.0);
            assert_eq!(
                hit(&st, &g, bx + bw / 2.0, by + 20.0, 0),
                Hit::PowerButton(i as u8)
            );
        }
    }
}
