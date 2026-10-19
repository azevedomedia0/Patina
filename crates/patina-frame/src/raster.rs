//! Draws a window titlebar into a premultiplied BGRA bitmap (DRM `ARGB8888` on little-endian).
//!
//! Matches `WindowFrame` in the design system: translucent ink with rounded top corners,
//! a 12 px semibold title, and 26 px circular controls with 14 px Lucide icons (stroke 2).

use std::sync::Arc;

use cosmic_text::{
    fontdb::Source, Attrs, Buffer, Family, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use patina_tokens as tok;
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform,
};

use crate::geometry::{
    control_rect, Control, CONTROL_SIZE, PAD_LEFT, PAD_RIGHT, RADIUS, TITLEBAR_H,
};

const SANS: &[u8] =
    include_bytes!("../../../assets/fonts/plus-jakarta-sans/PlusJakartaSans-Variable.ttf");
const KAPPA: f32 = 0.552_284_8;

/// Everything that affects how a titlebar looks. Equal values draw identical bitmaps.
#[derive(Debug, Clone, PartialEq)]
pub struct TitleBar {
    pub width: i32,
    pub title: String,
    pub active: bool,
    pub hover: Option<Control>,
    /// Maximised windows have square corners.
    pub maximized: bool,
}

/// Premultiplied BGRA pixels, row-major, `width * height * 4` bytes.
#[derive(Debug, Clone)]
pub struct Bitmap {
    pub width: i32,
    pub height: i32,
    pub bgra: Vec<u8>,
}

pub struct Rasterizer {
    fonts: FontSystem,
    swash: SwashCache,
}

impl Default for Rasterizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Rasterizer {
    pub fn new() -> Self {
        let fonts = FontSystem::new_with_fonts([Source::Binary(Arc::new(SANS.to_vec()))]);
        Self {
            fonts,
            swash: SwashCache::new(),
        }
    }

    pub fn draw(&mut self, bar: &TitleBar) -> Bitmap {
        let (w, h) = (bar.width.max(1), TITLEBAR_H);
        let mut pm = Pixmap::new(w as u32, h as u32).expect("non-empty titlebar");

        // Body and hairline.
        let radius = if bar.maximized { 0.0 } else { RADIUS };
        if let Some(body) = top_rounded(0.0, 0.0, w as f32, h as f32, radius, true) {
            pm.fill_path(
                &body,
                &paint(color(tok::SURFACE_WINDOW)),
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }
        // Controls.
        for c in [Control::Minimize, Control::Maximize, Control::Close] {
            let r = control_rect(w, c);
            let (cx, cy) = (r.x as f32 + r.w as f32 / 2.0, r.y as f32 + r.h as f32 / 2.0);
            if bar.hover == Some(c) {
                if let Some(circle) = PathBuilder::from_circle(cx, cy, CONTROL_SIZE as f32 / 2.0) {
                    pm.fill_path(
                        &circle,
                        &paint(color(tok::SURFACE_HOVER)),
                        FillRule::Winding,
                        Transform::identity(),
                        None,
                    );
                }
            }
            draw_icon(&mut pm, c, cx, cy);
        }

        // Title, truncated with an ellipsis so it never runs under the controls.
        let left = PAD_LEFT;
        let right = control_rect(w, Control::Minimize).x - PAD_RIGHT;
        let avail = (right - left).max(0) as f32;
        let text_color = if bar.active {
            tok::TEXT_PRIMARY
        } else {
            tok::TEXT_MUTED
        };
        self.draw_title(&mut pm, &bar.title, left as f32, avail, text_color);

        Bitmap {
            width: w,
            height: h,
            bgra: to_bgra(&pm),
        }
    }

    fn draw_title(&mut self, pm: &mut Pixmap, title: &str, x: f32, avail: f32, c: tok::Color) {
        let size = tok::FS_SMALL;
        let line = size * tok::LH_BODY;
        let attrs = Attrs::new()
            .family(Family::Name("Plus Jakarta Sans"))
            .weight(Weight::SEMIBOLD);
        let mut buf = Buffer::new(&mut self.fonts, Metrics::new(size, line));
        buf.set_size(&mut self.fonts, None, None);

        let mut text = title.to_string();
        loop {
            buf.set_text(&mut self.fonts, &text, &attrs, Shaping::Advanced, None);
            buf.shape_until_scroll(&mut self.fonts, false);
            let width = buf.layout_runs().map(|r| r.line_w).fold(0.0_f32, f32::max);
            if width <= avail || text.is_empty() {
                break;
            }
            text.pop();
            while text.ends_with('\u{2026}') || text.ends_with(' ') {
                text.pop();
            }
            text.push('\u{2026}');
            // Re-measure; drop one more char each pass until it fits.
            buf.set_text(&mut self.fonts, &text, &attrs, Shaping::Advanced, None);
            buf.shape_until_scroll(&mut self.fonts, false);
            let w2 = buf.layout_runs().map(|r| r.line_w).fold(0.0_f32, f32::max);
            if w2 <= avail {
                break;
            }
            text.pop();
        }

        let y = ((TITLEBAR_H as f32 - line) / 2.0).round();
        let fg = cosmic_text::Color::rgb(c.r, c.g, c.b);
        buf.draw(
            &mut self.fonts,
            &mut self.swash,
            fg,
            |gx, gy, gw, gh, gc| {
                if gc.a() == 0 {
                    return;
                }
                if let Some(rect) =
                    Rect::from_xywh(x + gx as f32, y + gy as f32, gw as f32, gh as f32)
                {
                    let p = paint(Color::from_rgba8(gc.r(), gc.g(), gc.b(), gc.a()));
                    pm.fill_rect(rect, &p, Transform::identity(), None);
                }
            },
        );
    }
}

fn color(c: tok::Color) -> Color {
    Color::from_rgba8(c.r, c.g, c.b, (c.a * 255.0).round() as u8)
}

fn paint(c: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color(c);
    p.anti_alias = true;
    p
}

/// Rectangle with only the top corners rounded. `closed` adds the bottom edge.
fn top_rounded(x: f32, y: f32, w: f32, h: f32, r: f32, closed: bool) -> Option<Path> {
    let r = r.min(w / 2.0).min(h);
    let k = r * (1.0 - KAPPA);
    let mut pb = PathBuilder::new();
    pb.move_to(x, y + h);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + k, x + k, y, x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - k, y, x + w, y + k, x + w, y + r);
    pb.line_to(x + w, y + h);
    if closed {
        pb.close();
    }
    pb.finish()
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let k = r * (1.0 - KAPPA);
    let mut pb = PathBuilder::new();
    pb.move_to(x + r, y);
    pb.line_to(x + w - r, y);
    pb.cubic_to(x + w - k, y, x + w, y + k, x + w, y + r);
    pb.line_to(x + w, y + h - r);
    pb.cubic_to(x + w, y + h - k, x + w - k, y + h, x + w - r, y + h);
    pb.line_to(x + r, y + h);
    pb.cubic_to(x + k, y + h, x, y + h - k, x, y + h - r);
    pb.line_to(x, y + r);
    pb.cubic_to(x, y + k, x + k, y, x + r, y);
    pb.close();
    pb.finish()
}

/// Lucide `minus`, `square` and `x` (24 px grid, stroke 2), scaled to 14 px and centred on `(cx, cy)`.
fn draw_icon(pm: &mut Pixmap, control: Control, cx: f32, cy: f32) {
    let scale = 14.0 / 24.0;
    let t = Transform::from_scale(scale, scale).post_translate(cx - 7.0, cy - 7.0);
    let mut pb = PathBuilder::new();
    let path = match control {
        Control::Minimize => {
            pb.move_to(5.0, 12.0);
            pb.line_to(19.0, 12.0);
            pb.finish()
        }
        Control::Maximize => rounded_rect(3.0, 3.0, 18.0, 18.0, 2.0),
        Control::Close => {
            pb.move_to(18.0, 6.0);
            pb.line_to(6.0, 18.0);
            pb.move_to(6.0, 6.0);
            pb.line_to(18.0, 18.0);
            pb.finish()
        }
    };
    let Some(path) = path else { return };
    let stroke = Stroke {
        width: 2.0,
        line_cap: LineCap::Round,
        line_join: LineJoin::Round,
        ..Stroke::default()
    };
    pm.stroke_path(&path, &paint(color(tok::TEXT_SECONDARY)), &stroke, t, None);
}

/// tiny-skia stores premultiplied RGBA; DRM ARGB8888 on little-endian is BGRA in memory.
fn to_bgra(pm: &Pixmap) -> Vec<u8> {
    let mut out = pm.data().to_vec();
    for px in out.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(width: i32) -> TitleBar {
        TitleBar {
            width,
            title: "Terminal".into(),
            active: true,
            hover: None,
            maximized: false,
        }
    }

    fn px(b: &Bitmap, x: i32, y: i32) -> [u8; 4] {
        let i = ((y * b.width + x) * 4) as usize;
        [b.bgra[i], b.bgra[i + 1], b.bgra[i + 2], b.bgra[i + 3]]
    }

    #[test]
    fn size_matches_the_design() {
        let b = Rasterizer::new().draw(&bar(400));
        assert_eq!((b.width, b.height), (400, 38));
        assert_eq!(b.bgra.len(), 400 * 38 * 4);
    }

    #[test]
    fn top_corners_are_rounded_and_bottom_corners_are_square() {
        let b = Rasterizer::new().draw(&bar(400));
        assert_eq!(px(&b, 0, 0)[3], 0, "top-left corner is cut away");
        assert_eq!(px(&b, 399, 0)[3], 0, "top-right corner is cut away");
        assert!(px(&b, 1, 37)[3] > 150, "bottom-left stays filled");
    }

    #[test]
    fn body_is_the_translucent_window_surface() {
        let b = Rasterizer::new().draw(&bar(400));
        // surface-window rgba(26,31,41,.78), premultiplied: 26*.78≈20, 31*.78≈24, 41*.78≈32, a≈199.
        let [bl, g, r, a] = px(&b, 200, 30);
        assert!(
            (r as i32 - 20).abs() <= 2 && (g as i32 - 24).abs() <= 2 && (bl as i32 - 32).abs() <= 2
        );
        assert!((a as i32 - 199).abs() <= 2);
    }

    #[test]
    fn title_text_is_drawn_and_inactive_is_dimmer() {
        let mut r = Rasterizer::new();
        let bright = |b: &Bitmap| -> u8 {
            (14..120)
                .flat_map(|x| (8..30).map(move |y| (x, y)))
                .map(|(x, y)| px(b, x, y)[2])
                .max()
                .unwrap()
        };
        let active = bright(&r.draw(&bar(400)));
        let mut inactive_bar = bar(400);
        inactive_bar.active = false;
        let inactive = bright(&r.draw(&inactive_bar));
        assert!(active > 200, "text-primary is near white, got {active}");
        assert!(
            inactive < active - 40,
            "text-muted is dimmer: {inactive} vs {active}"
        );
    }

    #[test]
    fn hovering_a_control_draws_a_wash_behind_it() {
        let mut r = Rasterizer::new();
        let plain = r.draw(&bar(400));
        let mut hov = bar(400);
        hov.hover = Some(Control::Close);
        let hovered = r.draw(&hov);
        // Inside the close circle but away from the glyph strokes.
        let c = control_rect(400, Control::Close);
        let (x, y) = (c.x + 3, c.y + 13);
        assert!(px(&hovered, x, y)[2] > px(&plain, x, y)[2]);
        // The minimise button is untouched.
        let m = control_rect(400, Control::Minimize);
        assert_eq!(
            px(&hovered, m.x + 3, m.y + 13),
            px(&plain, m.x + 3, m.y + 13)
        );
    }

    #[test]
    fn long_titles_never_reach_the_controls() {
        let mut r = Rasterizer::new();
        let mut long = bar(300);
        long.title = "A very long window title that cannot possibly fit in this titlebar".into();
        let b = r.draw(&long);
        let left_of_controls = control_rect(300, Control::Minimize).x - PAD_RIGHT;
        // No bright text pixels in the gap between the title area and the first control.
        for x in left_of_controls + 1..control_rect(300, Control::Minimize).x {
            for y in 8..30 {
                assert!(px(&b, x, y)[2] < 120, "text spilled at x={x}");
            }
        }
    }

    #[test]
    fn maximised_bars_have_square_corners() {
        let mut m = bar(400);
        m.maximized = true;
        let b = Rasterizer::new().draw(&m);
        assert!(px(&b, 0, 0)[3] > 150);
    }
}
