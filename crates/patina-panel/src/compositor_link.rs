//! What the panel learns from the compositor, and what it asks of it, on a private Wayland
//! connection: the focused window's title (`wlr-foreign-toplevel-management`) and the
//! workspaces (`ext-workspace-v1`).

use std::{
    collections::HashMap,
    sync::{mpsc, Arc, Mutex},
};

use tokio::sync::mpsc::UnboundedSender;
use wayland_client::{
    event_created_child,
    globals::{registry_queue_init, GlobalList, GlobalListContents},
    protocol::wl_registry::WlRegistry,
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

/// Something the compositor told us.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Update {
    /// Title of the focused window; `None` when no window has focus.
    ActiveTitle(Option<String>),
    /// How many workspaces there are and which one is active.
    Workspaces { count: usize, active: usize },
}

/// Something the panel asks the compositor to do.
#[derive(Debug, Clone, Copy)]
pub enum Command {
    /// Switch to workspace `n`, counting from zero.
    ActivateWorkspace(usize),
}

#[derive(Default)]
struct Window {
    title: String,
    app_id: String,
    activated: bool,
}

struct Workspace {
    handle: ExtWorkspaceHandleV1,
    active: bool,
}

/// Shared between the dispatch thread and the command thread.
#[derive(Default)]
struct Shared {
    manager: Option<ExtWorkspaceManagerV1>,
    handles: Vec<ExtWorkspaceHandleV1>,
}

struct App {
    updates: UnboundedSender<Update>,
    shared: Arc<Mutex<Shared>>,
    windows: HashMap<u32, Window>,
    workspaces: Vec<Workspace>,
    sent_title: Option<Option<String>>,
    sent_workspaces: Option<(usize, usize)>,
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

    fn publish_title(&mut self) {
        let now = self.active_title();
        if self.sent_title.as_ref() != Some(&now) {
            self.sent_title = Some(now.clone());
            let _ = self.updates.send(Update::ActiveTitle(now));
        }
    }

    fn publish_workspaces(&mut self) {
        let count = self.workspaces.len();
        let active = self.workspaces.iter().position(|w| w.active).unwrap_or(0);
        if count > 0 && self.sent_workspaces != Some((count, active)) {
            self.sent_workspaces = Some((count, active));
            let _ = self.updates.send(Update::Workspaces { count, active });
        }
        self.shared.lock().unwrap().handles =
            self.workspaces.iter().map(|w| w.handle.clone()).collect();
    }
}

/// Starts the link. Updates arrive on `updates`; commands go to the returned sender. If the
/// compositor offers neither protocol, nothing is ever sent and commands do nothing.
pub fn spawn(updates: UnboundedSender<Update>) -> mpsc::Sender<Command> {
    let (commands, command_rx) = mpsc::channel();
    std::thread::spawn(move || {
        if let Err(e) = run(updates, command_rx) {
            eprintln!("patina-panel: compositor link unavailable: {e}");
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
    shared.lock().unwrap().manager = bind_optional(&globals, &qh, 1..=1);

    // Commands run on their own thread: the dispatch thread is blocked waiting for events.
    {
        let shared = shared.clone();
        let conn = conn.clone();
        std::thread::spawn(move || {
            while let Ok(command) = command_rx.recv() {
                let Command::ActivateWorkspace(n) = command;
                let shared = shared.lock().unwrap();
                if let (Some(manager), Some(handle)) = (&shared.manager, shared.handles.get(n)) {
                    handle.activate();
                    manager.commit();
                    let _ = conn.flush();
                }
            }
        });
    }

    let mut app = App {
        updates,
        shared,
        windows: HashMap::new(),
        workspaces: Vec::new(),
        sent_title: None,
        sent_workspaces: None,
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
            E::Title { title } => app.windows.entry(id).or_default().title = title,
            E::AppId { app_id } => app.windows.entry(id).or_default().app_id = app_id,
            E::State { state } => {
                // An array of native-endian u32 state values.
                let activated = state
                    .chunks_exact(4)
                    .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .any(|s| s == ToplevelState::Activated as u32);
                app.windows.entry(id).or_default().activated = activated;
            }
            // `done` ends a batch of changes: now they are consistent.
            E::Done => app.publish_title(),
            E::Closed => {
                app.windows.remove(&id);
                handle.destroy();
                app.publish_title();
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
