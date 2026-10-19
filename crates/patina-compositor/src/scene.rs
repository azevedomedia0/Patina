//! Composes the frame into an offscreen texture, bottom to top, so that glass surfaces can
//! blur what is beneath them.
//!
//! A plain element list cannot do this: blurring needs the pixels already drawn below. The scene
//! is rebuilt only when something in it changed, and handed to the output as one texture.

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            element::{
                texture::{TextureRenderBuffer, TextureRenderElement},
                Element, Id, Kind,
            },
            gles::{GlesError, GlesRenderer, GlesTexture},
            utils::{draw_render_elements, CommitCounter},
            Bind, Color32F, Frame, Offscreen, Renderer,
        },
    },
    utils::{Buffer, Physical, Rectangle, Scale, Size, Transform},
};

use crate::{
    blur::{Backdrop, Blur},
    render::{Group, PatinaElement},
};

/// What identifies an element's appearance. If a whole frame's worth of these is unchanged, the
/// scene does not need composing again.
#[derive(Debug, PartialEq)]
struct Signature {
    id: Id,
    commit: CommitCounter,
    geometry: Rectangle<i32, Physical>,
    alpha: u32,
}

fn signature(groups: &[Group], scale: Scale<f64>) -> (Vec<Signature>, Vec<Option<Backdrop>>) {
    let elements = groups
        .iter()
        .flat_map(|g| &g.elements)
        .map(|e| Signature {
            id: e.id().clone(),
            commit: e.current_commit(),
            geometry: e.geometry(scale),
            alpha: e.alpha().to_bits(),
        })
        .collect();
    (elements, groups.iter().map(|g| g.backdrop).collect())
}

pub struct Scene {
    blur: Blur,
    buffer: TextureRenderBuffer<GlesTexture>,
    signature: Option<(Vec<Signature>, Vec<Option<Backdrop>>)>,
}

impl Scene {
    pub fn new(renderer: &mut GlesRenderer, size: Size<i32, Physical>) -> Result<Self, GlesError> {
        let blur = Blur::new(renderer, size)?;
        let texture = renderer.create_buffer(Fourcc::Abgr8888, (size.w, size.h).into())?;
        // Everything is drawn over an opaque wallpaper, so the scene is opaque.
        let full = Rectangle::<i32, Buffer>::from_size((size.w, size.h).into());
        let buffer = TextureRenderBuffer::from_texture(
            renderer,
            texture,
            1,
            Transform::Normal,
            Some(vec![full]),
        );
        Ok(Self {
            blur,
            buffer,
            signature: None,
        })
    }

    pub fn size(&self) -> Size<i32, Physical> {
        self.blur.size()
    }

    /// Recomposes the scene if anything in `groups` changed. Groups run bottom to top.
    pub fn update(
        &mut self,
        renderer: &mut GlesRenderer,
        groups: &[Group],
        scale: Scale<f64>,
    ) -> Result<(), GlesError> {
        let now = signature(groups, scale);
        if self.signature.as_ref() == Some(&now) {
            return Ok(());
        }

        let started = std::time::Instant::now();
        let size = self.blur.size();
        let full = Rectangle::<i32, Physical>::from_size(size);
        let full_buffer = Rectangle::<i32, Buffer>::from_size((size.w, size.h).into());
        let blur = &mut self.blur;
        self.buffer.render().draw(|scene| {
            {
                let mut target = renderer.bind(scene)?;
                let mut frame = renderer.render(&mut target, size, Transform::Normal)?;
                frame.clear(Color32F::BLACK, &[full])?;
                let _ = frame.finish()?;
            }
            for group in groups {
                if let Some(backdrop) = &group.backdrop {
                    blur.apply(renderer, scene, backdrop)?;
                }
                if group.elements.is_empty() {
                    continue;
                }
                let mut target = renderer.bind(scene)?;
                let mut frame = renderer.render(&mut target, size, Transform::Normal)?;
                draw_render_elements::<GlesRenderer, _, PatinaElement>(
                    &mut frame,
                    scale,
                    &group.elements,
                    &[full],
                )?;
                let _ = frame.finish()?;
            }
            Ok::<_, GlesError>(vec![full_buffer])
        })?;
        tracing::trace!(
            ms = started.elapsed().as_secs_f32() * 1000.0,
            blurred = groups.iter().filter(|g| g.backdrop.is_some()).count(),
            "scene composed"
        );
        self.signature = Some(now);
        Ok(())
    }

    /// The composed scene, as an element covering the whole output.
    pub fn element(&self) -> PatinaElement {
        PatinaElement::Scene(TextureRenderElement::from_texture_render_buffer(
            (0.0, 0.0),
            &self.buffer,
            None,
            None,
            None,
            Kind::Unspecified,
        ))
    }
}
