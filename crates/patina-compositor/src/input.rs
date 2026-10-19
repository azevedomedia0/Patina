//! Pointer and keyboard input from the backend.

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
    },
    input::{
        keyboard::{FilterResult, Keysym},
        pointer::{AxisFrame, ButtonEvent, MotionEvent},
    },
    utils::SERIAL_COUNTER,
};

use patina_frame::{Control, Hit};

use crate::{
    state::Patina,
    wm::{Action, Pick},
};

impl Patina {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let Some(keyboard) = self.seat.get_keyboard() else {
            return;
        };

        match event {
            InputEvent::Keyboard { event } => {
                let pressed = event.state() == smithay::backend::input::KeyState::Pressed;
                let action = keyboard.input::<Action, _>(
                    self,
                    event.key_code(),
                    event.state(),
                    SERIAL_COUNTER.next_serial(),
                    event.time_msec(),
                    |_, mods, handle| {
                        if pressed && mods.alt && handle.modified_sym() == Keysym::m {
                            FilterResult::Intercept(Action::RestoreMinimized)
                        } else {
                            FilterResult::Forward
                        }
                    },
                );
                if let Some(Action::RestoreMinimized) = action {
                    self.restore_minimized();
                }
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
                let serial = SERIAL_COUNTER.next_serial();
                let time = event.time_msec();

                if self.drag.is_some() {
                    self.apply_drag(pos);
                    pointer.motion(
                        self,
                        None,
                        &MotionEvent {
                            location: pos,
                            serial,
                            time,
                        },
                    );
                    pointer.frame(self);
                    return;
                }

                let pick = self.pick(pos);
                self.update_hover(&pick);
                let focus = match pick {
                    Pick::Surface(wl, origin, _) => Some((wl, origin)),
                    _ => None,
                };
                pointer.motion(
                    self,
                    focus,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time,
                    },
                );
                pointer.frame(self);
            }
            InputEvent::PointerButton { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let pos = pointer.current_location();

                if event.state() == ButtonState::Released {
                    if self.frame_press || self.drag.is_some() {
                        self.frame_press = false;
                        self.end_drag();
                        return;
                    }
                } else {
                    match self.pick(pos) {
                        Pick::Frame(window, hit) => {
                            self.frame_press = true;
                            self.focus_window(&window);
                            match hit {
                                Hit::Titlebar => {
                                    if self.titlebar_double_click(&window) {
                                        self.toggle_maximize(&window);
                                    } else {
                                        self.begin_move(&window, pos);
                                    }
                                }
                                Hit::Control(Control::Close) => self.close(&window),
                                Hit::Control(Control::Maximize) => self.toggle_maximize(&window),
                                Hit::Control(Control::Minimize) => self.minimize(&window),
                                Hit::ResizeGrip => self.begin_resize(&window, pos),
                                Hit::Client => {}
                            }
                            return;
                        }
                        Pick::Surface(_, _, Some(window)) => self.focus_window(&window),
                        _ => {}
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
