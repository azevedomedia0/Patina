//! Ending the session from inside it: log off, restart, shut down, and lock, asked for on the
//! compositor's control socket by `patina-session-ctl` (and the quick settings, later).
//!
//! Log off ends the compositor, so the display manager shows its sign-in screen again, as in
//! the prototype. Restart and shut down show the prototype's dark screen ("Restarting…",
//! "Shutting down…") and ask logind; if logind refuses, the desktop comes back.
//!
//! The socket is `$XDG_RUNTIME_DIR/patina-compositor-$WAYLAND_DISPLAY.sock`; one request per
//! connection, one word: `logout`, `reboot`, `poweroff` or `lock`.

use std::{
    io::Read,
    os::unix::net::{UnixListener, UnixStream},
    path::PathBuf,
    time::Duration,
};

use smithay::reexports::calloop::{
    generic::Generic,
    timer::{TimeoutAction, Timer},
    Interest, Mode, PostAction,
};

use crate::state::Patina;

pub fn socket_path(display: &str) -> Option<PathBuf> {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")?;
    Some(PathBuf::from(dir).join(format!("patina-compositor-{display}.sock")))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    LogOut,
    Restart,
    PowerOff,
    Lock,
}

impl Request {
    pub fn parse(word: &str) -> Option<Self> {
        match word.trim() {
            "logout" => Some(Self::LogOut),
            "reboot" | "restart" => Some(Self::Restart),
            "poweroff" | "shutdown" => Some(Self::PowerOff),
            "lock" => Some(Self::Lock),
            _ => None,
        }
    }
}

/// Starts listening on the control socket.
pub fn listen(state: &mut Patina) {
    let Some(path) = socket_path(&state.socket_name.to_string_lossy()) else {
        return;
    };
    let _ = std::fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!(error = %e, ?path, "no control socket");
            return;
        }
    };
    let _ = listener.set_nonblocking(true);
    let _ = state.loop_handle.insert_source(
        Generic::new(listener, Interest::READ, Mode::Level),
        |_, listener, state| {
            while let Ok((stream, _)) = listener.accept() {
                if let Some(req) = read_request(stream) {
                    state.session_request(req);
                }
            }
            Ok(PostAction::Continue)
        },
    );
    state.control_socket = Some(path);
}

fn read_request(mut stream: UnixStream) -> Option<Request> {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
    let mut buf = [0u8; 32];
    let n = stream.read(&mut buf).ok()?;
    Request::parse(std::str::from_utf8(&buf[..n]).ok()?)
}

impl Patina {
    pub fn session_request(&mut self, req: Request) {
        tracing::info!(?req, "session request");
        match req {
            Request::Lock => self.start_locker(),
            Request::LogOut => self.loop_signal.stop(),
            Request::Restart => self.end_with("Restarting…", crate::logind::Request::Reboot),
            Request::PowerOff => self.end_with("Shutting down…", crate::logind::Request::PowerOff),
        }
    }

    /// Shows the dark screen with `message`, then asks logind. The screen stays until the
    /// system goes down, or for 10 seconds if nothing happens (logind or polkit refused).
    fn end_with(&mut self, message: &str, req: crate::logind::Request) {
        self.ending = Some(message.to_string());
        self.queue_redraw();
        if self.logind.is_some() {
            self.logind_request(req);
        } else {
            let verb = match req {
                crate::logind::Request::Reboot => "reboot",
                _ => "poweroff",
            };
            let _ = std::process::Command::new("systemctl").arg(verb).spawn();
        }
        let _ = self.loop_handle.insert_source(
            Timer::from_duration(Duration::from_secs(10)),
            |_, _, state| {
                tracing::warn!("still running 10 s after asking to end: back to the desktop");
                state.ending = None;
                state.queue_redraw();
                TimeoutAction::Drop
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_single_words() {
        assert_eq!(Request::parse("logout\n"), Some(Request::LogOut));
        assert_eq!(Request::parse("restart"), Some(Request::Restart));
        assert_eq!(Request::parse("reboot"), Some(Request::Restart));
        assert_eq!(Request::parse("shutdown"), Some(Request::PowerOff));
        assert_eq!(Request::parse("lock"), Some(Request::Lock));
        assert_eq!(Request::parse("format c:"), None);
    }
}
