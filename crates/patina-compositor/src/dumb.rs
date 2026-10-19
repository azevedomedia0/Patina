//! Output for display-only devices: ones that can show a picture but cannot render it, such as
//! USB and DisplayLink adapters, `simpledrm`, or QEMU's `bochs-display`. Their OpenGL is
//! software only and cannot share buffers with the display hardware, so the frame is rendered
//! into a texture, read back, and copied into one of two CPU-mapped "dumb" buffers that the
//! display scans out.

use std::error::Error;

use smithay::{
    backend::{
        allocator::Fourcc,
        drm::{DrmDeviceFd, DrmSurface, PlaneConfig, PlaneState},
        renderer::{
            damage::OutputDamageTracker,
            gles::{GlesRenderer, GlesTexture},
            Bind, ExportMem, Offscreen,
        },
    },
    output::Output,
    reexports::drm::{
        buffer::{Buffer as _, DrmFourcc},
        control::{dumbbuffer::DumbBuffer, framebuffer, Device as ControlDevice},
    },
    utils::{Buffer, Rectangle, Size, Transform},
};

use crate::render::PatinaElement;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

pub struct DumbPresenter {
    surface: DrmSurface,
    fd: DrmDeviceFd,
    buffers: Vec<(DumbBuffer, framebuffer::Handle)>,
    /// The buffer to draw into next; the other one is on screen.
    next: usize,
    target: GlesTexture,
    damage: OutputDamageTracker,
    size: Size<i32, Buffer>,
    /// Whether the first frame (a full modeset commit) has gone out.
    started: bool,
    /// Whether the texture holds the previous frame, so only damage needs drawing.
    target_valid: bool,
}

impl DumbPresenter {
    pub fn new(
        surface: DrmSurface,
        fd: DrmDeviceFd,
        renderer: &mut GlesRenderer,
        output: &Output,
    ) -> Result<Self> {
        let mode = surface.pending_mode();
        let (w, h) = mode.size();
        let size = Size::<i32, Buffer>::from((w as i32, h as i32));
        let mut buffers = Vec::new();
        for _ in 0..2 {
            let buffer = fd.create_dumb_buffer((w as u32, h as u32), DrmFourcc::Xrgb8888, 32)?;
            let fb = fd.add_framebuffer(&buffer, 24, 32)?;
            buffers.push((buffer, fb));
        }
        let target = Offscreen::<GlesTexture>::create_buffer(renderer, Fourcc::Abgr8888, size)?;
        Ok(Self {
            surface,
            fd,
            buffers,
            next: 0,
            target,
            damage: OutputDamageTracker::from_output(output),
            size,
            started: false,
            target_valid: false,
        })
    }

    /// Draws `elements` and shows them. Returns whether a frame was queued (false when nothing
    /// changed).
    pub fn render(
        &mut self,
        renderer: &mut GlesRenderer,
        elements: &[PatinaElement],
        clear: [f32; 4],
    ) -> Result<bool> {
        let (w, h) = (self.size.w as usize, self.size.h as usize);
        let rgba = {
            let mut fb = renderer.bind(&mut self.target)?;
            let age = usize::from(self.target_valid);
            let result = self
                .damage
                .render_output(renderer, &mut fb, age, elements, clear)
                .map_err(|e| format!("render: {e:?}"))?;
            if result.damage.is_none() && self.started {
                return Ok(false);
            }
            self.target_valid = true;
            let mapping = renderer.copy_framebuffer(
                &fb,
                Rectangle::from_size(self.size),
                Fourcc::Abgr8888,
            )?;
            renderer.map_texture(&mapping)?.to_vec()
        };

        // RGBA from OpenGL into the display's XRGB (bytes B, G, R, X), row by row.
        let (buffer, fb) = &mut self.buffers[self.next];
        let pitch = buffer.pitch() as usize;
        {
            let mut map = self.fd.map_dumb_buffer(buffer)?;
            let dst: &mut [u8] = map.as_mut();
            for y in 0..h {
                let src = &rgba[y * w * 4..(y + 1) * w * 4];
                let row = &mut dst[y * pitch..y * pitch + w * 4];
                for (d, s) in row.chunks_exact_mut(4).zip(src.chunks_exact(4)) {
                    d[0] = s[2];
                    d[1] = s[1];
                    d[2] = s[0];
                    d[3] = 0xff;
                }
            }
        }
        let fb = *fb;
        let plane = PlaneState {
            handle: self.surface.plane(),
            config: Some(PlaneConfig {
                src: Rectangle::from_size((w as f64, h as f64).into()),
                dst: Rectangle::from_size((w as i32, h as i32).into()),
                transform: Transform::Normal,
                alpha: 1.0,
                damage_clips: None,
                fb,
                fence: None,
            }),
        };
        if self.started {
            self.surface.page_flip([plane], true)?;
        } else {
            self.surface.commit([plane], true)?;
            self.started = true;
        }
        self.next = 1 - self.next;
        Ok(true)
    }
}

impl Drop for DumbPresenter {
    fn drop(&mut self) {
        for (buffer, fb) in self.buffers.drain(..) {
            let _ = self.fd.destroy_framebuffer(fb);
            let _ = self.fd.destroy_dumb_buffer(buffer);
        }
    }
}
