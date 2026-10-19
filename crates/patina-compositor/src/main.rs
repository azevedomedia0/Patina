//! Patina compositor.
//!
//! Milestone 1: a nested (winit) Wayland compositor with xdg-shell windows and
//! layer-shell surfaces, so `patina-panel` can attach to it.
//!
//! Usage: `patina-compositor [--greeter COMMAND] [--shell COMMAND]... [--per-output COMMAND]
//! [COMMAND...]`. `--greeter` runs a sign-in screen and ends the compositor when it ends. Each
//! COMMAND is a shell command started once the compositor is listening, with `WAYLAND_DISPLAY`
//! pointing at it. `--shell` ones (panel, dock) are started again if they stop. The
//! `--per-output` one is started for every output as it appears, with `PATINA_OUTPUT` set to the
//! output's name, and stopped when the output goes.

mod blur;
mod clip;
mod cursor;
mod drm;
mod dumb;
mod idle;
mod input;
mod keyboard;
mod layers;
mod logind;
mod outputs;
mod render;
mod scene;
mod session_end;
mod session_lock;
mod shell_ipc;
mod state;
mod supervise;
mod toplevel_info;
mod winit;
mod wm;
mod workspaces;

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
    let mut shell = Vec::new();
    let mut greeter = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--per-output" => state.per_output_cmd = args.next(),
            "--shell" => shell.extend(args.next()),
            "--greeter" => greeter = args.next(),
            _ => once.push(arg),
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

    state.logind = logind::start(&state.loop_handle);
    session_end::listen(&mut state);
    state.arm_idle_timer();
    tracing::info!(socket = ?state.socket_name, "listening");
    let socket = state.socket_name.to_string_lossy().into_owned();
    for cmd in shell {
        supervise::keep_running(&state.loop_handle, socket.clone(), cmd);
    }
    if let Some(cmd) = greeter {
        supervise::run_then_quit(&state.loop_handle, &socket, &cmd);
    }
    for cmd in once {
        match supervise::spawn_once(&socket, &cmd) {
            Ok(_) => tracing::info!(%cmd, "started"),
            Err(e) => tracing::warn!(%cmd, error = %e, "failed to start"),
        }
    }

    let mut dh = state.display_handle.clone();
    event_loop.run(None, &mut state, move |_| {
        let _ = dh.flush_clients();
    })?;
    if let Some(path) = &state.control_socket {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}
