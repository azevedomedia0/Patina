//! The pointer image from the design (`design/uploads/cursor.png`).
//!
//! The prototype shows it at 2.5x with the tip at (8, 6) in image pixels. It is scaled to the
//! output's scale here (1x on ordinary screens), so the hardware cursor plane can use it 1:1
//! and it stays sharp on HiDPI screens.

use std::{collections::HashMap, rc::Rc};

use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
    input::pointer::CursorIcon,
    utils::{Logical, Point, Size, Transform},
};
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Transform as SkTransform};

const PNG: &[u8] = include_bytes!("../../../design/uploads/cursor.png");
/// The PNG is drawn at 2.5 device pixels per logical pixel.
const PNG_SCALE: f32 = 2.5;
const TIP: (f32, f32) = (8.0, 6.0);

pub struct Cursor {
    pub buffer: MemoryRenderBuffer,
    /// Offset from the image's top-left to the pointer position, in logical pixels.
    pub hotspot: Point<i32, Logical>,
    /// How big it is drawn, in logical pixels.
    pub size: Size<i32, Logical>,
    /// The image's size in pixels.
    pub pixels: (i32, i32),
}

/// The design's arrow for an output of `scale`.
pub fn load(scale: f64) -> Option<Cursor> {
    let src = Pixmap::decode_png(PNG).ok()?;
    let k = scale.max(0.25) as f32 / PNG_SCALE;
    let side = (src.width() as f32 * k).ceil() as u32;
    let mut dst = Pixmap::new(side, side)?;
    dst.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bicubic,
            ..PixmapPaint::default()
        },
        SkTransform::from_scale(k, k),
        None,
    );
    // tiny-skia is premultiplied RGBA; DRM ARGB8888 is BGRA in memory on little-endian.
    let mut bgra = dst.data().to_vec();
    for px in bgra.chunks_exact_mut(4) {
        px.swap(0, 2);
    }
    let buffer = MemoryRenderBuffer::from_slice(
        &bgra,
        Fourcc::Argb8888,
        (side as i32, side as i32),
        1,
        Transform::Normal,
        None,
    );
    let logical = (src.width() as f32 / PNG_SCALE).ceil() as i32;
    Some(Cursor {
        buffer,
        hotspot: Point::from((
            (TIP.0 / PNG_SCALE).round() as i32,
            (TIP.1 / PNG_SCALE).round() as i32,
        )),
        size: Size::from((logical, logical)),
        pixels: (side as i32, side as i32),
    })
}

/// The design's arrow at every scale asked for so far.
#[derive(Default)]
pub struct Arrows {
    cache: Vec<(u64, Option<Rc<Cursor>>)>,
}

impl Arrows {
    pub fn get(&mut self, scale: f64) -> Option<Rc<Cursor>> {
        let key = scale.to_bits();
        if let Some((_, c)) = self.cache.iter().find(|(k, _)| *k == key) {
            return c.clone();
        }
        let c = load(scale).map(Rc::new);
        self.cache.push((key, c.clone()));
        c
    }
}

/// Nominal size used when looking up cursors in the system theme.
const THEME_SIZE: u32 = 24;

/// A cursor from the system XCursor theme.
pub struct NamedCursor {
    pub buffer: MemoryRenderBuffer,
    /// In logical pixels.
    pub hotspot: Point<i32, Logical>,
    /// How big it is drawn, in logical pixels.
    pub size: Size<i32, Logical>,
    /// The image's size in pixels.
    pub pixels: (i32, i32),
}

/// Cursors other than the design's arrow come from the user's XCursor theme, as in the
/// prototype, which only customises the default pointer and uses stock cursors for the rest.
pub struct Theme {
    theme: xcursor::CursorTheme,
    size: u32,
    cache: HashMap<(CursorIcon, u64), Option<Rc<NamedCursor>>>,
}

impl Theme {
    pub fn load() -> Self {
        let name = std::env::var("XCURSOR_THEME").unwrap_or_else(|_| "default".into());
        let size = std::env::var("XCURSOR_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(THEME_SIZE);
        Self {
            theme: xcursor::CursorTheme::load(&name),
            size,
            cache: HashMap::new(),
        }
    }

    /// The theme's image for `icon` on an output of `scale`, or `None` if the theme has no such
    /// cursor.
    pub fn get(&mut self, icon: CursorIcon, scale: f64) -> Option<Rc<NamedCursor>> {
        let key = (icon, scale.to_bits());
        if let Some(hit) = self.cache.get(&key) {
            return hit.clone();
        }
        let loaded = self.read(icon, scale).map(Rc::new);
        if loaded.is_none() {
            tracing::debug!(icon = icon.name(), "cursor not in the theme");
        }
        self.cache.insert(key, loaded.clone());
        loaded
    }

    fn read(&self, icon: CursorIcon, scale: f64) -> Option<NamedCursor> {
        // On HiDPI screens, the theme's bigger image, drawn at the usual logical size.
        let want = (self.size as f64 * scale.max(0.25)).round() as i64;
        let path = std::iter::once(icon.name())
            .chain(icon.alt_names().iter().copied())
            .find_map(|name| self.theme.load_icon(name))?;
        let data = std::fs::read(path).ok()?;
        let images = xcursor::parser::parse_xcursor(&data)?;
        // The size closest to what we want; an animated cursor shows its first frame.
        let size = images
            .iter()
            .min_by_key(|i| (want - i.size as i64).abs())?
            .size;
        let image = images.iter().find(|i| i.size == size)?;
        // xcursor's ARGB is premultiplied, little-endian: B, G, R, A in memory.
        let buffer = MemoryRenderBuffer::from_slice(
            &image.pixels_argb,
            Fourcc::Argb8888,
            (image.width as i32, image.height as i32),
            1,
            Transform::Normal,
            None,
        );
        // The image's own scale: its nominal size against the size wanted at 1x.
        let k = image.size as f64 / self.size as f64;
        let logical = |v: u32| (v as f64 / k).round() as i32;
        Some(NamedCursor {
            buffer,
            hotspot: Point::from((logical(image.xhot), logical(image.yhot))),
            size: Size::from((logical(image.width), logical(image.height))),
            pixels: (image.width as i32, image.height as i32),
        })
    }
}
