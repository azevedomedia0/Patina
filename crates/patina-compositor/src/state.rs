//! Compositor state and the Wayland protocol handlers.

use std::{ffi::OsString, os::unix::io::OwnedFd, sync::Arc, time::Instant};

use smithay::{
    delegate_compositor, delegate_cursor_shape, delegate_data_device, delegate_fractional_scale,
    delegate_layer_shell, delegate_output, delegate_seat, delegate_shm, delegate_viewporter,
    delegate_xdg_decoration, delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, get_popup_toplevel_coords, layer_map_for_output,
        LayerSurface as DesktopLayerSurface, PopupKeyboardGrab, PopupKind, PopupManager,
        PopupPointerGrab, PopupUngrabStrategy, Space, Window, WindowSurfaceType,
    },
    input::pointer::{CursorIcon, Focus},
    input::{keyboard::XkbConfig, pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    output::Output,
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, LoopSignal, Mode, PostAction},
        wayland_protocols::xdg::{
            decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode as DecorationMode,
            shell::server::xdg_toplevel,
        },
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::{wl_buffer::WlBuffer, wl_output::WlOutput, wl_seat, wl_surface::WlSurface},
            Client, Display, DisplayHandle,
        },
    },
    utils::{Logical, Rectangle, Serial, SERIAL_COUNTER},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, with_states, CompositorClientState, CompositorHandler,
            CompositorState,
        },
        cursor_shape::CursorShapeManagerState,
        fractional_scale::{
            with_fractional_scale, FractionalScaleHandler, FractionalScaleManagerState,
        },
        output::{OutputHandler, OutputManagerState},
        selection::{
            data_device::{
                ClientDndGrabHandler, DataDeviceHandler, DataDeviceState, ServerDndGrabHandler,
            },
            SelectionHandler,
        },
        shell::{
            wlr_layer::{Layer, LayerSurfaceData, WlrLayerShellHandler, WlrLayerShellState},
            xdg::{
                decoration::{XdgDecorationHandler, XdgDecorationState},
                PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            },
        },
        shm::{ShmHandler, ShmState},
        socket::ListeningSocketSource,
        viewporter::ViewporterState,
    },
};

pub struct Patina {
    pub start_time: Instant,
    pub loop_handle: smithay::reexports::calloop::LoopHandle<'static, Patina>,
    /// DRM backend state; `None` when running nested in winit.
    pub drm: Option<crate::drm::DrmBackend>,
    /// We draw the pointer ourselves (DRM); winit shows the host's pointer instead.
    pub software_cursor: bool,
    /// Something changed since the last frame. Rendering is driven by this, never by a timer.
    pub redraw_wanted: bool,
    /// Wakes the event loop to draw; registered by the active backend.
    pub redraw_ping: Option<smithay::reexports::calloop::ping::Ping>,
    /// The design's arrow, per output scale.
    pub cursor: crate::cursor::Arrows,
    pub cursor_theme: crate::cursor::Theme,
    /// What the client under the pointer last asked for. Smithay resets it on focus changes.
    pub cursor_status: CursorImageStatus,
    /// Set while the pointer is over a part of a window the compositor owns, such as the resize
    /// grip; takes priority over whatever a client asked for.
    pub frame_cursor: Option<CursorIcon>,
    pub rasterizer: patina_frame::Rasterizer,
    /// `PATINA_NO_BLUR=1`: draw without blur, for slow machines and for testing.
    pub no_blur: bool,
    /// The command started for every output, with `PATINA_OUTPUT` set to its name (the panel).
    pub per_output_cmd: Option<String>,
    pub panels: Vec<(String, std::process::Child)>,
    /// `PATINA_FULL_RECOMPOSE=1`: repaint the whole scene every time.
    pub full_recompose: bool,
    /// `PATINA_VERIFY_RECOMPOSE=1`: check each partial repaint against a full one.
    pub verify_recompose: bool,
    pub verified_frames: u32,
    pub mismatched_frames: u32,
    pub toplevel_info: crate::toplevel_info::ToplevelInfoState,
    pub workspace_protocol: crate::workspaces::WorkspaceProtocol,
    pub active_workspace: usize,
    /// Windows of the workspaces that are not active, with where they were, bottom to top.
    pub stashed: Vec<Vec<(Window, smithay::utils::Point<i32, smithay::utils::Logical>)>>,
    pub drag: Option<crate::wm::Drag>,
    /// A press was consumed by a window frame; its release must be swallowed too.
    pub frame_press: bool,
    pub minimized: Vec<(Window, smithay::utils::Point<i32, smithay::utils::Logical>)>,
    pub last_titlebar_click: Option<(Instant, Window)>,
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,
    pub socket_name: OsString,

    /// Windows, and the outputs placed side by side in it.
    pub space: Space<Window>,
    /// Menus and other popups of windows and panels.
    pub popups: PopupManager,

    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub layer_shell_state: WlrLayerShellState,
    /// Held only to keep the xdg-decoration global alive.
    #[allow(dead_code)]
    pub decoration_state: XdgDecorationState,
    pub shm_state: ShmState,
    /// Held only to keep the wp_viewporter global alive (required by iced_layershell).
    #[allow(dead_code)]
    pub viewporter_state: ViewporterState,
    /// Held only to keep the cursor-shape global alive.
    #[allow(dead_code)]
    pub cursor_shape_state: CursorShapeManagerState,
    /// Held only to keep the fractional-scale global alive.
    #[allow(dead_code)]
    pub fractional_scale_state: FractionalScaleManagerState,
    /// Held only to keep the xdg-output global alive.
    #[allow(dead_code)]
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,
}

impl Patina {
    pub fn new(event_loop: &mut EventLoop<'static, Self>, display: Display<Self>) -> Self {
        let dh = display.handle();
        let mut seat_state = SeatState::new();
        let mut seat = seat_state.new_wl_seat(&dh, "patina-seat0");
        seat.add_keyboard(XkbConfig::default(), 200, 25)
            .expect("keyboard");
        seat.add_pointer();

        let socket = ListeningSocketSource::new_auto().expect("wayland socket");
        let socket_name = socket.socket_name().to_os_string();
        let handle = event_loop.handle();
        handle
            .insert_source(socket, |stream, _, state: &mut Patina| {
                let _ = state
                    .display_handle
                    .insert_client(stream, Arc::new(ClientState::default()));
            })
            .expect("socket source");
        handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // SAFETY: the display is never dropped while the loop runs.
                    unsafe { display.get_mut().dispatch_clients(state).unwrap() };
                    Ok(PostAction::Continue)
                },
            )
            .expect("display source");

        Self {
            start_time: Instant::now(),
            loop_handle: event_loop.handle(),
            drm: None,
            software_cursor: false,
            redraw_wanted: false,
            redraw_ping: None,
            cursor: crate::cursor::Arrows::default(),
            cursor_theme: crate::cursor::Theme::load(),
            cursor_status: CursorImageStatus::default_named(),
            frame_cursor: None,
            rasterizer: patina_frame::Rasterizer::new(),
            no_blur: std::env::var_os("PATINA_NO_BLUR").is_some(),
            per_output_cmd: None,
            panels: Vec::new(),
            full_recompose: std::env::var_os("PATINA_FULL_RECOMPOSE").is_some(),
            verify_recompose: std::env::var_os("PATINA_VERIFY_RECOMPOSE").is_some(),
            verified_frames: 0,
            mismatched_frames: 0,
            toplevel_info: crate::toplevel_info::ToplevelInfoState::new(&dh),
            workspace_protocol: crate::workspaces::WorkspaceProtocol::new(&dh),
            active_workspace: 0,
            stashed: vec![Vec::new(); crate::workspaces::COUNT],
            drag: None,
            frame_press: false,
            minimized: Vec::new(),
            last_titlebar_click: None,
            loop_signal: event_loop.get_signal(),
            socket_name,
            space: Space::default(),
            popups: PopupManager::default(),
            compositor_state: CompositorState::new::<Self>(&dh),
            xdg_shell_state: XdgShellState::new::<Self>(&dh),
            layer_shell_state: WlrLayerShellState::new::<Self>(&dh),
            decoration_state: XdgDecorationState::new::<Self>(&dh),
            shm_state: ShmState::new::<Self>(&dh, vec![]),
            viewporter_state: ViewporterState::new::<Self>(&dh),
            cursor_shape_state: CursorShapeManagerState::new::<Self>(&dh),
            fractional_scale_state: FractionalScaleManagerState::new::<Self>(&dh),
            output_manager_state: OutputManagerState::new_with_xdg_output::<Self>(&dh),
            data_device_state: DataDeviceState::new::<Self>(&dh),
            seat_state,
            seat,
            display_handle: dh,
        }
    }

    /// Asks for a new frame. Cheap to call often: calls made before the loop wakes coalesce.
    pub fn queue_redraw(&mut self) {
        tracing::trace!(ping = self.redraw_ping.is_some(), "queue_redraw");
        self.redraw_wanted = true;
        if let Some(ping) = &self.redraw_ping {
            ping.ping();
        }
    }

    /// Draws frame callbacks for everything on `output` so clients render their next frame.
    pub fn send_frames(&self, output: &Output) {
        let elapsed = self.start_time.elapsed();
        let area = self.space.output_geometry(output);
        for window in self.space.elements() {
            // A window on another output gets its callbacks when that one draws.
            let on_output = self.frame_rect(window).is_some_and(|f| {
                area.is_none_or(|a| {
                    a.overlaps(Rectangle::<i32, Logical>::new(
                        (f.x, f.y).into(),
                        (f.w, f.h).into(),
                    ))
                })
            });
            if !on_output {
                continue;
            }
            window.send_frame(output, elapsed, Some(std::time::Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }
        for layer in layer_map_for_output(output).layers() {
            layer.send_frame(output, elapsed, Some(std::time::Duration::ZERO), |_, _| {
                Some(output.clone())
            });
        }
    }

    /// Puts the pointer in the middle of the first output.
    pub fn center_pointer(&mut self) {
        let Some(geo) = self
            .space
            .outputs()
            .next()
            .and_then(|o| self.space.output_geometry(o))
        else {
            return;
        };
        if let Some(pointer) = self.seat.get_pointer() {
            let centre = (
                geo.loc.x as f64 + geo.size.w as f64 / 2.0,
                geo.loc.y as f64 + geo.size.h as f64 / 2.0,
            );
            pointer.motion(
                self,
                None,
                &smithay::input::pointer::MotionEvent {
                    location: centre.into(),
                    serial: SERIAL_COUNTER.next_serial(),
                    time: 0,
                },
            );
            pointer.frame(self);
        }
    }

    /// Raises `window`, marks it active and gives it keyboard focus.
    pub fn focus_window(&mut self, window: &Window) {
        self.space.raise_element(window, true);
        if let Some(toplevel) = window.toplevel() {
            let surface = toplevel.wl_surface().clone();
            if let Some(keyboard) = self.seat.get_keyboard() {
                keyboard.set_focus(self, Some(surface), SERIAL_COUNTER.next_serial());
            }
        }
        for w in self.space.elements() {
            if let Some(t) = w
                .toplevel()
                .filter(|t| crate::wm::initial_configure_sent(t))
            {
                t.send_pending_configure();
            }
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _: ClientId) {}
    fn disconnected(&self, _: ClientId, _: DisconnectReason) {}
}

impl BufferHandler for Patina {
    fn buffer_destroyed(&mut self, _: &WlBuffer) {}
}

impl CompositorHandler for Patina {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        smithay::backend::renderer::utils::on_commit_buffer_handler::<Self>(surface);
        self.queue_redraw();

        // Popups wait for their initial configure, like toplevels.
        self.popups.commit(surface);
        if let Some(PopupKind::Xdg(popup)) = self.popups.find_popup(surface) {
            if !popup.is_initial_configure_sent() {
                let _ = popup.send_configure();
            }
        }

        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            let window = self
                .space
                .elements()
                .find(|w| w.toplevel().is_some_and(|t| t.wl_surface() == &root))
                .cloned();
            if let Some(window) = window {
                window.on_commit();
                if let Some(toplevel) = window.toplevel() {
                    // Clients wait for the initial configure before drawing their first frame.
                    if !crate::wm::initial_configure_sent(toplevel) {
                        toplevel.send_configure();
                    }
                    // Focus only once there is something to show: a keyboard enter before the
                    // initial configure crashes some clients.
                    let has_buffer =
                        smithay::backend::renderer::utils::with_renderer_surface_state(
                            toplevel.wl_surface(),
                            |s| s.buffer().is_some(),
                        )
                        .unwrap_or(false);
                    let first = {
                        let mut st = crate::wm::win_state(&window).borrow_mut();
                        let first = has_buffer && !st.focused_once;
                        st.focused_once |= has_buffer;
                        first
                    };
                    if first {
                        // Now that its size is known, give it its place.
                        self.place_new_window(&window);
                        self.focus_window(&window);
                    }
                }
            }
        }

        // Layer surfaces: re-arrange on commit, and send the initial configure the
        // client is waiting for before it may attach a buffer.
        let layer_output = self.outputs().into_iter().find(|o| {
            layer_map_for_output(o)
                .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .is_some()
        });
        if let Some(output) = layer_output {
            {
                let mut map = layer_map_for_output(&output);
                let initial_configure_sent = with_states(surface, |states| {
                    states
                        .data_map
                        .get::<LayerSurfaceData>()
                        .is_some_and(|d| d.lock().unwrap().initial_configure_sent)
                });
                map.arrange();
                if !initial_configure_sent {
                    if let Some(layer) = map.layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                    {
                        layer.layer_surface().send_configure();
                    }
                }
            }
            // The reserved area may have changed (a panel appeared or resized).
            self.fit_windows_to_zone();
        }
    }
}

impl ShmHandler for Patina {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl OutputHandler for Patina {}

impl SeatHandler for Patina {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
    fn focus_changed(&mut self, _: &Seat<Self>, _: Option<&WlSurface>) {}
    fn cursor_image(&mut self, _: &Seat<Self>, image: CursorImageStatus) {
        self.cursor_status = image;
        self.queue_redraw();
    }
}

// Required by cursor-shape; we do not draw separate cursors for tablet tools.
impl smithay::wayland::tablet_manager::TabletSeatHandler for Patina {}

impl SelectionHandler for Patina {
    type SelectionUserData = ();
}

impl DataDeviceHandler for Patina {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}
impl ClientDndGrabHandler for Patina {}
impl ServerDndGrabHandler for Patina {
    fn send(&mut self, _: String, _: OwnedFd, _: Seat<Self>) {}
}

impl XdgShellHandler for Patina {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        // Cascade new windows below the panel (and their titlebar) until real placement lands.
        let n = self.space.elements().count() as i32;
        let zone = self.usable_zone();
        let origin = (
            zone.loc.x + 96 + 32 * n,
            zone.loc.y + patina_frame::TITLEBAR_H + 28 + 32 * n,
        );
        let window = Window::new_wayland_window(surface);
        self.space.map_element(window, origin, false);
        self.queue_redraw();
    }

    // Titles and app ids change without a surface commit, but they show in the titlebar.
    fn title_changed(&mut self, _: ToplevelSurface) {
        self.queue_redraw();
    }
    fn app_id_changed(&mut self, _: ToplevelSurface) {
        self.queue_redraw();
    }

    fn new_popup(&mut self, surface: PopupSurface, _: PositionerState) {
        self.unconstrain_popup(&surface);
        if let Err(e) = self.popups.track_popup(PopupKind::Xdg(surface)) {
            tracing::warn!(error = ?e, "cannot track popup");
        }
    }

    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
            state.positioner = positioner;
        });
        self.unconstrain_popup(&surface);
        surface.send_repositioned(token);
    }

    /// A menu that takes the keyboard and pointer until it is dismissed (a click outside it
    /// closes it).
    fn grab(&mut self, surface: PopupSurface, seat: wl_seat::WlSeat, serial: Serial) {
        let Some(seat) = Seat::<Patina>::from_resource(&seat) else {
            return;
        };
        let kind = PopupKind::Xdg(surface);
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        let Ok(mut grab) = self.popups.grab_popup(root, kind, &seat, serial) else {
            return;
        };
        if let Some(keyboard) = seat.get_keyboard() {
            if keyboard.is_grabbed()
                && !(keyboard.has_grab(serial)
                    || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
            {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }
            keyboard.set_focus(self, grab.current_grab(), serial);
            keyboard.set_grab(self, PopupKeyboardGrab::new(&grab), serial);
        }
        if let Some(pointer) = seat.get_pointer() {
            if pointer.is_grabbed()
                && !(pointer.has_grab(serial)
                    || pointer.has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
            {
                grab.ungrab(PopupUngrabStrategy::All);
                return;
            }
            pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
        }
    }

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
        self.forget_window(surface.wl_surface());
        // Focus falls back to the top-most remaining window. The destroyed one may still be
        // mapped at this point, so skip it.
        let top = self
            .space
            .elements()
            .rev()
            .find(|w| w.toplevel() != Some(&surface))
            .cloned();
        if let Some(window) = top {
            self.focus_window(&window);
        }
        self.refresh_hover();
        self.queue_redraw();
    }
}

/// The compositor draws every titlebar, so clients are always told to use server-side decorations.
impl XdgDecorationHandler for Patina {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        toplevel.with_pending_state(|s| s.decoration_mode = Some(DecorationMode::ServerSide));
        if crate::wm::initial_configure_sent(&toplevel) {
            toplevel.send_pending_configure();
        }
    }

    fn request_mode(&mut self, toplevel: ToplevelSurface, _: DecorationMode) {
        self.new_decoration(toplevel);
    }

    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        self.new_decoration(toplevel);
    }
}

impl WlrLayerShellHandler for Patina {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: smithay::wayland::shell::wlr_layer::LayerSurface,
        output: Option<WlOutput>,
        _layer: Layer,
        namespace: String,
    ) {
        let output = output
            .as_ref()
            .and_then(Output::from_resource)
            .or_else(|| self.pointer_output())
            .or_else(|| self.space.outputs().next().cloned());
        let Some(output) = output else { return };
        {
            let mut map = layer_map_for_output(&output);
            let _ = map.map_layer(&DesktopLayerSurface::new(surface, namespace));
        }
        self.queue_redraw();
    }

    fn new_popup(
        &mut self,
        _parent: smithay::wayland::shell::wlr_layer::LayerSurface,
        popup: PopupSurface,
    ) {
        // Tracked by the xdg-shell handler already; now that it has a parent, keep it on screen.
        self.unconstrain_popup(&popup);
    }

    fn layer_destroyed(&mut self, surface: smithay::wayland::shell::wlr_layer::LayerSurface) {
        for output in self.outputs() {
            let mut map = layer_map_for_output(&output);
            let layer = map
                .layers()
                .find(|l| l.layer_surface() == &surface)
                .cloned();
            if let Some(layer) = layer {
                map.unmap_layer(&layer);
            }
        }
        // The map lock is released above; fitting windows takes it again.
        self.fit_windows_to_zone();
        self.queue_redraw();
    }
}

impl FractionalScaleHandler for Patina {
    fn new_fractional_scale(&mut self, surface: WlSurface) {
        // Until the surface is on an output, assume the first one.
        let scale = self
            .space
            .outputs()
            .next()
            .map(|o| o.current_scale().fractional_scale())
            .unwrap_or(1.0);
        with_states(&surface, |states| {
            with_fractional_scale(states, |fs| fs.set_preferred_scale(scale))
        });
        self.queue_redraw();
    }
}

impl Patina {
    /// Moves or flips a popup so it stays on the output of the window or panel it belongs to,
    /// as its positioner allows.
    fn unconstrain_popup(&self, popup: &PopupSurface) {
        let kind = PopupKind::Xdg(popup.clone());
        let Ok(root) = find_popup_root_surface(&kind) else {
            return;
        };
        // Positions are relative to the parent; parents of nested menus are offset again.
        let nested = get_popup_toplevel_coords(&kind);
        let window = self
            .space
            .elements()
            .find(|w| w.toplevel().is_some_and(|t| t.wl_surface() == &root))
            .cloned();
        let target = if let Some(window) = window {
            let (Some(loc), Some(output)) = (
                self.space.element_location(&window),
                self.output_for_window(&window),
            ) else {
                return;
            };
            let Some(mut area) = self.space.output_geometry(&output) else {
                return;
            };
            area.loc -= loc + nested;
            area
        } else {
            let found = self.outputs().into_iter().find_map(|output| {
                let map = layer_map_for_output(&output);
                let layer = map.layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)?;
                let geo = map.layer_geometry(layer)?;
                let size = self.space.output_geometry(&output)?.size;
                Some(Rectangle::<i32, Logical>::new(
                    (-geo.loc.x - nested.x, -geo.loc.y - nested.y).into(),
                    size,
                ))
            });
            let Some(area) = found else { return };
            area
        };
        popup.with_pending_state(|state| {
            state.geometry = state.positioner.get_unconstrained_geometry(target);
        });
    }
}

delegate_fractional_scale!(Patina);
delegate_compositor!(Patina);
delegate_shm!(Patina);
delegate_output!(Patina);
delegate_seat!(Patina);
delegate_data_device!(Patina);
delegate_xdg_shell!(Patina);
delegate_xdg_decoration!(Patina);
delegate_layer_shell!(Patina);
delegate_viewporter!(Patina);
delegate_cursor_shape!(Patina);

// Keeps the import used when xdg_toplevel states are set by later milestones.
#[allow(dead_code)]
type ToplevelState = xdg_toplevel::State;
