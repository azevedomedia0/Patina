//! DRM/KMS backend: runs the compositor on a real (or virtual) display, started from a TTY.
//!
//! Single GPU and a single output for now: the first connected connector at its preferred mode.

use std::{error::Error, time::Duration};

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
        renderer::gles::GlesRenderer,
        session::{libseat::LibSeatSession, Event as SessionEvent, Session},
        udev::{all_gpus, primary_gpu},
    },
    output::{Mode as OutputMode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::{
            ping::make_ping,
            timer::{TimeoutAction, Timer},
            EventLoop,
        },
        drm::control::{connector, crtc, Device as ControlDevice, ModeTypeFlags},
        input::Libinput,
        rustix::fs::OFlags,
    },
    utils::{DeviceFd, Transform},
};

use crate::state::Patina;

pub type Compositor =
    DrmCompositor<GbmAllocator<DrmDeviceFd>, GbmFramebufferExporter<DrmDeviceFd>, (), DrmDeviceFd>;

pub struct DrmBackend {
    pub session: LibSeatSession,
    pub device: DrmDevice,
    pub renderer: GlesRenderer,
    pub compositor: Compositor,
    /// A frame is queued and we are waiting for its vblank.
    pub frame_pending: bool,
}

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub fn init(event_loop: &mut EventLoop<Patina>, state: &mut Patina) -> Result<()> {
    let handle = event_loop.handle();

    // Session and input.
    let (mut session, notifier) = LibSeatSession::new().map_err(|e| format!("libseat: {e:?}"))?;
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
                    d.device.pause();
                }
            }
            SessionEvent::ActivateSession => {
                if libinput_ctx.resume().is_err() {
                    tracing::warn!("libinput resume failed");
                }
                if let Some(d) = &mut state.drm {
                    if let Err(e) = d.device.activate(true) {
                        tracing::warn!(error = %e, "drm activate failed");
                    }
                    d.compositor.reset_buffers();
                    d.frame_pending = false;
                }
                state.queue_redraw();
            }
        })
        .map_err(|e| format!("session source: {e}"))?;

    // GPU: the boot GPU if udev knows it, else the first one.
    let path = primary_gpu(&seat)?
        .or_else(|| all_gpus(&seat).ok().and_then(|g| g.into_iter().next()))
        .ok_or("no GPU found")?;
    tracing::info!(?path, "using GPU");
    let fd = session
        .open(
            &path,
            OFlags::RDWR | OFlags::CLOEXEC | OFlags::NOCTTY | OFlags::NONBLOCK,
        )
        .map_err(|e| format!("cannot open {path:?}: {e:?}"))?;
    let drm_fd = DrmDeviceFd::new(DeviceFd::from(fd));
    let (mut device, notifier) = DrmDevice::new(drm_fd.clone(), true)?;
    let gbm = GbmDevice::new(drm_fd.clone())?;
    let egl = unsafe { EGLDisplay::new(gbm.clone())? };
    let context = EGLContext::new(&egl)?;
    let mut renderer = unsafe { GlesRenderer::new(context)? };
    state.clip_program = crate::clip::compile_logged(&mut renderer);

    // Output: first connected connector, preferred mode.
    let res = device.resource_handles()?;
    let mut chosen = None;
    for conn in res.connectors() {
        let info = device.get_connector(*conn, true)?;
        if info.state() != connector::State::Connected || info.modes().is_empty() {
            continue;
        }
        let mode = info
            .modes()
            .iter()
            .find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED))
            .or_else(|| info.modes().first())
            .copied()
            .ok_or("connector without modes")?;
        let crtc = info
            .encoders()
            .iter()
            .filter_map(|e| device.get_encoder(*e).ok())
            .flat_map(|enc| res.filter_crtcs(enc.possible_crtcs()))
            .next()
            .ok_or("no CRTC for connector")?;
        chosen = Some((info, mode, crtc));
        break;
    }
    let (info, mode, crtc): (connector::Info, _, crtc::Handle) =
        chosen.ok_or("no connected display")?;
    let name = format!("{:?}-{}", info.interface(), info.interface_id());
    tracing::info!(%name, ?mode, "output");

    let output = Output::new(
        name,
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
    let _global = output.create_global::<Patina>(&state.display_handle);
    let out_mode = OutputMode::from(mode);
    output.change_current_state(
        Some(out_mode),
        Some(Transform::Normal),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(out_mode);
    state.space.map_output(&output, (0, 0));
    state.output = Some(output.clone());
    state.software_cursor = true;
    state.center_pointer();

    let surface = device.create_surface(crtc, mode, &[info.handle()])?;
    let allocator = GbmAllocator::new(
        gbm.clone(),
        GbmBufferFlags::RENDERING | GbmBufferFlags::SCANOUT,
    );
    let exporter = GbmFramebufferExporter::new(gbm.clone(), None);
    let render_formats: Vec<Format> = renderer
        .egl_context()
        .dmabuf_render_formats()
        .iter()
        .copied()
        .collect();
    let compositor = DrmCompositor::new(
        &output,
        surface,
        None,
        allocator,
        exporter,
        [Fourcc::Xrgb8888, Fourcc::Argb8888],
        render_formats,
        device.cursor_size(),
        Some(gbm),
    )
    .map_err(|e| format!("drm compositor: {e:?}"))?;

    // Rendering is woken by this ping and by vblank, never by a timer.
    let (ping, ping_source) = make_ping().map_err(|e| format!("ping: {e}"))?;
    handle
        .insert_source(ping_source, |_, _, state| {
            tracing::trace!("ping woke the loop");
            state.render_drm()
        })
        .map_err(|e| format!("ping source: {e}"))?;
    state.redraw_ping = Some(ping);

    handle
        .insert_source(notifier, |event, _, state| match event {
            DrmEvent::VBlank(_) => {
                tracing::trace!(wanted = state.redraw_wanted, "vblank");
                if let Some(d) = &mut state.drm {
                    if let Err(e) = d.compositor.frame_submitted() {
                        tracing::warn!(error = ?e, "frame_submitted");
                    }
                    d.frame_pending = false;
                }
                // Changes that arrived while this frame was on its way get drawn now.
                if state.redraw_wanted {
                    state.render_drm();
                }
            }
            DrmEvent::Error(e) => tracing::error!(error = ?e, "drm error"),
        })
        .map_err(|e| format!("drm source: {e}"))?;

    state.drm = Some(DrmBackend {
        session,
        device,
        renderer,
        compositor,
        frame_pending: false,
    });
    state.queue_redraw();
    Ok(())
}

impl Patina {
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

    pub fn render_drm(&mut self) {
        let Some(mut drm) = self.drm.take() else {
            return;
        };
        let Some(output) = self.output.clone() else {
            self.drm = Some(drm);
            return;
        };
        if drm.frame_pending {
            // The vblank handler draws again if redraw_wanted is still set.
            tracing::trace!("render deferred: frame in flight");
            self.drm = Some(drm);
            return;
        }
        self.redraw_wanted = false;

        self.space.refresh();
        self.refresh_toplevel_info();
        let mut elements = crate::render::elements(self, &mut drm.renderer, &output);
        // Cursor elements go in front: the list is top-most first.
        let cursor = crate::render::cursor_elements(self, &mut drm.renderer);
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
        match drm
            .compositor
            .render_frame(&mut drm.renderer, &elements, clear, flags)
        {
            Ok(result) if !result.is_empty => {
                // Without fence support (virtio-gpu, software GL) the GPU may still be drawing
                // when the frame is queued, and the screen would show a half-drawn frame.
                // Smithay leaves waiting for the render to finish to us in that case.
                if result.needs_sync() {
                    if let PrimaryPlaneElement::Swapchain(element) = result.primary_element {
                        let _ = element.sync.wait();
                    }
                }
                match drm.compositor.queue_frame(()) {
                    Ok(()) => {
                        tracing::trace!(elements = elements.len(), "frame queued");
                        drm.frame_pending = true;
                    }
                    Err(e) => {
                        tracing::warn!(error = ?e, "queue_frame");
                        failed = true;
                    }
                }
            }
            // Nothing changed on screen: stay idle until something does.
            Ok(_) => tracing::trace!(elements = elements.len(), "frame empty"),
            Err(e) => {
                tracing::warn!(error = ?e, "render_frame");
                failed = true;
            }
        }
        drop(elements);
        // Clients waiting on a frame callback may draw again; if they commit, that wakes us.
        self.send_frames(&output);
        self.drm = Some(drm);
        if failed {
            self.retry_render_later();
        }
    }
}
