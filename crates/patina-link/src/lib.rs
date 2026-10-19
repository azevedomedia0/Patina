//! What the shell (panel, dock) learns from the compositor and asks of it, on a private
//! Wayland connection, with standard protocols: the windows (`wlr-foreign-toplevel-management`),
//! the workspaces (`ext-workspace-v1`) and the outputs (`wl_output` names).
//!
//! The connection runs on its own thread. Updates arrive on a Tokio channel (what iced
//! subscriptions read); commands go the other way on a standard channel.

use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
};

use tokio::sync::mpsc::UnboundedSender;
use wayland_client::{
    event_created_child,
    globals::{registry_queue_init, GlobalList, GlobalListContents},
    protocol::{
        wl_output::{self, WlOutput},
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
    },
    Connection, Dispatch, Proxy, QueueHandle,
};
use wayland_protocols::ext::workspace::v1::client::{
    ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1},
    ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1, State as WorkspaceState},
    ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
};
use wayland_protocols_wlr::foreign_toplevel::v1::client::{
    zwlr_foreign_toplevel_handle_v1::{self, State as ToplevelState, ZwlrForeignToplevelHandleV1},
    zwlr_foreign_toplevel_manager_v1::{self, ZwlrForeignToplevelManagerV1},
};

/// One window, as the compositor reports it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WindowInfo {
    /// Stable for the window's life; used in commands.
    pub id: u32,
    pub title: String,
    pub app_id: String,
    pub activated: bool,
    pub minimized: bool,
    pub maximized: bool,
}

/// Something the compositor told us.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    /// All windows, after any change, in the order the compositor announced them.
    Windows(Vec<WindowInfo>),
    /// How many workspaces there are and which one is active.
    Workspaces { count: usize, active: usize },
    /// The outputs' names (such as `eDP-1`), after any change.
    Outputs(Vec<String>),
}

impl Update {
    /// Title of the focused window, falling back to its app id when it has no title.
    pub fn active_title(windows: &[WindowInfo]) -> Option<String> {
        windows
            .iter()
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
}

/// Something the shell asks the compositor to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Switch to workspace `n`, counting from zero.
    ActivateWorkspace(usize),
    /// Raise and focus a window (restoring it if minimised).
    Activate(u32),
    Minimize(u32),
    Close(u32),
}

struct Workspace {
    handle: ExtWorkspaceHandleV1,
    active: bool,
}

struct Output {
    global: u32,
    proxy: WlOutput,
    name: String,
}

/// Shared between the dispatch thread and the command thread.
#[derive(Default)]
struct Shared {
    manager: Option<ExtWorkspaceManagerV1>,
    workspaces: Vec<ExtWorkspaceHandleV1>,
    toplevels: HashMap<u32, ZwlrForeignToplevelHandleV1>,
    seat: Option<WlSeat>,
}

struct App {
    updates: UnboundedSender<Update>,
    shared: Arc<Mutex<Shared>>,
    windows: Vec<WindowInfo>,
    workspaces: Vec<Workspace>,
    outputs: Vec<Output>,
    sent_windows: Option<Vec<WindowInfo>>,
    sent_workspaces: Option<(usize, usize)>,
    sent_outputs: Option<Vec<String>>,
}

impl App {
    fn window(&mut self, id: u32) -> &mut WindowInfo {
        if let Some(i) = self.windows.iter().position(|w| w.id == id) {
            &mut self.windows[i]
        } else {
            self.windows.push(WindowInfo {
                id,
                ..WindowInfo::default()
            });
            self.windows.last_mut().unwrap()
        }
    }

    fn publish_windows(&mut self) {
        if self.sent_windows.as_ref() != Some(&self.windows) {
            self.sent_windows = Some(self.windows.clone());
            let _ = self.updates.send(Update::Windows(self.windows.clone()));
        }
    }

    fn publish_workspaces(&mut self) {
        let count = self.workspaces.len();
        let active = self.workspaces.iter().position(|w| w.active).unwrap_or(0);
        if count > 0 && self.sent_workspaces != Some((count, active)) {
            self.sent_workspaces = Some((count, active));
            let _ = self.updates.send(Update::Workspaces { count, active });
        }
        self.shared.lock().unwrap().workspaces =
            self.workspaces.iter().map(|w| w.handle.clone()).collect();
    }

    fn publish_outputs(&mut self) {
        let names: Vec<String> = self
            .outputs
            .iter()
            .map(|o| o.name.clone())
            .filter(|n| !n.is_empty())
            .collect();
        if self.sent_outputs.as_ref() != Some(&names) {
            self.sent_outputs = Some(names.clone());
            let _ = self.updates.send(Update::Outputs(names));
        }
    }
}

/// Starts the link. Updates arrive on `updates`; commands go to the returned sender. If the
/// compositor lacks a protocol, its updates never come and its commands do nothing.
pub fn spawn(updates: UnboundedSender<Update>, who: &'static str) -> mpsc::Sender<Command> {
    let (commands, command_rx) = mpsc::channel();
    std::thread::spawn(move || {
        if let Err(e) = run(updates, command_rx) {
            eprintln!("{who}: compositor link unavailable: {e}");
        }
    });
    commands
}

fn bind_optional<I, D>(
    globals: &GlobalList,
    qh: &QueueHandle<D>,
    versions: std::ops::RangeInclusive<u32>,
) -> Option<I>
where
    I: Proxy + 'static,
    D: Dispatch<I, ()> + 'static,
{
    globals.bind(qh, versions, ()).ok()
}

fn run(
    updates: UnboundedSender<Update>,
    command_rx: mpsc::Receiver<Command>,
) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, mut queue) = registry_queue_init::<App>(&conn)?;
    let qh = queue.handle();

    let _toplevels: Option<ZwlrForeignToplevelManagerV1> = bind_optional(&globals, &qh, 1..=3);
    let shared = Arc::new(Mutex::new(Shared::default()));
    {
        let mut s = shared.lock().unwrap();
        s.manager = bind_optional(&globals, &qh, 1..=1);
        s.seat = bind_optional(&globals, &qh, 1..=7);
    }

    let mut app = App {
        updates,
        shared: shared.clone(),
        windows: Vec::new(),
        workspaces: Vec::new(),
        outputs: Vec::new(),
        sent_windows: None,
        sent_workspaces: None,
        sent_outputs: None,
    };
    // Outputs present now; later ones arrive as registry events.
    for g in globals.contents().clone_list() {
        if g.interface == WlOutput::interface().name {
            bind_output(&mut app, globals.registry(), &qh, g.name, g.version);
        }
    }

    // Commands run on their own thread: the dispatch thread is blocked waiting for events.
    {
        let conn = conn.clone();
        std::thread::spawn(move || {
            while let Ok(command) = command_rx.recv() {
                let s = shared.lock().unwrap();
                match command {
                    Command::ActivateWorkspace(n) => {
                        if let (Some(m), Some(h)) = (&s.manager, s.workspaces.get(n)) {
                            h.activate();
                            m.commit();
                        }
                    }
                    Command::Activate(id) => {
                        if let (Some(h), Some(seat)) = (s.toplevels.get(&id), &s.seat) {
                            h.activate(seat);
                        }
                    }
                    Command::Minimize(id) => {
                        if let Some(h) = s.toplevels.get(&id) {
                            h.set_minimized();
                        }
                    }
                    Command::Close(id) => {
                        if let Some(h) = s.toplevels.get(&id) {
                            h.close();
                        }
                    }
                }
                let _ = conn.flush();
            }
        });
    }

    loop {
        queue.blocking_dispatch(&mut app)?;
    }
}

fn bind_output(
    app: &mut App,
    registry: &WlRegistry,
    qh: &QueueHandle<App>,
    name: u32,
    version: u32,
) {
    // Names come with version 4.
    let proxy: WlOutput = registry.bind(name, version.min(4), qh, ());
    app.outputs.push(Output {
        global: name,
        proxy,
        name: String::new(),
    });
}

impl Dispatch<WlRegistry, GlobalListContents> for App {
    fn event(
        app: &mut Self,
        registry: &WlRegistry,
        event: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match event {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } if interface == WlOutput::interface().name => {
                bind_output(app, registry, qh, name, version);
            }
            wl_registry::Event::GlobalRemove { name } => {
                if let Some(i) = app.outputs.iter().position(|o| o.global == name) {
                    let o = app.outputs.remove(i);
                    if o.proxy.version() >= 3 {
                        o.proxy.release();
                    }
                    app.publish_outputs();
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlOutput, ()> for App {
    fn event(
        app: &mut Self,
        proxy: &WlOutput,
        event: wl_output::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            wl_output::Event::Name { name } => {
                if let Some(o) = app.outputs.iter_mut().find(|o| &o.proxy == proxy) {
                    o.name = name;
                }
            }
            wl_output::Event::Done => app.publish_outputs(),
            _ => {}
        }
    }
}

impl Dispatch<WlSeat, ()> for App {
    fn event(
        _: &mut Self,
        _: &WlSeat,
        _: wl_seat::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

// ---- windows

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
            let id = toplevel.id().protocol_id();
            app.window(id);
            app.shared.lock().unwrap().toplevels.insert(id, toplevel);
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
            E::Title { title } => app.window(id).title = title,
            E::AppId { app_id } => app.window(id).app_id = app_id,
            E::State { state } => {
                // An array of native-endian u32 state values.
                let states: Vec<u32> = state
                    .chunks_exact(4)
                    .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                let w = app.window(id);
                w.activated = states.contains(&(ToplevelState::Activated as u32));
                w.minimized = states.contains(&(ToplevelState::Minimized as u32));
                w.maximized = states.contains(&(ToplevelState::Maximized as u32));
            }
            // `done` ends a batch of changes: now they are consistent.
            E::Done => app.publish_windows(),
            E::Closed => {
                app.windows.retain(|w| w.id != id);
                app.shared.lock().unwrap().toplevels.remove(&id);
                handle.destroy();
                app.publish_windows();
            }
            _ => {}
        }
    }
}

// ---- workspaces

impl Dispatch<ExtWorkspaceManagerV1, ()> for App {
    fn event(
        app: &mut Self,
        _: &ExtWorkspaceManagerV1,
        event: ext_workspace_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_workspace_manager_v1::Event::Workspace { workspace } => {
                app.workspaces.push(Workspace {
                    handle: workspace,
                    active: false,
                });
            }
            ext_workspace_manager_v1::Event::Done => app.publish_workspaces(),
            _ => {}
        }
    }

    event_created_child!(App, ExtWorkspaceManagerV1, [
        ext_workspace_manager_v1::EVT_WORKSPACE_GROUP_OPCODE => (ExtWorkspaceGroupHandleV1, ()),
        ext_workspace_manager_v1::EVT_WORKSPACE_OPCODE => (ExtWorkspaceHandleV1, ()),
    ]);
}

impl Dispatch<ExtWorkspaceGroupHandleV1, ()> for App {
    fn event(
        _: &mut Self,
        _: &ExtWorkspaceGroupHandleV1,
        _: ext_workspace_group_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ExtWorkspaceHandleV1, ()> for App {
    fn event(
        app: &mut Self,
        handle: &ExtWorkspaceHandleV1,
        event: ext_workspace_handle_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            ext_workspace_handle_v1::Event::State { state } => {
                let active = state
                    .into_result()
                    .is_ok_and(|s| s.contains(WorkspaceState::Active));
                if let Some(w) = app.workspaces.iter_mut().find(|w| &w.handle == handle) {
                    w.active = active;
                }
            }
            ext_workspace_handle_v1::Event::Removed => {
                app.workspaces.retain(|w| &w.handle != handle);
                handle.destroy();
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn w(id: u32, title: &str, app_id: &str, activated: bool) -> WindowInfo {
        WindowInfo {
            id,
            title: title.into(),
            app_id: app_id.into(),
            activated,
            ..WindowInfo::default()
        }
    }

    #[test]
    fn the_active_title_falls_back_to_the_app_id() {
        assert_eq!(
            Update::active_title(&[w(1, "A", "a", false), w(2, "B", "b", true)]),
            Some("B".into())
        );
        assert_eq!(
            Update::active_title(&[w(1, "", "foot", true)]),
            Some("foot".into())
        );
        assert_eq!(Update::active_title(&[w(1, "", "", true)]), None);
        assert_eq!(Update::active_title(&[]), None);
    }
}
