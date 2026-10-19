//! Asks the running Patina session to log off, restart, shut down or lock:
//! `patina-session-ctl logout|reboot|poweroff|lock`. Used by the shell's buttons; handy in
//! scripts and terminals too.

use std::{io::Write, os::unix::net::UnixStream, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let Some(word) = std::env::args().nth(1).filter(|w| {
        [
            "logout", "reboot", "restart", "poweroff", "shutdown", "lock",
        ]
        .contains(&w.as_str())
    }) else {
        eprintln!("usage: patina-session-ctl logout|reboot|poweroff|lock");
        return ExitCode::from(2);
    };
    let (Some(dir), Some(display)) = (
        std::env::var_os("XDG_RUNTIME_DIR"),
        std::env::var_os("WAYLAND_DISPLAY"),
    ) else {
        eprintln!("patina-session-ctl: not inside a Patina session (no WAYLAND_DISPLAY)");
        return ExitCode::FAILURE;
    };
    let path = PathBuf::from(dir).join(format!(
        "patina-compositor-{}.sock",
        display.to_string_lossy()
    ));
    match UnixStream::connect(&path).and_then(|mut s| s.write_all(word.as_bytes())) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("patina-session-ctl: {}: {e}", path.display());
            ExitCode::FAILURE
        }
    }
}
