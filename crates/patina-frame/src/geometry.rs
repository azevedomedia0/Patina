//! Frame geometry shared by drawing and hit-testing. Integer logical pixels.

use patina_tokens as tok;

/// Titlebar height (`--h-titlebar`).
pub const TITLEBAR_H: i32 = tok::H_TITLEBAR as i32;
/// Window corner radius (`--radius-xl`).
pub const RADIUS: f32 = tok::RADIUS_XL;
/// Titlebar padding: left 14, right 8.
pub const PAD_LEFT: i32 = 14;
pub const PAD_RIGHT: i32 = 8;
/// Control buttons are 26 px circles, 2 px apart.
pub const CONTROL_SIZE: i32 = 26;
pub const CONTROL_GAP: i32 = 2;
/// Side of the square resize grip at the bottom-right corner.
pub const RESIZE_GRIP: i32 = 20;
/// Smallest client area a window can be resized to.
pub const MIN_CLIENT: (i32, i32) = (240, 160);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f64, py: f64) -> bool {
        px >= self.x as f64
            && py >= self.y as f64
            && px < (self.x + self.w) as f64
            && py < (self.y + self.h) as f64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control {
    Minimize,
    Maximize,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Titlebar,
    Control(Control),
    ResizeGrip,
    Client,
}

/// Rectangle of a control, relative to the frame's top-left, for a frame `width` wide.
pub fn control_rect(width: i32, control: Control) -> Rect {
    let slot = match control {
        Control::Close => 0,
        Control::Maximize => 1,
        Control::Minimize => 2,
    };
    let x = width - PAD_RIGHT - CONTROL_SIZE - slot * (CONTROL_SIZE + CONTROL_GAP);
    Rect::new(
        x,
        (TITLEBAR_H - CONTROL_SIZE) / 2,
        CONTROL_SIZE,
        CONTROL_SIZE,
    )
}

/// What is under `(px, py)` for a frame whose outer rectangle (titlebar and client) is `frame`.
/// Returns `None` outside the frame.
pub fn hit_test(frame: Rect, px: f64, py: f64) -> Option<Hit> {
    if !frame.contains(px, py) {
        return None;
    }
    let (lx, ly) = (px - frame.x as f64, py - frame.y as f64);
    if ly < TITLEBAR_H as f64 {
        for c in [Control::Close, Control::Maximize, Control::Minimize] {
            if control_rect(frame.w, c).contains(lx, ly) {
                return Some(Hit::Control(c));
            }
        }
        return Some(Hit::Titlebar);
    }
    let in_grip = lx >= (frame.w - RESIZE_GRIP) as f64 && ly >= (frame.h - RESIZE_GRIP) as f64;
    Some(if in_grip {
        Hit::ResizeGrip
    } else {
        Hit::Client
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME: Rect = Rect::new(100, 50, 400, 300);

    #[test]
    fn controls_sit_right_aligned_and_vertically_centred() {
        let close = control_rect(400, Control::Close);
        assert_eq!((close.x, close.y, close.w, close.h), (366, 6, 26, 26));
        assert_eq!(control_rect(400, Control::Maximize).x, 338);
        assert_eq!(control_rect(400, Control::Minimize).x, 310);
    }

    #[test]
    fn hit_test_finds_each_region() {
        // Close button centre: frame.x + 366 + 13, frame.y + 19.
        assert_eq!(
            hit_test(FRAME, 479.0, 69.0),
            Some(Hit::Control(Control::Close))
        );
        assert_eq!(
            hit_test(FRAME, 451.0, 69.0),
            Some(Hit::Control(Control::Maximize))
        );
        assert_eq!(
            hit_test(FRAME, 423.0, 69.0),
            Some(Hit::Control(Control::Minimize))
        );
        assert_eq!(hit_test(FRAME, 200.0, 60.0), Some(Hit::Titlebar));
        assert_eq!(hit_test(FRAME, 200.0, 200.0), Some(Hit::Client));
        assert_eq!(hit_test(FRAME, 495.0, 345.0), Some(Hit::ResizeGrip));
    }

    #[test]
    fn outside_the_frame_is_none() {
        assert_eq!(hit_test(FRAME, 99.0, 60.0), None);
        assert_eq!(hit_test(FRAME, 200.0, 350.0), None);
    }

    #[test]
    fn grip_does_not_overlap_the_titlebar() {
        let tiny = Rect::new(0, 0, 30, 45);
        assert_eq!(hit_test(tiny, 29.0, 20.0), Some(Hit::Titlebar));
        assert_eq!(hit_test(tiny, 29.0, 44.0), Some(Hit::ResizeGrip));
    }
}
