//! Composes the frame into an offscreen texture, bottom to top, so that glass surfaces can
//! blur what is beneath them.
//!
//! A plain element list cannot do this: blurring needs the pixels already drawn below. The scene
//! is handed to the output as one texture, and it is only repainted where something changed:
//! what changed, plus what a glass surface above it shows of that area through its blur.

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
            Bind, Color32F, ExportMem, Frame, Offscreen, Renderer,
        },
    },
    reexports::wayland_server::backend::ObjectId,
    utils::{Buffer, Logical, Physical, Rectangle, Scale, Size, Transform},
};

use crate::{
    blur::{Backdrop, Blur},
    render::{Group, PatinaElement},
};

/// What identifies an element's appearance from one frame to the next.
#[derive(Debug, Clone, PartialEq)]
struct ElementSig {
    id: Id,
    commit: CommitCounter,
    geometry: Rectangle<i32, Physical>,
    alpha: u32,
}

#[derive(Debug, Clone, PartialEq)]
struct GroupSig {
    key: Option<ObjectId>,
    elements: Vec<ElementSig>,
    backdrop: Option<Backdrop>,
}

fn signatures(groups: &[Group], scale: Scale<f64>) -> Vec<GroupSig> {
    groups
        .iter()
        .map(|g| GroupSig {
            key: g.key.clone(),
            elements: g
                .elements
                .iter()
                .map(|e| ElementSig {
                    id: e.id().clone(),
                    commit: e.current_commit(),
                    geometry: e.geometry(scale),
                    alpha: e.alpha().to_bits(),
                })
                .collect(),
            backdrop: g.backdrop,
        })
        .collect()
}

#[derive(Debug, PartialEq)]
enum Damage {
    /// Nothing visible changed.
    None,
    /// Everything must be repainted, and why.
    Full(&'static str),
    /// Only these disjoint rectangles.
    Rects(Vec<Rectangle<i32, Physical>>),
}

fn grow(r: Rectangle<i32, Physical>, by: i32) -> Rectangle<i32, Physical> {
    Rectangle::new(
        (r.loc.x - by, r.loc.y - by).into(),
        (r.size.w + 2 * by, r.size.h + 2 * by).into(),
    )
}

/// Merges overlapping rectangles into their bounding boxes until none overlap. Disjoint
/// rectangles matter: repainting an overlap twice would blend translucent surfaces twice.
fn disjoint(mut rects: Vec<Rectangle<i32, Physical>>) -> Vec<Rectangle<i32, Physical>> {
    rects.retain(|r| !r.is_empty());
    'again: loop {
        for i in 0..rects.len() {
            for j in i + 1..rects.len() {
                if rects[i].overlaps(rects[j]) {
                    let merged = rects[i].merge(rects[j]);
                    rects.swap_remove(j);
                    rects[i] = merged;
                    continue 'again;
                }
            }
        }
        // A fixed order, so two equal sets of rectangles compare equal.
        rects.sort_by_key(|r| (r.loc.x, r.loc.y, r.size.w, r.size.h));
        return rects;
    }
}

/// Collects what changed in `now` since `prev`.
fn damage(
    prev: &[GroupSig],
    now: &[GroupSig],
    groups: &[Group],
    scale: Scale<f64>,
    screen: Rectangle<i32, Physical>,
) -> Damage {
    // A window appearing, disappearing or changing stacking position: repaint it all.
    if prev.len() != now.len() || prev.iter().zip(now).any(|(p, n)| p.key != n.key) {
        return Damage::Full("windows or stacking changed");
    }

    // What changed by itself, group by group.
    let mut own: Vec<Vec<Rectangle<i32, Physical>>> = vec![Vec::new(); now.len()];
    for (i, (p, n)) in prev.iter().zip(now).enumerate() {
        let d = &mut own[i];
        if p.backdrop != n.backdrop {
            d.extend(p.backdrop.map(|b| b.rect));
            d.extend(n.backdrop.map(|b| b.rect));
        }

        for (k, e) in n.elements.iter().enumerate() {
            match p.elements.iter().find(|pe| pe.id == e.id) {
                None => d.push(e.geometry),
                Some(pe) if pe.geometry != e.geometry || pe.alpha != e.alpha => {
                    d.push(pe.geometry);
                    d.push(e.geometry);
                }
                Some(pe) if pe.commit != e.commit => {
                    // The element says which parts of it changed, relative to its own corner.
                    let el = &groups[i].elements[k];
                    d.extend(
                        el.damage_since(scale, Some(pe.commit))
                            .into_iter()
                            .map(|r| Rectangle::new(r.loc + e.geometry.loc, r.size)),
                    );
                }
                Some(_) => {}
            }
        }
        d.extend(
            p.elements
                .iter()
                .filter(|pe| !n.elements.iter().any(|e| e.id == pe.id))
                .map(|pe| pe.geometry),
        );
    }

    // Glass shows what is below it through a blur. When something below changes, the part of the
    // glass within the blur's reach changes too, and that in turn is below any glass above it. So
    // go up the stack carrying the changes along. (Each glass blurs its own snapshot of what is
    // below it, so nothing beyond these changes needs repainting.)
    let mut changed: Vec<Rectangle<i32, Physical>> = Vec::new();
    let mut total: Vec<Rectangle<i32, Physical>> = Vec::new();
    for (i, n) in now.iter().enumerate() {
        if let Some(b) = &n.backdrop {
            let reach = b.reach();
            let seen: Vec<_> = changed
                .iter()
                .filter_map(|r| grow(*r, reach).intersection(b.rect))
                .collect();
            changed.extend(seen);
        }
        changed.extend(own[i].iter().copied());
        changed = disjoint(
            changed
                .into_iter()
                .filter_map(|r| r.intersection(screen))
                .collect(),
        );
        total = changed.clone();
    }
    let d = total;
    tracing::trace!(
        own = ?own.iter().map(|g| g.iter().map(|r| (r.loc.x, r.loc.y, r.size.w, r.size.h)).collect::<Vec<_>>()).collect::<Vec<_>>(),
        merged = ?d.iter().map(|r| (r.loc.x, r.loc.y, r.size.w, r.size.h)).collect::<Vec<_>>(),
        "damage inputs"
    );

    let area: i64 = d.iter().map(|r| r.size.w as i64 * r.size.h as i64).sum();
    let screen_area = screen.size.w as i64 * screen.size.h as i64;
    if d.is_empty() {
        Damage::None
    } else if area * 10 > screen_area * 6 {
        // So much changed that clipping costs more than it saves.
        Damage::Full("more than 60% of the screen")
    } else if d.len() > 16 {
        Damage::Full("more than 16 separate areas")
    } else {
        Damage::Rects(d)
    }
}

pub struct Scene {
    blur: Blur,
    buffer: TextureRenderBuffer<GlesTexture>,
    /// The same texture the buffer draws into, kept to read it back when verifying.
    texture: GlesTexture,
    signature: Option<Vec<GroupSig>>,
    /// What the last update repainted, for the verifier's report.
    last_painted: Vec<Rectangle<i32, Physical>>,
    /// Repaint everything every time (`PATINA_FULL_RECOMPOSE=1`, and the verifier's reference).
    always_full: bool,
}

impl Scene {
    pub fn new(
        renderer: &mut GlesRenderer,
        size: Size<i32, Physical>,
        always_full: bool,
    ) -> Result<Self, GlesError> {
        let blur = Blur::new(renderer, size)?;
        let texture: GlesTexture =
            renderer.create_buffer(Fourcc::Abgr8888, (size.w, size.h).into())?;
        // Everything is drawn over an opaque wallpaper, so the scene is opaque.
        let full = Rectangle::<i32, Buffer>::from_size((size.w, size.h).into());
        let buffer = TextureRenderBuffer::from_texture(
            renderer,
            texture.clone(),
            1,
            Transform::Normal,
            Some(vec![full]),
        );
        Ok(Self {
            blur,
            buffer,
            texture,
            signature: None,
            last_painted: Vec::new(),
            always_full,
        })
    }

    pub fn size(&self) -> Size<i32, Physical> {
        self.blur.size()
    }

    /// Repaints the parts of the scene that changed. Groups run bottom to top.
    pub fn update(
        &mut self,
        renderer: &mut GlesRenderer,
        groups: &[Group],
        scale: Scale<f64>,
    ) -> Result<(), GlesError> {
        let started = std::time::Instant::now();
        let now = signatures(groups, scale);
        let size = self.blur.size();
        let screen = Rectangle::<i32, Physical>::from_size(size);

        // Every glass surface needs a snapshot of its own. A new one holds nothing valid, so the
        // scene must be painted in full once to fill it.
        let glass: Vec<_> = groups
            .iter()
            .filter_map(|g| g.key.clone().zip(g.backdrop))
            .collect();
        let renewed = self.blur.sync(renderer, &glass)?;

        let what = match &self.signature {
            _ if renewed => Damage::Full("glass surface appeared or changed"),
            Some(prev) if !self.always_full => damage(prev, &now, groups, scale, screen),
            _ => Damage::Full("first frame or forced"),
        };
        let why = match &what {
            Damage::Full(reason) => *reason,
            Damage::Rects(_) => "partial",
            Damage::None => "none",
        };
        let rects = match what {
            Damage::None => {
                self.signature = Some(now);
                return Ok(());
            }
            Damage::Full(_) => vec![screen],
            Damage::Rects(r) => r,
        };

        self.last_painted = rects.clone();
        let blur = &mut self.blur;
        let painted = rects.clone();
        self.buffer.render().draw(|scene| {
            {
                let mut target = renderer.bind(scene)?;
                let mut frame = renderer.render(&mut target, size, Transform::Normal)?;
                frame.clear(Color32F::BLACK, &painted)?;
                // Later passes run in the same GL context, so they already follow this one.
                let _ = frame.finish()?;
            }
            for group in groups {
                // Glass: copy what is below it, then draw that blurred in its place.
                if let (Some(key), Some(_)) = (&group.key, &group.backdrop) {
                    blur.snapshot(renderer, scene, key, &painted)?;
                    blur.apply(renderer, scene, key, &painted)?;
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
                    &painted,
                )?;
                let _ = frame.finish()?;
            }
            // Tell the output what changed, in the texture's own coordinates.
            Ok::<_, GlesError>(
                painted
                    .iter()
                    .map(|r| {
                        Rectangle::<i32, Buffer>::new(
                            (r.loc.x, r.loc.y).into(),
                            (r.size.w, r.size.h).into(),
                        )
                    })
                    .collect(),
            )
        })?;

        let area: i64 = rects
            .iter()
            .map(|r| r.size.w as i64 * r.size.h as i64)
            .sum();
        tracing::trace!(
            ms = started.elapsed().as_secs_f32() * 1000.0,
            rects = rects.len(),
            percent = (area * 100 / (size.w as i64 * size.h as i64).max(1)),
            reason = why,
            "scene composed"
        );
        self.signature = Some(now);
        Ok(())
    }

    /// The rectangles the last update repainted.
    pub fn last_painted(&self) -> &[Rectangle<i32, Physical>] {
        &self.last_painted
    }

    /// The composed scene as raw RGBA bytes, for checking it against a full repaint.
    pub fn read_back(&self, renderer: &mut GlesRenderer) -> Result<Vec<u8>, GlesError> {
        let size = self.blur.size();
        let region = Rectangle::<i32, Buffer>::from_size((size.w, size.h).into());
        let mapping = renderer.copy_texture(&self.texture, region, Fourcc::Abgr8888)?;
        Ok(renderer.map_texture(&mapping)?.to_vec())
    }

    /// The composed scene, as an element covering the whole output.
    /// The composed frame as one element covering the output. The texture is in device pixels,
    /// so it is given the output's logical size, which `scale` maps back to those pixels.
    pub fn element(&self, scale: Scale<f64>) -> PatinaElement {
        let size = self.size();
        let logical = Size::<i32, Logical>::from((
            (size.w as f64 / scale.x).round() as i32,
            (size.h as f64 / scale.y).round() as i32,
        ));
        PatinaElement::Scene(TextureRenderElement::from_texture_render_buffer(
            (0.0, 0.0),
            &self.buffer,
            None,
            // All of the texture, mapped onto the logical size.
            Some(Rectangle::from_size(Size::from((
                size.w as f64,
                size.h as f64,
            )))),
            Some(logical),
            Kind::Unspecified,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x: i32, y: i32, w: i32, h: i32) -> Rectangle<i32, Physical> {
        Rectangle::new((x, y).into(), (w, h).into())
    }

    #[test]
    fn disjoint_rectangles_stay_as_they_are() {
        let out = disjoint(vec![r(0, 0, 10, 10), r(20, 20, 5, 5)]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn overlapping_rectangles_become_their_bounding_box() {
        let out = disjoint(vec![r(0, 0, 10, 10), r(5, 5, 10, 10)]);
        assert_eq!(out, vec![r(0, 0, 15, 15)]);
    }

    #[test]
    fn merging_cascades_until_nothing_overlaps() {
        // a overlaps b, and the merged box then overlaps c.
        let out = disjoint(vec![r(0, 0, 10, 10), r(8, 0, 10, 10), r(16, 0, 10, 10)]);
        assert_eq!(out, vec![r(0, 0, 26, 10)]);
    }

    #[test]
    fn empty_rectangles_are_dropped() {
        assert!(disjoint(vec![r(3, 3, 0, 4), r(1, 1, 5, 0)]).is_empty());
    }

    #[test]
    fn touching_edges_are_not_overlap() {
        let out = disjoint(vec![r(0, 0, 10, 10), r(10, 0, 10, 10)]);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn growing_extends_every_side() {
        assert_eq!(grow(r(10, 10, 4, 4), 3), r(7, 7, 10, 10));
    }
}
