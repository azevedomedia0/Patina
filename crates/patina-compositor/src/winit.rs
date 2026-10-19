//! Nested backend: runs the compositor inside a window of another compositor.

use std::{cell::RefCell, rc::Rc};

use smithay::{
    backend::{
        renderer::{damage::OutputDamageTracker, gles::GlesRenderer},
        winit::{self, WinitEvent},
    },
    desktop::layer_map_for_output,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::{ping::make_ping, EventLoop},
    utils::{Rectangle, Transform},
};

use crate::state::Patina;

pub fn init(
    event_loop: &mut EventLoop<Patina>,
    state: &mut Patina,
) -> Result<(), Box<dyn std::error::Error>> {
    let (backend, winit_source) = winit::init::<GlesRenderer>()?;
    let backend = Rc::new(RefCell::new(backend));

    state.clip_program = crate::clip::compile_logged(backend.borrow_mut().renderer());

    let mode = Mode {
        size: backend.borrow().window_size(),
        refresh: 60_000,
    };
    let output = Output::new(
        "patina-nested".into(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Patina".into(),
            model: "Nested".into(),
        },
    );
    let _global = output.create_global::<Patina>(&state.display_handle);
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);
    state.space.map_output(&output, (0, 0));
    state.output = Some(output.clone());

    let mut damage_tracker = OutputDamageTracker::from_output(&output);
    let bg = patina_tokens::BG_DESKTOP;
    let clear = [
        bg.r as f32 / 255.0,
        bg.g as f32 / 255.0,
        bg.b as f32 / 255.0,
        1.0,
    ];

    // A queued redraw asks the host window for a frame; nothing redraws on a timer.
    let (ping, ping_source) = make_ping()?;
    let wake = backend.clone();
    event_loop
        .handle()
        .insert_source(ping_source, move |_, _, _| {
            wake.borrow().window().request_redraw()
        })?;
    state.redraw_ping = Some(ping);

    let backend_for_events = backend.clone();
    event_loop
        .handle()
        .insert_source(winit_source, move |event, _, state| match event {
            WinitEvent::Resized { size, .. } => {
                output.change_current_state(
                    Some(Mode {
                        size,
                        refresh: 60_000,
                    }),
                    None,
                    None,
                    None,
                );
                layer_map_for_output(&output).arrange();
                state.fit_windows_to_zone();
                state.queue_redraw();
            }
            WinitEvent::Input(event) => state.process_input_event(event),
            WinitEvent::Redraw => {
                let mut backend = backend_for_events.borrow_mut();
                state.redraw_wanted = false;
                let size = backend.window_size();
                let damage = Rectangle::from_size(size);
                state.space.refresh();
                state.refresh_toplevel_info();
                {
                    let (renderer, mut framebuffer) = backend.bind().unwrap();
                    let elements = crate::render::elements(state, renderer, &output);
                    damage_tracker
                        .render_output(renderer, &mut framebuffer, 0, &elements, clear)
                        .unwrap();
                }
                backend.submit(Some(&[damage])).unwrap();

                state.send_frames(&output);
            }
            WinitEvent::CloseRequested => state.loop_signal.stop(),
            _ => {}
        })?;
    Ok(())
}
