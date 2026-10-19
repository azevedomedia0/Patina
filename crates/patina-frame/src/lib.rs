//! Window frame for the Patina compositor: geometry, hit-testing and titlebar drawing.
//!
//! The compositor draws one frame per window: a 38 px titlebar above the client area,
//! with minimise, maximise and close controls. Sizes and colours come from
//! `patina-tokens`. This crate has no Wayland dependency, so it builds and tests on any OS.

pub mod geometry;
pub mod raster;
pub mod shadow;

pub use geometry::{control_rect, hit_test, Control, Hit, Rect, TITLEBAR_H};
pub use raster::{Bitmap, Rasterizer, TitleBar};
pub use shadow::Shadow;
