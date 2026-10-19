//! Patina design tokens.
//!
//! Constants are generated at build time from `design/tokens/*.css`, which is
//! the source of truth. Colours are [`Color`], pixel sizes and unitless values
//! are `f32`, durations are [`std::time::Duration`], and everything else
//! (font stacks, shadows, gradients, easing) is `&str`.

pub mod parse;
pub use parse::Color;

include!(concat!(env!("OUT_DIR"), "/tokens.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_documented_design_values() {
        assert_eq!(INK_850, Color::new(0x1a, 0x1f, 0x29, 1.0));
        assert_eq!(TIDE_500, Color::new(0x2f, 0x7d, 0xe1, 1.0));
        assert_eq!(H_PANEL, 36.0);
        assert_eq!(H_TITLEBAR, 38.0);
        assert_eq!(FS_BODY, 13.0);
        assert_eq!(RADIUS_XL, 20.0);
        assert_eq!(DUR_FAST.as_millis(), 120);
        assert_eq!(DUR.as_millis(), 180);
        assert_eq!(EASE, "cubic-bezier(.2,.8,.2,1)");
    }

    #[test]
    fn semantic_tokens_resolve_to_palette_values() {
        assert_eq!(ACCENT, TIDE_500);
        assert_eq!(BG_DESKTOP, INK_950);
        assert_eq!(SURFACE_WINDOW.a, 0.78);
    }

    #[test]
    fn light_theme_overrides_resolve() {
        assert_eq!(light::ACCENT, RUST_500);
        assert_eq!(light::BG_DESKTOP, PAPER_200);
    }

    #[test]
    fn spacing_scale_is_complete() {
        let scale = [
            SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_5, SPACE_6, SPACE_7, SPACE_8, SPACE_9,
        ];
        assert_eq!(scale, [2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0]);
    }
}
