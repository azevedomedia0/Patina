//! Where the outputs (monitors) are and what lives on them.
//!
//! Outputs sit side by side in one coordinate space, in the order they were connected. Windows
//! live in that space; each output draws the part of it that falls on its rectangle. Panels are
//! layer surfaces, which smithay keeps per output.

use patina_frame::TITLEBAR_H;
use smithay::{
    desktop::{layer_map_for_output, Window},
    output::{Output, Scale},
    utils::{Logical, Point, Rectangle, Transform},
    wayland::{compositor::send_surface_state, fractional_scale::with_fractional_scale},
};

use crate::state::Patina;

/// The scale for a display: `PATINA_SCALE` if it says so, else from its pixel density.
///
/// `PATINA_SCALE` is either one number for every display (`1.5`) or a list by display name
/// (`eDP-1=2,HDMI-A-1=1`). Without it, about 96 dots per inch is 100%, rounded to the
/// nearest quarter between 100% and 200%. Displays that report no size, or an implausible
/// one (projectors, virtual machines), get 100%.
pub fn scale_for(name: &str, pixels: (i32, i32), millimetres: (u32, u32)) -> f64 {
    if let Some(s) = std::env::var("PATINA_SCALE")
        .ok()
        .and_then(|v| scale_override(&v, name))
    {
        return s;
    }
    auto_scale(pixels, millimetres)
}

/// The scale `spec` (the value of `PATINA_SCALE`) gives display `name`, if any.
fn scale_override(spec: &str, name: &str) -> Option<f64> {
    let valid = |s: f64| (0.5..=4.0).contains(&s).then_some(s);
    if let Ok(all) = spec.trim().parse::<f64>() {
        return valid(all);
    }
    spec.split(',').find_map(|entry| {
        let (n, v) = entry.split_once('=')?;
        (n.trim() == name).then(|| v.trim().parse().ok().and_then(valid))?
    })
}

fn auto_scale((w, _h): (i32, i32), (mm_w, mm_h): (u32, u32)) -> f64 {
    // Under 10 x 6 cm is not a real screen size; neither is a 1 cm-per-pixel projector guess.
    if mm_w < 100 || mm_h < 60 || w <= 0 {
        return 1.0;
    }
    let dpi = w as f64 * 25.4 / mm_w as f64;
    ((dpi / 96.0 * 4.0).round() / 4.0).clamp(1.0, 2.0)
}

/// What to tell smithay: whole scales as integers, so clients without fractional scaling get
/// exact buffers.
pub fn output_scale(scale: f64) -> Scale {
    if (scale - scale.round()).abs() < f64::EPSILON {
        Scale::Integer(scale.round() as i32)
    } else {
        Scale::Fractional(scale)
    }
}

/// The point of `rect` closest to `p`.
fn nearest_in(rect: Rectangle<i32, Logical>, p: Point<f64, Logical>) -> Point<f64, Logical> {
    Point::from((
        p.x.clamp(rect.loc.x as f64, (rect.loc.x + rect.size.w) as f64 - 1.0),
        p.y.clamp(rect.loc.y as f64, (rect.loc.y + rect.size.h) as f64 - 1.0),
    ))
}

fn squared_distance(a: Point<f64, Logical>, b: Point<f64, Logical>) -> f64 {
    (a.x - b.x).powi(2) + (a.y - b.y).powi(2)
}

impl Patina {
    pub fn outputs(&self) -> Vec<Output> {
        self.space.outputs().cloned().collect()
    }

    /// The smallest rectangle holding every output.
    pub fn layout_bounds(&self) -> Option<Rectangle<i32, Logical>> {
        self.space
            .outputs()
            .filter_map(|o| self.space.output_geometry(o))
            .reduce(|a, b| a.merge(b))
    }

    /// Where the next output goes: to the right of the others.
    pub fn next_output_x(&self) -> i32 {
        self.space
            .outputs()
            .filter_map(|o| self.space.output_geometry(o))
            .map(|g| g.loc.x + g.size.w)
            .max()
            .unwrap_or(0)
    }

    /// Moves `p` onto the closest output, so the pointer cannot leave the screens or sit in a
    /// gap between two of different heights.
    pub fn clamp_to_outputs(&self, p: Point<f64, Logical>) -> Point<f64, Logical> {
        self.space
            .outputs()
            .filter_map(|o| self.space.output_geometry(o))
            .map(|g| nearest_in(g, p))
            .min_by(|a, b| squared_distance(*a, p).total_cmp(&squared_distance(*b, p)))
            .unwrap_or(p)
    }

    /// The output showing `p`, or the closest one.
    pub fn output_at(&self, p: Point<f64, Logical>) -> Option<Output> {
        self.space
            .outputs()
            .filter_map(|o| self.space.output_geometry(o).map(|g| (o, g)))
            .min_by(|(_, a), (_, b)| {
                squared_distance(nearest_in(*a, p), p)
                    .total_cmp(&squared_distance(nearest_in(*b, p), p))
            })
            .map(|(o, _)| o.clone())
    }

    /// The output the pointer is on; the first one when there is no pointer yet.
    pub fn pointer_output(&self) -> Option<Output> {
        let pos = self.seat.get_pointer().map(|p| p.current_location())?;
        self.output_at(pos)
    }

    /// The output with the most of `window` on it.
    pub fn output_for_window(&self, window: &Window) -> Option<Output> {
        let frame = self.frame_rect(window)?;
        let centre = Point::from((
            (frame.x + frame.w / 2) as f64,
            (frame.y + frame.h / 2) as f64,
        ));
        self.output_at(centre)
    }

    /// The part of `output` not covered by panels and docks, in compositor space.
    pub fn zone_of(&self, output: &Output) -> Rectangle<i32, Logical> {
        let loc = self
            .space
            .output_geometry(output)
            .map(|g| g.loc)
            .unwrap_or_default();
        let mut zone = layer_map_for_output(output).non_exclusive_zone();
        zone.loc += loc;
        zone
    }

    /// The usable zone of the output the pointer is on.
    pub fn usable_zone(&self) -> Rectangle<i32, Logical> {
        self.pointer_output()
            .or_else(|| self.space.outputs().next().cloned())
            .map(|o| self.zone_of(&o))
            .unwrap_or_default()
    }

    /// The usable zone of the output `window` is on.
    pub fn zone_for_window(&self, window: &Window) -> Rectangle<i32, Logical> {
        self.output_for_window(window)
            .or_else(|| self.space.outputs().next().cloned())
            .map(|o| self.zone_of(&o))
            .unwrap_or_default()
    }

    /// Tells every window and panel the scale of the output it is on, so clients draw at the
    /// right density (`wp_fractional_scale` for those that support it, the integer buffer scale
    /// for the rest). Cheap: nothing is sent unless it changed.
    pub fn update_surface_scales(&self) {
        let first = self.space.outputs().next().cloned();
        let tell = |output: &Output| {
            let scale = output.current_scale();
            let (int, frac) = (scale.integer_scale(), scale.fractional_scale());
            move |surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
                  states: &smithay::wayland::compositor::SurfaceData| {
                send_surface_state(surface, states, int, Transform::Normal);
                with_fractional_scale(states, |fs| fs.set_preferred_scale(frac));
            }
        };
        for window in self.space.elements() {
            if let Some(output) = self.output_for_window(window).or_else(|| first.clone()) {
                window.with_surfaces(tell(&output));
            }
        }
        for output in self.space.outputs() {
            for layer in layer_map_for_output(output).layers() {
                layer.with_surfaces(tell(output));
            }
            if let Some(surface) = self.lock_surface(output) {
                smithay::wayland::compositor::with_surface_tree_downward(
                    &surface,
                    (),
                    |_, _, _| smithay::wayland::compositor::TraversalAction::DoChildren(()),
                    |s, states, _| tell(output)(s, states),
                    |_, _, _| true,
                );
            }
        }
    }

    /// Starts the per-output command (the panel) for `output`.
    pub fn spawn_panel(&mut self, output: &Output) {
        let Some(cmd) = self.per_output_cmd.clone() else {
            return;
        };
        let name = output.name();
        match std::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .env("WAYLAND_DISPLAY", &self.socket_name)
            .env("PATINA_OUTPUT", &name)
            .spawn()
        {
            Ok(child) => {
                tracing::info!(%cmd, output = %name, "started for output");
                self.panels.push((name, child));
            }
            Err(e) => tracing::warn!(%cmd, output = %name, error = %e, "failed to start"),
        }
    }

    /// Stops the per-output command of an output that is gone.
    pub fn stop_panel(&mut self, name: &str) {
        let mut i = 0;
        while i < self.panels.len() {
            if self.panels[i].0 == name {
                let (_, mut child) = self.panels.swap_remove(i);
                let _ = child.kill();
                let _ = child.wait();
            } else {
                i += 1;
            }
        }
    }

    /// Cleans up after `output` was unplugged: windows that were mostly on it move to the first
    /// output (remembering where they were, for when it comes back), layer surfaces of other
    /// clients are told to close, and its panel stops.
    pub fn output_removed(&mut self, output: &Output) {
        let name = output.name();
        self.stop_panel(&name);
        let layers: Vec<_> = layer_map_for_output(output).layers().cloned().collect();
        for layer in layers {
            layer.layer_surface().send_close();
        }
        let area = self.space.output_geometry(output);
        let leaving: Vec<Window> = self
            .space
            .elements()
            .filter(|w| self.output_for_window(w).as_ref() == Some(output))
            .cloned()
            .collect();
        self.space.unmap_output(output);

        let Some(first) = self.space.outputs().next().cloned() else {
            return;
        };
        let zone = self.zone_of(&first);
        for (n, window) in leaving.into_iter().enumerate() {
            if let (Some(area), Some(rect)) = (area, self.client_rect(&window)) {
                crate::wm::win_state(&window).borrow_mut().home =
                    Some((name.clone(), rect.loc - area.loc));
            }
            let at = (
                zone.loc.x + 96 + 32 * n as i32,
                zone.loc.y + TITLEBAR_H + 28 + 32 * n as i32,
            );
            self.space.map_element(window, at, false);
        }
        self.fit_windows_to_zone();
        self.refresh_hover();
        self.queue_redraw();
    }

    /// An output came (back): windows that were moved off it when it went return to where they
    /// were on it.
    pub fn output_returned(&mut self, output: &Output) {
        let Some(area) = self.space.output_geometry(output) else {
            return;
        };
        let name = output.name();
        let returning: Vec<(Window, Point<i32, Logical>)> = self
            .space
            .elements()
            .filter_map(|w| {
                let home = crate::wm::win_state(w).borrow().home.clone()?;
                (home.0 == name).then(|| (w.clone(), home.1))
            })
            .collect();
        for (window, offset) in returning {
            crate::wm::win_state(&window).borrow_mut().home = None;
            self.space.map_element(window, area.loc + offset, false);
        }
        self.fit_windows_to_zone();
        self.refresh_hover();
        self.queue_redraw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Logical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    #[test]
    fn a_point_inside_stays_put() {
        let p = Point::from((10.0, 20.0));
        assert_eq!(nearest_in(r(0, 0, 100, 100), p), p);
    }

    #[test]
    fn a_point_outside_moves_to_the_edge() {
        let p = nearest_in(r(1280, 0, 1280, 800), Point::from((3000.0, -5.0)));
        assert_eq!(p, Point::from((2559.0, 0.0)));
    }

    #[test]
    fn ordinary_monitors_are_100_percent() {
        // 24 inch 1080p: about 92 dpi.
        assert_eq!(auto_scale((1920, 1080), (531, 299)), 1.0);
    }

    #[test]
    fn dense_monitors_scale_up_in_quarters() {
        // 27 inch 4K: about 163 dpi, 170% rounds to 175%.
        assert_eq!(auto_scale((3840, 2160), (597, 336)), 1.75);
        // 13 inch 2560x1600 laptop: about 227 dpi, capped at 200%.
        assert_eq!(auto_scale((2560, 1600), (286, 179)), 2.0);
        // 14 inch 1080p laptop: about 157 dpi, 175%.
        assert_eq!(auto_scale((1920, 1080), (310, 174)), 1.75);
    }

    #[test]
    fn missing_or_silly_sizes_mean_100_percent() {
        assert_eq!(auto_scale((1280, 800), (0, 0)), 1.0);
        assert_eq!(auto_scale((1920, 1080), (16, 9)), 1.0);
    }

    #[test]
    fn the_override_applies_to_all_or_by_name() {
        assert_eq!(scale_override("1.5", "eDP-1"), Some(1.5));
        assert_eq!(scale_override("eDP-1=2, HDMI-A-1=1", "eDP-1"), Some(2.0));
        assert_eq!(scale_override("eDP-1=2,HDMI-A-1=1", "HDMI-A-1"), Some(1.0));
        assert_eq!(scale_override("eDP-1=2", "DP-1"), None);
        assert_eq!(scale_override("banana", "DP-1"), None);
        assert_eq!(scale_override("9", "DP-1"), None);
    }

    #[test]
    fn whole_scales_are_integers() {
        assert!(matches!(output_scale(2.0), Scale::Integer(2)));
        assert!(matches!(output_scale(1.5), Scale::Fractional(f) if f == 1.5));
    }

    #[test]
    fn distance_is_squared_euclidean() {
        assert_eq!(
            squared_distance(Point::from((0.0, 0.0)), Point::from((3.0, 4.0))),
            25.0
        );
    }
}
