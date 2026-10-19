//! Showing another user's sign-in while this session stays locked, through the display
//! manager: LightDM and SDDM (`org.freedesktop.DisplayManager`, the seat's `SwitchToUser` or
//! `SwitchToGreeter`) and GDM (`org.gnome.DisplayManager`, `CreateTransientDisplay`). greetd
//! cannot run a second sign-in beside a session, so there it is not available.

use zbus::blocking::{Connection, Proxy};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Switcher {
    /// The freedesktop display manager interface on this seat (LightDM, SDDM).
    Freedesktop {
        seat: String,
    },
    Gdm,
}

fn has_owner(conn: &Connection, name: &str) -> bool {
    Proxy::new(
        conn,
        "org.freedesktop.DBus",
        "/org/freedesktop/DBus",
        "org.freedesktop.DBus",
    )
    .and_then(|p| p.call::<_, _, bool>("NameHasOwner", &(name,)))
    .unwrap_or(false)
}

/// What can switch users here, if anything.
pub fn available() -> Option<Switcher> {
    let conn = Connection::system().ok()?;
    if has_owner(&conn, "org.freedesktop.DisplayManager") {
        // LightDM tells the session its seat; SDDM has one seat object.
        let seat = std::env::var("XDG_SEAT_PATH")
            .unwrap_or_else(|_| "/org/freedesktop/DisplayManager/Seat0".into());
        return Some(Switcher::Freedesktop { seat });
    }
    if has_owner(&conn, "org.gnome.DisplayManager") {
        return Some(Switcher::Gdm);
    }
    None
}

/// Shows the sign-in for `user` (or the display manager's own sign-in screen where it cannot
/// preselect a user).
pub fn switch_to(switcher: &Switcher, user: &str) -> Result<(), String> {
    let conn = Connection::system().map_err(|e| e.to_string())?;
    match switcher {
        Switcher::Freedesktop { seat } => {
            let p = Proxy::new(
                &conn,
                "org.freedesktop.DisplayManager",
                seat.as_str(),
                "org.freedesktop.DisplayManager.Seat",
            )
            .map_err(|e| e.to_string())?;
            p.call::<_, _, ()>("SwitchToUser", &(user, ""))
                .or_else(|_| p.call::<_, _, ()>("SwitchToGreeter", &()))
                .map_err(|e| e.to_string())
        }
        Switcher::Gdm => Proxy::new(
            &conn,
            "org.gnome.DisplayManager",
            "/org/gnome/DisplayManager/LocalDisplayFactory",
            "org.gnome.DisplayManager.LocalDisplayFactory",
        )
        .and_then(|p| {
            p.call::<_, _, zbus::zvariant::OwnedObjectPath>("CreateTransientDisplay", &())
        })
        .map(|_| ())
        .map_err(|e| e.to_string()),
    }
}
