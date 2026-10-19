//! Patina compositor.
//!
//! Milestone 1: a nested (winit) Wayland compositor with xdg-shell windows and
//! layer-shell surfaces, so `patina-panel` can attach to it.
//!
//! Usage: `patina-compositor [COMMAND...]`. Each argument is a shell command started
//! once the compositor is listening, with `WAYLAND_DISPLAY` pointing at it.

mod clip;
mod input;
mod render;
mod state;
mod winit;
mod wm;

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

    let mut event_loop: EventLoop<Patina> = EventLoop::try_new()?;
    let display = smithay::reexports::wayland_server::Display::new()?;
    let mut state = Patina::new(&mut event_loop, display);

    winit::init(&mut event_loop, &mut state)?;

    tracing::info!(socket = ?state.socket_name, "listening");
    for cmd in std::env::args().skip(1) {
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
