//! Window shadow and outline, drawn into a bitmap larger than the window.
//!
//! Matches the design tokens `--shadow-window` (`0 24px 48px -12px rgba(0,0,0,.6)`, plus a
//! 1 px white 6% outline) and `--shadow-window-inactive` (`0 12px 24px -8px rgba(0,0,0,.4)`).
//! Like CSS `box-shadow`, nothing is drawn under the window itself, so the translucent
//! window surface does not show the shadow through.

use tiny_skia::{BlendMode, Color, FillRule, Paint, Path, PathBuilder, Pixmap, Stroke, Transform};

use crate::{geometry::RADIUS, raster::Bitmap};

#[derive(Debug, Clone, Copy, PartialEq)]
struct Spec {
    offset_y: i32,
    blur: i32,
    spread: i32,
    alpha: f32,
}

const ACTIVE: Spec = Spec {
    offset_y: 24,
    blur: 48,
    spread: -12,
    alpha: 0.6,
};
const INACTIVE: Spec = Spec {
    offset_y: 12,
    blur: 24,
    spread: -8,
    alpha: 0.4,
};

/// A shadow bitmap and how far it extends beyond the window on every side.
pub struct Shadow {
    pub bitmap: Bitmap,
    pub pad: i32,
}

/// Shadow for a window frame (titlebar and client area) of `width` x `height`.
pub fn render(width: i32, height: i32, active: bool) -> Shadow {
    render_scaled(width, height, active, 1.0)
}

/// `--shadow-popover`: `0 16px 32px -8px rgba(0,0,0,.55)`, for the dock, popovers and menus.
const POPOVER: Spec = Spec {
    offset_y: 16,
    blur: 32,
    spread: -8,
    alpha: 0.55,
};

/// Like [`render`], at `scale` device pixels per logical pixel. `width`, `height` and the
/// returned `pad` are logical; the bitmap is in device pixels.
pub fn render_scaled(width: i32, height: i32, active: bool, scale: f32) -> Shadow {
    let spec = if active { ACTIVE } else { INACTIVE };
    draw(width, height, spec, RADIUS, true, scale)
}

/// The popover shadow under a shell surface of `width` x `height` with corner `radius`
/// (logical), at `scale`. It has no outline (the surfaces draw their own borders).
pub fn render_popover(width: i32, height: i32, radius: f32, scale: f32) -> Shadow {
    draw(width, height, POPOVER, radius, false, scale)
}

fn draw(width: i32, height: i32, logical: Spec, radius: f32, outline: bool, scale: f32) -> Shadow {
    let s = if scale > 0.0 { scale } else { 1.0 };
    let logical_pad = logical.blur + logical.offset_y;
    let px = |v: i32| (v as f32 * s).round() as i32;
    let spec = Spec {
        offset_y: px(logical.offset_y),
        blur: px(logical.blur),
        spread: px(logical.spread),
        alpha: logical.alpha,
    };
    let pad = px(logical_pad);
    let (width, height) = (px(width), px(height));
    let radius = radius * s;
    let (w, h) = (width + 2 * pad, height + 2 * pad);

    // 1. The shadow shape: the window shrunk by the spread, shifted down.
    let inset = (-spec.spread) as f32;
    let shape = rounded_rect(
        pad as f32 + inset,
        (pad + spec.offset_y) as f32 + inset,
        width as f32 - 2.0 * inset,
        height as f32 - 2.0 * inset,
        (radius - inset).max(0.0),
    );
    let mut mask = Pixmap::new(w as u32, h as u32).expect("non-empty shadow");
    if let Some(shape) = shape {
        mask.fill_path(
            &shape,
            &black(),
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }

    // 2. Gaussian blur of the alpha channel (sigma = blur / 2, as in CSS).
    let mut alpha: Vec<f32> = mask
        .data()
        .chunks_exact(4)
        .map(|p| p[3] as f32 / 255.0)
        .collect();
    gaussian_blur(&mut alpha, w as usize, h as usize, spec.blur as f32 / 2.0);
    for (px, a) in mask.data_mut().chunks_exact_mut(4).zip(&alpha) {
        let v = (a * spec.alpha * 255.0).round() as u8;
        px.copy_from_slice(&[0, 0, 0, v]); // premultiplied black
    }

    // 3. Cut the window out, then add the 1 px outline just outside it.
    let window = rounded_rect(pad as f32, pad as f32, width as f32, height as f32, radius);
    if let Some(window) = &window {
        let clear = Paint {
            blend_mode: BlendMode::Clear,
            ..Paint::default()
        };
        mask.fill_path(
            window,
            &clear,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
    }
    // A 1 logical pixel outline: `s` device pixels wide, just outside the window.
    let half = 0.5 * s;
    let ring = if outline {
        rounded_rect(
            pad as f32 - half,
            pad as f32 - half,
            width as f32 + 2.0 * half,
            height as f32 + 2.0 * half,
            radius + half,
        )
    } else {
        None
    };
    if let Some(ring) = ring {
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        paint.set_color(Color::from_rgba8(255, 255, 255, 15));
        mask.stroke_path(
            &ring,
            &paint,
            &Stroke {
                width: s,
                ..Stroke::default()
            },
            Transform::identity(),
            None,
        );
    }

    let mut bgra = mask.data().to_vec();
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    Shadow {
        bitmap: Bitmap {
            width: w,
            height: h,
            bgra,
        },
        pad: logical_pad,
    }
}

fn black() -> Paint<'static> {
    let mut p = Paint {
        anti_alias: true,
        ..Paint::default()
    };
    p.set_color(Color::BLACK);
    p
}

fn rounded_rect(x: f32, y: f32, w: f32, h: f32, r: f32) -> Option<Path> {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);
    let k = r * (1.0 - 0.552_284_8);
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

/// Three box blurs approximate a Gaussian of standard deviation `sigma`.
fn gaussian_blur(data: &mut [f32], w: usize, h: usize, sigma: f32) {
    let boxes = box_sizes(sigma, 3);
    let mut tmp = vec![0.0_f32; data.len()];
    for b in boxes {
        let r = (b - 1) / 2;
        box_blur_h(data, &mut tmp, w, h, r);
        box_blur_v(&tmp, data, w, h, r);
    }
}

/// Box widths (odd) for `n` passes, after Ivan Kutskir's "fastest Gaussian blur".
fn box_sizes(sigma: f32, n: usize) -> Vec<usize> {
    let ideal = ((12.0 * sigma * sigma / n as f32) + 1.0).sqrt();
    let mut wl = ideal.floor() as usize;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let m = ((12.0 * sigma * sigma - (n * wl * wl) as f32 - 4.0 * (n * wl) as f32 - 3.0 * n as f32)
        / (-4.0 * wl as f32 - 4.0))
        .round() as usize;
    (0..n).map(|i| if i < m { wl } else { wu }).collect()
}

fn box_blur_h(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize) {
    let norm = 1.0 / (2 * r + 1) as f32;
    for y in 0..h {
        let row = &src[y * w..(y + 1) * w];
        let out = &mut dst[y * w..(y + 1) * w];
        let mut acc: f32 = row[..(r + 1).min(w)].iter().sum();
        for x in 0..w {
            out[x] = acc * norm;
            if x + r + 1 < w {
                acc += row[x + r + 1];
            }
            if x >= r {
                acc -= row[x - r];
            }
        }
    }
}

fn box_blur_v(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize) {
    let norm = 1.0 / (2 * r + 1) as f32;
    for x in 0..w {
        let mut acc: f32 = (0..(r + 1).min(h)).map(|y| src[y * w + x]).sum();
        for y in 0..h {
            dst[y * w + x] = acc * norm;
            if y + r + 1 < h {
                acc += src[(y + r + 1) * w + x];
            }
            if y >= r {
                acc -= src[(y - r) * w + x];
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(s: &Shadow, x: i32, y: i32) -> u8 {
        s.bitmap.bgra[((y * s.bitmap.width + x) * 4 + 3) as usize]
    }

    #[test]
    fn bitmap_covers_the_window_plus_padding() {
        let s = render(400, 300, true);
        assert_eq!(s.pad, 72);
        assert_eq!((s.bitmap.width, s.bitmap.height), (400 + 144, 300 + 144));
    }

    #[test]
    fn nothing_is_drawn_under_the_window() {
        let s = render(400, 300, true);
        assert_eq!(alpha(&s, s.pad + 200, s.pad + 150), 0);
        assert_eq!(alpha(&s, s.pad + 200, s.pad + 290), 0);
    }

    #[test]
    fn shadow_falls_below_more_than_above() {
        let s = render(400, 300, true);
        let below = alpha(&s, s.pad + 200, s.pad + 300 + 10);
        let above = alpha(&s, s.pad + 200, s.pad - 10);
        assert!(below > 60, "visible just below the window, got {below}");
        assert!(
            below > above + 30,
            "offset 24 px down: below {below}, above {above}"
        );
    }

    #[test]
    fn shadow_fades_to_nothing_at_the_edge_of_the_bitmap() {
        let s = render(400, 300, true);
        assert!(alpha(&s, s.pad + 200, s.bitmap.height - 1) < 3);
        assert!(alpha(&s, 0, s.pad + 150) < 3);
    }

    #[test]
    fn outline_sits_one_pixel_outside_the_window() {
        let s = render(400, 300, true);
        // Just above the top edge, centred: white 6% over a faint shadow.
        let i = (((s.pad - 1) * s.bitmap.width + s.pad + 200) * 4) as usize;
        assert!(s.bitmap.bgra[i + 2] >= 12, "ring has a white component");
    }

    #[test]
    fn inactive_windows_have_a_tighter_lighter_shadow() {
        let a = render(400, 300, true);
        let i = render(400, 300, false);
        assert_eq!(i.pad, 36);
        let a_below = alpha(&a, a.pad + 200, a.pad + 300 + 10);
        let i_below = alpha(&i, i.pad + 200, i.pad + 300 + 10);
        assert!(i_below < a_below);
    }

    #[test]
    fn hidpi_shadows_double_the_bitmap_but_keep_the_logical_padding() {
        let one = render(300, 200, true);
        let two = render_scaled(300, 200, true, 2.0);
        assert_eq!(two.pad, one.pad);
        assert_eq!(two.bitmap.width, one.bitmap.width * 2);
        assert_eq!(two.bitmap.height, one.bitmap.height * 2);
    }

    #[test]
    fn popover_shadows_extend_by_blur_plus_offset_and_have_no_outline() {
        let s = render_popover(320, 200, 20.0, 1.0);
        assert_eq!(s.pad, 32 + 16);
        // Just outside the top edge: no white outline pixel, only (dark) shadow or nothing.
        let i = (((s.pad - 1) * s.bitmap.width + s.pad + 100) * 4) as usize;
        assert!(s.bitmap.bgra[i] < 5 && s.bitmap.bgra[i + 1] < 5 && s.bitmap.bgra[i + 2] < 5);
    }

    #[test]
    fn blur_preserves_total_mass() {
        let (w, h) = (64, 64);
        let mut d = vec![0.0_f32; w * h];
        d[32 * w + 32] = 1000.0;
        gaussian_blur(&mut d, w, h, 4.0);
        let sum: f32 = d.iter().sum();
        assert!((sum - 1000.0).abs() < 1.0, "mass {sum}");
    }
}
