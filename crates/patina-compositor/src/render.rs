//! Builds what is drawn on an output.
//!
//! Everything is gathered into groups, bottom-most first. Each group is a surface (a window with
//! its titlebar and shadow, or a layer surface such as the panel) and, for glass surfaces, the
//! area behind it to blur. The scene compositor draws the groups in order into an offscreen
//! texture, which is what the output shows.

use smithay::{
    backend::{
        allocator::Fourcc,
        renderer::{
            element::{
                memory::{MemoryRenderBuffer, MemoryRenderBufferRenderElement},
                render_elements,
                surface::WaylandSurfaceRenderElement,
                texture::TextureRenderElement,
                AsRenderElements, Kind,
            },
            gles::{GlesRenderer, GlesTexture},
        },
    },
    desktop::{layer_map_for_output, LayerSurface as DesktopLayerSurface, Window},
    output::Output,
    utils::{Logical, Physical, Point, Rectangle, Scale, Size, Transform},
    wayland::shell::wlr_layer::Layer,
};

use crate::{blur::Backdrop, clip::ClippedSurface, scene::Scene, state::Patina, wm::win_state};
use patina_frame::{glass, shadow, TITLEBAR_H};

render_elements! {
    pub PatinaElement<=GlesRenderer>;
    Surface = WaylandSurfaceRenderElement<GlesRenderer>,
    Clipped = ClippedSurface,
    Image = MemoryRenderBufferRenderElement<GlesRenderer>,
    Scene = TextureRenderElement<GlesTexture>,
}

/// One surface and, if it is glass, the area behind it to blur first.
pub struct Group {
    /// The surface's elements, top-most first.
    pub elements: Vec<PatinaElement>,
    pub backdrop: Option<Backdrop>,
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

fn physical(r: Rectangle<i32, Logical>, scale: Scale<f64>) -> Rectangle<i32, Physical> {
    Rectangle::new(
        Point::from((r.loc.x as f64 * scale.x, r.loc.y as f64 * scale.y)).to_i32_round(),
        Size::from((r.size.w as f64 * scale.x, r.size.h as f64 * scale.y)).to_i32_round(),
    )
}

/// Layer surfaces that are glass, by namespace. Only the panel exists so far; the dock, launcher
/// and quick settings will join it.
fn layer_glass(namespace: &str) -> Option<glass::Glass> {
    match namespace {
        "patina-panel" => Some(glass::panel()),
        _ => None,
    }
}

struct LayerInfo {
    layer: DesktopLayerSurface,
    geometry: Rectangle<i32, Logical>,
    kind: Layer,
}

fn layer_group(renderer: &mut GlesRenderer, info: &LayerInfo, scale: Scale<f64>) -> Group {
    let elements = info
        .layer
        .render_elements::<WaylandSurfaceRenderElement<GlesRenderer>>(
            renderer,
            info.geometry.loc.to_physical_precise_round(scale),
            scale,
            1.0,
        )
        .into_iter()
        .map(PatinaElement::Surface)
        .collect();
    let backdrop = layer_glass(info.layer.namespace()).map(|g| Backdrop {
        rect: physical(info.geometry, scale),
        radius: 0.0,
        sigma: g.sigma * scale.x as f32,
        saturation: g.saturation,
    });
    Group { elements, backdrop }
}

/// The wallpaper as an element at the bottom of the scene, redrawn only when the output size
/// changes.
fn wallpaper_element(
    state: &mut Patina,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
) -> Option<PatinaElement> {
    let key = (size.w, size.h);
    if state.wallpaper.as_ref().is_none_or(|(k, _)| *k != key) {
        let bitmap = patina_frame::wallpaper::render(size.w, size.h);
        state.wallpaper = Some((key, upload(&bitmap.bgra, bitmap.width, bitmap.height)));
    }
    let (_, buffer) = state.wallpaper.as_ref()?;
    MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        Point::<f64, Physical>::from((0.0, 0.0)),
        buffer,
        None,
        None,
        None,
        Kind::Unspecified,
    )
    .ok()
    .map(PatinaElement::Image)
}

/// Everything on `output`, bottom-most group first.
pub fn groups(state: &mut Patina, renderer: &mut GlesRenderer, output: &Output) -> Vec<Group> {
    let scale = Scale::from(output.current_scale().fractional_scale());
    let size = output.current_mode().map(|m| m.size).unwrap_or_default();
    let mut groups: Vec<Group> = Vec::new();

    // Layer surfaces, top-most first.
    let layers: Vec<LayerInfo> = {
        let map = layer_map_for_output(output);
        map.layers()
            .rev()
            .filter_map(|l| {
                map.layer_geometry(l).map(|geometry| LayerInfo {
                    layer: l.clone(),
                    geometry,
                    kind: l.layer(),
                })
            })
            .collect()
    };
    let is_below = |l: Layer| matches!(l, Layer::Background | Layer::Bottom);

    // The bottom group: background and bottom layers over the wallpaper.
    let mut base: Vec<PatinaElement> = layers
        .iter()
        .filter(|l| is_below(l.kind))
        .flat_map(|l| layer_group(renderer, l, scale).elements)
        .collect();
    base.extend(wallpaper_element(state, renderer, size));
    groups.push(Group {
        elements: base,
        backdrop: None,
    });

    // Windows, bottom-most first. Each: surfaces, then titlebar, then shadow underneath.
    let windows: Vec<Window> = state.space.elements().cloned().collect();
    for window in windows {
        let Some(geo) = state.client_rect(&window) else {
            continue;
        };
        let mut out: Vec<PatinaElement> = Vec::new();
        let bar = state.title_bar_for(&window, geo.size.w);
        let alpha = if bar.active { 1.0 } else { INACTIVE_ALPHA };
        let maximized = bar.maximized;

        // Client surfaces, with the bottom corners rounded unless the window fills the screen.
        let origin =
            state.space.element_location(&window).unwrap_or_default() - window.geometry().loc;
        let content = physical(geo, scale);
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

        // The glass behind the whole window frame, titlebar and client area.
        let g = glass::window();
        let frame = Rectangle::<i32, Logical>::new(
            (geo.loc.x, geo.loc.y - TITLEBAR_H).into(),
            (geo.size.w, geo.size.h + TITLEBAR_H).into(),
        );
        groups.push(Group {
            elements: out,
            backdrop: Some(Backdrop {
                rect: physical(frame, scale),
                radius: if maximized { 0.0 } else { radius },
                sigma: g.sigma * scale.x as f32,
                saturation: g.saturation,
            }),
        });
    }

    // Layers above windows (top, overlay), bottom-most first.
    for info in layers.iter().rev().filter(|l| !is_below(l.kind)) {
        groups.push(layer_group(renderer, info, scale));
    }
    groups
}

/// Top-most element first, without blur: the fallback when the scene cannot be composed.
fn flatten(groups: Vec<Group>) -> Vec<PatinaElement> {
    groups.into_iter().rev().flat_map(|g| g.elements).collect()
}

/// What to hand the output for this frame, top-most first (the pointer is added by the caller).
pub fn frame_elements(
    state: &mut Patina,
    renderer: &mut GlesRenderer,
    output: &Output,
) -> Vec<PatinaElement> {
    let scale = Scale::from(output.current_scale().fractional_scale());
    let groups = groups(state, renderer, output);
    let Some(size) = output.current_mode().map(|m| m.size) else {
        return flatten(groups);
    };
    if state.scene_unavailable {
        return flatten(groups);
    }

    if state.scene.as_ref().is_none_or(|s| s.size() != size) {
        match Scene::new(renderer, size) {
            Ok(scene) => state.scene = Some(scene),
            Err(e) => {
                tracing::warn!(error = %e, "blur unavailable: cannot create the scene");
                state.scene_unavailable = true;
                return flatten(groups);
            }
        }
    }
    let Some(scene) = state.scene.as_mut() else {
        return flatten(groups);
    };
    if let Err(e) = scene.update(renderer, &groups, scale) {
        tracing::warn!(error = %e, "blur unavailable: cannot compose the scene");
        state.scene = None;
        state.scene_unavailable = true;
        return flatten(groups);
    }
    vec![scene.element()]
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
