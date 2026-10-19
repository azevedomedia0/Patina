//! Where the outputs (monitors) are and what lives on them.
//!
//! Outputs sit side by side in one coordinate space, in the order they were connected. Windows
//! live in that space; each output draws the part of it that falls on its rectangle. Panels are
//! layer surfaces, which smithay keeps per output.

use patina_frame::TITLEBAR_H;
use smithay::{
    desktop::{layer_map_for_output, Window},
    output::Output,
    utils::{Logical, Point, Rectangle},
};

use crate::state::Patina;

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

    /// Cleans up after `output` was unplugged: windows that were on it come to the first output,
    /// layer surfaces of other clients are told to close, and its panel stops.
    pub fn output_removed(&mut self, output: &Output) {
        let name = output.name();
        self.stop_panel(&name);
        let layers: Vec<_> = layer_map_for_output(output).layers().cloned().collect();
        for layer in layers {
            layer.layer_surface().send_close();
        }
        self.space.unmap_output(output);

        // Windows with nothing left on screen go to the first output, below its panel.
        let Some(first) = self.space.outputs().next().cloned() else {
            return;
        };
        let zone = self.zone_of(&first);
        let stranded: Vec<Window> = self
            .space
            .elements()
            .filter(|w| {
                self.frame_rect(w).is_some_and(|f| {
                    let r = Rectangle::<i32, Logical>::new((f.x, f.y).into(), (f.w, f.h).into());
                    !self
                        .space
                        .outputs()
                        .filter_map(|o| self.space.output_geometry(o))
                        .any(|g| g.overlaps(r))
                })
            })
            .cloned()
            .collect();
        for (n, window) in stranded.into_iter().enumerate() {
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
    fn distance_is_squared_euclidean() {
        assert_eq!(
            squared_distance(Point::from((0.0, 0.0)), Point::from((3.0, 4.0))),
            25.0
        );
    }
}
