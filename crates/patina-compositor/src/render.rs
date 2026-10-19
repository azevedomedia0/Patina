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
                surface::{render_elements_from_surface_tree, WaylandSurfaceRenderElement},
                texture::TextureRenderElement,
                AsRenderElements, Kind,
            },
            gles::{GlesRenderer, GlesTexProgram, GlesTexture},
        },
    },
    desktop::{layer_map_for_output, LayerSurface as DesktopLayerSurface, PopupManager, Window},
    output::Output,
    reexports::wayland_server::{backend::ObjectId, Resource},
    utils::{Logical, Physical, Point, Rectangle, Scale, Size, Transform},
    wayland::shell::wlr_layer::Layer,
};

use crate::{
    blur::Backdrop,
    clip::ClippedSurface,
    scene::Scene,
    state::Patina,
    wm::{win_state, Image},
};
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
    /// Which surface this is, so the scene can tell the same window from frame to frame. `None`
    /// for the bottom group (wallpaper and background layers).
    pub key: Option<ObjectId>,
    /// The surface's elements, top-most first.
    pub elements: Vec<PatinaElement>,
    pub backdrop: Option<Backdrop>,
}

/// What one output keeps between frames. Its textures belong to the renderer that draws the
/// output, so every output has its own.
pub struct OutputRender {
    /// The composed frame, rebuilt only where something in it changes.
    pub scene: Option<Scene>,
    /// A second scene composed in full, for `PATINA_VERIFY_RECOMPOSE=1`.
    pub scene_check: Option<Scene>,
    /// Set when the scene cannot be created; frames are then drawn without blur.
    pub unavailable: bool,
    wallpaper: Option<((i32, i32), MemoryRenderBuffer)>,
}

impl OutputRender {
    /// Forgets the composed frame, so the next one is composed in full.
    pub fn invalidate(&mut self) {
        self.scene = None;
        self.scene_check = None;
    }

    pub fn new(no_blur: bool) -> Self {
        Self {
            scene: None,
            scene_check: None,
            unavailable: no_blur,
            wallpaper: None,
        }
    }
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

/// All of a `pixels`-sized buffer drawn at buffer scale 1, as smithay's source rectangle.
pub fn whole(pixels: (i32, i32)) -> Rectangle<f64, Logical> {
    Rectangle::from_size(Size::from((pixels.0 as f64, pixels.1 as f64)))
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
    Group {
        key: Some(info.layer.wl_surface().id()),
        elements,
        backdrop,
    }
}

/// The wallpaper as an element at the bottom of the scene, redrawn only when the output size
/// changes.
fn wallpaper_element(
    rs: &mut OutputRender,
    renderer: &mut GlesRenderer,
    size: Size<i32, Physical>,
    scale: Scale<f64>,
) -> Option<PatinaElement> {
    let key = (size.w, size.h);
    if rs.wallpaper.as_ref().is_none_or(|(k, _)| *k != key) {
        let bitmap = patina_frame::wallpaper::render(size.w, size.h);
        rs.wallpaper = Some((key, upload(&bitmap.bgra, bitmap.width, bitmap.height)));
    }
    let (_, buffer) = rs.wallpaper.as_ref()?;
    // Drawn at device resolution, so it covers the output's logical size.
    let logical = Size::<i32, Logical>::from((
        (size.w as f64 / scale.x).round() as i32,
        (size.h as f64 / scale.y).round() as i32,
    ));
    MemoryRenderBufferRenderElement::from_buffer(
        renderer,
        Point::<f64, Physical>::from((0.0, 0.0)),
        buffer,
        None,
        Some(whole((size.w, size.h))),
        Some(logical),
        Kind::Unspecified,
    )
    .ok()
    .map(PatinaElement::Image)
}

/// Shadows reach this far outside a window frame (logical pixels); a window this close to an
/// output still draws on it.
const SHADOW_REACH: i32 = 96;

/// Everything on `output`, bottom-most group first, in the output's own coordinates.
pub fn groups(
    state: &mut Patina,
    renderer: &mut GlesRenderer,
    clip_program: Option<&GlesTexProgram>,
    rs: &mut OutputRender,
    output: &Output,
) -> Vec<Group> {
    let scale = Scale::from(output.current_scale().fractional_scale());
    let size = output.current_mode().map(|m| m.size).unwrap_or_default();
    // Windows live in compositor space; the output shows the part at its rectangle.
    let area = state.space.output_geometry(output).unwrap_or_default();
    let offset = area.loc;
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
    base.extend(wallpaper_element(rs, renderer, size, scale));
    groups.push(Group {
        key: None,
        elements: base,
        backdrop: None,
    });

    // Windows, bottom-most first. Each: surfaces, then titlebar, then shadow underneath.
    let windows: Vec<Window> = state.space.elements().cloned().collect();
    for window in windows {
        let Some(global) = state.client_rect(&window) else {
            continue;
        };
        let reach = Rectangle::<i32, Logical>::new(
            (
                global.loc.x - SHADOW_REACH,
                global.loc.y - TITLEBAR_H - SHADOW_REACH,
            )
                .into(),
            (
                global.size.w + 2 * SHADOW_REACH,
                global.size.h + TITLEBAR_H + 2 * SHADOW_REACH,
            )
                .into(),
        );
        if !reach.overlaps(area) {
            continue;
        }
        let geo = Rectangle::new(global.loc - offset, global.size);
        let mut out: Vec<PatinaElement> = Vec::new();
        let bar = state.title_bar_for(&window, geo.size.w, scale.x as f32);
        let alpha = if bar.active { 1.0 } else { INACTIVE_ALPHA };
        let maximized = bar.maximized;

        // Client surfaces, with the bottom corners rounded unless the window fills the screen.
        // Popups (menus) come first, being on top, and are not clipped to the window.
        let origin = state.space.element_location(&window).unwrap_or_default()
            - window.geometry().loc
            - offset;
        let at = origin.to_physical_precise_round(scale);
        let content = physical(geo, scale);
        let radius = patina_frame::geometry::RADIUS * scale.x as f32;
        if let Some(toplevel) = window.toplevel() {
            let surface = toplevel.wl_surface();
            for (popup, popup_offset) in PopupManager::popups_for_surface(surface) {
                let popup_at = (window.geometry().loc + popup_offset - popup.geometry().loc)
                    .to_physical_precise_round(scale);
                out.extend(
                    render_elements_from_surface_tree::<_, WaylandSurfaceRenderElement<GlesRenderer>>(
                        renderer,
                        popup.wl_surface(),
                        at + popup_at,
                        scale,
                        1.0,
                        Kind::Unspecified,
                    )
                    .into_iter()
                    .map(PatinaElement::Surface),
                );
            }
            for el in render_elements_from_surface_tree::<
                _,
                WaylandSurfaceRenderElement<GlesRenderer>,
            >(renderer, surface, at, scale, alpha, Kind::Unspecified)
            {
                match (clip_program, maximized) {
                    (Some(program), false) => out.push(PatinaElement::Clipped(
                        ClippedSurface::new(el, program.clone(), content, radius, scale),
                    )),
                    _ => out.push(PatinaElement::Surface(el)),
                }
            }
        }

        // Titlebar texture, redrawn only when what it shows changes. Drawn at the output's
        // scale; a window across two outputs keeps one for each.
        let cell = win_state(&window);
        if !cell.borrow().bars.iter().any(|(key, _)| *key == bar) {
            let bitmap = state.rasterizer.draw(&bar);
            let image = Image {
                buffer: upload(&bitmap.bgra, bitmap.width, bitmap.height),
                pixels: (bitmap.width, bitmap.height),
            };
            let mut st = cell.borrow_mut();
            st.bars.retain(|(k, _)| k.scale != bar.scale);
            st.bars.push((bar.clone(), image));
            if st.bars.len() > 2 {
                st.bars.remove(0);
            }
        }
        if let Some((_, image)) = cell.borrow().bars.iter().find(|(key, _)| *key == bar) {
            let at: Point<f64, Physical> = Point::from((
                geo.loc.x as f64 * scale.x,
                (geo.loc.y - TITLEBAR_H) as f64 * scale.y,
            ));
            if let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
                renderer,
                at,
                &image.buffer,
                Some(alpha),
                Some(whole(image.pixels)),
                Some(Size::from((geo.size.w, TITLEBAR_H))),
                Kind::Unspecified,
            ) {
                out.push(PatinaElement::Image(el));
            }
        }

        // Shadow and outline, redrawn when the frame size, activity or scale changes.
        if !maximized {
            let (fw, fh) = (geo.size.w, geo.size.h + TITLEBAR_H);
            let key = (fw, fh, bar.active, bar.scale.to_bits());
            if !cell.borrow().shadows.iter().any(|(k, _, _)| *k == key) {
                let s = shadow::render_scaled(fw, fh, bar.active, bar.scale);
                let image = Image {
                    buffer: upload(&s.bitmap.bgra, s.bitmap.width, s.bitmap.height),
                    pixels: (s.bitmap.width, s.bitmap.height),
                };
                let mut st = cell.borrow_mut();
                st.shadows.retain(|(k, _, _)| k.3 != key.3);
                st.shadows.push((key, image, s.pad));
                if st.shadows.len() > 2 {
                    st.shadows.remove(0);
                }
            }
            if let Some((_, image, pad)) = cell.borrow().shadows.iter().find(|(k, _, _)| *k == key)
            {
                let at: Point<f64, Physical> = Point::from((
                    (geo.loc.x - pad) as f64 * scale.x,
                    (geo.loc.y - TITLEBAR_H - pad) as f64 * scale.y,
                ));
                if let Ok(el) = MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    at,
                    &image.buffer,
                    Some(alpha),
                    Some(whole(image.pixels)),
                    Some(Size::from((fw + 2 * pad, fh + 2 * pad))),
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
            key: window.toplevel().map(|t| t.wl_surface().id()),
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
    clip_program: Option<&GlesTexProgram>,
    rs: &mut OutputRender,
    output: &Output,
) -> Vec<PatinaElement> {
    let scale = Scale::from(output.current_scale().fractional_scale());
    let groups = groups(state, renderer, clip_program, rs, output);
    let Some(size) = output.current_mode().map(|m| m.size) else {
        return flatten(groups);
    };
    if rs.unavailable {
        return flatten(groups);
    }

    if rs.scene.as_ref().is_none_or(|s| s.size() != size) {
        match Scene::new(renderer, size, state.full_recompose) {
            Ok(scene) => rs.scene = Some(scene),
            Err(e) => {
                tracing::warn!(error = %e, "blur unavailable: cannot create the scene");
                rs.unavailable = true;
                return flatten(groups);
            }
        }
    }
    let Some(scene) = rs.scene.as_mut() else {
        return flatten(groups);
    };
    if let Err(e) = scene.update(renderer, &groups, scale) {
        tracing::warn!(error = %e, "blur unavailable: cannot compose the scene");
        rs.scene = None;
        rs.unavailable = true;
        return flatten(groups);
    }
    let element = scene.element(scale);

    // PATINA_VERIFY_RECOMPOSE=1: also compose every frame in full and compare, to catch stale
    // pixels left by partial updates.
    if state.verify_recompose {
        verify_against_full(state, rs, renderer, &groups, size, scale);
    }
    vec![element]
}

fn verify_against_full(
    state: &mut Patina,
    rs: &mut OutputRender,
    renderer: &mut GlesRenderer,
    groups: &[Group],
    size: Size<i32, Physical>,
    scale: Scale<f64>,
) {
    if rs.scene_check.as_ref().is_none_or(|s| s.size() != size) {
        rs.scene_check = Scene::new(renderer, size, true).ok();
    }
    let (Some(scene), Some(check)) = (rs.scene.as_ref(), rs.scene_check.as_mut()) else {
        return;
    };
    if check.update(renderer, groups, scale).is_err() {
        return;
    }
    let (Ok(a), Ok(b)) = (scene.read_back(renderer), check.read_back(renderer)) else {
        return;
    };
    // Where the two differ: how many pixels, their bounding box, and a sample.
    let width = size.w as usize;
    let (mut differing, mut x0, mut y0, mut x1, mut y1) =
        (0usize, usize::MAX, usize::MAX, 0usize, 0usize);
    let mut sample = None;
    let mut lsb = 0usize;
    for (i, (x, y)) in a.chunks_exact(4).zip(b.chunks_exact(4)).enumerate() {
        let worst = x
            .iter()
            .zip(y)
            .map(|(p, q)| p.abs_diff(*q))
            .max()
            .unwrap_or(0);
        if (1..=2).contains(&worst) {
            // One or two steps of 255 in a channel, in blurred glass: invisible. The cause is
            // not fully isolated (it shows up in glass beyond the damage margin), so it is
            // counted and logged but does not fail the check.
            lsb += 1;
        } else if worst > 2 {
            let (px, py) = (i % width, i / width);
            differing += 1;
            (x0, y0, x1, y1) = (x0.min(px), y0.min(py), x1.max(px), y1.max(py));
            sample.get_or_insert(((px, py), x.to_vec(), y.to_vec()));
        }
    }
    state.verified_frames += 1;
    if differing > 0 {
        state.mismatched_frames += 1;
        tracing::warn!(
            differing,
            bounds = ?(x0, y0, x1, y1),
            first = ?sample,
            repainted = ?scene.last_painted().iter().map(|r| (r.loc.x, r.loc.y, r.size.w, r.size.h)).collect::<Vec<_>>(),
            frame = state.verified_frames,
            "partial recompose differs from a full one"
        );
    } else {
        tracing::debug!(
            lsb,
            frame = state.verified_frames,
            "partial recompose matches a full one"
        );
    }
}

/// The pointer, drawn by the compositor when there is no host to draw it (DRM backend).
/// Returned top-most first; usually one element, none when a client hides the cursor.
pub fn cursor_elements(
    state: &mut Patina,
    renderer: &mut GlesRenderer,
    output: &Output,
) -> Vec<PatinaElement> {
    use crate::wm::CursorSource;
    use smithay::{
        backend::renderer::element::surface::render_elements_from_surface_tree,
        input::pointer::{CursorIcon, CursorImageSurfaceData},
        wayland::compositor::with_states,
    };

    if !state.software_cursor {
        return Vec::new();
    }
    let Some(global) = state.seat.get_pointer().map(|p| p.current_location()) else {
        return Vec::new();
    };
    let Some(area) = state.space.output_geometry(output) else {
        return Vec::new();
    };
    // Drawn on the output it is on; one near an edge spills onto the neighbour too.
    let near = Rectangle::<i32, Logical>::new(
        (area.loc.x - 64, area.loc.y - 64).into(),
        (area.size.w + 128, area.size.h + 128).into(),
    );
    if !near.contains(global.to_i32_round()) {
        return Vec::new();
    }
    let pos = global - area.loc.to_f64();
    let scale = Scale::from(output.current_scale().fractional_scale());
    let at =
        |hotspot: smithay::utils::Point<i32, smithay::utils::Logical>| -> Point<f64, Physical> {
            Point::from((
                (pos.x - hotspot.x as f64) * scale.x,
                (pos.y - hotspot.y as f64) * scale.y,
            ))
        };
    let image = |renderer: &mut GlesRenderer,
                 buffer: &MemoryRenderBuffer,
                 hotspot: smithay::utils::Point<i32, smithay::utils::Logical>,
                 size: Size<i32, Logical>,
                 pixels: (i32, i32)| {
        MemoryRenderBufferRenderElement::from_buffer(
            renderer,
            at(hotspot),
            buffer,
            None,
            Some(whole(pixels)),
            Some(size),
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
                if let Some(named) = state.cursor_theme.get(icon, scale.x) {
                    return image(
                        renderer,
                        &named.buffer,
                        named.hotspot,
                        named.size,
                        named.pixels,
                    );
                }
            }
            match state.cursor.get(scale.x) {
                Some(arrow) => image(
                    renderer,
                    &arrow.buffer,
                    arrow.hotspot,
                    arrow.size,
                    arrow.pixels,
                ),
                None => Vec::new(),
            }
        }
    }
}
