//! Nested backend: runs the compositor inside a window of another compositor.

use std::time::Duration;

use smithay::{
    backend::{
        renderer::{damage::OutputDamageTracker, gles::GlesRenderer},
        winit::{self, WinitEvent},
    },
    desktop::layer_map_for_output,
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::EventLoop,
    utils::{Rectangle, Transform},
};

use crate::state::Patina;

pub fn init(
    event_loop: &mut EventLoop<Patina>,
    state: &mut Patina,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut backend, winit_source) = winit::init::<GlesRenderer>()?;

    state.clip_program = match crate::clip::compile(backend.renderer()) {
        Ok(program) => Some(program),
        Err(e) => {
            tracing::warn!(error = %e, "corner clipping disabled");
            None
        }
    };

    let mode = Mode {
        size: backend.window_size(),
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
            }
            WinitEvent::Input(event) => state.process_input_event(event),
            WinitEvent::Redraw => {
                let size = backend.window_size();
                let damage = Rectangle::from_size(size);
                state.space.refresh();
                {
                    let (renderer, mut framebuffer) = backend.bind().unwrap();
                    let elements = crate::render::elements(state, renderer, &output);
                    damage_tracker
                        .render_output(renderer, &mut framebuffer, 0, &elements, clear)
                        .unwrap();
                }
                backend.submit(Some(&[damage])).unwrap();

                let elapsed = state.start_time.elapsed();
                for window in state.space.elements() {
                    window.send_frame(&output, elapsed, Some(Duration::ZERO), |_, _| {
                        Some(output.clone())
                    });
                }
                for layer in layer_map_for_output(&output).layers() {
                    layer.send_frame(&output, elapsed, Some(Duration::ZERO), |_, _| {
                        Some(output.clone())
                    });
                }
                backend.window().request_redraw();
            }
            WinitEvent::CloseRequested => state.loop_signal.stop(),
            _ => {}
        })?;
    Ok(())
}
