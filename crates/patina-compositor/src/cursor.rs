//! The pointer image from the design (`design/uploads/cursor.png`).
//!
//! The prototype shows it at 2.5x with the tip at (8, 6) in image pixels. It is scaled down
//! once here so the hardware cursor plane can use it 1:1.

use smithay::{
    backend::{allocator::Fourcc, renderer::element::memory::MemoryRenderBuffer},
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
