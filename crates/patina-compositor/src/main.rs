//! Patina compositor.
//!
//! Milestone 1: a nested (winit) Wayland compositor with xdg-shell windows and
//! layer-shell surfaces, so `patina-panel` can attach to it.
//!
//! Usage: `patina-compositor [--per-output COMMAND] [COMMAND...]`. Each COMMAND is a shell
//! command started once the compositor is listening, with `WAYLAND_DISPLAY` pointing at it. The
//! `--per-output` one (the panel) is started for every output as it appears, with
//! `PATINA_OUTPUT` set to the output's name, and stopped when the output goes.

mod blur;
mod clip;
mod cursor;
mod drm;
mod dumb;
mod input;
mod outputs;
mod render;
mod scene;
mod state;
mod toplevel_info;
mod winit;
mod wm;
mod workspaces;

use std::process::Command;

use smithay::reexports::calloop::EventLoop;
use state::Patina;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let mut event_loop: EventLoop<'static, Patina> = EventLoop::try_new()?;
    let display = smithay::reexports::wayland_server::Display::new()?;
    let mut state = Patina::new(&mut event_loop, display);

    let mut once = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--per-output" {
            state.per_output_cmd = args.next();
        } else {
            once.push(arg);
        }
    }

    // Nested inside another session: winit. From a TTY: DRM. PATINA_BACKEND=drm|winit overrides.
    let nested =
        std::env::var_os("WAYLAND_DISPLAY").is_some() || std::env::var_os("DISPLAY").is_some();
    match std::env::var("PATINA_BACKEND").as_deref() {
        Ok("winit") => winit::init(&mut event_loop, &mut state)?,
        Ok("drm") => drm::init(&mut event_loop, &mut state)?,
        _ if nested => winit::init(&mut event_loop, &mut state)?,
        _ => drm::init(&mut event_loop, &mut state)?,
    }

    tracing::info!(socket = ?state.socket_name, "listening");
    for cmd in once {
        match Command::new("sh")
            .arg("-c")
            .arg(&cmd)
            .env("WAYLAND_DISPLAY", &state.socket_name)
            .spawn()
        {
            Ok(_) => tracing::info!(%cmd, "started"),
            Err(e) => tracing::warn!(%cmd, error = %e, "failed to start"),
        }
    }

    let mut dh = state.display_handle.clone();
    event_loop.run(None, &mut state, move |_| {
        let _ = dh.flush_clients();
    })?;
    Ok(())
}
