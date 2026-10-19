//! The title of the focused window, learned from the compositor through
//! `wlr-foreign-toplevel-management` on a second, private Wayland connection.

use std::collections::HashMap;

use tokio::sync::mpsc::UnboundedSender;
use wayland_client::{
    event_created_child,
    globals::{registry_queue_init, GlobalListContents},
    protocol::wl_registry::WlRegistry,
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, State, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

#[derive(Default)]
struct Window {
    title: String,
    app_id: String,
    activated: bool,
}

struct App {
    windows: HashMap<u32, Window>,
    tx: UnboundedSender<Option<String>>,
    /// What was last sent, so unchanged state is not sent again.
    sent: Option<Option<String>>,
}

impl App {
    /// Title of the active window, falling back to its app id when it has no title.
    fn active_title(&self) -> Option<String> {
        self.windows
            .values()
            .find(|w| w.activated)
            .map(|w| {
                if w.title.is_empty() {
                    w.app_id.clone()
                } else {
                    w.title.clone()
                }
            })
            .filter(|t| !t.is_empty())
    }

    fn publish(&mut self) {
        let now = self.active_title();
        if self.sent.as_ref() != Some(&now) {
            self.sent = Some(now.clone());
            let _ = self.tx.send(now);
        }
    }
}

/// Starts watching windows. Sends `Some(title)` for the active window and `None` when no window
/// is active. If the compositor does not offer the protocol, nothing is ever sent.
pub fn spawn(tx: UnboundedSender<Option<String>>) {
    std::thread::spawn(move || {
        if let Err(e) = run(tx) {
            eprintln!("patina-panel: window titles unavailable: {e}");
        }
    });
}

fn run(tx: UnboundedSender<Option<String>>) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<App>(&conn)?;
    let qh = queue.handle();
    let _manager: ZwlrForeignToplevelManagerV1 = globals.bind(&qh, 1..=3, ())?;
    let mut app = App {
        windows: HashMap::new(),
        tx,
        sent: None,
    };
    loop {
        queue.blocking_dispatch(&mut app)?;
    }
}

impl Dispatch<WlRegistry, GlobalListContents> for App {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wayland_client::protocol::wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, ()> for App {
    fn event(
        app: &mut Self,
        _: &ZwlrForeignToplevelManagerV1,
        event: zwlr_foreign_toplevel_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Event::Toplevel { toplevel } = event {
            app.windows
                .insert(toplevel.id().protocol_id(), Window::default());
        }
    }

    event_created_child!(App, ZwlrForeignToplevelManagerV1, [
        zwlr_foreign_toplevel_manager_v1::EVT_TOPLEVEL_OPCODE => (ZwlrForeignToplevelHandleV1, ()),
    ]);
}

impl Dispatch<ZwlrForeignToplevelHandleV1, ()> for App {
    fn event(
        app: &mut Self,
        handle: &ZwlrForeignToplevelHandleV1,
        event: zwlr_foreign_toplevel_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use zwlr_foreign_toplevel_handle_v1::Event as E;
        let id = handle.id().protocol_id();
        match event {
            E::Title { title } => {
                app.windows.entry(id).or_default().title = title;
            }
            E::AppId { app_id } => {
                app.windows.entry(id).or_default().app_id = app_id;
            }
            E::State { state } => {
                // An array of native-endian u32 state values.
                let activated = state
                    .chunks_exact(4)
                    .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .any(|s| s == State::Activated as u32);
                app.windows.entry(id).or_default().activated = activated;
            }
            // `done` ends a batch of changes: now they are consistent.
            E::Done => app.publish(),
            E::Closed => {
                app.windows.remove(&id);
                handle.destroy();
                app.publish();
            }
            _ => {}
        }
    }
}
