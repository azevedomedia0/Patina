//! Starts the session's autostart programs (XDG Autostart), then exits. `patina-session`
//! runs it once the compositor is up.

fn main() {
    for app in patina_apps::autostart::entries() {
        match patina_apps::launch(&app) {
            Ok(()) => eprintln!("patina-autostart: started {}", app.id),
            Err(e) => eprintln!("patina-autostart: cannot start {}: {e}", app.id),
        }
    }
}
