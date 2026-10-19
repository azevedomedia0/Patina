//! How the compositor draws the Patina shell's layer surfaces, by their layer-shell namespace:
//! which glass (blur) is behind them, their corner radius, and whether they cast the popover
//! shadow. The surfaces themselves draw their fill and border; the compositor adds what a client
//! cannot: the blur of what is behind, and a shadow outside its own surface.

use patina_tokens as tok;

use crate::glass::{self, Glass};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShellSurface {
    pub glass: Option<Glass>,
    /// Corner radius in logical pixels; the blur and the shadow follow it.
    pub radius: f32,
    /// Whether it casts `--shadow-popover`.
    pub shadow: bool,
}

/// The rules for a layer surface with `namespace`, or `None` for one that is not Patina's.
pub fn for_namespace(namespace: &str) -> Option<ShellSurface> {
    let s = |glass, radius, shadow| {
        Some(ShellSurface {
            glass,
            radius,
            shadow,
        })
    };
    match namespace {
        // Full width under the top edge: square, no shadow (the design's TopPanel).
        "patina-panel" => s(Some(glass::panel()), 0.0, false),
        // Dock: `--surface-panel` with `--blur-panel`, `--radius-xl`, `--shadow-popover`.
        "patina-dock" => s(Some(glass::panel()), tok::RADIUS_XL, true),
        // Launcher, quick settings and notifications: `--blur-glass`, `--radius-xl`.
        "patina-launcher" | "patina-quick-settings" | "patina-notifications" => {
            s(Some(glass::window()), tok::RADIUS_XL, true)
        }
        // Context menus: `--blur-glass`, `--radius-lg`.
        "patina-menu" => s(Some(glass::window()), tok::RADIUS_LG, true),
        // Notification banners are an opaque card: no blur to show through it.
        "patina-banner" => s(None, tok::RADIUS_LG, true),
        _ => None,
    }
}

/// Popups (menus) opened by a Patina shell surface look like `patina-menu`.
pub fn popup() -> ShellSurface {
    ShellSurface {
        glass: Some(glass::window()),
        radius: tok::RADIUS_LG,
        shadow: true,
    }
}

/// Shell surfaces that close when the user presses anywhere outside them (and outside the
/// program that opened them, which handles its own clicks): the notification centre and menus,
/// as in the prototype. The launcher and quick settings stay until toggled.
pub fn closes_on_outside_press(namespace: &str) -> bool {
    matches!(namespace, "patina-notifications" | "patina-menu")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popovers_are_glass_with_the_xl_radius() {
        for ns in [
            "patina-launcher",
            "patina-quick-settings",
            "patina-notifications",
        ] {
            let s = for_namespace(ns).unwrap();
            assert_eq!(s.radius, 20.0);
            assert!(s.shadow);
            assert_eq!(s.glass.unwrap().sigma, 24.0);
        }
    }

    #[test]
    fn the_panel_is_square_and_flat_and_the_dock_floats() {
        assert_eq!(for_namespace("patina-panel").unwrap().radius, 0.0);
        assert!(!for_namespace("patina-panel").unwrap().shadow);
        let dock = for_namespace("patina-dock").unwrap();
        assert_eq!((dock.radius, dock.shadow), (20.0, true));
        assert_eq!(dock.glass.unwrap().sigma, 16.0);
    }

    #[test]
    fn other_programs_get_nothing() {
        assert_eq!(for_namespace("waybar"), None);
        assert!(!closes_on_outside_press("patina-launcher"));
        assert!(closes_on_outside_press("patina-menu"));
    }
}
