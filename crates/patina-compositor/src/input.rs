//! Pointer and keyboard input from the backend.

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    desktop::{layer_map_for_output, WindowSurfaceType},
    input::{
        keyboard::FilterResult,
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, SERIAL_COUNTER},
    wayland::shell::wlr_layer::Layer,
};

use crate::state::Patina;

impl Patina {
    /// The surface under `pos`, with that surface's origin in compositor space.
    /// Order: overlay and top layers, windows, then bottom and background layers.
    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        let output = self.output.as_ref()?;
        let map = layer_map_for_output(output);

        let layer_hit = |layers: &[Layer]| {
            layers.iter().find_map(|&layer| {
                let surface = map.layer_under(layer, pos)?;
                let geo = map.layer_geometry(surface)?;
                let (wl, loc) =
                    surface.surface_under(pos - geo.loc.to_f64(), WindowSurfaceType::ALL)?;
                Some((wl, (loc + geo.loc).to_f64()))
            })
        };

        if let Some(hit) = layer_hit(&[Layer::Overlay, Layer::Top]) {
            return Some(hit);
        }
        if let Some((window, origin)) = self.space.element_under(pos) {
            let (wl, loc) = window.surface_under(pos - origin.to_f64(), WindowSurfaceType::ALL)?;
            return Some((wl, (loc + origin).to_f64()));
        }
        layer_hit(&[Layer::Bottom, Layer::Background])
    }

    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };

        match event {
            InputEvent::Keyboard { event } => {
                keyboard.input::<(), _>(
                    self,
                    event.key_code(),
                    event.state(),
                    SERIAL_COUNTER.next_serial(),
                    event.time_msec(),
                    |_, _, _| FilterResult::Forward,
                );
            }
            InputEvent::PointerMotionAbsolute { event } => {
                let Some(size) = self
                    .output
                    .as_ref()
                    .and_then(|o| self.space.output_geometry(o))
                    .map(|g| g.size)
                else {
                    return;
                };
                let pos = event.position_transformed(size);
                let focus = self.surface_under(pos);
                pointer.motion(
                    self,
                    focus,
                    &MotionEvent {
                        location: pos,
                        serial: SERIAL_COUNTER.next_serial(),
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                if event.state() == ButtonState::Pressed && !pointer.is_grabbed() {
                    let pos = pointer.current_location();
                    if let Some((window, _)) =
                        self.space.element_under(pos).map(|(w, l)| (w.clone(), l))
                    {
                        self.focus_window(&window);
                    }
                }
                pointer.button(
                    self,
                    &ButtonEvent {
                        button: event.button_code(),
                        state: event.state(),
                        serial,
                        time: event.time_msec(),
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerAxis { event } => {
                let amount = |axis| {
                    event
                        .amount(axis)
                        .unwrap_or_else(|| event.amount_v120(axis).unwrap_or(0.0) * 15.0 / 120.0)
                };
                let mut frame = AxisFrame::new(event.time_msec()).source(event.source());
                for axis in [Axis::Horizontal, Axis::Vertical] {
                    let value = amount(axis);
                    if value != 0.0 {
                        frame = frame.value(axis, value);
                        if let Some(v120) = event.amount_v120(axis) {
                            frame = frame.v120(axis, v120 as i32);
                        }
                    } else if event.source() == AxisSource::Finger {
                        frame = frame.stop(axis);
                    }
                }
                pointer.axis(self, frame);
                pointer.frame(self);
            }
            _ => {}
        }
    }
}
