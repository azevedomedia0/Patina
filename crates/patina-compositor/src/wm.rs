//! Window management: frames, hit-testing, move, resize, minimise, maximise, close.

use std::{cell::RefCell, time::Instant};

use patina_frame::{hit_test, Control, Hit, Rect, TitleBar, TITLEBAR_H};
use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    desktop::{layer_map_for_output, Window, WindowSurfaceType},
    input::pointer::{CursorIcon, CursorImageStatus},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{IsAlive, Logical, Point, Rectangle, Size, SERIAL_COUNTER},
    wayland::{
        compositor::with_states,
        shell::{
            wlr_layer::Layer,
            xdg::{ToplevelSurface, XdgToplevelSurfaceData},
        },
    },
};

use crate::state::Patina;

/// Two clicks on a titlebar within this time maximise or restore the window.
const DOUBLE_CLICK_MS: u128 = 400;

/// A drawn bitmap and its size in pixels, which smithay needs to map all of it onto a
/// logical size (it would otherwise crop to the logical size).
pub struct Image {
    pub buffer: MemoryRenderBuffer,
    pub pixels: (i32, i32),
}

/// What a cached shadow was drawn for: frame width, frame height, active, scale (`f32` bits).
pub type ShadowKey = (i32, i32, bool, u32);

/// Per-window compositor state, stored in the window's user data.
#[derive(Default)]
pub struct WinState {
    /// Keyboard focus is given when the window first shows a buffer, never before.
    pub focused_once: bool,
    pub maximized: bool,
    /// Client rectangle to return to when un-maximising.
    pub restore: Option<Rectangle<i32, Logical>>,
    pub hover: Option<Control>,
    /// Cached titlebar textures and the descriptions they were drawn from: one per output
    /// scale the window is on (at most two).
    pub bars: Vec<(TitleBar, Image)>,
    /// Cached shadow textures: (frame width, frame height, active, scale bits) each was drawn
    /// for, the texture, and its padding around the window (logical pixels).
    pub shadows: Vec<(ShadowKey, Image, i32)>,
    /// The window has been given its place on screen (done once it knows its size).
    pub placed: bool,
    /// The output a window was moved off when that output was unplugged, and where on it the
    /// window was (relative to its corner). It goes back there when the output returns, unless
    /// the user has moved it since.
    pub home: Option<(String, Point<i32, Logical>)>,
}

/// Whether the initial configure has gone out. Until the client's first commit, configures
/// must not be sent: the initial one is sent from the commit handler.
pub fn initial_configure_sent(toplevel: &ToplevelSurface) -> bool {
    with_states(toplevel.wl_surface(), |states| {
        states
            .data_map
            .get::<XdgToplevelSurfaceData>()
            .is_some_and(|d| d.lock().unwrap().initial_configure_sent)
    })
}

pub fn win_state(window: &Window) -> &RefCell<WinState> {
    window
        .user_data()
        .get_or_insert(|| RefCell::new(WinState::default()))
}

pub enum Drag {
    Move {
        window: Window,
        pointer_start: Point<f64, Logical>,
        window_start: Point<i32, Logical>,
    },
    Resize {
        window: Window,
        pointer_start: Point<f64, Logical>,
        size_start: Size<i32, Logical>,
    },
}

/// What is under the pointer.
pub enum Pick {
    /// A Wayland surface (layer or window client) and its origin in compositor space.
    Surface(WlSurface, Point<f64, Logical>, Option<Window>),
    /// Part of a window's frame, handled by the compositor.
    Frame(Window, Hit),
    Nothing,
}

/// What to draw as the pointer.
pub enum CursorSource {
    Hidden,
    /// A named cursor: the design's arrow for `Default`, the system theme for the rest.
    Icon(CursorIcon),
    /// A surface supplied by the client under the pointer.
    Surface(WlSurface),
}

#[derive(Debug, Clone, Copy)]
pub enum Action {
    /// A Super tap: open or close the launcher.
    ToggleLauncher,
    RestoreMinimized,
    /// Switch to virtual terminal `n` (Ctrl+Alt+Fn).
    VtSwitch(i32),
    /// Switch to workspace `n`, counting from zero (Super+1..4).
    SwitchWorkspace(usize),
    /// Move the focused window to workspace `n` and follow it (Super+Shift+1..4).
    MoveToWorkspace(usize),
}

impl Patina {
    /// Client area of `window` in compositor space.
    pub fn client_rect(&self, window: &Window) -> Option<Rectangle<i32, Logical>> {
        self.space.element_geometry(window)
    }

    /// Titlebar and client area together.
    pub fn frame_rect(&self, window: &Window) -> Option<Rect> {
        let g = self.client_rect(window)?;
        Some(Rect::new(
            g.loc.x,
            g.loc.y - TITLEBAR_H,
            g.size.w,
            g.size.h + TITLEBAR_H,
        ))
    }

    /// Top-most thing at `pos`: overlay and top layers, then windows, then bottom layers.
    pub fn pick(&self, pos: Point<f64, Logical>) -> Pick {
        let Some(output) = self.output_at(pos) else {
            return Pick::Nothing;
        };
        // Layer geometry is relative to its output.
        let origin = self
            .space
            .output_geometry(&output)
            .map(|g| g.loc)
            .unwrap_or_default();
        let local = pos - origin.to_f64();
        let map = layer_map_for_output(&output);
        let layer_hit = |layers: &[Layer]| {
            layers.iter().find_map(|&layer| {
                let surface = map.layer_under(layer, local)?;
                let geo = map.layer_geometry(surface)?;
                let (wl, loc) =
                    surface.surface_under(local - geo.loc.to_f64(), WindowSurfaceType::ALL)?;
                Some(Pick::Surface(wl, (loc + geo.loc + origin).to_f64(), None))
            })
        };

        if let Some(hit) = layer_hit(&[Layer::Overlay, Layer::Top]) {
            return hit;
        }
        for window in self.space.elements().rev() {
            // A window's menus first: they may reach outside its frame.
            let origin =
                self.space.element_location(window).unwrap_or_default() - window.geometry().loc;
            if let Some((wl, loc)) = window.surface_under(
                pos - origin.to_f64(),
                WindowSurfaceType::POPUP | WindowSurfaceType::SUBSURFACE,
            ) {
                return Pick::Surface(wl, (loc + origin).to_f64(), Some(window.clone()));
            }
            let Some(frame) = self.frame_rect(window) else {
                continue;
            };
            match hit_test(frame, pos.x, pos.y) {
                Some(Hit::Client) => {
                    let origin = self.space.element_location(window).unwrap_or_default()
                        - window.geometry().loc;
                    if let Some((wl, loc)) =
                        window.surface_under(pos - origin.to_f64(), WindowSurfaceType::ALL)
                    {
                        return Pick::Surface(wl, (loc + origin).to_f64(), Some(window.clone()));
                    }
                }
                Some(hit) => return Pick::Frame(window.clone(), hit),
                None => {}
            }
        }
        layer_hit(&[Layer::Bottom, Layer::Background]).unwrap_or(Pick::Nothing)
    }

    /// The cursor to show now: compositor-owned areas first, otherwise what the client under
    /// the pointer asked for.
    pub fn effective_cursor(&self) -> CursorSource {
        if let Some(icon) = self.frame_cursor {
            return CursorSource::Icon(icon);
        }
        match &self.cursor_status {
            CursorImageStatus::Hidden => CursorSource::Hidden,
            CursorImageStatus::Named(icon) => CursorSource::Icon(*icon),
            CursorImageStatus::Surface(surface) if surface.alive() => {
                CursorSource::Surface(surface.clone())
            }
            CursorImageStatus::Surface(_) => CursorSource::Icon(CursorIcon::Default),
        }
    }

    pub fn update_hover(&mut self, pick: &Pick) {
        let hovered = match pick {
            Pick::Frame(w, Hit::Control(c)) => Some((w.clone(), *c)),
            _ => None,
        };
        for w in self.space.elements() {
            let want = hovered.as_ref().filter(|(hw, _)| hw == w).map(|(_, c)| *c);
            let mut st = win_state(w).borrow_mut();
            if st.hover != want {
                st.hover = want;
            }
        }
    }

    /// Re-evaluates hover under the current pointer position, for when windows move or
    /// disappear beneath a stationary pointer.
    pub fn refresh_hover(&mut self) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let pick = self.pick(pointer.current_location());
        self.update_hover(&pick);
    }

    /// Moves windows out from under panels and refits maximised ones, after the area reserved
    /// by layer-shell surfaces changed.
    pub fn fit_windows_to_zone(&mut self) {
        let windows: Vec<Window> = self.space.elements().cloned().collect();
        for window in windows {
            let Some(rect) = self.client_rect(&window) else {
                continue;
            };
            let zone = self.zone_for_window(&window);
            if win_state(&window).borrow().maximized {
                let area = Rectangle::new(
                    (zone.loc.x, zone.loc.y + TITLEBAR_H).into(),
                    (zone.size.w, zone.size.h - TITLEBAR_H).into(),
                );
                if rect != area {
                    if let Some(t) = window.toplevel() {
                        t.with_pending_state(|s| s.size = Some(area.size));
                        if initial_configure_sent(t) {
                            t.send_pending_configure();
                        }
                    }
                    self.space.map_element(window, area.loc, false);
                }
            } else if rect.loc.y - TITLEBAR_H < zone.loc.y {
                let loc = Point::from((rect.loc.x, zone.loc.y + TITLEBAR_H));
                self.space.map_element(window, loc, false);
            }
        }
    }

    /// Gives a new window its place, once its size is known: centred in the free area of the
    /// output the pointer is on, stepped 32 px right and down while another window already
    /// starts there, and kept inside that area (a window too big for it starts at its top-left).
    pub fn place_new_window(&mut self, window: &Window) {
        if std::mem::replace(&mut win_state(window).borrow_mut().placed, true) {
            return;
        }
        let Some(rect) = self.client_rect(window) else {
            return;
        };
        let zone = self.usable_zone();
        let others: Vec<Point<i32, Logical>> = self
            .space
            .elements()
            .filter(|w| *w != window)
            .filter_map(|w| self.client_rect(w).map(|r| r.loc))
            .collect();
        let loc = placement(zone, rect.size, &others);
        self.space.map_element(window.clone(), loc, false);
    }

    pub fn title_bar_for(&self, window: &Window, width: i32, scale: f32) -> TitleBar {
        let title = window
            .toplevel()
            .and_then(|t| {
                with_states(t.wl_surface(), |s| {
                    s.data_map
                        .get::<XdgToplevelSurfaceData>()
                        .and_then(|d| d.lock().unwrap().title.clone())
                })
            })
            .unwrap_or_default();
        let active = window.toplevel().is_some_and(|t| {
            t.with_pending_state(|s| s.states.contains(xdg_toplevel::State::Activated))
        });
        let st = win_state(window).borrow();
        TitleBar {
            width,
            title,
            active,
            hover: st.hover,
            maximized: st.maximized,
            scale,
        }
    }

    /// The window that has keyboard focus, if any.
    pub fn focused_window(&self) -> Option<Window> {
        let surface = self.seat.get_keyboard()?.current_focus()?;
        self.space
            .elements()
            .find(|w| w.toplevel().is_some_and(|t| t.wl_surface() == &surface))
            .cloned()
    }

    pub fn begin_move(&mut self, window: &Window, pos: Point<f64, Logical>) {
        tracing::debug!(?pos, "move begin");
        // Moved by hand: it lives where the user puts it now.
        win_state(window).borrow_mut().home = None;
        if win_state(window).borrow().maximized {
            return;
        }
        if let Some(r) = self.client_rect(window) {
            self.drag = Some(Drag::Move {
                window: window.clone(),
                pointer_start: pos,
                window_start: r.loc,
            });
        }
    }

    pub fn begin_resize(&mut self, window: &Window, pos: Point<f64, Logical>) {
        if win_state(window).borrow().maximized {
            return;
        }
        let (Some(r), Some(t)) = (self.client_rect(window), window.toplevel()) else {
            return;
        };
        tracing::debug!(size = ?r.size, "resize begin");
        t.with_pending_state(|s| s.states.set(xdg_toplevel::State::Resizing));
        t.send_pending_configure();
        self.drag = Some(Drag::Resize {
            window: window.clone(),
            pointer_start: pos,
            size_start: r.size,
        });
    }

    pub fn apply_drag(&mut self, pos: Point<f64, Logical>) {
        let zone = self.usable_zone();
        match &self.drag {
            Some(Drag::Move {
                window,
                pointer_start,
                window_start,
            }) => {
                let d = pos - *pointer_start;
                let mut loc = Point::from((
                    window_start.x + d.x.round() as i32,
                    window_start.y + d.y.round() as i32,
                ));
                // Keep the titlebar reachable below the panel.
                loc.y = loc.y.max(zone.loc.y + TITLEBAR_H);
                let window = window.clone();
                self.space.map_element(window, loc, false);
            }
            Some(Drag::Resize {
                window,
                pointer_start,
                size_start,
            }) => {
                let d = pos - *pointer_start;
                let (min_w, min_h) = patina_frame::geometry::MIN_CLIENT;
                let size = Size::from((
                    (size_start.w + d.x.round() as i32).max(min_w),
                    (size_start.h + d.y.round() as i32).max(min_h),
                ));
                tracing::debug!(?size, "resize configure");
                if let Some(t) = window.toplevel() {
                    t.with_pending_state(|s| s.size = Some(size));
                    t.send_pending_configure();
                }
            }
            None => {}
        }
    }

    pub fn end_drag(&mut self) {
        if let Some(Drag::Resize { window, .. }) = self.drag.take() {
            if let Some(t) = window.toplevel() {
                t.with_pending_state(|s| s.states.unset(xdg_toplevel::State::Resizing));
                t.send_pending_configure();
            }
        }
    }

    pub fn toggle_maximize(&mut self, window: &Window) {
        tracing::debug!("toggle maximize");
        let Some(t) = window.toplevel() else { return };
        let Some(current) = self.client_rect(window) else {
            return;
        };
        let maximized = win_state(window).borrow().maximized;
        if maximized {
            let restore = win_state(window)
                .borrow_mut()
                .restore
                .take()
                .unwrap_or(current);
            t.with_pending_state(|s| {
                s.states.unset(xdg_toplevel::State::Maximized);
                s.size = Some(restore.size);
            });
            win_state(window).borrow_mut().maximized = false;
            self.space.map_element(window.clone(), restore.loc, false);
        } else {
            let zone = self.zone_for_window(window);
            let area = Rectangle::new(
                (zone.loc.x, zone.loc.y + TITLEBAR_H).into(),
                (zone.size.w, zone.size.h - TITLEBAR_H).into(),
            );
            t.with_pending_state(|s| {
                s.states.set(xdg_toplevel::State::Maximized);
                s.size = Some(area.size);
            });
            let mut st = win_state(window).borrow_mut();
            st.restore = Some(current);
            st.maximized = true;
            drop(st);
            self.space.map_element(window.clone(), area.loc, false);
        }
        t.send_pending_configure();
        self.refresh_hover();
    }

    pub fn minimize(&mut self, window: &Window) {
        tracing::debug!("minimize");
        if let Some(r) = self.client_rect(window) {
            self.minimized.push((window.clone(), r.loc));
            self.space.unmap_elem(window);
            let next = self.space.elements().next_back().cloned();
            if let Some(next) = next {
                self.focus_window(&next);
            } else if let Some(kb) = self.seat.get_keyboard() {
                kb.set_focus(self, None, SERIAL_COUNTER.next_serial());
            }
            self.refresh_hover();
        }
    }

    /// Temporary until the dock exists: Alt+M brings back the last minimised window.
    pub fn restore_minimized(&mut self) {
        if let Some((window, loc)) = self.minimized.pop() {
            self.space.map_element(window.clone(), loc, true);
            self.focus_window(&window);
        }
    }

    pub fn close(&mut self, window: &Window) {
        tracing::debug!("close");
        if let Some(t) = window.toplevel() {
            t.send_close();
        }
    }

    /// Titlebar press. Returns true when it was a double-click.
    pub fn titlebar_double_click(&mut self, window: &Window) -> bool {
        let now = Instant::now();
        let double = self.last_titlebar_click.as_ref().is_some_and(|(t, w)| {
            w == window && now.duration_since(*t).as_millis() < DOUBLE_CLICK_MS
        });
        self.last_titlebar_click = if double {
            None
        } else {
            Some((now, window.clone()))
        };
        double
    }
}

/// Where a new window with a client area of `size` goes in `zone` (the free area of an output),
/// given where the other windows' client areas start. Returns the client area's top-left; the
/// titlebar sits above it.
pub fn placement(
    zone: Rectangle<i32, Logical>,
    size: Size<i32, Logical>,
    others: &[Point<i32, Logical>],
) -> Point<i32, Logical> {
    const STEP: i32 = 32;
    let (fw, fh) = (size.w, size.h + TITLEBAR_H);
    let keep_in = |x: i32, y: i32| {
        let max_x = (zone.loc.x + zone.size.w - fw).max(zone.loc.x);
        let max_y = (zone.loc.y + zone.size.h - fh).max(zone.loc.y);
        (x.clamp(zone.loc.x, max_x), y.clamp(zone.loc.y, max_y))
    };
    let (mut x, mut y) = keep_in(
        zone.loc.x + (zone.size.w - fw) / 2,
        zone.loc.y + (zone.size.h - fh) / 2,
    );
    let taken = |x: i32, y: i32| {
        others
            .iter()
            .any(|p| (p.x - x).abs() < STEP / 2 && (p.y - TITLEBAR_H - y).abs() < STEP / 2)
    };
    for _ in 0..64 {
        if !taken(x, y) {
            break;
        }
        let (nx, ny) = keep_in(x + STEP, y + STEP);
        // At the bottom-right corner of the area: start again from its top-left.
        (x, y) = if (nx, ny) == (x, y) {
            (zone.loc.x, zone.loc.y)
        } else {
            (nx, ny)
        };
    }
    Point::from((x, y + TITLEBAR_H))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone() -> Rectangle<i32, Logical> {
        Rectangle::new((0, 36).into(), (1280, 764).into())
    }

    #[test]
    fn the_first_window_is_centred_below_the_panel() {
        let p = placement(zone(), Size::from((600, 400)), &[]);
        // Frame 600 x 438 centred in the zone; the client area starts below the titlebar.
        assert_eq!(p, Point::from((340, 36 + (764 - 438) / 2 + TITLEBAR_H)));
    }

    #[test]
    fn the_next_window_steps_down_and_right() {
        let first = placement(zone(), Size::from((600, 400)), &[]);
        let second = placement(zone(), Size::from((600, 400)), &[first]);
        assert_eq!(second, Point::from((first.x + 32, first.y + 32)));
        let third = placement(zone(), Size::from((600, 400)), &[first, second]);
        assert_eq!(third, Point::from((first.x + 64, first.y + 64)));
    }

    #[test]
    fn a_window_too_big_for_the_area_starts_at_its_top_left() {
        let p = placement(zone(), Size::from((2000, 1200)), &[]);
        assert_eq!(p, Point::from((0, 36 + TITLEBAR_H)));
    }

    #[test]
    fn windows_never_leave_the_area() {
        let mut others = Vec::new();
        for _ in 0..40 {
            let p = placement(zone(), Size::from((900, 600)), &others);
            assert!(p.x >= 0 && p.x + 900 <= 1280);
            assert!(p.y - TITLEBAR_H >= 36 && p.y + 600 <= 800);
            others.push(p);
        }
    }
}
