//! Rounds the bottom corners of window content to the design's 20 px window radius.
//!
//! Wraps a surface element and draws it with a custom texture shader that masks the
//! corners. The titlebar and shadow are separate elements; only client pixels are clipped.

use smithay::{
    backend::renderer::{
        element::{
            surface::WaylandSurfaceRenderElement, Element, Id, Kind, RenderElement,
            UnderlyingStorage,
        },
        gles::{
            GlesError, GlesFrame, GlesRenderer, GlesTexProgram, Uniform, UniformName, UniformType,
        },
        utils::{CommitCounter, DamageSet, OpaqueRegions},
    },
    utils::{Buffer, Physical, Point, Rectangle, Scale, Transform},
};

const SHADER: &str = include_str!("clip.frag");

pub fn compile(renderer: &mut GlesRenderer) -> Result<GlesTexProgram, GlesError> {
    renderer.compile_custom_texture_shader(
        SHADER,
        &[
            UniformName::new("win_size", UniformType::_2f),
            UniformName::new("elem_origin", UniformType::_2f),
            UniformName::new("elem_size", UniformType::_2f),
            UniformName::new("radius", UniformType::_1f),
        ],
    )
}

/// Compiles the shader, logging and returning `None` on failure (windows then keep square corners).
pub fn compile_logged(renderer: &mut GlesRenderer) -> Option<GlesTexProgram> {
    match compile(renderer) {
        Ok(program) => Some(program),
        Err(e) => {
            tracing::warn!(error = %e, "corner clipping disabled");
            None
        }
    }
}

pub struct ClippedSurface {
    inner: WaylandSurfaceRenderElement<GlesRenderer>,
    program: GlesTexProgram,
    uniforms: Vec<Uniform<'static>>,
}

impl ClippedSurface {
    /// `window` is the window content rectangle in physical pixels.
    pub fn new(
        inner: WaylandSurfaceRenderElement<GlesRenderer>,
        program: GlesTexProgram,
        window: Rectangle<i32, Physical>,
        radius: f32,
        scale: Scale<f64>,
    ) -> Self {
        let g = inner.geometry(scale);
        let uniforms = vec![
            Uniform::new("win_size", [window.size.w as f32, window.size.h as f32]),
            Uniform::new(
                "elem_origin",
                [
                    (g.loc.x - window.loc.x) as f32,
                    (g.loc.y - window.loc.y) as f32,
                ],
            ),
            Uniform::new("elem_size", [g.size.w as f32, g.size.h as f32]),
            Uniform::new("radius", radius),
        ];
        Self {
            inner,
            program,
            uniforms,
        }
    }
}

impl Element for ClippedSurface {
    fn id(&self) -> &Id {
        self.inner.id()
    }
    fn current_commit(&self) -> CommitCounter {
        self.inner.current_commit()
    }
    fn location(&self, scale: Scale<f64>) -> Point<i32, Physical> {
        self.inner.location(scale)
    }
    fn src(&self) -> Rectangle<f64, Buffer> {
        self.inner.src()
    }
    fn transform(&self) -> Transform {
        self.inner.transform()
    }
    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        self.inner.geometry(scale)
    }
    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        self.inner.damage_since(scale, commit)
    }
    /// The corners are translucent, so nothing here is opaque as far as blending goes.
    fn opaque_regions(&self, _scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        OpaqueRegions::default()
    }
    fn alpha(&self) -> f32 {
        self.inner.alpha()
    }
    fn kind(&self) -> Kind {
        self.inner.kind()
    }
}

impl RenderElement<GlesRenderer> for ClippedSurface {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        frame.override_default_tex_program(self.program.clone(), self.uniforms.clone());
        let result = RenderElement::<GlesRenderer>::draw(
            &self.inner,
            frame,
            src,
            dst,
            damage,
            opaque_regions,
        );
        frame.clear_tex_program_override();
        result
    }

    fn underlying_storage(&self, _renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        // Must go through the shader; never scan out directly.
        None
    }
}
