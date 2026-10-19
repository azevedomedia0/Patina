//! Pointer and keyboard input from the backend.

use smithay::{
    backend::input::{
        AbsolutePositionEvent, Axis, AxisSource, ButtonState, Event, InputBackend, InputEvent,
        KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent, PointerMotionEvent, TouchEvent,
    },
    input::{
        keyboard::{FilterResult, Keysym},
        pointer::{AxisFrame, ButtonEvent, CursorIcon, MotionEvent},
    },
    utils::{Logical, Point, SERIAL_COUNTER},
};

use patina_frame::{Control, Hit};

use crate::{
    state::Patina,
    wm::{Action, Drag, Pick},
};

/// The workspace a number-row key stands for. Physical keys, not symbols, so it works on layouts
/// where the digits need Shift (xkb keycodes 10..=13 are the keys labelled 1 to 4 on a PC
/// keyboard).
/// The right mouse button (`BTN_RIGHT` in `linux/input-event-codes.h`).
const BTN_RIGHT: u32 = 0x111;

fn workspace_for_key(xkb_keycode: u32) -> Option<usize> {
    (10..10 + crate::workspaces::COUNT as u32)
        .contains(&xkb_keycode)
        .then(|| (xkb_keycode - 10) as usize)
}

impl Patina {
    /// Where a touch lands. A touchscreen covers one output: the built-in panel (eDP, LVDS,
    /// DSI) if there is one, else the first output.
    fn touch_position<E: AbsolutePositionEvent<I>, I: InputBackend>(
        &self,
        event: &E,
    ) -> Option<Point<f64, Logical>> {
        let outputs = self.outputs();
        let output = outputs
            .iter()
            .find(|o| {
                let n = o.name();
                n.starts_with("eDP") || n.starts_with("LVDS") || n.starts_with("DSI")
            })
            .or_else(|| outputs.first())?;
        let area = self.space.output_geometry(output)?;
        Some(event.position_transformed(area.size) + area.loc.to_f64())
    }

    /// Asks the shell for the desktop's right-click menu at `pos`.
    fn open_desktop_menu(&mut self, pos: Point<f64, Logical>) {
        let Some(output) = self.output_at(pos) else {
            return;
        };
        let zone = self.zone_of(&output);
        let (x, y) = (pos.x as i32 - zone.loc.x, pos.y as i32 - zone.loc.y);
        crate::shell_ipc::send(
            &self.socket_name.to_string_lossy(),
            &format!(
                "desktop-menu {} {x} {y} {} {}",
                output.name(),
                zone.size.w,
                zone.size.h
            ),
        );
    }

    /// The pointer moved to `pos`: continue a drag, or update hover and pointer focus.
    fn pointer_moved(&mut self, pos: Point<f64, Logical>, time: u32) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let serial = SERIAL_COUNTER.next_serial();

        if self.drag.is_some() {
            self.frame_cursor = Some(match self.drag {
                Some(Drag::Resize { .. }) => CursorIcon::NwseResize,
                _ => CursorIcon::Default,
            });
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
        // Over a client the client decides; elsewhere the compositor does.
        self.frame_cursor = match &pick {
            Pick::Surface(..) => None,
            Pick::Frame(_, Hit::ResizeGrip) => Some(CursorIcon::NwseResize),
            Pick::Frame(..) | Pick::Nothing => Some(CursorIcon::Default),
        };
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

    /// Handles one input event, then asks for a frame: input can move the cursor, change hover,
    /// or move, resize and close windows.
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        if !matches!(
            event,
            InputEvent::DeviceAdded { .. } | InputEvent::DeviceRemoved { .. }
        ) {
            self.reset_idle();
        }
        self.handle_input_event(event);
        self.queue_redraw();
    }

    fn handle_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
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
                    |state, mods, handle| {
                        let sym = handle.modified_sym();
                        // A Super tap (pressed and released with nothing in between) opens the
                        // launcher; the keys still reach the client.
                        let is_super = matches!(sym, Keysym::Super_L | Keysym::Super_R);
                        if pressed {
                            state.super_alone = is_super;
                        } else if is_super && std::mem::take(&mut state.super_alone) {
                            // Forwarded all the same, so the client sees Super go up.
                            state.launcher_wanted = true;
                        }
                        if pressed && mods.ctrl && mods.alt && sym == Keysym::l {
                            // Ctrl+Alt+L locks, as in the prototype.
                            return FilterResult::Intercept(Action::Lock);
                        }
                        if state.is_locked() {
                            // Locked: only VT switching; everything else goes to the locker.
                            return if pressed && (0x1008_FE01..=0x1008_FE0C).contains(&sym.raw()) {
                                FilterResult::Intercept(Action::VtSwitch(
                                    (sym.raw() - 0x1008_FE01 + 1) as i32,
                                ))
                            } else {
                                FilterResult::Forward
                            };
                        }
                        if pressed && mods.alt && sym == Keysym::m {
                            FilterResult::Intercept(Action::RestoreMinimized)
                        } else if let Some(n) = pressed
                            .then(|| workspace_for_key(handle.raw_code().raw()))
                            .flatten()
                            .filter(|_| mods.logo)
                        {
                            // Super+1..4 switch workspace, Super+Shift+1..4 move the window.
                            FilterResult::Intercept(if mods.shift {
                                Action::MoveToWorkspace(n)
                            } else {
                                Action::SwitchWorkspace(n)
                            })
                        } else if pressed && (0x1008_FE01..=0x1008_FE0C).contains(&sym.raw()) {
                            // Ctrl+Alt+F1..F12 arrive as XF86Switch_VT_1..12.
                            FilterResult::Intercept(Action::VtSwitch(
                                (sym.raw() - 0x1008_FE01 + 1) as i32,
                            ))
                        } else {
                            FilterResult::Forward
                        }
                    },
                );
                let action = if std::mem::take(&mut self.launcher_wanted) {
                    Some(Action::ToggleLauncher)
                } else {
                    action
                };
                match action {
                    Some(Action::Lock) => self.start_locker(),
                    Some(Action::ToggleLauncher) if self.is_locked() => {}
                    Some(Action::ToggleLauncher) => {
                        if let Some(o) = self.pointer_output() {
                            crate::shell_ipc::send(
                                &self.socket_name.to_string_lossy(),
                                &format!("launcher {}", o.name()),
                            );
                        }
                    }
                    Some(Action::RestoreMinimized) => self.restore_minimized(),
                    Some(Action::SwitchWorkspace(n)) => self.switch_workspace(n),
                    Some(Action::MoveToWorkspace(n)) => {
                        if let Some(window) = self.focused_window() {
                            self.move_window_to_workspace(&window, n);
                        }
                    }
                    Some(Action::VtSwitch(vt)) => {
                        if let Some(drm) = &mut self.drm {
                            if let Err(e) =
                                smithay::backend::session::Session::change_vt(&mut drm.session, vt)
                            {
                                tracing::warn!(vt, error = ?e, "VT switch failed");
                            }
                        }
                    }
                    None => {}
                }
            }
            InputEvent::PointerMotionAbsolute { event } => {
                // Tablets and virtual machine pointers span all outputs together.
                let Some(bounds) = self.layout_bounds() else {
                    return;
                };
                let pos = event.position_transformed(bounds.size) + bounds.loc.to_f64();
                let pos = self.clamp_to_outputs(pos);
                self.pointer_moved(pos, event.time_msec());
            }
            InputEvent::PointerMotion { event } => {
                if self.layout_bounds().is_none() {
                    return;
                }
                let pos = self.clamp_to_outputs(pointer.current_location() + event.delta());
                self.pointer_moved(pos, event.time_msec());
            }
            InputEvent::PointerButton { event } => {
                let serial = SERIAL_COUNTER.next_serial();
                let pos = pointer.current_location();

                if self.is_locked() {
                    // Locked: the press goes to the lock surface under the pointer, nothing else.
                } else if pointer.is_grabbed() && self.drag.is_none() && !self.frame_press {
                    // An open menu has the pointer: it decides, and closes on a click outside.
                } else if event.state() == ButtonState::Released {
                    if self.frame_press || self.drag.is_some() {
                        self.frame_press = false;
                        self.end_drag();
                        return;
                    }
                } else {
                    self.super_alone = false;
                    let pick = self.pick(pos);
                    let target = match &pick {
                        Pick::Surface(wl, _, _) => Some(wl.clone()),
                        _ => None,
                    };
                    self.close_transient_layers(target.as_ref());
                    if let Pick::Surface(wl, _, None) = &pick {
                        self.layer_clicked(wl);
                    }
                    if matches!(pick, Pick::Nothing) && event.button_code() == BTN_RIGHT {
                        self.open_desktop_menu(pos);
                    }
                    match pick {
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
            InputEvent::TouchDown { event } => {
                let Some(pos) = self.touch_position(&event) else {
                    return;
                };
                let serial = SERIAL_COUNTER.next_serial();
                let Some(touch) = self.seat.get_touch() else {
                    return;
                };
                let pick = self.pick(pos);
                let target = match &pick {
                    Pick::Surface(wl, _, _) => Some(wl.clone()),
                    _ => None,
                };
                self.close_transient_layers(target.as_ref());
                match pick {
                    // The frame is the compositor's: a touch there acts like the pointer.
                    Pick::Frame(window, hit) => {
                        self.focus_window(&window);
                        match hit {
                            Hit::Titlebar => {
                                self.begin_move(&window, pos);
                                self.touch_drag = Some(event.slot());
                            }
                            Hit::ResizeGrip => {
                                self.begin_resize(&window, pos);
                                self.touch_drag = Some(event.slot());
                            }
                            Hit::Control(Control::Close) => self.close(&window),
                            Hit::Control(Control::Maximize) => self.toggle_maximize(&window),
                            Hit::Control(Control::Minimize) => self.minimize(&window),
                            Hit::Client => {}
                        }
                    }
                    Pick::Surface(wl, origin, window) => {
                        if let Some(window) = window {
                            self.focus_window(&window);
                        }
                        touch.down(
                            self,
                            Some((wl, origin)),
                            &smithay::input::touch::DownEvent {
                                slot: event.slot(),
                                location: pos,
                                serial,
                                time: event.time_msec(),
                            },
                        );
                    }
                    Pick::Nothing => {}
                }
            }
            InputEvent::TouchMotion { event } => {
                let Some(pos) = self.touch_position(&event) else {
                    return;
                };
                if self.touch_drag == Some(event.slot()) {
                    self.apply_drag(pos);
                } else if let Some(touch) = self.seat.get_touch() {
                    let focus = match self.pick(pos) {
                        Pick::Surface(wl, origin, _) => Some((wl, origin)),
                        _ => None,
                    };
                    touch.motion(
                        self,
                        focus,
                        &smithay::input::touch::MotionEvent {
                            slot: event.slot(),
                            location: pos,
                            time: event.time_msec(),
                        },
                    );
                }
            }
            InputEvent::TouchUp { event } => {
                if self.touch_drag == Some(event.slot()) {
                    self.touch_drag = None;
                    self.end_drag();
                } else if let Some(touch) = self.seat.get_touch() {
                    touch.up(
                        self,
                        &smithay::input::touch::UpEvent {
                            slot: event.slot(),
                            serial: SERIAL_COUNTER.next_serial(),
                            time: event.time_msec(),
                        },
                    );
                }
            }
            InputEvent::TouchFrame { .. } => {
                if let Some(touch) = self.seat.get_touch() {
                    touch.frame(self);
                }
            }
            InputEvent::TouchCancel { .. } => {
                if self.touch_drag.take().is_some() {
                    self.end_drag();
                }
                if let Some(touch) = self.seat.get_touch() {
                    touch.cancel(self);
                }
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

#[cfg(test)]
mod tests {
    use super::workspace_for_key;

    #[test]
    fn number_row_keys_map_to_workspaces() {
        assert_eq!(workspace_for_key(10), Some(0));
        assert_eq!(workspace_for_key(13), Some(3));
    }

    #[test]
    fn other_keys_do_not() {
        assert_eq!(workspace_for_key(9), None); // Escape
        assert_eq!(workspace_for_key(14), None); // the 5 key: only four workspaces
        assert_eq!(workspace_for_key(38), None); // A
    }
}
