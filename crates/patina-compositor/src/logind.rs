//! The bits of systemd-logind the compositor uses: locking before the machine sleeps, the
//! session's Lock signal (`loginctl lock-session`), suspending, and setting the backlight.
//!
//! Runs on threads with blocking D-Bus calls; events reach the event loop through a channel.
//! Without logind (a seatd session on a system without it) nothing happens.

use std::{os::fd::OwnedFd, sync::mpsc};

use smithay::reexports::calloop::{channel, LoopHandle};

use crate::state::Patina;

#[derive(Debug)]
pub enum Event {
    /// The machine is about to sleep (`true`) or has woken (`false`).
    PrepareForSleep(bool),
    /// Someone asked to lock this session (`loginctl lock-session`).
    Lock,
    /// A restart or shut down was refused (by polkit, usually).
    EndRefused,
}

#[derive(Debug)]
pub enum Request {
    Suspend,
    Reboot,
    PowerOff,
    /// Backlight `name` (in /sys/class/backlight) to `value`.
    SetBrightness(String, u32),
    /// Let the pending sleep go ahead (we have locked).
    ReleaseSleepDelay,
    /// After waking: hold the delay again for the next sleep.
    TakeSleepDelay,
}

const DEST: &str = "org.freedesktop.login1";
const MANAGER: &str = "/org/freedesktop/login1";

/// Connects to logind; returns where to send requests. Events are handled by
/// `Patina::logind_event`.
pub fn start(handle: &LoopHandle<'static, Patina>) -> Option<mpsc::Sender<Request>> {
    let conn = zbus::blocking::Connection::system()
        .map_err(|e| tracing::info!(error = %e, "no system bus: logind features off"))
        .ok()?;
    let manager =
        zbus::blocking::Proxy::new(&conn, DEST, MANAGER, "org.freedesktop.login1.Manager").ok()?;
    let session_path: zbus::zvariant::OwnedObjectPath = match std::env::var("XDG_SESSION_ID") {
        Ok(id) if !id.is_empty() => manager.call("GetSession", &(id,)).ok()?,
        _ => manager
            .call("GetSessionByPID", &(std::process::id(),))
            .map_err(|e| tracing::info!(error = %e, "not in a logind session"))
            .ok()?,
    };
    tracing::info!(session = %session_path.as_str(), "logind session");

    let (tx, rx) = channel::channel::<Event>();
    let _ = handle.insert_source(rx, |event, _, state| {
        if let channel::Event::Msg(e) = event {
            state.logind_event(e);
        }
    });

    // Sleep: hold a delay inhibitor so the screen is locked before the machine sleeps.
    {
        let conn = conn.clone();
        let tx = tx.clone();
        std::thread::spawn(move || {
            let Ok(manager) =
                zbus::blocking::Proxy::new(&conn, DEST, MANAGER, "org.freedesktop.login1.Manager")
            else {
                return;
            };
            let Ok(signals) = manager.receive_signal("PrepareForSleep") else {
                return;
            };
            for msg in signals {
                if let Ok(going) = msg.body().deserialize::<bool>() {
                    let _ = tx.send(Event::PrepareForSleep(going));
                }
            }
        });
    }
    // The session's Lock signal.
    {
        let conn = conn.clone();
        let tx = tx.clone();
        let path = session_path.clone();
        std::thread::spawn(move || {
            let Ok(session) =
                zbus::blocking::Proxy::new(&conn, DEST, path, "org.freedesktop.login1.Session")
            else {
                return;
            };
            let Ok(signals) = session.receive_signal("Lock") else {
                return;
            };
            for _ in signals {
                let _ = tx.send(Event::Lock);
            }
        });
    }

    let (req_tx, req_rx) = mpsc::channel::<Request>();
    let events = tx.clone();
    std::thread::spawn(move || {
        let Ok(manager) =
            zbus::blocking::Proxy::new(&conn, DEST, MANAGER, "org.freedesktop.login1.Manager")
        else {
            return;
        };
        let Ok(session) =
            zbus::blocking::Proxy::new(&conn, DEST, session_path, "org.freedesktop.login1.Session")
        else {
            return;
        };
        let inhibit = || -> Option<OwnedFd> {
            manager
                .call::<_, _, zbus::zvariant::OwnedFd>(
                    "Inhibit",
                    &(
                        "sleep",
                        "Patina",
                        "Lock the screen before sleeping",
                        "delay",
                    ),
                )
                .map(Into::into)
                .map_err(|e| tracing::info!(error = %e, "no sleep delay inhibitor"))
                .ok()
        };
        let mut delay = inhibit();
        while let Ok(req) = req_rx.recv() {
            let ends = matches!(req, Request::Reboot | Request::PowerOff);
            let result = match req {
                Request::Suspend => manager.call::<_, _, ()>("Suspend", &(false,)),
                // `true`: polkit may ask (other users signed in).
                Request::Reboot => manager.call::<_, _, ()>("Reboot", &(true,)),
                Request::PowerOff => manager.call::<_, _, ()>("PowerOff", &(true,)),
                Request::SetBrightness(name, value) => {
                    session.call::<_, _, ()>("SetBrightness", &("backlight", name.as_str(), value))
                }
                Request::ReleaseSleepDelay => {
                    // Closing the descriptor lets sleep proceed.
                    drop(delay.take());
                    Ok(())
                }
                Request::TakeSleepDelay => {
                    if delay.is_none() {
                        delay = inhibit();
                    }
                    Ok(())
                }
            };
            if let Err(e) = result {
                tracing::warn!(error = %e, "logind request failed");
                if ends {
                    let _ = events.send(Event::EndRefused);
                }
            }
        }
    });
    Some(req_tx)
}
