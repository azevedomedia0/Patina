//! The pointer image from the design (`design/uploads/cursor.png`).
//!
//! The prototype shows it at 2.5x with the tip at (8, 6) in image pixels. It is scaled down
//! once here so the hardware cursor plane can use it 1:1.

use std::{collections::HashMap, rc::Rc};

use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
    input::pointer::CursorIcon,
    utils::{Logical, Point, Transform},
};
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Transform as SkTransform};

const PNG: &[u8] = include_bytes!("../../../design/uploads/cursor.png");
const SCALE: f32 = 1.0 / 2.5;
const TIP: (f32, f32) = (8.0, 6.0);

pub struct Cursor {
    pub buffer: MemoryRenderBuffer,
    /// Offset from the image's top-left to the pointer position.
    pub hotspot: Point<i32, Logical>,
}

pub fn load() -> Option<Cursor> {
    let src = Pixmap::decode_png(PNG).ok()?;
    let side = (src.width() as f32 * SCALE).ceil() as u32;
    let mut dst = Pixmap::new(side, side)?;
    dst.draw_pixmap(
        0,
        0,
        src.as_ref(),
        &PixmapPaint {
            quality: FilterQuality::Bicubic,
            ..PixmapPaint::default()
        },
        SkTransform::from_scale(SCALE, SCALE),
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
    Some(Cursor {
        buffer,
        hotspot: Point::from((
            (TIP.0 * SCALE).round() as i32,
            (TIP.1 * SCALE).round() as i32,
        )),
    })
}

/// Nominal size used when looking up cursors in the system theme.
const THEME_SIZE: u32 = 24;

/// A cursor from the system XCursor theme.
pub struct NamedCursor {
    pub buffer: MemoryRenderBuffer,
    pub hotspot: Point<i32, Logical>,
}

/// Cursors other than the design's arrow come from the user's XCursor theme, as in the
/// prototype, which only customises the default pointer and uses stock cursors for the rest.
pub struct Theme {
    theme: xcursor::CursorTheme,
    size: u32,
    cache: HashMap<CursorIcon, Option<Rc<NamedCursor>>>,
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

    /// The theme's image for `icon`, or `None` if the theme has no such cursor.
    pub fn get(&mut self, icon: CursorIcon) -> Option<Rc<NamedCursor>> {
        if let Some(hit) = self.cache.get(&icon) {
            return hit.clone();
        }
        let loaded = self.read(icon).map(Rc::new);
        if loaded.is_none() {
            tracing::debug!(icon = icon.name(), "cursor not in the theme");
        }
        self.cache.insert(icon, loaded.clone());
        loaded
    }

    fn read(&self, icon: CursorIcon) -> Option<NamedCursor> {
        let path = std::iter::once(icon.name())
            .chain(icon.alt_names().iter().copied())
            .find_map(|name| self.theme.load_icon(name))?;
        let data = std::fs::read(path).ok()?;
        let images = xcursor::parser::parse_xcursor(&data)?;
        // The size closest to what we want; an animated cursor shows its first frame.
        let size = images
            .iter()
            .min_by_key(|i| (self.size as i64 - i.size as i64).abs())?
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
        Some(NamedCursor {
            buffer,
            hotspot: Point::from((image.xhot as i32, image.yhot as i32)),
        })
    }
}
