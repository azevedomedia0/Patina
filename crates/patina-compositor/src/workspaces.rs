//! Workspaces: four of them, each with its own windows, switched from the panel.
//!
//! Only the active workspace's windows are mapped in the space; the others are stashed with
//! their positions and stacking order and restored when their workspace becomes active. Shells
//! see and drive them through `ext-workspace-v1`.

use smithay::{
    desktop::Window,
    reexports::{
        wayland_protocols::ext::workspace::v1::server::{
            ext_workspace_group_handle_v1::{self, ExtWorkspaceGroupHandleV1, GroupCapabilities},
            ext_workspace_handle_v1::{self, ExtWorkspaceHandleV1, State, WorkspaceCapabilities},
            ext_workspace_manager_v1::{self, ExtWorkspaceManagerV1},
        },
        wayland_server::{
            Client, DataInit, Dispatch, DisplayHandle, GlobalDispatch, New, Resource,
        },
    },
    utils::{Logical, Point, SERIAL_COUNTER},
};

use crate::state::Patina;

pub const COUNT: usize = 4;

struct Bound {
    manager: ExtWorkspaceManagerV1,
    workspaces: Vec<ExtWorkspaceHandleV1>,
}

pub struct WorkspaceProtocol {
    bound: Vec<Bound>,
    /// `activate` requests wait here until the client sends `commit`.
    pending: Vec<usize>,
}

impl WorkspaceProtocol {
    pub fn new(dh: &DisplayHandle) -> Self {
        dh.create_global::<Patina, ExtWorkspaceManagerV1, ()>(1, ());
        Self {
            bound: Vec::new(),
            pending: Vec::new(),
        }
    }

    /// Tells every client which workspace is active.
    fn broadcast(&mut self, active: usize) {
        self.bound.retain(|b| b.manager.is_alive());
        for b in &self.bound {
            for (i, ws) in b.workspaces.iter().enumerate() {
                ws.state(state_bits(i == active));
            }
            b.manager.done();
        }
    }
}

fn state_bits(active: bool) -> State {
    if active {
        State::Active
    } else {
        State::empty()
    }
}

impl GlobalDispatch<ExtWorkspaceManagerV1, (), Patina> for Patina {
    fn bind(
        state: &mut Patina,
        dh: &DisplayHandle,
        client: &Client,
        resource: New<ExtWorkspaceManagerV1>,
        _: &(),
        data_init: &mut DataInit<'_, Patina>,
    ) {
        let manager = data_init.init(resource, ());
        let Ok(group) = client.create_resource::<ExtWorkspaceGroupHandleV1, (), Patina>(
            dh,
            manager.version(),
            (),
        ) else {
            return;
        };
        manager.workspace_group(&group);
        group.capabilities(GroupCapabilities::empty());

        let mut workspaces = Vec::new();
        for i in 0..COUNT {
            let Ok(ws) = client.create_resource::<ExtWorkspaceHandleV1, (), Patina>(
                dh,
                manager.version(),
                (),
            ) else {
                continue;
            };
            manager.workspace(&ws);
            ws.id((i + 1).to_string());
            ws.name((i + 1).to_string());
            ws.coordinates((i as u32).to_ne_bytes().to_vec());
            ws.state(state_bits(i == state.active_workspace));
            ws.capabilities(WorkspaceCapabilities::Activate);
            group.workspace_enter(&ws);
            workspaces.push(ws);
        }
        manager.done();
        state.workspace_protocol.bound.push(Bound {
            manager,
            workspaces,
        });
    }
}

impl Dispatch<ExtWorkspaceManagerV1, (), Patina> for Patina {
    fn request(
        state: &mut Patina,
        _: &Client,
        manager: &ExtWorkspaceManagerV1,
        request: ext_workspace_manager_v1::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Patina>,
    ) {
        match request {
            ext_workspace_manager_v1::Request::Commit => {
                // The last activation requested wins.
                let target = state.workspace_protocol.pending.pop();
                state.workspace_protocol.pending.clear();
                if let Some(i) = target {
                    state.switch_workspace(i);
                }
            }
            ext_workspace_manager_v1::Request::Stop => {
                manager.finished();
                state
                    .workspace_protocol
                    .bound
                    .retain(|b| &b.manager != manager);
            }
            _ => {}
        }
    }
}

impl Dispatch<ExtWorkspaceGroupHandleV1, (), Patina> for Patina {
    fn request(
        _: &mut Patina,
        _: &Client,
        _: &ExtWorkspaceGroupHandleV1,
        _: ext_workspace_group_handle_v1::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Patina>,
    ) {
        // The set of workspaces is fixed, so creating one is not offered.
    }
}

impl Dispatch<ExtWorkspaceHandleV1, (), Patina> for Patina {
    fn request(
        state: &mut Patina,
        _: &Client,
        handle: &ExtWorkspaceHandleV1,
        request: ext_workspace_handle_v1::Request,
        _: &(),
        _: &DisplayHandle,
        _: &mut DataInit<'_, Patina>,
    ) {
        if let ext_workspace_handle_v1::Request::Activate = request {
            let index = state
                .workspace_protocol
                .bound
                .iter()
                .find_map(|b| b.workspaces.iter().position(|w| w == handle));
            if let Some(i) = index {
                state.workspace_protocol.pending.push(i);
            }
        }
    }
}

impl Patina {
    /// Which workspace a window is on, or `None` for a window we do not know.
    pub fn window_workspace(&self, window: &Window) -> Option<usize> {
        if self.space.elements().any(|w| w == window) {
            return Some(self.active_workspace);
        }
        self.stashed
            .iter()
            .position(|stash| stash.iter().any(|(w, _)| w == window))
    }

    /// Makes workspace `n` the active one.
    pub fn switch_workspace(&mut self, n: usize) {
        if n >= COUNT || n == self.active_workspace {
            return;
        }
        // A drag across a switch would carry a window along; end it.
        self.end_drag();

        let old = self.active_workspace;
        let leaving: Vec<(Window, Point<i32, Logical>)> = self
            .space
            .elements()
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .filter_map(|w| self.client_rect(&w).map(|r| (w, r.loc)))
            .collect();
        for (w, _) in &leaving {
            self.space.unmap_elem(w);
        }
        self.stashed[old] = leaving;

        // Bottom to top, so the stacking order comes back as it was.
        for (w, loc) in std::mem::take(&mut self.stashed[n]) {
            self.space.map_element(w, loc, false);
        }
        self.active_workspace = n;

        let top = self.space.elements().next_back().cloned();
        match top {
            Some(window) => self.focus_window(&window),
            None => {
                if let Some(kb) = self.seat.get_keyboard() {
                    kb.set_focus(self, None, SERIAL_COUNTER.next_serial());
                }
            }
        }
        self.refresh_hover();
        self.workspace_protocol.broadcast(n);
        self.queue_redraw();
    }

    /// Moves `window` to workspace `n`, puts it on top there, and switches to that workspace so
    /// the window is not lost from sight.
    pub fn move_window_to_workspace(&mut self, window: &Window, n: usize) {
        if n >= COUNT || n == self.active_workspace {
            return;
        }
        let Some(rect) = self.client_rect(window) else {
            return;
        };
        self.end_drag();
        self.space.unmap_elem(window);
        // Last in the stash means top-most when the workspace is restored.
        self.stashed[n].push((window.clone(), rect.loc));
        self.switch_workspace(n);
    }

    /// Forgets a window that no longer exists, wherever it was kept.
    pub fn forget_window(
        &mut self,
        surface: &smithay::reexports::wayland_server::protocol::wl_surface::WlSurface,
    ) {
        for stash in &mut self.stashed {
            stash.retain(|(w, _)| w.toplevel().is_none_or(|t| t.wl_surface() != surface));
        }
        self.minimized
            .retain(|(w, _)| w.toplevel().is_none_or(|t| t.wl_surface() != surface));
    }
}
