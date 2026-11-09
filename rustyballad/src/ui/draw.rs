//! tiny-skia drawing helpers: rectangles, rounded rectangles, gradients,
//! and the discrete LED-block meters that define the ballad aesthetic.

use tiny_skia::{
    Color, FillRule, GradientStop, LinearGradient, Paint, PathBuilder, PixmapMut, Rect, SpreadMode,
    Stroke, Transform,
};

pub fn paint(color: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(color);
    p.anti_alias = true;
    p
}

pub fn rect(pm: &mut PixmapMut, x: f32, y: f32, w: f32, h: f32, color: Color) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    if let Some(r) = Rect::from_xywh(x, y, w, h) {
        pm.fill_rect(r, &paint(color), Transform::identity(), None);
    }
}

/// Vertical alpha multiplier for colors used at reduced opacity.
pub fn alpha(color: Color, a: f32) -> Color {
    let mut c = color;
    c.set_alpha(color.alpha() * a);
    c
}

pub fn rounded_rect_path(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<tiny_skia::Path> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let mut pb = PathBuilder::new();
    if r <= 0.0 {
        pb.push_rect(Rect::from_xywh(x, y, w, h)?);
        return pb.finish();
    }
    const K: f32 = 0.5522847498; // circle-to-cubic constant
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - r + K * r, y, x + w, y + r - K * r, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - r + K * r, x + w - r + K * r, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + r - K * r, y + h, x, y + h - r + K * r, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + r - K * r, x + r - K * r, y, x + r, y);
    pb.close();
    pb.finish()
}

pub fn rounded_rect(pm: &mut PixmapMut, x: f32, y: f32, w: f32, h: f32, r: f32, color: Color) {
    if let Some(path) = rounded_rect_path(x, y, w, h, r) {
        pm.fill_path(&path, &paint(color), FillRule::Winding, Transform::identity(), None);
    }
}

pub fn border(pm: &mut PixmapMut, x: f32, y: f32, w: f32, h: f32, r: f32, width: f32, color: Color) {
    if let Some(path) = rounded_rect_path(x, y, w, h, r) {
        pm.stroke_path(
            &path,
            &paint(color),
            &Stroke {
                width,
                ..Default::default()
            },
            Transform::identity(),
            None,
        );
    }
}

pub fn circle(pm: &mut PixmapMut, cx: f32, cy: f32, radius: f32, color: Color) {
    let mut pb = PathBuilder::new();
    pb.push_circle(cx, cy, radius);
    if let Some(path) = pb.finish() {
        pm.fill_path(&path, &paint(color), FillRule::Winding, Transform::identity(), None);
    }
}

/// Top-to-bottom (or bottom-to-top) vertical vignette.
pub fn v_gradient(pm: &mut PixmapMut, x: f32, y: f32, w: f32, h: f32, top: Color, bottom: Color) {
    let Some(r) = Rect::from_xywh(x, y, w, h) else { return };
    let mut p = Paint::default();
    if let Some(shader) = LinearGradient::new(
        tiny_skia::Point { x: 0.0, y },
        tiny_skia::Point { x: 0.0, y: y + h },
        vec![GradientStop::new(0.0, top), GradientStop::new(1.0, bottom)],
        SpreadMode::Pad,
        Transform::identity(),
    ) {
        p.shader = shader;
    }
    pm.fill_rect(r, &p, Transform::identity(), None);
}

/// Left-to-right horizontal gradient (bar's right-hand shadow).
pub fn h_gradient(pm: &mut PixmapMut, x: f32, y: f32, w: f32, h: f32, left: Color, right: Color) {
    let Some(r) = Rect::from_xywh(x, y, w, h) else { return };
    let mut p = Paint::default();
    if let Some(shader) = LinearGradient::new(
        tiny_skia::Point { x, y: 0.0 },
        tiny_skia::Point { x: x + w, y: 0.0 },
        vec![GradientStop::new(0.0, left), GradientStop::new(1.0, right)],
        SpreadMode::Pad,
        Transform::identity(),
    ) {
        p.shader = shader;
    }
    pm.fill_rect(r, &p, Transform::identity(), None);
}

/// Horizontal LED-block meter (desktop: 24 blocks, 4×9, gap 3).
#[allow(clippy::too_many_arguments)]
pub fn led_row(
    pm: &mut PixmapMut,
    x: f32,
    y: f32,
    count: u32,
    lit: u32,
    bw: f32,
    bh: f32,
    gap: f32,
    active: Color,
    inactive: Color,
    active_border: Option<Color>,
) {
    for i in 0..count {
        let bx = x + i as f32 * (bw + gap);
        if i < lit {
            rounded_rect(pm, bx, y, bw, bh, 1.0, active);
            if let Some(bc) = active_border {
                border(pm, bx, y, bw, bh, 1.0, 0.5, bc);
            }
        } else {
            rounded_rect(pm, bx, y, bw, bh, 1.0, inactive);
        }
    }
}

/// Vertical LED-block meter (bar: 8 blocks, filled from the bottom).
#[allow(clippy::too_many_arguments)]
pub fn led_col(
    pm: &mut PixmapMut,
    x: f32,
    y: f32,
    count: u32,
    lit: u32,
    bw: f32,
    bh: f32,
    gap: f32,
    active: Color,
    inactive: Color,
) {
    for i in 0..count {
        let by = y + i as f32 * (bh + gap);
        let color = if i >= count.saturating_sub(lit) { active } else { inactive };
        rect(pm, x, by, bw, bh, color);
    }
}
