//! Compositor-side backdrop blur: what is behind a glass surface, blurred and saturated.
//!
//! The scene is downsampled to a quarter of its size, blurred with a separable Gaussian there,
//! and drawn back inside the surface's rounded rectangle. Working at a quarter of the size
//! keeps a 24 px blur cheap while looking the same.

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
    utils::{Buffer, Physical, Rectangle, Size, Transform},
};

const GAUSSIAN: &str = include_str!("gaussian.frag");
const BACKDROP: &str = include_str!("backdrop.frag");

/// How much smaller the blur runs than the screen, per side.
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

pub struct Blur {
    size: Size<i32, Physical>,
    gaussian: GlesTexProgram,
    backdrop: GlesTexProgram,
    half: GlesTexture,
    quarter_a: GlesTexture,
    quarter_b: GlesTexture,
}

fn buffer_size(w: i32, h: i32) -> Size<i32, Buffer> {
    (w.max(1), h.max(1)).into()
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
        let make = |r: &mut GlesRenderer, div: i32| -> Result<GlesTexture, GlesError> {
            r.create_buffer(Fourcc::Abgr8888, buffer_size(size.w / div, size.h / div))
        };
        Ok(Self {
            size,
            gaussian,
            backdrop,
            half: make(renderer, 2)?,
            quarter_a: make(renderer, DOWNSCALE)?,
            quarter_b: make(renderer, DOWNSCALE)?,
        })
    }

    pub fn size(&self) -> Size<i32, Physical> {
        self.size
    }

    /// Replaces what is behind `backdrop.rect` in `scene` with its blurred, saturated self.
    pub fn apply(
        &mut self,
        renderer: &mut GlesRenderer,
        scene: &mut GlesTexture,
        backdrop: &Backdrop,
    ) -> Result<(), GlesError> {
        // Down to a quarter, in two halvings so each step averages a 2x2 block.
        pass(renderer, scene, &mut self.half, None, &[])?;
        pass(renderer, &self.half, &mut self.quarter_a, None, &[])?;

        let (qw, qh) = (
            self.quarter_a.size().w as f32,
            self.quarter_a.size().h as f32,
        );
        let sigma = (backdrop.sigma / DOWNSCALE as f32).max(0.5);
        pass(
            renderer,
            &self.quarter_a,
            &mut self.quarter_b,
            Some(&self.gaussian),
            &[
                Uniform::new("step", [1.0 / qw, 0.0]),
                Uniform::new("sigma", sigma),
            ],
        )?;
        pass(
            renderer,
            &self.quarter_b,
            &mut self.quarter_a,
            Some(&self.gaussian),
            &[
                Uniform::new("step", [0.0, 1.0 / qh]),
                Uniform::new("sigma", sigma),
            ],
        )?;

        // Draw the blurred quarter-size image back over the surface's rectangle.
        let r = backdrop.rect;
        let src = Rectangle::<f64, Buffer>::new(
            (
                r.loc.x as f64 / DOWNSCALE as f64,
                r.loc.y as f64 / DOWNSCALE as f64,
            )
                .into(),
            (
                r.size.w as f64 / DOWNSCALE as f64,
                r.size.h as f64 / DOWNSCALE as f64,
            )
                .into(),
        );
        let dest = Rectangle::<i32, Physical>::from_size(r.size);
        let size = scene.size();
        let mut target = renderer.bind(scene)?;
        let mut frame = renderer.render(&mut target, (size.w, size.h).into(), Transform::Normal)?;
        // The frame covers the whole scene; draw at the rectangle's position.
        let at = Rectangle::<i32, Physical>::new(r.loc, r.size);
        frame.render_texture_from_to(
            &self.quarter_a,
            src,
            at,
            &[dest],
            &[],
            Transform::Normal,
            1.0,
            Some(&self.backdrop),
            &[
                Uniform::new("uv_to_px", [qw * DOWNSCALE as f32, qh * DOWNSCALE as f32]),
                Uniform::new(
                    "rect",
                    [
                        r.loc.x as f32,
                        r.loc.y as f32,
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
