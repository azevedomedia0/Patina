//! Builds the render list for one output, top-most element first.

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            element::{
                memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
                render_elements,
                surface::WaylandSurfaceRenderElement,
                AsRenderElements, Kind,
            },
            gles::GlesRenderer,
            ImportAll, ImportMem,
        },
    },
    desktop::{layer_map_for_output, Window},
    output::Output,
    utils::{Physical, Point, Scale, Transform},
    wayland::shell::wlr_layer::Layer,
};

use crate::{state::Patina, wm::win_state};
use patina_frame::TITLEBAR_H;

render_elements! {
    pub PatinaElement<R> where R: ImportAll + ImportMem;
    Surface = WaylandSurfaceRenderElement<R>,
    Bar = MemoryRenderBufferRenderElement<R>,
}

/// Inactive windows are drawn slightly translucent (`opacity: .92` in the design).
const INACTIVE_ALPHA: f32 = 0.92;

pub fn elements(
    state: &mut Patina,
    renderer: &mut GlesRenderer,
    output: &Output,
) -> Vec<PatinaElement<GlesRenderer>> {
    let scale = Scale::from(output.current_scale().fractional_scale());
    let mut out: Vec<PatinaElement<GlesRenderer>> = Vec::new();

    // Layers above windows (overlay, top), top-most first.
    let layers: Vec<_> = {
        let map = layer_map_for_output(output);
        map.layers()
            .rev()
            .filter_map(|l| map.layer_geometry(l).map(|g| (l.clone(), g.loc, l.layer())))
            .collect()
    };
    let layer_elems = |renderer: &mut GlesRenderer, wanted: fn(Layer) -> bool| {
        let mut v = Vec::new();
        for (layer, loc, kind) in &layers {
            if wanted(*kind) {
                v.extend(
                    layer
                        .render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
                            renderer,
                            loc.to_physical_precise_round(scale),
                            scale,
                            1.0,
                        )
                        .into_iter()
                        .map(PatinaElement::Surface),
                );
            }
        }
        v
    };
    out.extend(layer_elems(renderer, |l| {
        matches!(l, Layer::Overlay | Layer::Top)
    }));

    // Windows, top-most first, each with its titlebar.
    let windows: Vec<Window> = state.space.elements().rev().cloned().collect();
    for window in windows {
        let Some(geo) = state.client_rect(&window) else {
            continue;
        };
        let bar = state.title_bar_for(&window, geo.size.w);
        let alpha = if bar.active { 1.0 } else { INACTIVE_ALPHA };

        let origin =
            state.space.element_location(&window).unwrap_or_default() - window.geometry().loc;
        out.extend(
            window
                .render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
                    renderer,
                    origin.to_physical_precise_round(scale),
                    scale,
                    alpha,
                )
                .into_iter()
                .map(PatinaElement::Surface),
        );

        let cell = win_state(&window);
        let stale = cell
            .borrow()
            .bar
            .as_ref()
            .is_none_or(|(key, _)| *key != bar);
        if stale {
            let bitmap = state.rasterizer.draw(&bar);
            let buffer = MemoryRenderBuffer::from_slice(
                &bitmap.bgra,
                Fourcc::Argb8888,
                (bitmap.width, bitmap.height),
                1,
                Transform::Normal,
                None,
            );
            cell.borrow_mut().bar = Some((bar, buffer));
        }
        let st = cell.borrow();
        if let Some((_, buffer)) = st.bar.as_ref() {
            let at: Point<f64, Physical> = Point::from((
                geo.loc.x as f64 * scale.x,
                (geo.loc.y - TITLEBAR_H) as f64 * scale.y,
            ));
            if let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                at,
                buffer,
                Some(alpha),
                None,
                None,
                Kind::Unspecified,
            ) {
                out.push(PatinaElement::Bar(el));
            }
        }
    }

    out.extend(layer_elems(renderer, |l| {
        matches!(l, Layer::Bottom | Layer::Background)
    }));
    out
}
