//! Keyboard focus for layer surfaces, and closing the shell's transient surfaces.
//!
//! A layer surface that asks for exclusive keyboard focus (the launcher's search field) gets it
//! while it is mapped, and focus goes back where it was when it goes. Shell surfaces that close
//! on an outside press (`patina_frame::shell::closes_on_outside_press`) are told to close when
//! the user presses somewhere that is not theirs.

use smithay::{
    backend::renderer::utils::with_renderer_surface_state,
    desktop::{layer_map_for_output, WindowSurfaceType},
    output::Output,
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    utils::SERIAL_COUNTER,
    wayland::shell::wlr_layer::{KeyboardInteractivity, Layer},
};

use crate::state::Patina;

impl Patina {
    /// After a layer surface committed: give it the keyboard if it asks for it exclusively.
    pub fn layer_committed(&mut self, output: &Output, surface: &WlSurface) {
        let wants = {
            let map = layer_map_for_output(output);
            let Some(layer) = map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL) else {
                return;
            };
            layer.cached_state().keyboard_interactivity == KeyboardInteractivity::Exclusive
                && matches!(layer.layer(), Layer::Top | Layer::Overlay)
        };
        let mapped =
            with_renderer_surface_state(surface, |s| s.buffer().is_some()).unwrap_or(false);
        if !(wants && mapped) {
            return;
        }
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        let current = keyboard.current_focus();
        if current.as_ref() == Some(surface) {
            return;
        }
        self.focus_before_layer = current;
        keyboard.set_focus(self, Some(surface.clone()), SERIAL_COUNTER.next_serial());
    }

    /// Gives a focusable layer surface the keyboard when it is clicked (on-demand interactivity).
    pub fn layer_clicked(&mut self, surface: &WlSurface) {
        let interactive = self.outputs().into_iter().any(|o| {
            layer_map_for_output(&o)
                .layer_for_surface(surface, WindowSurfaceType::ALL)
                .is_some_and(|l| {
                    l.cached_state().keyboard_interactivity != KeyboardInteractivity::None
                })
        });
        if !interactive {
            return;
        }
        if let Some(keyboard) = self.seat.get_keyboard() {
            if keyboard.current_focus().as_ref() != Some(surface) {
                self.focus_before_layer = keyboard.current_focus();
                keyboard.set_focus(self, Some(surface.clone()), SERIAL_COUNTER.next_serial());
            }
        }
    }

    /// A layer surface went away: if it had the keyboard, give it back.
    pub fn layer_gone(&mut self, surface: &WlSurface) {
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };
        if keyboard.current_focus().as_ref() != Some(surface) {
            return;
        }
        let back = self
            .focus_before_layer
            .take()
            .filter(|s| s.is_alive() && s != surface);
        match back {
            Some(s) => keyboard.set_focus(self, Some(s), SERIAL_COUNTER.next_serial()),
            None => {
                let top = self.space.elements().next_back().cloned();
                match top {
                    Some(w) => self.focus_window(&w),
                    None => keyboard.set_focus(self, None, SERIAL_COUNTER.next_serial()),
                }
            }
        }
    }

    /// A press landed on `target` (or on nothing the clients own): close the shell's
    /// notification centre and menus, unless the press is on the program that opened them.
    pub fn close_transient_layers(&mut self, target: Option<&WlSurface>) {
        let target_client = target.and_then(|s| s.client());
        for output in self.outputs() {
            let map = layer_map_for_output(&output);
            for layer in map.layers() {
                if !patina_frame::shell::closes_on_outside_press(layer.namespace()) {
                    continue;
                }
                let owner = layer.wl_surface().client();
                if owner.is_some() && owner == target_client {
                    continue;
                }
                layer.layer_surface().send_close();
            }
        }
    }
}
