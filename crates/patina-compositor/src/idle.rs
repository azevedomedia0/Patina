//! What happens when nobody uses the computer: after a minute the screen dims ("Dim when idle:
//! lower brightness after a minute"), after 15 minutes it locks and the computer sleeps ("Sleep
//! after: 15 min"), as the prototype's defaults. Any input undims. Programs that ask not to be
//! interrupted (`idle-inhibit`, such as a video player) hold both off, and other programs can
//! watch idleness themselves (`ext-idle-notify`).
//!
//! The screen dims through the backlight when there is one (to 30% of where it was), else by
//! drawing black at 50% over it. `PATINA_IDLE_DIM` and `PATINA_IDLE_SLEEP` set the delays in
//! seconds (0 is never) until Settings has them.

use std::time::{Duration, Instant};

use smithay::{
    delegate_idle_inhibit, delegate_idle_notify,
    reexports::{
        calloop::timer::{TimeoutAction, Timer},
        wayland_server::{protocol::wl_surface::WlSurface, Resource},
    },
    wayland::{
        idle_inhibit::IdleInhibitHandler,
        idle_notify::{IdleNotifierHandler, IdleNotifierState},
    },
};

use crate::state::Patina;

/// How dark the overlay dimming is when there is no backlight to turn down.
pub const OVERLAY_ALPHA: f32 = 0.5;
/// The backlight drops to this fraction of where it was.
const DIM_TO: f32 = 0.3;

fn seconds(var: &str, default: u64) -> Option<Duration> {
    let s = std::env::var(var)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(default);
    (s > 0).then(|| Duration::from_secs(s))
}

#[derive(Debug)]
pub enum Dim {
    Backlight { name: String, previous: u32 },
    Overlay,
}

pub struct Idle {
    pub last_activity: Instant,
    pub dim_after: Option<Duration>,
    pub sleep_after: Option<Duration>,
    pub dimmed: Option<Dim>,
    /// Slept since the last activity (so it happens once).
    pub slept: bool,
    pub timer_armed: bool,
    pub inhibitors: Vec<WlSurface>,
}

impl Default for Idle {
    fn default() -> Self {
        Self {
            last_activity: Instant::now(),
            dim_after: seconds("PATINA_IDLE_DIM", 60),
            sleep_after: seconds("PATINA_IDLE_SLEEP", 15 * 60),
            dimmed: None,
            slept: false,
            timer_armed: false,
            inhibitors: Vec::new(),
        }
    }
}

impl Idle {
    /// When the next thing happens, if anything is left to happen.
    fn next_deadline(&self) -> Option<Instant> {
        let at = |d: Option<Duration>| d.map(|d| self.last_activity + d);
        let dim = at(self.dim_after).filter(|_| self.dimmed.is_none());
        let sleep = at(self.sleep_after).filter(|_| !self.slept);
        [dim, sleep].into_iter().flatten().min()
    }

    pub fn overlay(&self) -> bool {
        matches!(self.dimmed, Some(Dim::Overlay))
    }
}

/// The first backlight, and its current and maximum brightness.
fn backlight() -> Option<(String, u32, u32)> {
    let dir = std::fs::read_dir("/sys/class/backlight")
        .ok()?
        .flatten()
        .next()?;
    let read = |f: &str| -> Option<u32> {
        std::fs::read_to_string(dir.path().join(f))
            .ok()?
            .trim()
            .parse()
            .ok()
    };
    Some((
        dir.file_name().to_string_lossy().into(),
        read("brightness")?,
        read("max_brightness")?,
    ))
}

impl Patina {
    fn inhibited(&mut self) -> bool {
        self.idle.inhibitors.retain(|s| s.is_alive());
        !self.idle.inhibitors.is_empty()
    }

    /// Input happened: undim, and start counting again.
    pub fn reset_idle(&mut self) {
        self.idle.last_activity = Instant::now();
        self.idle.slept = false;
        let seat = self.seat.clone();
        self.idle_notifier_state.notify_activity(&seat);
        if let Some(dim) = self.idle.dimmed.take() {
            match dim {
                Dim::Backlight { name, previous } => {
                    self.logind_request(crate::logind::Request::SetBrightness(name, previous))
                }
                Dim::Overlay => self.queue_redraw(),
            }
        }
        self.arm_idle_timer();
    }

    /// Makes sure a timer will fire at the next idle deadline. While someone is active it fires
    /// about once a minute (it checks and re-arms), never more.
    pub fn arm_idle_timer(&mut self) {
        if self.idle.timer_armed {
            return;
        }
        let Some(deadline) = self.idle.next_deadline() else {
            return;
        };
        self.idle.timer_armed = true;
        let _ = self
            .loop_handle
            .insert_source(Timer::from_deadline(deadline), |_, _, state| {
                state.idle_check();
                match state.idle.next_deadline() {
                    Some(next) => TimeoutAction::ToInstant(
                        next.max(Instant::now() + Duration::from_millis(100)),
                    ),
                    None => {
                        state.idle.timer_armed = false;
                        TimeoutAction::Drop
                    }
                }
            });
    }

    fn idle_check(&mut self) {
        let idle_for = self.idle.last_activity.elapsed();
        if self.inhibited() {
            // Someone is watching a video: count from now.
            self.idle.last_activity = Instant::now();
            return;
        }
        if let Some(d) = self.idle.dim_after {
            if idle_for >= d && self.idle.dimmed.is_none() {
                self.dim();
            }
        }
        if let Some(s) = self.idle.sleep_after {
            if idle_for >= s && !self.idle.slept {
                self.idle.slept = true;
                tracing::info!("idle: locking and suspending");
                self.start_locker();
                self.logind_request(crate::logind::Request::Suspend);
            }
        }
    }

    fn dim(&mut self) {
        tracing::debug!("idle: dimming");
        match backlight() {
            Some((name, current, max)) if self.logind.is_some() && max > 0 => {
                let low = ((current as f32) * DIM_TO).round().max(1.0) as u32;
                self.logind_request(crate::logind::Request::SetBrightness(name.clone(), low));
                self.idle.dimmed = Some(Dim::Backlight {
                    name,
                    previous: current,
                });
            }
            _ => {
                self.idle.dimmed = Some(Dim::Overlay);
                self.queue_redraw();
            }
        }
    }

    pub fn logind_request(&self, req: crate::logind::Request) {
        if let Some(tx) = &self.logind {
            let _ = tx.send(req);
        }
    }

    pub fn logind_event(&mut self, event: crate::logind::Event) {
        match event {
            crate::logind::Event::PrepareForSleep(true) => {
                tracing::info!("going to sleep: locking first");
                self.start_locker();
                self.sleep_delay_pending = true;
                // If the locker is slow, sleep anyway after a short while.
                let _ = self.loop_handle.insert_source(
                    Timer::from_duration(Duration::from_secs(2)),
                    |_, _, state| {
                        state.release_sleep_delay();
                        TimeoutAction::Drop
                    },
                );
            }
            crate::logind::Event::PrepareForSleep(false) => {
                self.logind_request(crate::logind::Request::TakeSleepDelay);
                self.reset_idle();
                self.queue_redraw();
            }
            crate::logind::Event::Lock => self.start_locker(),
        }
    }

    /// Lets a pending sleep go ahead (the screen is locked, or the locker took too long).
    pub fn release_sleep_delay(&mut self) {
        if std::mem::take(&mut self.sleep_delay_pending) {
            self.logind_request(crate::logind::Request::ReleaseSleepDelay);
        }
    }
}

impl IdleNotifierHandler for Patina {
    fn idle_notifier_state(&mut self) -> &mut IdleNotifierState<Self> {
        &mut self.idle_notifier_state
    }
}

impl IdleInhibitHandler for Patina {
    fn inhibit(&mut self, surface: WlSurface) {
        self.idle.inhibitors.push(surface);
        self.idle_notifier_state.set_is_inhibited(true);
    }

    fn uninhibit(&mut self, surface: WlSurface) {
        self.idle.inhibitors.retain(|s| s != &surface);
        let still = self.inhibited();
        self.idle_notifier_state.set_is_inhibited(still);
        self.reset_idle();
    }
}

delegate_idle_notify!(Patina);
delegate_idle_inhibit!(Patina);
