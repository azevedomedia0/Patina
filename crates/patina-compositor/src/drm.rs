//! DRM/KMS backend: runs the compositor on real (or virtual) displays, started from a TTY.
//!
//! Every GPU with display connectors is opened, and every connected connector becomes an
//! output, placed to the right of the others at its preferred mode. Each GPU draws its own
//! outputs with its own renderer. udev reports GPUs coming and going and connectors being
//! plugged or unplugged; outputs follow.

use std::{
    error::Error,
    path::{Path, PathBuf},
    time::Duration,
};

use smithay::{
    backend::{
        allocator::{
            gbm::{GbmAllocator, GbmBufferFlags, GbmDevice},
            Format, Fourcc,
        },
        drm::{
            compositor::{DrmCompositor, FrameFlags, PrimaryPlaneElement},
            exporter::gbm::GbmFramebufferExporter,
            DrmDevice, DrmDeviceFd, DrmEvent,
        },
        egl::{EGLContext, EGLDisplay},
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::gles::{GlesRenderer, GlesTexProgram},
        session::{libseat::LibSeatSession, Event as SessionEvent, Session},
        udev::{all_gpus, primary_gpu, UdevBackend, UdevEvent},
    },
    desktop::layer_map_for_output,
    output::{Mode as OutputMode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            ping::make_ping,
            timer::{TimeoutAction, Timer},
            EventLoop, RegistrationToken,
        },
        drm::control::{connector, crtc, Device as ControlDevice, ModeTypeFlags},
        input::Libinput,
        rustix::fs::OFlags,
        wayland_server::backend::GlobalId,
    },
    utils::{DeviceFd, Transform},
};

use crate::{render::OutputRender, state::Patina};

pub type Compositor =
    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, (), DrmDeviceFd>;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct DrmBackend {
    pub session: LibSeatSession,
    pub gpus: Vec<Gpu>,
}

/// How frames reach a display.
enum Presenter {
    /// Rendered straight into buffers the display scans out, with hardware planes.
    Direct(Box<Compositor>),
    /// Rendered offscreen and copied, for display-only devices.
    Copy(Box<crate::dumb::DumbPresenter>),
}

/// One connected display.
pub struct OutputSlot {
    pub output: Output,
    global: GlobalId,
    connector: connector::Handle,
    crtc: crtc::Handle,
    presenter: Presenter,
    /// A frame is queued and we are waiting for its vblank.
    frame_pending: bool,
    /// Something changed that this output has not drawn yet.
    wanted: bool,
    render: OutputRender,
}

/// One GPU with display connectors. Field order is drop order: outputs before the renderer,
/// the renderer before the device it renders on.
pub struct Gpu {
    pub id: DevId,
    pub path: PathBuf,
    pub outputs: Vec<OutputSlot>,
    /// Created when the first display on this GPU is connected, so a GPU without displays
    /// (a laptop's discrete one, say) is not woken up for nothing.
    renderer: Option<GlesRenderer>,
    /// Shader that rounds window content corners; `None` if it failed to compile.
    clip_program: Option<GlesTexProgram>,
    gbm: GbmDevice<DrmDeviceFd>,
    device: DrmDevice,
    vblank_source: RegistrationToken,
}

/// A device number (`dev_t`), as udev reports it.
type DevId = u64;

pub fn init(event_loop: &mut EventLoop<Patina>, state: &mut Patina) -> Result<()> {
    let handle = event_loop.handle();

    // Session and input.
    let (session, notifier) = LibSeatSession::new().map_err(|e| format!("libseat: {e:?}"))?;
    let seat = session.seat();
    let mut libinput =
        Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(session.clone().into());
    libinput
        .udev_assign_seat(&seat)
        .map_err(|_| "libinput: cannot assign seat")?;
    handle
        .insert_source(
            LibinputInputBackend::new(libinput.clone()),
            |event, _, state| state.process_input_event(event),
        )
        .map_err(|e| format!("libinput source: {e}"))?;
    let mut libinput_ctx = libinput;
    handle
        .insert_source(notifier, move |event, _, state| match event {
            SessionEvent::PauseSession => {
                libinput_ctx.suspend();
                if let Some(d) = &mut state.drm {
                    for gpu in &mut d.gpus {
                        gpu.device.pause();
                    }
                }
            }
            SessionEvent::ActivateSession => {
                if libinput_ctx.resume().is_err() {
                    tracing::warn!("libinput resume failed");
                }
                if let Some(d) = &mut state.drm {
                    for gpu in &mut d.gpus {
                        if let Err(e) = gpu.device.activate(true) {
                            tracing::warn!(error = %e, "drm activate failed");
                        }
                        for slot in &mut gpu.outputs {
                            if let Presenter::Direct(c) = &mut slot.presenter {
                                c.reset_buffers();
                            }
                            slot.frame_pending = false;
                        }
                    }
                }
                // Displays may have been plugged or unplugged while we were away.
                let ids: Vec<_> = state
                    .drm
                    .as_ref()
                    .map(|d| d.gpus.iter().map(|g| g.id).collect())
                    .unwrap_or_default();
                for id in ids {
                    state.with_drm(|state, drm| scan_connectors(state, drm, id));
                }
                state.queue_redraw();
            }
        })
        .map_err(|e| format!("session source: {e}"))?;

    // Rendering is woken by this ping and by vblank, never by a timer.
    let (ping, ping_source) = make_ping().map_err(|e| format!("ping: {e}"))?;
    handle
        .insert_source(ping_source, |_, _, state| {
            tracing::trace!("ping woke the loop");
            state.render_drm()
        })
        .map_err(|e| format!("ping source: {e}"))?;
    state.redraw_ping = Some(ping);
    state.software_cursor = true;
    state.drm = Some(DrmBackend {
        session,
        gpus: Vec::new(),
    });

    // GPUs: the boot GPU first, so its displays come first (left-most).
    let udev = UdevBackend::new(&seat).map_err(|e| format!("udev: {e}"))?;
    let primary = primary_gpu(&seat).ok().flatten();
    let mut paths: Vec<PathBuf> = all_gpus(&seat).unwrap_or_default();
    paths.sort_by_key(|p| Some(p) != primary.as_ref());
    for path in paths {
        let Some(id) = udev
            .device_list()
            .find(|(_, p)| *p == path.as_path())
            .map(|(id, _)| id)
        else {
            continue;
        };
        state.with_drm(|state, drm| {
            if let Err(e) = add_gpu(state, drm, id, &path) {
                tracing::warn!(?path, error = %e, "cannot use GPU");
            }
        });
    }
    handle
        .insert_source(udev, |event, _, state| {
            tracing::debug!(?event, "udev");
            match event {
                UdevEvent::Added { device_id, path } => state.with_drm(|state, drm| {
                    if let Err(e) = add_gpu(state, drm, device_id, &path) {
                        tracing::warn!(?path, error = %e, "cannot use new GPU");
                    }
                }),
                UdevEvent::Changed { device_id } => {
                    state.with_drm(|state, drm| scan_connectors(state, drm, device_id))
                }
                UdevEvent::Removed { device_id } => {
                    state.with_drm(|state, drm| remove_gpu(state, drm, device_id))
                }
            }
        })
        .map_err(|e| format!("udev source: {e}"))?;

    if state.space.outputs().next().is_none() {
        tracing::warn!("no connected display yet; waiting for one");
    }
    state.queue_redraw();
    Ok(())
}

fn add_gpu(state: &mut Patina, drm: &mut DrmBackend, id: DevId, path: &Path) -> Result<()> {
    if drm.gpus.iter().any(|g| g.id == id) {
        return Ok(());
    }
    let fd = drm
        .session
        .open(
            path,
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
        )
        .map_err(|e| format!("cannot open {path:?}: {e:?}"))?;
    let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));
    let (device, notifier) = DrmDevice::new(drm_fd.clone(), true)?;
    if device.resource_handles()?.connectors().is_empty() {
        tracing::info!(?path, "GPU has no display connectors; not used");
        return Ok(());
    }
    let gbm = GbmDevice::new(drm_fd)?;
    let vblank_source = state
        .loop_handle
        .insert_source(notifier, move |event, _, state| match event {
            DrmEvent::VBlank(crtc) => state.vblank(id, crtc),
            DrmEvent::Error(e) => tracing::error!(error = ?e, "drm error"),
        })
        .map_err(|e| format!("drm source: {e}"))?;
    tracing::info!(?path, "using GPU");
    drm.gpus.push(Gpu {
        id,
        path: path.to_path_buf(),
        outputs: Vec::new(),
        renderer: None,
        clip_program: None,
        gbm,
        device,
        vblank_source,
    });
    scan_connectors(state, drm, id);
    Ok(())
}

fn remove_gpu(state: &mut Patina, drm: &mut DrmBackend, id: DevId) {
    let Some(i) = drm.gpus.iter().position(|g| g.id == id) else {
        return;
    };
    let mut gpu = drm.gpus.remove(i);
    tracing::info!(path = ?gpu.path, "GPU removed");
    for slot in gpu.outputs.drain(..) {
        drop_output(state, slot);
    }
    state.loop_handle.remove(gpu.vblank_source);
}

/// A connector's name as users know it, such as `HDMI-A-1`, kept unique across GPUs.
fn output_name(state: &Patina, info: &connector::Info) -> String {
    let base = format!("{}-{}", info.interface().as_str(), info.interface_id());
    let taken = |n: &str| state.space.outputs().any(|o| o.name() == n);
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|k| format!("{base}~{k}"))
        .find(|n| !taken(n))
        .unwrap_or(base)
}

/// Brings the outputs of GPU `id` in line with what is plugged in.
fn scan_connectors(state: &mut Patina, drm: &mut DrmBackend, id: DevId) {
    let Some(gpu) = drm.gpus.iter_mut().find(|g| g.id == id) else {
        return;
    };
    let Ok(res) = gpu.device.resource_handles() else {
        return;
    };
    let mut connected = Vec::new();
    for conn in res.connectors() {
        match gpu.device.get_connector(*conn, true) {
            Ok(info) if info.state() == connector::State::Connected && !info.modes().is_empty() => {
                connected.push(info)
            }
            _ => {}
        }
    }

    // Unplugged.
    let mut i = 0;
    while i < gpu.outputs.len() {
        if connected
            .iter()
            .any(|c| c.handle() == gpu.outputs[i].connector)
        {
            i += 1;
        } else {
            let slot = gpu.outputs.remove(i);
            tracing::info!(output = %slot.output.name(), "display unplugged");
            drop_output(state, slot);
        }
    }

    // Plugged in.
    for info in connected {
        if gpu.outputs.iter().any(|s| s.connector == info.handle()) {
            continue;
        }
        if let Err(e) = add_output(state, gpu, &res, &info) {
            tracing::warn!(error = %e, connector = ?info.handle(), "cannot use display");
        }
    }
    state.queue_redraw();
}

fn add_output(
    state: &mut Patina,
    gpu: &mut Gpu,
    res: &smithay::reexports::drm::control::ResourceHandles,
    info: &connector::Info,
) -> Result<()> {
    let mode = info
        .modes()
        .iter()
        .find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
        .or_else(|| info.modes().first())
        .copied()
        .ok_or("connector without modes")?;
    let used: Vec<crtc::Handle> = gpu.outputs.iter().map(|s| s.crtc).collect();
    let crtc = info
        .encoders()
        .iter()
        .filter_map(|e| gpu.device.get_encoder(*e).ok())
        .flat_map(|enc| res.filter_crtcs(enc.possible_crtcs()))
        .find(|c| !used.contains(c))
        .ok_or("no free CRTC for this display")?;

    if gpu.renderer.is_none() {
        let egl = unsafe { EGLDisplay::new(gpu.gbm.clone())? };
        let context = EGLContext::new(&egl)?;
        let mut renderer = unsafe { GlesRenderer::new(context)? };
        gpu.clip_program = crate::clip::compile_logged(&mut renderer);
        gpu.renderer = Some(renderer);
    }
    let renderer = gpu.renderer.as_mut().ok_or("no renderer")?;

    let name = output_name(state, info);
    let output = Output::new(
        name.clone(),
        PhysicalProperties {
            size: info
                .size()
                .map(|(w, h)| (w as i32, h as i32))
                .unwrap_or((0, 0))
                .into(),
            subpixel: Subpixel::Unknown,
            make: "Unknown".into(),
            model: "Unknown".into(),
        },
    );
    let out_mode = OutputMode::from(mode);
    let x = state.next_output_x();
    let (mw, mh) = mode.size();
    let scale =
        crate::outputs::scale_for(&name, (mw as i32, mh as i32), info.size().unwrap_or((0, 0)));
    output.change_current_state(
        Some(out_mode),
        Some(Transform::Normal),
        Some(crate::outputs::output_scale(scale)),
        Some((x, 0).into()),
    );
    output.set_preferred(out_mode);

    let surface = gpu.device.create_surface(crtc, mode, &[info.handle()])?;
    let allocator = GbmAllocator::new(
        gpu.gbm.clone(),
        GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
    );
    let exporter = GbmFramebufferExporter::new(gpu.gbm.clone(), None);
    let render_formats: Vec<Format> = renderer
        .egl_context()
        .dmabuf_render_formats()
        .iter()
        .copied()
        .collect();
    // A GPU whose OpenGL cannot render into buffers the display can show is display-only:
    // render offscreen and copy.
    let presenter = if render_formats.is_empty() {
        tracing::info!(output = %name, "display-only device: frames are copied through memory");
        Presenter::Copy(Box::new(crate::dumb::DumbPresenter::new(
            surface,
            gpu.device.device_fd().clone(),
            renderer,
            &output,
        )?))
    } else {
        Presenter::Direct(Box::new(
            DrmCompositor::new(
                &output,
                surface,
                None,
                allocator,
                exporter,
                [Fourcc::Xrgb8888, Fourcc::Argb8888],
                render_formats,
                gpu.device.cursor_size(),
                Some(gpu.gbm.clone()),
            )
            .map_err(|e| format!("drm compositor: {e:?}"))?,
        ))
    };

    tracing::info!(output = %name, ?mode, x, scale, mm = ?info.size(), gpu = ?gpu.path, "display connected");
    let first = state.space.outputs().next().is_none();
    let global = output.create_global::<Patina>(&state.display_handle);
    state.space.map_output(&output, (x, 0));
    layer_map_for_output(&output).arrange();
    gpu.outputs.push(OutputSlot {
        output: output.clone(),
        global,
        connector: info.handle(),
        crtc,
        presenter,
        frame_pending: false,
        wanted: true,
        render: OutputRender::new(state.no_blur),
    });
    if first {
        state.center_pointer();
    }
    state.spawn_panel(&output);
    state.fit_windows_to_zone();
    Ok(())
}

/// Takes an output away: its global goes (after a moment, so clients that are binding it do
/// not error), and windows on it move to the remaining outputs.
fn drop_output(state: &mut Patina, slot: OutputSlot) {
    let dh = state.display_handle.clone();
    dh.disable_global::<Patina>(slot.global.clone());
    let global = slot.global.clone();
    let _ = state.loop_handle.insert_source(
        Timer::from_duration(Duration::from_secs(5)),
        move |_, _, state| {
            state.display_handle.remove_global::<Patina>(global.clone());
            TimeoutAction::Drop
        },
    );
    state.output_removed(&slot.output);
    drop(slot);
}

impl Patina {
    /// Runs `f` with the DRM backend taken out of the state, so both can be borrowed.
    fn with_drm(&mut self, f: impl FnOnce(&mut Patina, &mut DrmBackend)) {
        if let Some(mut drm) = self.drm.take() {
            f(self, &mut drm);
            self.drm = Some(drm);
        }
    }

    /// Tries again shortly after a failure. Not used for empty frames: those mean there is
    /// nothing to show, and the loop sleeps until something changes.
    fn retry_render_later(&mut self) {
        let _ = self.loop_handle.insert_source(
            Timer::from_duration(Duration::from_millis(100)),
            |_, _, state| {
                state.queue_redraw();
                TimeoutAction::Drop
            },
        );
    }

    fn vblank(&mut self, gpu_id: DevId, crtc: crtc::Handle) {
        self.with_drm(|state, drm| {
            let Some(gpu) = drm.gpus.iter_mut().find(|g| g.id == gpu_id) else {
                return;
            };
            let Some(i) = gpu.outputs.iter().position(|s| s.crtc == crtc) else {
                return;
            };
            let slot = &mut gpu.outputs[i];
            tracing::trace!(output = %slot.output.name(), wanted = slot.wanted, "vblank");
            if let Presenter::Direct(c) = &mut slot.presenter {
                if let Err(e) = c.frame_submitted() {
                    tracing::warn!(error = ?e, "frame_submitted");
                }
            }
            slot.frame_pending = false;
            // Changes that arrived while this frame was on its way get drawn now.
            if slot.wanted && gpu.render_output(state, i) {
                state.retry_render_later();
            }
        });
    }

    /// Draws every output that has something new to show and no frame in flight.
    pub fn render_drm(&mut self) {
        let fresh = std::mem::take(&mut self.redraw_wanted);
        self.space.refresh();
        self.popups.cleanup();
        self.refresh_toplevel_info();
        self.update_surface_scales();
        let mut failed = false;
        self.with_drm(|state, drm| {
            for gpu in &mut drm.gpus {
                for i in 0..gpu.outputs.len() {
                    gpu.outputs[i].wanted |= fresh;
                    if gpu.outputs[i].wanted {
                        failed |= gpu.render_output(state, i);
                    }
                }
            }
        });
        if failed {
            self.retry_render_later();
        }
    }
}

impl Gpu {
    /// Draws output `i` unless its last frame is still on the way. Returns true on failure.
    fn render_output(&mut self, state: &mut Patina, i: usize) -> bool {
        let Gpu {
            outputs,
            renderer,
            clip_program,
            ..
        } = self;
        let Some(renderer) = renderer.as_mut() else {
            return false;
        };
        let slot = &mut outputs[i];
        if slot.frame_pending {
            // The vblank handler draws again while `wanted` is still set.
            tracing::trace!(output = %slot.output.name(), "render deferred: frame in flight");
            return false;
        }
        slot.wanted = false;
        let output = slot.output.clone();

        let mut elements = crate::render::frame_elements(
            state,
            renderer,
            clip_program.as_ref(),
            &mut slot.render,
            &output,
        );
        // Cursor elements go in front: the list is top-most first.
        let cursor = crate::render::cursor_elements(state, renderer, &output);
        elements.splice(0..0, cursor);
        let bg = patina_tokens::BG_DESKTOP;
        let clear = [
            bg.r as f32 / 255.0,
            bg.g as f32 / 255.0,
            bg.b as f32 / 255.0,
            1.0,
        ];

        // PATINA_NO_PLANES=1 composites everything into the primary plane (the hardware
        // cursor is then part of the frame, which screen capture needs).
        let flags = if std::env::var_os("PATINA_NO_PLANES").is_some() {
            FrameFlags::empty()
        } else {
            FrameFlags::DEFAULT
        };
        let mut failed = false;
        let compositor = match &mut slot.presenter {
            Presenter::Direct(c) => c,
            Presenter::Copy(copy) => {
                match copy.render(renderer, &elements, clear) {
                    Ok(true) => slot.frame_pending = true,
                    Ok(false) => tracing::trace!(output = %output.name(), "frame empty"),
                    Err(e) => {
                        tracing::warn!(error = %e, "copying frame to display-only device");
                        failed = true;
                    }
                }
                drop(elements);
                state.send_frames(&output);
                return failed;
            }
        };
        match compositor.render_frame(renderer, &elements, clear, flags) {
            Ok(result) if !result.is_empty => {
                // Without fence support (virtio-gpu, software GL) the GPU may still be drawing
                // when the frame is queued, and the screen would show a half-drawn frame.
                // Smithay leaves waiting for the render to finish to us in that case.
                if result.needs_sync() {
                    if let PrimaryPlaneElement::Swapchain(element) = result.primary_element {
                        let _ = element.sync.wait();
                    }
                }
                match compositor.queue_frame(()) {
                    Ok(()) => {
                        tracing::trace!(output = %output.name(), elements = elements.len(), "frame queued");
                        slot.frame_pending = true;
                    }
                    Err(e) => {
                        tracing::warn!(error = ?e, "queue_frame");
                        failed = true;
                    }
                }
            }
            // Nothing changed on screen: stay idle until something does.
            Ok(_) => tracing::trace!(output = %output.name(), "frame empty"),
            Err(e) => {
                tracing::warn!(error = ?e, "render_frame");
                failed = true;
            }
        }
        drop(elements);
        // Clients waiting on a frame callback may draw again; if they commit, that wakes us.
        state.send_frames(&output);
        failed
    }
}
