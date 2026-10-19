//! Locking the session (`ext-session-lock-v1`).
//!
//! A locker (`patina-lock`) asks to lock; from then on every output shows only its lock surface
//! (black until it has one), input goes only to it, and no window content is drawn. The lock is
//! confirmed once every output has drawn a locked frame. If the locker dies without unlocking,
//! the session stays locked and a new locker may take over (Ctrl+Alt+L starts one).

use std::collections::HashMap;

use smithay::{
    delegate_session_lock,
    output::Output,
    reexports::wayland_server::protocol::{wl_output::WlOutput, wl_surface::WlSurface},
    utils::SERIAL_COUNTER,
    wayland::session_lock::{
        LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker,
    },
};

use crate::state::Patina;

/// The program started to lock the screen (Ctrl+Alt+L, idle, before suspend).
pub fn locker_command() -> String {
    std::env::var("PATINA_LOCKER").unwrap_or_else(|_| "patina-lock".into())
}

#[derive(Default)]
pub struct Lock {
    /// Locked (or locking): nothing but lock surfaces is drawn.
    pub active: bool,
    /// The locker's request, confirmed once every output has drawn a locked frame.
    pub pending: Option<SessionLocker>,
    /// Lock surfaces by output name.
    pub surfaces: HashMap<String, LockSurface>,
}

impl Lock {
    /// Whether a live locker is showing something.
    pub fn has_live_surface(&self) -> bool {
        self.surfaces.values().any(|s| s.alive())
    }
}

impl Patina {
    pub fn is_locked(&self) -> bool {
        self.lock.active
    }

    /// The lock surface on `output`, if the locker has made one.
    pub fn lock_surface(&self, output: &Output) -> Option<WlSurface> {
        self.lock
            .surfaces
            .get(&output.name())
            .filter(|s| s.alive())
            .map(|s| s.wl_surface().clone())
    }

    /// Starts the locker, unless a live one is already showing.
    pub fn start_locker(&mut self) {
        if self.lock.active && self.lock.has_live_surface() {
            return;
        }
        let cmd = locker_command();
        match crate::supervise::spawn_once(&self.socket_name.to_string_lossy(), &cmd) {
            Ok(_) => tracing::info!(%cmd, "locker started"),
            Err(e) => tracing::warn!(%cmd, error = %e, "cannot start the locker"),
        }
    }

    /// Called after a frame was drawn while locked: once every output has, confirm the lock.
    pub fn confirm_lock_if_drawn(&mut self) {
        if self.lock.active {
            if let Some(locker) = self.lock.pending.take() {
                locker.lock();
                tracing::info!("session locked");
            }
        }
    }

    /// Gives the keyboard to the lock surface of the output the pointer is on.
    pub fn focus_lock_surface(&mut self) {
        let surface = self
            .pointer_output()
            .and_then(|o| self.lock_surface(&o))
            .or_else(|| {
                self.lock
                    .surfaces
                    .values()
                    .find(|s| s.alive())
                    .map(|s| s.wl_surface().clone())
            });
        if let Some(keyboard) = self.seat.get_keyboard() {
            keyboard.set_focus(self, surface, SERIAL_COUNTER.next_serial());
        }
    }

    /// Sizes a lock surface to its output.
    fn configure_lock_surface(&self, surface: &LockSurface, output: &Output) {
        let Some(geo) = self.space.output_geometry(output) else {
            return;
        };
        surface.with_pending_state(|s| {
            s.size = Some((geo.size.w as u32, geo.size.h as u32).into());
        });
        surface.send_configure();
    }
}

impl SessionLockHandler for Patina {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.session_lock_state
    }

    fn lock(&mut self, confirmation: SessionLocker) {
        if self.lock.active && self.lock.has_live_surface() {
            // Locked by someone alive: refuse (dropping the locker tells the client).
            tracing::warn!("lock refused: already locked");
            return;
        }
        // A new lock, or a new locker taking over from one that died.
        self.lock.active = true;
        self.lock.surfaces.clear();
        self.lock.pending = Some(confirmation);
        self.drag = None;
        if let Some(keyboard) = self.seat.get_keyboard() {
            keyboard.set_focus(self, None, SERIAL_COUNTER.next_serial());
        }
        self.queue_redraw();
    }

    fn unlock(&mut self) {
        tracing::info!("session unlocked");
        self.lock = Lock::default();
        let top = self.space.elements().next_back().cloned();
        match top {
            Some(w) => self.focus_window(&w),
            None => {
                if let Some(keyboard) = self.seat.get_keyboard() {
                    keyboard.set_focus(self, None, SERIAL_COUNTER.next_serial());
                }
            }
        }
        self.reset_idle();
        self.queue_redraw();
    }

    fn new_surface(&mut self, surface: LockSurface, output: WlOutput) {
        let Some(output) = Output::from_resource(&output) else {
            return;
        };
        self.configure_lock_surface(&surface, &output);
        self.lock.surfaces.insert(output.name(), surface);
        self.focus_lock_surface();
        self.queue_redraw();
    }
}

delegate_session_lock!(Patina);
