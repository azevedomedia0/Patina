//! Window management: frames, hit-testing, move, resize, minimise, maximise, close.

use std::{cell::RefCell, time::Instant};

use patina_frame::{hit_test, Control, Hit, Rect, TitleBar, TITLEBAR_H};
use smithay::{
    backend::renderer::element::memory::MemoryRenderBuffer,
    desktop::{layer_map_for_output, Window, WindowSurfaceType},
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{Logical, Point, Rectangle, Size, SERIAL_COUNTER},
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

/// Per-window compositor state, stored in the window's user data.
#[derive(Default)]
pub struct WinState {
    /// Keyboard focus is given when the window first shows a buffer, never before.
    pub focused_once: bool,
    pub maximized: bool,
    /// Client rectangle to return to when un-maximising.
    pub restore: Option<Rectangle<i32, Logical>>,
    pub hover: Option<Control>,
    /// Cached titlebar texture and the description it was drawn from.
    pub bar: Option<(TitleBar, MemoryRenderBuffer)>,
    /// Cached shadow texture: (frame width, frame height, active) it was drawn for, the
    /// texture, and its padding around the window.
    pub shadow: Option<((i32, i32, bool), MemoryRenderBuffer, i32)>,
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

#[derive(Debug, Clone, Copy)]
pub enum Action {
    RestoreMinimized,
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

    /// Area not covered by panels and docks.
    pub fn usable_zone(&self) -> Rectangle<i32, Logical> {
        self.output
            .as_ref()
            .map(|o| layer_map_for_output(o).non_exclusive_zone())
            .unwrap_or_default()
    }

    /// Top-most thing at `pos`: overlay and top layers, then windows, then bottom layers.
    pub fn pick(&self, pos: Point<f64, Logical>) -> Pick {
        let Some(output) = self.output.as_ref() else {
            return Pick::Nothing;
        };
        let map = layer_map_for_output(output);
        let layer_hit = |layers: &[Layer]| {
            layers.iter().find_map(|&layer| {
                let surface = map.layer_under(layer, pos)?;
                let geo = map.layer_geometry(surface)?;
                let (wl, loc) =
                    surface.surface_under(pos - geo.loc.to_f64(), WindowSurfaceType::ALL)?;
                Some(Pick::Surface(wl, (loc + geo.loc).to_f64(), None))
            })
        };

        if let Some(hit) = layer_hit(&[Layer::Overlay, Layer::Top]) {
            return hit;
        }
        for window in self.space.elements().rev() {
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

    pub fn title_bar_for(&self, window: &Window, width: i32) -> TitleBar {
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
        }
    }

    pub fn begin_move(&mut self, window: &Window, pos: Point<f64, Logical>) {
        tracing::debug!(?pos, "move begin");
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
            let zone = self.usable_zone();
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
