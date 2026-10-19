//! The default wallpaper: the design's `--wallpaper` token, rendered at any size.
//!
//! The token is a stack of `radial-gradient(ellipse at X% Y%, COLOR 0%, transparent N%)` layers
//! over a base colour. Only that form is understood; anything else falls back to the base colour
//! (or to the desktop background colour if even that cannot be read).

use patina_tokens as tok;

use crate::raster::Bitmap;

#[derive(Debug, Clone, PartialEq)]
struct Layer {
    /// Centre as fractions of the width and height.
    cx: f32,
    cy: f32,
    color: tok::Color,
    /// Gradient position (0..1) where the colour has faded to fully transparent.
    end: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct Wallpaper {
    /// Top-most layer first, as in CSS.
    layers: Vec<Layer>,
    base: tok::Color,
}

/// Splits on commas that are not inside parentheses.
fn split_top_level(s: &str) -> Vec<&str> {
    let (mut depth, mut start, mut out) = (0i32, 0usize, Vec::new());
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => {
                out.push(s[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(s[start..].trim());
    out
}

fn percent(s: &str) -> Option<f32> {
    s.trim()
        .strip_suffix('%')?
        .trim()
        .parse::<f32>()
        .ok()
        .map(|p| p / 100.0)
}

fn parse_layer(part: &str) -> Option<Layer> {
    let inner = part.strip_prefix("radial-gradient(")?.strip_suffix(')')?;
    let args = split_top_level(inner);
    let [shape, first, last] = args.as_slice() else {
        return None;
    };
    // "ellipse at 75% 15%"
    let pos = shape.strip_prefix("ellipse at ")?;
    let (x, y) = pos.split_once(char::is_whitespace)?;
    // "#2f5fa8 0%" then "transparent 55%"
    let (color, start) = first.rsplit_once(char::is_whitespace)?;
    let (end_word, end) = last.rsplit_once(char::is_whitespace)?;
    if end_word.trim() != "transparent" || percent(start)? != 0.0 {
        return None;
    }
    Some(Layer {
        cx: percent(x)?,
        cy: percent(y)?,
        color: crate::parse_color(color)?,
        end: percent(end)?,
    })
}

fn parse(css: &str) -> Option<Wallpaper> {
    let parts = split_top_level(css);
    let (base, layers) = parts.split_last()?;
    Some(Wallpaper {
        layers: layers
            .iter()
            .map(|p| parse_layer(p))
            .collect::<Option<_>>()?,
        base: crate::parse_color(base)?,
    })
}

/// Renders the wallpaper as an opaque premultiplied BGRA bitmap.
pub fn render(width: i32, height: i32) -> Bitmap {
    let (w, h) = (width.max(1), height.max(1));
    let wp = parse(tok::WALLPAPER).unwrap_or(Wallpaper {
        layers: Vec::new(),
        base: tok::BG_DESKTOP,
    });
    let mut bgra = vec![0u8; (w * h * 4) as usize];

    // CSS `ellipse farthest-corner`: radii are the farthest-side distances times sqrt(2).
    let radii: Vec<(f32, f32, f32, f32)> = wp
        .layers
        .iter()
        .map(|l| {
            let (cx, cy) = (l.cx * w as f32, l.cy * h as f32);
            let rx = cx.max(w as f32 - cx) * std::f32::consts::SQRT_2;
            let ry = cy.max(h as f32 - cy) * std::f32::consts::SQRT_2;
            (cx, cy, rx.max(1.0), ry.max(1.0))
        })
        .collect();

    for y in 0..h {
        for x in 0..w {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut rgb = [wp.base.r as f32, wp.base.g as f32, wp.base.b as f32];
            // Bottom layer first, so the first-listed layer ends up on top.
            for (layer, &(cx, cy, rx, ry)) in wp.layers.iter().zip(&radii).rev() {
                let t = (((px - cx) / rx).powi(2) + ((py - cy) / ry).powi(2)).sqrt();
                let a = (1.0 - t / layer.end).clamp(0.0, 1.0) * layer.color.a;
                let c = [
                    layer.color.r as f32,
                    layer.color.g as f32,
                    layer.color.b as f32,
                ];
                for i in 0..3 {
                    rgb[i] = rgb[i] * (1.0 - a) + c[i] * a;
                }
            }
            let i = ((y * w + x) * 4) as usize;
            bgra[i] = rgb[2].round() as u8;
            bgra[i + 1] = rgb[1].round() as u8;
            bgra[i + 2] = rgb[0].round() as u8;
            bgra[i + 3] = 255;
        }
    }
    Bitmap {
        width: w,
        height: h,
        bgra,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn px(b: &Bitmap, x: i32, y: i32) -> [u8; 3] {
        let i = ((y * b.width + x) * 4) as usize;
        [b.bgra[i + 2], b.bgra[i + 1], b.bgra[i]]
    }

    #[test]
    fn parses_the_design_token() {
        let wp = parse(tok::WALLPAPER).expect("the design token is understood");
        assert_eq!(wp.layers.len(), 2);
        assert_eq!(
            (wp.layers[0].cx, wp.layers[0].cy, wp.layers[0].end),
            (0.75, 0.15, 0.55)
        );
        assert_eq!(
            (wp.layers[1].cx, wp.layers[1].cy, wp.layers[1].end),
            (0.15, 0.90, 0.50)
        );
        assert_eq!(wp.layers[0].color, tok::Color::new(0x2f, 0x5f, 0xa8, 1.0));
        assert_eq!(wp.base, tok::INK_950);
    }

    #[test]
    fn rejects_forms_it_does_not_understand() {
        assert!(parse("linear-gradient(red, blue)").is_none());
        assert!(parse("radial-gradient(circle at 10% 10%,#fff 0%,transparent 50%),#000").is_none());
    }

    #[test]
    fn glow_centres_take_the_stop_colours() {
        let b = render(1280, 800);
        // Centre of the first glow is its start colour #2f5fa8.
        assert_eq!(
            px(&b, (0.75 * 1280.0) as i32, (0.15 * 800.0) as i32),
            [0x2f, 0x5f, 0xa8]
        );
        // Centre of the second glow, #1d3a66, with a little of the first on top of it.
        let p = px(&b, (0.15 * 1280.0) as i32, (0.90 * 800.0) as i32);
        assert!(p[2] >= 0x66 - 2 && p[2] <= 0x80, "blue channel {p:?}");
    }

    #[test]
    fn far_corners_fall_back_to_the_base_colour() {
        let b = render(1280, 800);
        let base = tok::INK_950;
        assert_eq!(px(&b, 1279, 799), [base.r, base.g, base.b]);
    }

    #[test]
    fn output_is_opaque() {
        let b = render(64, 48);
        assert!(b.bgra.chunks_exact(4).all(|p| p[3] == 255));
        assert_eq!(b.bgra.len(), 64 * 48 * 4);
    }
}
