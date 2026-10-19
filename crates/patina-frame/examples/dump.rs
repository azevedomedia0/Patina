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
}
