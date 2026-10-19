//! Writes sample titlebars to PNG for visual inspection: `cargo run -p patina-frame --example dump -- OUT_DIR`.
use patina_frame::{Control, Rasterizer, TitleBar};

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let mut r = Rasterizer::new();
    let cases = [
        ("active", true, None, false),
        ("inactive", false, None, false),
        ("hover-close", true, Some(Control::Close), false),
        ("hover-max", true, Some(Control::Maximize), false),
        ("maximised", true, None, true),
    ];
    for (name, active, hover, maximized) in cases {
        let bar = TitleBar {
            width: 480,
            title: "Terminal".into(),
            active,
            hover,
            maximized,
            scale: 1.0,
        };
        let bm = r.draw(&bar);
        // Composite over the wallpaper blue so translucency is visible, then save RGBA.
        let mut pm = tiny_skia::Pixmap::new(bm.width as u32, bm.height as u32).unwrap();
        pm.fill(tiny_skia::Color::from_rgba8(0x2f, 0x5f, 0xa8, 255));
        let mut src = tiny_skia::Pixmap::new(bm.width as u32, bm.height as u32).unwrap();
        for (d, s) in src
            .data_mut()
            .chunks_exact_mut(4)
            .zip(bm.bgra.chunks_exact(4))
        {
            d.copy_from_slice(&[s[2], s[1], s[0], s[3]]);
        }
        pm.draw_pixmap(
            0,
            0,
            src.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::identity(),
            None,
        );
        pm.save_png(format!("{dir}/titlebar-{name}.png")).unwrap();
    }
    mock(&dir, &mut r, true);
    mock(&dir, &mut r, false);
}

/// Composites a mock window (shadow, titlebar, content) over the wallpaper blue.
#[allow(dead_code)]
pub fn mock(dir: &str, r: &mut Rasterizer, active: bool) {
    use tiny_skia::{Color, Paint, Pixmap, PixmapPaint, Rect, Transform};
    let (w, h) = (480, 300);
    let sh = patina_frame::shadow::render(w, h, active);
    let (cw, ch) = (w + 2 * sh.pad + 40, h + 2 * sh.pad + 40);
    let mut canvas = Pixmap::new(cw as u32, ch as u32).unwrap();
    canvas.fill(Color::from_rgba8(0x2f, 0x5f, 0xa8, 255));
    let load = |b: &patina_frame::Bitmap| {
        let mut p = Pixmap::new(b.width as u32, b.height as u32).unwrap();
        for (d, s) in p.data_mut().chunks_exact_mut(4).zip(b.bgra.chunks_exact(4)) {
            d.copy_from_slice(&[s[2], s[1], s[0], s[3]]);
        }
        p
    };
    let (ox, oy) = (20 + sh.pad, 20 + sh.pad);
    canvas.draw_pixmap(
        ox - sh.pad,
        oy - sh.pad,
        load(&sh.bitmap).as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    let bar = r.draw(&TitleBar {
        width: w,
        title: "File Bin".into(),
        active,
        hover: None,
        maximized: false,
        scale: 1.0,
    });
    canvas.draw_pixmap(
        ox,
        oy,
        load(&bar).as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    let mut p = Paint::default();
    p.set_color(Color::from_rgba8(17, 17, 17, 255));
    canvas.fill_rect(
        Rect::from_xywh((ox) as f32, (oy + 38) as f32, w as f32, (h - 38) as f32).unwrap(),
        &p,
        Transform::identity(),
        None,
    );
    canvas
        .save_png(format!(
            "{dir}/mock-{}.png",
            if active { "active" } else { "inactive" }
        ))
        .unwrap();
}
