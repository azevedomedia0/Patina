//! `wlr-foreign-toplevel-management`: tells shells such as the panel (and later the dock) which
//! windows exist, what they are called, and which one is active, and lets them ask for a window
//! to be activated, minimised, maximised or closed.

use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols_wlr::foreign_toplevel::v1::server::{
            zwlr_foreign_toplevel_handle_v1::{
                self, State as HandleState, ZwlrForeignToplevelHandleV1,
            },
            zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
        },
        wayland_server::{
            protocol::wl_surface::WlSurface, Client, DataInit, Dispatch, DisplayHandle,
            GlobalDispatch, New, Resource,
        },
    },
    wayland::{compositor::with_states, shell::xdg::XdgToplevelSurfaceData},
};

use crate::{state::Patina, wm::win_state};

const VERSION: u32 = 3;

/// What a client was last told about a window.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Snapshot {
    pub title: String,
    pub app_id: String,
    pub activated: bool,
    pub maximized: bool,
    pub minimized: bool,
}

impl Snapshot {
    fn states(&self) -> Vec<u8> {
        let mut states = Vec::new();
        for (on, s) in [
            (self.maximized, HandleState::Maximized),
            (self.minimized, HandleState::Minimized),
            (self.activated, HandleState::Activated),
        ] {
            if on {
                states.extend((s as u32).to_ne_bytes());
            }
        }
        states
    }
}

struct Entry {
    surface: WlSurface,
    handles: Vec<ZwlrForeignToplevelHandleV1>,
    sent: Snapshot,
}

pub struct ToplevelInfoState {
    managers: Vec<ZwlrForeignToplevelManagerV1>,
    entries: Vec<Entry>,
}

impl ToplevelInfoState {
    pub fn new(dh: &DisplayHandle) -> Self {
        dh.create_global::<Patina, ZwlrForeignToplevelManagerV1, ()>(VERSION, ());
        Self {
            managers: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// The window surface a protocol handle stands for.
    fn surface_for(&self, handle: &ZwlrForeignToplevelHandleV1) -> Option<WlSurface> {
        self.entries
            .iter()
            .find(|e| e.handles.iter().any(|h| h == handle))
            .map(|e| e.surface.clone())
    }

    /// Brings every client up to date with `current`: announces new windows, sends what changed,
    /// and reports windows that are gone as closed.
    pub fn update(&mut self, dh: &DisplayHandle, current: Vec<(WlSurface, Snapshot)>) {
        self.managers.retain(|m| m.is_alive());

        // Windows that disappeared.
        self.entries.retain(|e| {
            let alive = current.iter().any(|(s, _)| *s == e.surface);
            if !alive {
                for h in e.handles.iter().filter(|h| h.is_alive()) {
                    h.closed();
                }
            }
            alive
        });

        for (surface, snap) in current {
            match self.entries.iter_mut().find(|e| e.surface == surface) {
                Some(entry) => {
                    entry.handles.retain(|h| h.is_alive());
                    if entry.sent != snap {
                        for h in &entry.handles {
                            send_changes(h, &entry.sent, &snap);
                        }
                        entry.sent = snap;
                    }
                }
                None => {
                    let mut handles = Vec::new();
                    for manager in &self.managers {
                        if let Some(h) = announce(dh, manager, &snap) {
                            handles.push(h);
                        }
                    }
                    self.entries.push(Entry {
                        surface,
                        handles,
                        sent: snap,
                    });
                }
            }
        }
    }
}

/// Creates a handle for `manager`'s client and sends everything known about the window.
fn announce(
    dh: &DisplayHandle,
    manager: &ZwlrForeignToplevelManagerV1,
    snap: &Snapshot,
) -> Option<ZwlrForeignToplevelHandleV1> {
    let client = manager.client()?;
    let handle = client
        .create_resource::<ZwlrForeignToplevelHandleV1, (), Patina>(dh, manager.version(), ())
        .ok()?;
    manager.toplevel(&handle);
    send_changes(&handle, &Snapshot::default(), snap);
    // The first announcement always carries title, app id and state, even when empty.
    handle.title(snap.title.clone());
    handle.app_id(snap.app_id.clone());
    handle.state(snap.states());
    handle.done();
    Some(handle)
}

fn send_changes(handle: &ZwlrForeignToplevelHandleV1, old: &Snapshot, new: &Snapshot) {
    if old.title != new.title {
        handle.title(new.title.clone());
    }
    if old.app_id != new.app_id {
        handle.app_id(new.app_id.clone());
    }
    if (old.activated, old.maximized, old.minimized)
        != (new.activated, new.maximized, new.minimized)
    {
        handle.state(new.states());
    }
    handle.done();
}

impl GlobalDispatch<ZwlrForeignToplevelManagerV1, (), Patina> for Patina {
    fn bind(
        state: &mut Patina,
        dh: &DisplayHandle,
        _client: &Client,
        resource: New<ZwlrForeignToplevelManagerV1>,
        _: &(),
        data_init: &mut DataInit<'_, Patina>,
    ) {
        let manager = data_init.init(resource, ());
        // Tell the new client about every window that already exists.
        let existing: Vec<(usize, Snapshot)> = state
            .toplevel_info
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| (i, e.sent.clone()))
            .collect();
        for (i, snap) in existing {
            if let Some(h) = announce(dh, &manager, &snap) {
                state.toplevel_info.entries[i].handles.push(h);
            }
        }
        state.toplevel_info.managers.push(manager);
    }
}

impl Dispatch<ZwlrForeignToplevelManagerV1, (), Patina> for Patina {
    fn request(
        state: &mut Patina,
        _: &Client,
        manager: &ZwlrForeignToplevelManagerV1,
        request: zwlr_foreign_toplevel_manager_v1::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Patina>,
    ) {
        if let zwlr_foreign_toplevel_manager_v1::Request::Stop = request {
            manager.finished();
            state.toplevel_info.managers.retain(|m| m != manager);
        }
    }
}

impl Dispatch<ZwlrForeignToplevelHandleV1, (), Patina> for Patina {
    fn request(
        state: &mut Patina,
        _: &Client,
        handle: &ZwlrForeignToplevelHandleV1,
        request: zwlr_foreign_toplevel_handle_v1::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Patina>,
    ) {
        use zwlr_foreign_toplevel_handle_v1::Request as R;
        let Some(window) = state
            .toplevel_info
            .surface_for(handle)
            .and_then(|surface| state.window_by_surface(&surface))
        else {
            return;
        };
        match request {
            R::Activate { .. } => state.activate_window(&window),
            R::Close => state.close(&window),
            R::SetMinimized => state.minimize_if_visible(&window),
            R::UnsetMinimized => state.activate_window(&window),
            R::SetMaximized => state.set_maximized(&window, true),
            R::UnsetMaximized => state.set_maximized(&window, false),
            R::Destroy => {}
            // Rectangles for minimise animations and fullscreen are not supported yet.
            _ => {}
        }
        state.queue_redraw();
    }
}

impl Patina {
    /// Window for a toplevel surface, whether it is on screen or minimised.
    pub fn window_by_surface(&self, surface: &WlSurface) -> Option<Window> {
        self.space
            .elements()
            .chain(self.minimized.iter().map(|(w, _)| w))
            .find(|w| w.toplevel().is_some_and(|t| t.wl_surface() == surface))
            .cloned()
    }

    /// Brings a window to the front and focuses it, restoring it first if it was minimised.
    pub fn activate_window(&mut self, window: &Window) {
        if let Some(i) = self.minimized.iter().position(|(w, _)| w == window) {
            let (w, loc) = self.minimized.remove(i);
            self.space.map_element(w, loc, true);
        }
        self.focus_window(window);
    }

    pub fn minimize_if_visible(&mut self, window: &Window) {
        if self.space.elements().any(|w| w == window) {
            self.minimize(window);
        }
    }

    pub fn set_maximized(&mut self, window: &Window, want: bool) {
        if win_state(window).borrow().maximized != want {
            self.toggle_maximize(window);
        }
    }

    /// Sends window changes to shells. Called once per frame, so a burst of changes becomes one
    /// update.
    pub fn refresh_toplevel_info(&mut self) {
        let focus = self.seat.get_keyboard().and_then(|k| k.current_focus());
        let mut current = Vec::new();
        let windows = self
            .space
            .elements()
            .map(|w| (w, false))
            .chain(self.minimized.iter().map(|(w, _)| (w, true)));
        for (window, minimized) in windows {
            let Some(toplevel) = window.toplevel() else {
                continue;
            };
            // Not announced until the client has committed once, so shells never see a window
            // that is not on screen yet.
            if !crate::wm::initial_configure_sent(toplevel) {
                continue;
            }
            let surface = toplevel.wl_surface().clone();
            let (title, app_id) = with_states(&surface, |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .map(|d| {
                        let d = d.lock().unwrap();
                        (
                            d.title.clone().unwrap_or_default(),
                            d.app_id.clone().unwrap_or_default(),
                        )
                    })
                    .unwrap_or_default()
            });
            let snapshot = Snapshot {
                title,
                app_id,
                activated: !minimized && focus.as_ref() == Some(&surface),
                maximized: win_state(window).borrow().maximized,
                minimized,
            };
            current.push((surface, snapshot));
        }
        let dh = self.display_handle.clone();
        self.toplevel_info.update(&dh, current);
    }
}
