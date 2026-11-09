//! Theme constants — faithful port of `quickshell/Config.qml` palette,
//! typography and dimensions. Parsed once at startup.

use tiny_skia::Color;

/// sRGB color from `#RRGGBB` or `#AARRGGBB` (QML alpha-first convention).
pub fn color(hex: &str) -> Color {
    let h = hex.trim_start_matches('#');
    let v = u32::from_str_radix(h, 16).unwrap_or(0);
    match h.len() {
        8 => Color::from_rgba8(
            ((v >> 16) & 0xff) as u8,
            ((v >> 8) & 0xff) as u8,
            (v & 0xff) as u8,
            ((v >> 24) & 0xff) as u8,
        ),
        _ => Color::from_rgba8(
            ((v >> 16) & 0xff) as u8,
            ((v >> 8) & 0xff) as u8,
            (v & 0xff) as u8,
            0xff,
        ),
    }
}

#[derive(Debug)]
pub struct Theme {
    pub bg0: Color,
    pub bg1: Color,
    pub bg2: Color,
    pub fg: Color,
    pub red: Color,
    pub red_dim: Color,
    pub gold: Color,
    pub gold_bright: Color,
    pub green: Color,

    pub tick_active: Color,
    pub tick_active_border: Color,
    pub tick_inactive: Color,

    pub shadow: Color,
    pub bar_vignette: Color,

    pub font_victorian: &'static str,
    pub font_bodoni: &'static str,
    pub font_title: &'static str,
    pub font_roman: &'static str,
    pub font_accent: &'static str,
    pub font_widget: &'static str,
    pub font_icons: &'static str,

    pub bar_width: u32,
    pub bar_shadow: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg0: color("#f1e5ce"),
            bg1: color("#e8d2b4"),
            bg2: color("#d5c4a1"),
            fg: color("#3a2a20"),
            red: color("#7a362d"),
            red_dim: color("#4c2d26"),
            gold: color("#b5893f"),
            gold_bright: color("#d5a86d"),
            green: color("#7a7038"),

            tick_active: color("#a17228"),
            tick_active_border: color("#734e1b"),
            // Qt.rgba(0.23, 0.16, 0.13, 0.14)
            tick_inactive: Color::from_rgba(0.23, 0.16, 0.13, 0.14).unwrap(),

            shadow: color("#353a2a20"),
            bar_vignette: color("#284c2d26"),

            font_victorian: "Victorian Free Ornaments",
            font_bodoni: "Bodoni Ornaments ITC",
            font_title: "Cinzel",
            font_roman: "Cinzel Decorative",
            font_accent: "Berkshire Swash",
            font_widget: "EB Garamond",
            // QML asks for "AurulentSansMNerdFontPropo"; the installed OTF
            // family is "AurulentSansM Nerd Font" (fontdb matches exact names).
            font_icons: "AurulentSansM Nerd Font Propo",

            bar_width: 50,
            bar_shadow: 15,
        }
    }
}

impl Theme {
    pub fn bar_total_width(&self) -> u32 {
        self.bar_width + self.bar_shadow
    }
}
