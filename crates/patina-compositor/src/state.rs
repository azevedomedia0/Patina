//! Compositor state and the Wayland protocol handlers.

use std::{ffi::OsString, os::unix::io::OwnedFd, sync::Arc, time::Instant};

use smithay::{
    delegate_compositor, delegate_data_device, delegate_layer_shell, delegate_output,
    delegate_seat, delegate_shm, delegate_viewporter, delegate_xdg_decoration, delegate_xdg_shell,
    desktop::{
        layer_map_for_output, LayerSurface as DesktopLayerSurface, Space, Window, WindowSurfaceType,
    },
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
    utils::{Serial, SERIAL_COUNTER},
    wayland::{
        buffer::BufferHandler,
        compositor::{
            get_parent, is_sync_subsurface, with_states, CompositorClientState, CompositorHandler,
            CompositorState,
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
    pub rasterizer: patina_frame::Rasterizer,
    pub drag: Option<crate::wm::Drag>,
    /// A press was consumed by a window frame; its release must be swallowed too.
    pub frame_press: bool,
    pub minimized: Vec<(Window, smithay::utils::Point<i32, smithay::utils::Logical>)>,
    pub last_titlebar_click: Option<(Instant, Window)>,
    pub display_handle: DisplayHandle,
    pub loop_signal: LoopSignal,
    pub socket_name: OsString,

    pub space: Space<Window>,
    pub output: Option<Output>,

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
    /// Held only to keep the xdg-output global alive.
    #[allow(dead_code)]
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,
}

impl Patina {
    pub fn new(event_loop: &mut EventLoop<Self>, display: Display<Self>) -> Self {
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
            rasterizer: patina_frame::Rasterizer::new(),
            drag: None,
            frame_press: false,
            minimized: Vec::new(),
            last_titlebar_click: None,
            loop_signal: event_loop.get_signal(),
            socket_name,
            space: Space::default(),
            output: None,
            compositor_state: CompositorState::new::<Self>(&dh),
            xdg_shell_state: XdgShellState::new::<Self>(&dh),
            layer_shell_state: WlrLayerShellState::new::<Self>(&dh),
            decoration_state: XdgDecorationState::new::<Self>(&dh),
            shm_state: ShmState::new::<Self>(&dh, vec![]),
            viewporter_state: ViewporterState::new::<Self>(&dh),
            output_manager_state: OutputManagerState::new_with_xdg_output::<Self>(&dh),
            data_device_state: DataDeviceState::new::<Self>(&dh),
            seat_state,
            seat,
            display_handle: dh,
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
                        self.focus_window(&window);
                    }
                }
            }
        }

        // Layer surfaces: re-arrange on commit, and send the initial configure the
        // client is waiting for before it may attach a buffer.
        if let Some(output) = &self.output {
            let mut map = layer_map_for_output(output);
            if map
                .layer_for_surface(surface, WindowSurfaceType::TOPLEVEL)
                .is_some()
            {
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
    fn cursor_image(&mut self, _: &Seat<Self>, _: CursorImageStatus) {}
}

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
    }

    fn new_popup(&mut self, _: PopupSurface, _: PositionerState) {}
    fn grab(&mut self, _: PopupSurface, _: wl_seat::WlSeat, _: Serial) {}
    fn reposition_request(&mut self, _: PopupSurface, _: PositionerState, _: u32) {}

    fn toplevel_destroyed(&mut self, surface: ToplevelSurface) {
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
            .or_else(|| self.output.clone());
        let Some(output) = output else { return };
        let mut map = layer_map_for_output(&output);
        let _ = map.map_layer(&DesktopLayerSurface::new(surface, namespace));
    }
}

delegate_compositor!(Patina);
delegate_shm!(Patina);
delegate_output!(Patina);
delegate_seat!(Patina);
delegate_data_device!(Patina);
delegate_xdg_shell!(Patina);
delegate_xdg_decoration!(Patina);
delegate_layer_shell!(Patina);
delegate_viewporter!(Patina);

// Keeps the import used when xdg_toplevel states are set by later milestones.
#[allow(dead_code)]
type ToplevelState = xdg_toplevel::State;
