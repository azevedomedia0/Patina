//! App icons from the icon theme (freedesktop Icon Theme Specification), rendered to RGBA.
//!
//! The theme is the user's GTK icon theme (`~/.config/gtk-3.0/settings.ini`) if set, else
//! Adwaita, then its parents and `hicolor`, then `/usr/share/pixmaps`. PNG and SVG icons are
//! supported; XPM is not.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

/// An icon ready to draw: `width * height` straight (not premultiplied) RGBA pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct Rgba {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum DirType {
    Fixed,
    Scalable,
    Threshold,
}

#[derive(Debug, Clone)]
struct Dir {
    path: String,
    size: u32,
    scale: u32,
    kind: DirType,
    min: u32,
    max: u32,
    threshold: u32,
}

impl Dir {
    fn matches(&self, size: u32, scale: u32) -> bool {
        if self.scale != scale {
            return false;
        }
        match self.kind {
            DirType::Fixed => self.size == size,
            DirType::Scalable => (self.min..=self.max).contains(&size),
            DirType::Threshold => {
                size + self.threshold >= self.size && size <= self.size + self.threshold
            }
        }
    }

    fn distance(&self, size: u32, scale: u32) -> u32 {
        let want = size * scale;
        let (lo, hi) = match self.kind {
            DirType::Fixed => (self.size, self.size),
            DirType::Scalable => (self.min, self.max),
            DirType::Threshold => (
                self.size.saturating_sub(self.threshold),
                self.size + self.threshold,
            ),
        };
        let (lo, hi) = (lo * self.scale, hi * self.scale);
        if want < lo {
            lo - want
        } else {
            want.saturating_sub(hi)
        }
    }
}

#[derive(Debug, Clone, Default)]
struct Theme {
    dirs: Vec<Dir>,
    inherits: Vec<String>,
}

/// Parses an `index.theme`.
fn parse_index(text: &str) -> Theme {
    let mut sections: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].to_string();
        } else if let Some((k, v)) = line.split_once('=') {
            sections
                .entry(current.clone())
                .or_default()
                .insert(k.trim().into(), v.trim().into());
        }
    }
    let head = sections.get("Icon Theme").cloned().unwrap_or_default();
    let list = |k: &str| -> Vec<String> {
        head.get(k)
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut names = list("Directories");
    names.extend(list("ScaledDirectories"));
    let dirs = names
        .into_iter()
        .filter_map(|name| {
            let s = sections.get(&name)?;
            let num = |k: &str| s.get(k).and_then(|v| v.parse::<u32>().ok());
            let size = num("Size")?;
            let kind = match s.get("Type").map(String::as_str) {
                Some("Fixed") => DirType::Fixed,
                Some("Scalable") => DirType::Scalable,
                _ => DirType::Threshold,
            };
            Some(Dir {
                path: name,
                size,
                scale: num("Scale").unwrap_or(1),
                kind,
                min: num("MinSize").unwrap_or(size),
                max: num("MaxSize").unwrap_or(size),
                threshold: num("Threshold").unwrap_or(2),
            })
        })
        .collect();
    Theme {
        dirs,
        inherits: list("Inherits"),
    }
}

/// Where icon themes live: `~/.icons`, then `icons/` in every XDG data directory.
fn base_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        dirs.push(PathBuf::from(home).join(".icons"));
    }
    dirs.extend(crate::data_dirs().into_iter().map(|d| d.join("icons")));
    dirs
}

/// The user's GTK icon theme name, if set.
fn gtk_theme() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    let path = PathBuf::from(home).join(".config/gtk-3.0/settings.ini");
    let text = std::fs::read_to_string(path).ok()?;
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == "gtk-icon-theme-name").then(|| v.trim().trim_matches('"').to_string())
    })
}

/// Finds icons by name in the theme chain; remembers parsed themes.
pub struct IconFinder {
    bases: Vec<PathBuf>,
    chain: Vec<String>,
    themes: HashMap<String, Theme>,
}

impl Default for IconFinder {
    fn default() -> Self {
        Self::new()
    }
}

impl IconFinder {
    pub fn new() -> Self {
        let bases = base_dirs();
        let first = std::env::var("PATINA_ICON_THEME")
            .ok()
            .or_else(gtk_theme)
            .unwrap_or_else(|| "Adwaita".into());
        let mut finder = Self {
            bases,
            chain: Vec::new(),
            themes: HashMap::new(),
        };
        let mut todo = vec![first];
        while let Some(name) = todo.pop() {
            if finder.chain.contains(&name) {
                continue;
            }
            let theme = finder.load(&name);
            finder.chain.push(name.clone());
            todo.extend(theme.inherits.iter().rev().cloned());
        }
        if !finder.chain.iter().any(|t| t == "hicolor") {
            finder.load("hicolor");
            finder.chain.push("hicolor".into());
        }
        finder
    }

    fn load(&mut self, name: &str) -> Theme {
        if let Some(t) = self.themes.get(name) {
            return t.clone();
        }
        let theme = self
            .bases
            .iter()
            .find_map(|b| std::fs::read_to_string(b.join(name).join("index.theme")).ok())
            .map(|t| parse_index(&t))
            .unwrap_or_default();
        self.themes.insert(name.into(), theme.clone());
        theme
    }

    /// The file for icon `name` at `size` logical pixels and integer `scale`.
    pub fn find(&self, name: &str, size: u32, scale: u32) -> Option<PathBuf> {
        if name.starts_with('/') {
            return Path::new(name).is_file().then(|| name.into());
        }
        let exts = ["png", "svg"];
        for theme_name in &self.chain {
            let Some(theme) = self.themes.get(theme_name) else {
                continue;
            };
            let file = |dir: &Dir| {
                self.bases.iter().find_map(|b| {
                    exts.iter().find_map(|e| {
                        let p = b
                            .join(theme_name)
                            .join(&dir.path)
                            .join(format!("{name}.{e}"));
                        p.is_file().then_some(p)
                    })
                })
            };
            if let Some(p) = theme
                .dirs
                .iter()
                .filter(|d| d.matches(size, scale))
                .find_map(file)
            {
                return Some(p);
            }
            let mut by_distance: Vec<&Dir> = theme.dirs.iter().collect();
            by_distance.sort_by_key(|d| d.distance(size, scale));
            if let Some(p) = by_distance.into_iter().find_map(file) {
                return Some(p);
            }
        }
        ["/usr/share/pixmaps"].iter().find_map(|d| {
            exts.iter().find_map(|e| {
                let p = Path::new(d).join(format!("{name}.{e}"));
                p.is_file().then_some(p)
            })
        })
    }

    /// Icon `name` rendered to a `px` by `px` square, keeping its aspect ratio.
    pub fn load_rgba(&self, name: &str, px: u32, scale: u32) -> Option<Rgba> {
        let path = self.find(name, px / scale.max(1), scale.max(1))?;
        render_file(&path, px)
    }
}

/// Renders an icon file (PNG or SVG) to a `px` by `px` square.
pub fn render_file(path: &Path, px: u32) -> Option<Rgba> {
    let data = std::fs::read(path).ok()?;
    let is_svg = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("svg"));
    if is_svg {
        render_svg(&data, px)
    } else {
        render_png(&data, px)
    }
}

pub fn render_svg(data: &[u8], px: u32) -> Option<Rgba> {
    let tree = resvg::usvg::Tree::from_data(data, &resvg::usvg::Options::default()).ok()?;
    let size = tree.size();
    let k = px as f32 / size.width().max(size.height());
    let mut pixmap = tiny_skia::Pixmap::new(px, px)?;
    let dx = (px as f32 - size.width() * k) / 2.0;
    let dy = (px as f32 - size.height() * k) / 2.0;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(k, k).post_translate(dx, dy),
        &mut pixmap.as_mut(),
    );
    Some(demultiply(pixmap))
}

pub fn render_png(data: &[u8], px: u32) -> Option<Rgba> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(data));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let (w, h) = (info.width, info.height);
    // To premultiplied RGBA, as tiny-skia wants.
    let mut rgba = Vec::with_capacity((w * h * 4) as usize);
    for px in buf[..info.buffer_size()].chunks(info.color_type.samples()) {
        let (r, g, b, a) = match px.len() {
            1 => (px[0], px[0], px[0], 255),
            2 => (px[0], px[0], px[0], px[1]),
            3 => (px[0], px[1], px[2], 255),
            _ => (px[0], px[1], px[2], px[3]),
        };
        let m = |c: u8| ((c as u16 * a as u16 + 127) / 255) as u8;
        rgba.extend_from_slice(&[m(r), m(g), m(b), a]);
    }
    let src = tiny_skia::Pixmap::from_vec(rgba, tiny_skia::IntSize::from_wh(w, h)?)?;
    let k = px as f32 / w.max(h) as f32;
    let mut dst = tiny_skia::Pixmap::new(px, px)?;
    dst.draw_pixmap(
        ((px as f32 - w as f32 * k) / 2.0 / k) as i32,
        ((px as f32 - h as f32 * k) / 2.0 / k) as i32,
        src.as_ref(),
        &tiny_skia::PixmapPaint {
            quality: tiny_skia::FilterQuality::Bicubic,
            ..Default::default()
        },
        tiny_skia::Transform::from_scale(k, k),
        None,
    );
    Some(demultiply(dst))
}

fn demultiply(pixmap: tiny_skia::Pixmap) -> Rgba {
    let (width, height) = (pixmap.width(), pixmap.height());
    let pixels = pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    Rgba {
        width,
        height,
        pixels,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX: &str = "[Icon Theme]\nName=Test\nInherits=hicolor\nDirectories=48x48/apps,scalable/apps,16x16/apps\n\n[48x48/apps]\nSize=48\nType=Fixed\n\n[scalable/apps]\nSize=128\nMinSize=8\nMaxSize=512\nType=Scalable\n\n[16x16/apps]\nSize=16\n";

    #[test]
    fn parses_theme_directories() {
        let t = parse_index(INDEX);
        assert_eq!(t.inherits, vec!["hicolor"]);
        assert_eq!(t.dirs.len(), 3);
        assert!(t.dirs[0].matches(48, 1));
        assert!(!t.dirs[0].matches(56, 1));
        assert!(t.dirs[1].matches(56, 1));
        assert!(
            t.dirs[2].matches(18, 1),
            "threshold dirs allow 2 px either way"
        );
    }

    #[test]
    fn nearest_size_wins_when_nothing_matches() {
        let t = parse_index(INDEX);
        let mut d: Vec<&Dir> = t
            .dirs
            .iter()
            .filter(|d| d.kind != DirType::Scalable)
            .collect();
        d.sort_by_key(|d| d.distance(40, 1));
        assert_eq!(d[0].size, 48);
    }

    #[test]
    fn renders_svg_to_the_requested_size() {
        let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/></svg>"##;
        let icon = render_svg(svg, 32).unwrap();
        assert_eq!((icon.width, icon.height), (32, 32));
        assert_eq!(&icon.pixels[0..4], &[255, 0, 0, 255]);
    }
}
