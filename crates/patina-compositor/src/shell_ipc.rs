//! Requests from the compositor to the shell (the panel process): open the launcher, show the
//! desktop menu. One line per request on a Unix socket the shell listens on, named after the
//! Wayland socket so nested sessions do not mix:
//! `$XDG_RUNTIME_DIR/patina-shell-$WAYLAND_DISPLAY.sock`.
//!
//! - `launcher OUTPUT`: toggle the launcher on that output.
//! - `desktop-menu OUTPUT X Y W H`: the desktop's right-click menu at (X, Y) in the output's free
//!   area of W x H (logical pixels, below the panel).

use std::{io::Write, os::unix::net::UnixStream, path::PathBuf, time::Duration};

/// Where the shell listens, for the session on Wayland socket `display`.
pub fn socket_path(display: &str) -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(dir).join(format!("patina-shell-{display}.sock")))
}

/// Sends one request; does nothing (but log) when the shell is not running.
pub fn send(display: &str, line: &str) {
    let Some(path) = socket_path(display) else {
        return;
    };
    match UnixStream::connect(&path) {
        Ok(mut s) => {
            let _ = s.set_write_timeout(Some(Duration::from_millis(200)));
            if let Err(e) = s.write_all(format!("{line}\n").as_bytes()) {
                tracing::warn!(error = %e, "shell request not sent");
            }
        }
        Err(e) => tracing::debug!(error = %e, ?path, "shell not listening"),
    }
}
