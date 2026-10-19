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
        },
    },
    desktop::{layer_map_for_output, Window},
    output::Output,
    utils::{Physical, Point, Rectangle, Scale, Size, Transform},
    wayland::shell::wlr_layer::Layer,
};

use crate::{clip::ClippedSurface, state::Patina, wm::win_state};
use patina_frame::{shadow, TITLEBAR_H};

render_elements! {
    pub PatinaElement<=GlesRenderer>;
    Surface = WaylandSurfaceRenderElement<GlesRenderer>,
    Clipped = ClippedSurface,
    Image = MemoryRenderBufferRenderElement<GlesRenderer>,
}

/// Inactive windows are drawn slightly translucent (`opacity: .92` in the design).
const INACTIVE_ALPHA: f32 = 0.92;

fn upload(bgra: &[u8], width: i32, height: i32) -> MemoryRenderBuffer {
    MemoryRenderBuffer::from_slice(
        bgra,
        Fourcc::Argb8888,
        (width, height),
        1,
        Transform::Normal,
        None,
    )
}

pub fn elements(
    state: &mut Patina,
    renderer: &mut GlesRenderer,
    output: &Output,
) -> Vec<PatinaElement> {
    let scale = Scale::from(output.current_scale().fractional_scale());
    let mut out: Vec<PatinaElement> = Vec::new();

    // Layers, top-most first.
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

    // Windows, top-most first: surfaces, then titlebar, then shadow underneath.
    let windows: Vec<Window> = state.space.elements().rev().cloned().collect();
    for window in windows {
        let Some(geo) = state.client_rect(&window) else {
            continue;
        };
        let bar = state.title_bar_for(&window, geo.size.w);
        let alpha = if bar.active { 1.0 } else { INACTIVE_ALPHA };
        let maximized = bar.maximized;

        // Client surfaces, with the bottom corners rounded unless the window fills the screen.
        let origin =
            state.space.element_location(&window).unwrap_or_default() - window.geometry().loc;
        let content = Rectangle::<i32, Physical>::new(
            Point::from((geo.loc.x, geo.loc.y))
                .to_f64()
                .to_physical(scale)
                .to_i32_round(),
            Size::from((geo.size.w, geo.size.h))
                .to_f64()
                .to_physical(scale)
                .to_i32_round(),
        );
        let radius = patina_frame::geometry::RADIUS * scale.x as f32;
        for el in window.render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
            renderer,
            origin.to_physical_precise_round(scale),
            scale,
            alpha,
        ) {
            match (&state.clip_program, maximized) {
                (Some(program), false) => out.push(PatinaElement::Clipped(ClippedSurface::new(
                    el,
                    program.clone(),
                    content,
                    radius,
                    scale,
                ))),
                _ => out.push(PatinaElement::Surface(el)),
            }
        }

        // Titlebar texture, redrawn only when what it shows changes.
        let cell = win_state(&window);
        if cell
            .borrow()
            .bar
            .as_ref()
            .is_none_or(|(key, _)| *key != bar)
        {
            let bitmap = state.rasterizer.draw(&bar);
            let buffer = upload(&bitmap.bgra, bitmap.width, bitmap.height);
            cell.borrow_mut().bar = Some((bar.clone(), buffer));
        }
        if let Some((_, buffer)) = cell.borrow().bar.as_ref() {
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
                out.push(PatinaElement::Image(el));
            }
        }

        // Shadow and outline, redrawn when the frame size or activity changes.
        if !maximized {
            let (fw, fh) = (geo.size.w, geo.size.h + TITLEBAR_H);
            let key = (fw, fh, bar.active);
            if cell
                .borrow()
                .shadow
                .as_ref()
                .is_none_or(|(k, _, _)| *k != key)
            {
                let s = shadow::render(fw, fh, bar.active);
                let buffer = upload(&s.bitmap.bgra, s.bitmap.width, s.bitmap.height);
                cell.borrow_mut().shadow = Some((key, buffer, s.pad));
            }
            if let Some((_, buffer, pad)) = cell.borrow().shadow.as_ref() {
                let at: Point<f64, Physical> = Point::from((
                    (geo.loc.x - pad) as f64 * scale.x,
                    (geo.loc.y - TITLEBAR_H - pad) as f64 * scale.y,
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
                    out.push(PatinaElement::Image(el));
                }
            }
        }
    }

    out.extend(layer_elems(renderer, |l| {
        matches!(l, Layer::Bottom | Layer::Background)
    }));
    out
}

/// The pointer, drawn by the compositor when there is no host to draw it (DRM backend).
/// Returned top-most first; usually one element, none when a client hides the cursor.
pub fn cursor_elements(state: &mut Patina, renderer: &mut GlesRenderer) -> Vec<PatinaElement> {
    use crate::wm::CursorSource;
    use smithay::{
        backend::renderer::element::surface::render_elements_from_surface_tree,
        input::pointer::{CursorIcon, CursorImageSurfaceData},
        wayland::compositor::with_states,
    };

    if !state.software_cursor {
        return Vec::new();
    }
    let Some(pos) = state.seat.get_pointer().map(|p| p.current_location()) else {
        return Vec::new();
    };
    let scale = state
        .output
        .as_ref()
        .map(|o| Scale::from(o.current_scale().fractional_scale()))
        .unwrap_or(Scale::from(1.0));
    let at =
        |hotspot: smithay::utils::Point<i32, smithay::utils::Logical>| -> Point<f64, Physical> {
            Point::from((
                (pos.x - hotspot.x as f64) * scale.x,
                (pos.y - hotspot.y as f64) * scale.y,
            ))
        };
    let image = |renderer: &mut GlesRenderer,
                 buffer: &MemoryRenderBuffer,
                 hotspot: smithay::utils::Point<i32, smithay::utils::Logical>| {
        MemoryRenderBufferRenderElement::from_buffer(
            renderer,
            at(hotspot),
            buffer,
            None,
            None,
            None,
            Kind::Cursor,
        )
        .ok()
        .map(PatinaElement::Image)
        .into_iter()
        .collect::<Vec<_>>()
    };

    match state.effective_cursor() {
        CursorSource::Hidden => Vec::new(),
        CursorSource::Surface(surface) => {
            let hotspot = with_states(&surface, |states| {
                states
                    .data_map
                    .get::<CursorImageSurfaceData>()
                    .map(|d| d.lock().unwrap().hotspot)
                    .unwrap_or_default()
            });
            render_elements_from_surface_tree::<_, WaylandSurfaceRenderElement<GlesRenderer>>(
                renderer,
                &surface,
                at(hotspot).to_i32_round(),
                scale,
                1.0,
                Kind::Cursor,
            )
            .into_iter()
            .map(PatinaElement::Surface)
            .collect()
        }
        CursorSource::Icon(icon) => {
            // The design's arrow is the default pointer; everything else is the system theme's,
            // falling back to the arrow if the theme lacks it.
            if icon != CursorIcon::Default {
                if let Some(named) = state.cursor_theme.get(icon) {
                    return image(renderer, &named.buffer, named.hotspot);
                }
            }
            match &state.cursor {
                Some(arrow) => image(renderer, &arrow.buffer, arrow.hotspot),
                None => Vec::new(),
            }
        }
    }
}
