//! Shell programs (`--shell`): started with the compositor and started again if they stop, so a
//! crashed panel or dock comes back. Each runs `sh -c CMD` with `WAYLAND_DISPLAY` set. A program
//! that stops more than five times within a minute is left stopped, so a broken one does not
//! spin. Waiting happens on a thread per program, so an idle session stays idle.

use std::{
    process::Command,
    time::{Duration, Instant},
};

use smithay::reexports::calloop::{
    channel::{self, Event},
    LoopHandle,
};

use crate::state::Patina;

const LIMIT: usize = 5;
const WINDOW: Duration = Duration::from_secs(60);

/// Starts `cmd` and keeps it running.
pub fn keep_running(handle: &LoopHandle<'static, Patina>, socket: String, cmd: String) {
    let (tx, rx) = channel::channel::<()>();
    let mut stops: Vec<Instant> = Vec::new();
    let spawn = {
        let socket = socket.clone();
        let cmd = cmd.clone();
        move |tx: channel::Sender<()>| {
            let child = Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .env("WAYLAND_DISPLAY", &socket)
                .env("PATINA_COMPOSITOR_PID", std::process::id().to_string())
                .spawn();
            match child {
                Ok(mut child) => {
                    tracing::info!(%cmd, pid = child.id(), "shell program started");
                    let cmd = cmd.clone();
                    std::thread::spawn(move || {
                        let status = child.wait();
                        tracing::warn!(%cmd, ?status, "shell program stopped");
                        let _ = tx.send(());
                    });
                }
                Err(e) => tracing::warn!(%cmd, error = %e, "cannot start shell program"),
            }
        }
    };
    spawn(tx.clone());
    let restart = spawn.clone();
    let _ = handle.insert_source(rx, move |event, _, _| {
        if let Event::Msg(()) = event {
            let now = Instant::now();
            stops.retain(|t| now.duration_since(*t) < WINDOW);
            stops.push(now);
            if stops.len() > LIMIT {
                tracing::error!(%cmd, "shell program keeps stopping; not starting it again");
                return;
            }
            restart(tx.clone());
        }
    });
}

/// Starts `sh -c cmd` once with `WAYLAND_DISPLAY` set, and reaps it when it ends (so no zombie
/// is left behind).
pub fn spawn_once(socket: &str, cmd: &str) -> std::io::Result<u32> {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .env("WAYLAND_DISPLAY", socket)
        .env("PATINA_COMPOSITOR_PID", std::process::id().to_string())
        .spawn()?;
    let pid = child.id();
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(pid)
}

/// Starts `cmd` (the greeter) and stops the compositor when it ends: greetd then starts the
/// signed-in session, or the greeter again.
pub fn run_then_quit(handle: &LoopHandle<'static, Patina>, socket: &str, cmd: &str) {
    let (tx, rx) = channel::channel::<()>();
    match Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .env("WAYLAND_DISPLAY", socket)
        .spawn()
    {
        Ok(mut child) => {
            tracing::info!(%cmd, "greeter started");
            std::thread::spawn(move || {
                let status = child.wait();
                tracing::info!(?status, "greeter ended");
                let _ = tx.send(());
            });
        }
        Err(e) => {
            tracing::error!(%cmd, error = %e, "cannot start the greeter");
            let _ = tx.send(());
        }
    }
    let _ = handle.insert_source(rx, |event, _, state| {
        if let Event::Msg(()) = event {
            state.loop_signal.stop();
        }
    });
}
