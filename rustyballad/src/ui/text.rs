//! Text shaping/rasterization via cosmic-text, blended into tiny-skia
//! pixmaps. One shared `FontSystem` + `SwashCache` for the whole process;
//! buffers are created per draw call and dropped immediately (no retained
//! layout tree — this is a large part of the RAM saving vs Qt).

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, Style as CosmicStyle,
    SwashCache, Weight,
};
use tiny_skia::{Color, PixmapMut};

#[derive(Clone, Copy)]
pub struct Style<'a> {
    pub family: &'a str,
    pub size: f32,
    /// Absolute line height in px (QML `lineHeight` multiplies font size).
    pub line_height: f32,
    pub weight: Weight,
    pub italic: bool,
}

impl<'a> Style<'a> {
    pub const fn new(family: &'a str, size: f32) -> Self {
        Self {
            family,
            size,
            line_height: size * 1.2,
            weight: Weight::NORMAL,
            italic: false,
        }
    }

    pub const fn bold(mut self) -> Self {
        self.weight = Weight::BOLD;
        self
    }

    pub const fn black(mut self) -> Self {
        self.weight = Weight::BLACK;
        self
    }

    pub const fn line_height(mut self, lh: f32) -> Self {
        self.line_height = lh;
        self
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum HAlign {
    Left,
    Center,
    Right,
}

pub struct Text {
    fs: FontSystem,
    cache: SwashCache,
}

impl Default for Text {
    fn default() -> Self {
        Self::new()
    }
}

impl Text {
    pub fn new() -> Self {
        Self {
            fs: FontSystem::new(),
            cache: SwashCache::new(),
        }
    }

    fn layout(&mut self, line: &str, style: &Style) -> Buffer {
        let mut attrs = Attrs::new().family(Family::Name(style.family));
        attrs = attrs.weight(style.weight);
        if style.italic {
            attrs = attrs.style(CosmicStyle::Italic);
        }
        let mut buf = Buffer::new(
            &mut self.fs,
            Metrics::new(style.size, style.line_height),
        );
        buf.set_text(line, &attrs, Shaping::Advanced, None);
        buf.shape_until_scroll(&mut self.fs, false);
        buf
    }

    fn line_width(buf: &Buffer) -> f32 {
        buf.layout_runs().map(|r| r.line_w).fold(0.0, f32::max)
    }

    /// Measured size of `text` (may contain `\n`) in `style`.
    pub fn measure(&mut self, text: &str, style: &Style) -> (f32, f32) {
        let mut w = 0.0f32;
        let mut lines = 0usize;
        for line in text.split('\n') {
            let mut buf = self.layout(line, style);
            w = w.max(Self::line_width(&buf));
            lines += 1;
        }
        (w, lines as f32 * style.line_height)
    }

    /// Draws `text` into `pm` with its box origin at `(x, y)`. Each line is
    /// aligned inside `max_w` according to `align`. Returns the box height.
    pub fn draw(
        &mut self,
        pm: &mut PixmapMut,
        text: &str,
        style: &Style,
        x: f32,
        y: f32,
        max_w: f32,
        align: HAlign,
        color: Color,
    ) -> f32 {
        for (i, line) in text.split('\n').enumerate() {
            let mut buf = self.layout(line, style);
            let lw = Self::line_width(&buf);
            let dx = match align {
                HAlign::Left => 0.0,
                HAlign::Center => ((max_w - lw) / 2.0).max(0.0),
                HAlign::Right => (max_w - lw).max(0.0),
            };
            self.draw_buffer(pm, &mut buf, x + dx, y + i as f32 * style.line_height, color);
        }
        text.split('\n').count() as f32 * style.line_height
    }

    fn draw_buffer(&mut self, pm: &mut PixmapMut, buf: &mut Buffer, dx: f32, dy: f32, color: Color) {
        let cosmic_color = CosmicColor::rgba(
            (color.red() * 255.0).round() as u8,
            (color.green() * 255.0).round() as u8,
            (color.blue() * 255.0).round() as u8,
            (color.alpha() * 255.0).round() as u8,
        );
        let pw = pm.width() as i32;
        let ph = pm.height() as i32;
        let data = pm.data_mut();
        buf.draw(
            &mut self.fs,
            &mut self.cache,
            cosmic_color,
            |gx, gy, gw, gh, c| {
                let x0 = (dx as i32 + gx).max(0);
                let y0 = (dy as i32 + gy).max(0);
                let x1 = (dx as i32 + gx + gw as i32).min(pw);
                let y1 = (dy as i32 + gy + gh as i32).min(ph);
                let a = c.a() as u32;
                if a == 0 {
                    return;
                }
                // Source-over with premultiplied source.
                let sr = (c.r() as u32 * a + 127) / 255;
                let sg = (c.g() as u32 * a + 127) / 255;
                let sb = (c.b() as u32 * a + 127) / 255;
                let inv = 255 - a;
                for py in y0..y1 {
                    for px in x0..x1 {
                        let idx = ((py * pw + px) * 4) as usize;
                        if idx + 3 >= data.len() {
                            continue;
                        }
                        data[idx] = (sr + (data[idx] as u32 * inv + 127) / 255).min(255) as u8;
                        data[idx + 1] = (sg + (data[idx + 1] as u32 * inv + 127) / 255).min(255) as u8;
                        data[idx + 2] = (sb + (data[idx + 2] as u32 * inv + 127) / 255).min(255) as u8;
                        data[idx + 3] = (a + (data[idx + 3] as u32 * inv + 127) / 255).min(255) as u8;
                    }
                }
            },
        );
    }
}


