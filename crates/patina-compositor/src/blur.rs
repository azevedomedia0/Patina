//! Compositor-side backdrop blur: what is behind a glass surface, blurred and saturated.
//!
//! Each glass surface owns a snapshot of the scene as it was just *before* that surface was drawn,
//! covering the surface plus the reach of the blur. The blur reads that snapshot rather than the
//! scene itself, which by then also holds everything drawn above the glass. That keeps the blur
//! exact when only part of the scene is repainted.
//!
//! The snapshot is downsampled to a quarter of its size, blurred there with a separable Gaussian,
//! and drawn back inside the surface's rounded rectangle. Working at a quarter keeps a 24 px blur
//! cheap while looking the same.

use std::collections::HashMap;

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            gles::{
                GlesError, GlesRenderer, GlesTexProgram, GlesTexture, Uniform, UniformName,
                UniformType,
            },
            Bind, Color32F, Frame, Offscreen, Renderer, Texture,
        },
    },
    reexports::wayland_server::backend::ObjectId,
    utils::{Buffer, Physical, Rectangle, Size, Transform},
};

const GAUSSIAN: &str = include_str!("gaussian.frag");
const BACKDROP: &str = include_str!("backdrop.frag");

/// How much smaller the blur runs than the surface, per side.
const DOWNSCALE: i32 = 4;

/// What to blur: the area behind one surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Backdrop {
    /// The surface's rectangle in scene pixels.
    pub rect: Rectangle<i32, Physical>,
    /// Corner radius of the glass, in pixels.
    pub radius: f32,
    /// Blur standard deviation in pixels (CSS `blur(Npx)`).
    pub sigma: f32,
    /// 1.0 for none; the design's glass uses 1.4.
    pub saturation: f32,
}

impl Backdrop {
    /// How far the blur reaches, in pixels: three sigma (where the Gaussian is cut off), plus up to
    /// 4 px for the 4x4 downsampling blocks on the way in and 4 px for the bilinear upsampling on
    /// the way out, with some room to spare. A smaller margin showed up in verification as
    /// pixels just outside the repainted area that were off by one.
    pub fn reach(&self) -> i32 {
        (3.0 * self.sigma).ceil() as i32 + 48
    }

    /// The area the blur reads: the surface grown by its reach, kept on screen.
    fn region(&self, screen: Rectangle<i32, Physical>) -> Option<Rectangle<i32, Physical>> {
        let r = self.reach();
        Rectangle::new(
            (self.rect.loc.x - r, self.rect.loc.y - r).into(),
            (self.rect.size.w + 2 * r, self.rect.size.h + 2 * r).into(),
        )
        .intersection(screen)
    }
}

/// One glass surface's snapshot and the smaller textures its blur runs in.
struct Layer {
    backdrop: Backdrop,
    region: Rectangle<i32, Physical>,
    below: GlesTexture,
    half: GlesTexture,
    quarter_a: GlesTexture,
    quarter_b: GlesTexture,
}

pub struct Blur {
    size: Size<i32, Physical>,
    gaussian: GlesTexProgram,
    backdrop: GlesTexProgram,
    layers: HashMap<ObjectId, Layer>,
}

fn texture(renderer: &mut GlesRenderer, w: i32, h: i32) -> Result<GlesTexture, GlesError> {
    let size: Size<i32, Buffer> = (w.max(1), h.max(1)).into();
    renderer.create_buffer(Fourcc::Abgr8888, size)
}

fn div_ceil(n: i32, d: i32) -> i32 {
    (n + d - 1) / d
}

impl Blur {
    pub fn new(renderer: &mut GlesRenderer, size: Size<i32, Physical>) -> Result<Self, GlesError> {
        let gaussian = renderer.compile_custom_texture_shader(
            GAUSSIAN,
            &[
                UniformName::new("step", UniformType::_2f),
                UniformName::new("sigma", UniformType::_1f),
            ],
        )?;
        let backdrop = renderer.compile_custom_texture_shader(
            BACKDROP,
            &[
                UniformName::new("uv_to_px", UniformType::_2f),
                UniformName::new("rect", UniformType::_4f),
                UniformName::new("radius", UniformType::_1f),
                UniformName::new("saturation", UniformType::_1f),
            ],
        )?;
        Ok(Self {
            size,
            gaussian,
            backdrop,
            layers: HashMap::new(),
        })
    }

    pub fn size(&self) -> Size<i32, Physical> {
        self.size
    }

    /// Gives every glass surface in `glass` snapshot textures that match it, and drops the ones of
    /// surfaces that are gone. Returns whether any snapshot was created or replaced: such a
    /// snapshot holds nothing valid yet, so the scene must be repainted in full.
    pub fn sync(
        &mut self,
        renderer: &mut GlesRenderer,
        glass: &[(ObjectId, Backdrop)],
    ) -> Result<bool, GlesError> {
        let screen = Rectangle::<i32, Physical>::from_size(self.size);
        self.layers
            .retain(|key, _| glass.iter().any(|(k, _)| k == key));

        let mut renewed = false;
        for (key, backdrop) in glass {
            let Some(region) = backdrop.region(screen) else {
                // Entirely off screen: nothing to blur.
                self.layers.remove(key);
                continue;
            };
            if self
                .layers
                .get(key)
                .is_some_and(|l| l.backdrop == *backdrop && l.region == region)
            {
                continue;
            }
            let (w, h) = (region.size.w, region.size.h);
            let layer = Layer {
                backdrop: *backdrop,
                region,
                below: texture(renderer, w, h)?,
                half: texture(renderer, div_ceil(w, 2), div_ceil(h, 2))?,
                quarter_a: texture(renderer, div_ceil(w, DOWNSCALE), div_ceil(h, DOWNSCALE))?,
                quarter_b: texture(renderer, div_ceil(w, DOWNSCALE), div_ceil(h, DOWNSCALE))?,
            };
            self.layers.insert(key.clone(), layer);
            renewed = true;
        }
        Ok(renewed)
    }

    /// Copies the parts of `scene` inside `damage` into the glass surface's snapshot. Called just
    /// before the surface is drawn, when the scene holds only what is below it.
    pub fn snapshot(
        &mut self,
        renderer: &mut GlesRenderer,
        scene: &GlesTexture,
        key: &ObjectId,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        let Some(layer) = self.layers.get_mut(key) else {
            return Ok(());
        };
        let region = layer.region;
        let relative: Vec<_> = damage
            .iter()
            .filter_map(|d| d.intersection(region))
            .map(|d| Rectangle::new(d.loc - region.loc, d.size))
            .collect();
        if relative.is_empty() {
            return Ok(());
        }
        let src = Rectangle::<f64, Buffer>::new(
            (region.loc.x as f64, region.loc.y as f64).into(),
            (region.size.w as f64, region.size.h as f64).into(),
        );
        let size = region.size;
        let mut target = renderer.bind(&mut layer.below)?;
        let mut frame = renderer.render(&mut target, size, Transform::Normal)?;
        // Clear first, so that this replaces the old snapshot exactly instead of blending over it:
        // a pixel with alpha a hair under 1.0 would otherwise keep a trace of what was there.
        frame.clear(Color32F::TRANSPARENT, &relative)?;
        frame.render_texture_from_to(
            scene,
            src,
            Rectangle::from_size(size),
            &relative,
            &[],
            Transform::Normal,
            1.0,
            None,
            &[],
        )?;
        // Later passes run in the same GL context, so they already follow this one.
        let _ = frame.finish()?;
        Ok(())
    }

    /// Replaces what is behind the glass surface in `scene` with its blurred, saturated snapshot,
    /// but only inside `damage` (scene pixels): the rest of the scene is already right.
    pub fn apply(
        &mut self,
        renderer: &mut GlesRenderer,
        scene: &mut GlesTexture,
        key: &ObjectId,
        damage: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        let Some(layer) = self.layers.get_mut(key) else {
            return Ok(());
        };
        let backdrop = layer.backdrop;
        // The parts of the damage that fall on this surface, relative to its own corner.
        let relative: Vec<Rectangle<i32, Physical>> = damage
            .iter()
            .filter_map(|d| d.intersection(backdrop.rect))
            .map(|d| Rectangle::new(d.loc - backdrop.rect.loc, d.size))
            .collect();
        if relative.is_empty() {
            return Ok(());
        }

        // Down to a quarter, in two halvings so each step averages a 2x2 block.
        pass(renderer, &layer.below, &mut layer.half, None, &[])?;
        pass(renderer, &layer.half, &mut layer.quarter_a, None, &[])?;

        let qa = layer.quarter_a.size();
        let (qw, qh) = (qa.w as f32, qa.h as f32);
        let sigma = (backdrop.sigma / DOWNSCALE as f32).max(0.5);
        pass(
            renderer,
            &layer.quarter_a,
            &mut layer.quarter_b,
            Some(&self.gaussian),
            &[
                Uniform::new("step", [1.0 / qw, 0.0]),
                Uniform::new("sigma", sigma),
            ],
        )?;
        pass(
            renderer,
            &layer.quarter_b,
            &mut layer.quarter_a,
            Some(&self.gaussian),
            &[
                Uniform::new("step", [0.0, 1.0 / qh]),
                Uniform::new("sigma", sigma),
            ],
        )?;

        // Draw the blurred quarter-size image back over the surface's rectangle. The snapshot
        // starts at `region.loc`, so the surface sits at an offset inside it.
        let r = backdrop.rect;
        let offset = r.loc - layer.region.loc;
        let src = Rectangle::<f64, Buffer>::new(
            (
                offset.x as f64 / DOWNSCALE as f64,
                offset.y as f64 / DOWNSCALE as f64,
            )
                .into(),
            (
                r.size.w as f64 / DOWNSCALE as f64,
                r.size.h as f64 / DOWNSCALE as f64,
            )
                .into(),
        );
        let scene_size = scene.size();
        let mut target = renderer.bind(scene)?;
        let mut frame = renderer.render(
            &mut target,
            (scene_size.w, scene_size.h).into(),
            Transform::Normal,
        )?;
        frame.render_texture_from_to(
            &layer.quarter_a,
            src,
            r,
            &relative,
            &[],
            Transform::Normal,
            1.0,
            Some(&self.backdrop),
            &[
                Uniform::new("uv_to_px", [qw * DOWNSCALE as f32, qh * DOWNSCALE as f32]),
                Uniform::new(
                    "rect",
                    [
                        offset.x as f32,
                        offset.y as f32,
                        r.size.w as f32,
                        r.size.h as f32,
                    ],
                ),
                Uniform::new("radius", backdrop.radius),
                Uniform::new("saturation", backdrop.saturation),
            ],
        )?;
        // Later passes run in the same GL context, so they already follow this one.
        let _ = frame.finish()?;
        Ok(())
    }
}

/// Draws all of `src` into all of `dst`, optionally through a program.
fn pass(
    renderer: &mut GlesRenderer,
    src: &GlesTexture,
    dst: &mut GlesTexture,
    program: Option<&GlesTexProgram>,
    uniforms: &[Uniform<'_>],
) -> Result<(), GlesError> {
    let dst_size = dst.size();
    let size: Size<i32, Physical> = (dst_size.w, dst_size.h).into();
    let full = Rectangle::from_size(size);
    let src_size = src.size();
    let mut target = renderer.bind(dst)?;
    let mut frame = renderer.render(&mut target, size, Transform::Normal)?;
    frame.clear(Color32F::TRANSPARENT, &[full])?;
    frame.render_texture_from_to(
        src,
        Rectangle::from_size((src_size.w as f64, src_size.h as f64).into()),
        full,
        &[full],
        &[],
        Transform::Normal,
        1.0,
        program,
        uniforms,
    )?;
    // Later passes run in the same GL context, so they already follow this one.
    let _ = frame.finish()?;
    Ok(())
}
