//! System information shown by the Patina shell. No UI and no Wayland, so it builds and tests
//! on any OS.

pub mod battery;
pub mod clock;

pub use battery::Battery;
