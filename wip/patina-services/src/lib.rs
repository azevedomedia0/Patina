//! The system services behind the shell's quick settings and notifications.
//!
//! - [`network`]: Wi-Fi and VPN through NetworkManager (D-Bus, system bus).
//! - [`bluetooth`]: BlueZ (D-Bus, system bus).
//! - [`audio`]: output volume through `pactl` (PulseAudio, or PipeWire's PulseAudio server).
//! - [`brightness`]: the screen backlight, read from sysfs and set through logind.
//! - [`battery`]: UPower's display device, falling back to sysfs.
//! - [`power`]: log off, restart and power off through logind.
//! - [`notifications`]: the `org.freedesktop.Notifications` server.
//!
//! Each `watch` function runs until its channel closes and sends a fresh snapshot whenever the
//! service reports a change (no polling while nothing happens). A service that is missing is
//! reported as unavailable, and looked for again now and then.

pub mod audio;
pub mod battery;
pub mod bluetooth;
pub mod brightness;
pub mod network;
pub mod notifications;
pub mod power;

use std::time::Duration;

use futures_util::StreamExt;

/// How long to wait before looking for a missing service again.
pub(crate) const RETRY: Duration = Duration::from_secs(15);

/// Waits until any signal from `sender` arrives on `conn` (after the first, also waits for the
/// burst to settle: services send several in a row). Returns false if the stream ended.
pub(crate) async fn next_change(stream: &mut zbus::MessageStream) -> bool {
    if stream.next().await.is_none() {
        return false;
    }
    loop {
        match tokio::time::timeout(Duration::from_millis(150), stream.next()).await {
            Ok(Some(_)) => continue,
            Ok(None) => return false,
            Err(_) => return true,
        }
    }
}

/// A stream of every signal `sender` emits.
pub(crate) async fn signals_from(
    conn: &zbus::Connection,
    sender: &'static str,
) -> zbus::Result<zbus::MessageStream> {
    let rule = zbus::MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .sender(sender)?
        .build();
    zbus::MessageStream::for_match_rule(rule, conn, Some(64)).await
}

/// "5 h 20 min", "45 min", or "" for nothing, as the prototype shows battery time.
pub fn format_duration(d: Duration) -> String {
    let mins = d.as_secs() / 60;
    match (mins / 60, mins % 60) {
        (0, 0) => String::new(),
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_like_the_prototype() {
        assert_eq!(format_duration(Duration::from_secs(5 * 3600 + 20 * 60)), "5 h 20 min");
        assert_eq!(format_duration(Duration::from_secs(45 * 60 + 30)), "45 min");
        assert_eq!(format_duration(Duration::from_secs(2 * 3600)), "2 h");
        assert_eq!(format_duration(Duration::from_secs(20)), "");
    }
}
