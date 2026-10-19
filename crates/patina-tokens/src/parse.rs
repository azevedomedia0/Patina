//! Parser for the Patina design tokens in `design/tokens/*.css`.
//!
//! Only custom properties declared in a plain selector block (for example
//! `:root { --ink-950: #0d1016; }`) are read. `var(--name)` references are
//! expanded so every token ends up with a literal value.
//!
//! This file is shared: `build.rs` includes it through `#[path]` to generate
//! Rust constants, and the library re-exports it for runtime tooling.

use std::collections::BTreeMap;

/// A single `--name: value` declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// Name without the leading `--`, for example `ink-950`.
    pub name: String,
    /// Raw declared value, possibly containing `var()` references.
    pub value: String,
}

/// An sRGB colour with alpha in the range 0.0..=1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: f32,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: f32) -> Self {
        Self { r, g, b, a }
    }
}

/// Removes `/* ... */` comments. An unterminated comment runs to the end.
pub fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => rest = "",
        }
    }
    out.push_str(rest);
    out
}

/// Returns the declarations of every block whose selector is exactly `selector`,
/// in source order. Selectors are compared after trimming whitespace.
pub fn declarations_in(css: &str, selector: &str) -> Vec<Token> {
    let css = strip_comments(css);
    let mut tokens = Vec::new();
    let mut rest = css.as_str();
    while let Some(open) = rest.find('{') {
        // The selector starts after the previous block or at-rule (`}` or `;`).
        let head = rest[..open].rsplit(['}', ';']).next().unwrap_or("").trim();
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        if head == selector {
            tokens.extend(parse_block(&after[..close]));
        }
        rest = &after[close + 1..];
    }
    tokens
}

fn parse_block(body: &str) -> Vec<Token> {
    body.split(';')
        .filter_map(|decl| {
            let decl = decl.trim();
            if !decl.starts_with("--") {
                return None;
            }
            let (name, value) = decl.split_once(':')?;
            Some(Token {
                name: name.trim().trim_start_matches("--").to_string(),
                value: value.trim().to_string(),
            })
        })
        .collect()
}

/// Expands `var(--name)` references using `tokens`. Fails on unknown names or cycles.
pub fn resolve(tokens: &[Token]) -> Result<Vec<Token>, String> {
    let map: BTreeMap<&str, &str> = tokens
        .iter()
        .map(|t| (t.name.as_str(), t.value.as_str()))
        .collect();
    tokens
        .iter()
        .map(|t| {
            Ok(Token {
                name: t.name.clone(),
                value: expand(&t.value, &map, 0)?,
            })
        })
        .collect()
}

fn expand(value: &str, map: &BTreeMap<&str, &str>, depth: usize) -> Result<String, String> {
    if depth > 8 {
        return Err(format!("var() cycle while expanding `{value}`"));
    }
    let mut out = String::new();
    let mut rest = value;
    while let Some(i) = rest.find("var(--") {
        out.push_str(&rest[..i]);
        let after = &rest[i + "var(".len()..];
        let end = after
            .find(')')
            .ok_or_else(|| format!("unclosed var() in `{value}`"))?;
        let name = after[..end]
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches("--");
        let target = map
            .get(name)
            .ok_or_else(|| format!("undefined token `--{name}` referenced from `{value}`"))?;
        out.push_str(&expand(target, map, depth + 1)?);
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// Parses `#rgb`, `#rrggbb`, `#rrggbbaa`, `rgb(...)` or `rgba(...)`.
pub fn parse_color(value: &str) -> Option<Color> {
    let v = value.trim();
    if let Some(hex) = v.strip_prefix('#') {
        let nib = |c: char| c.to_digit(16).map(|d| d as u8);
        let chars: Vec<char> = hex.chars().collect();
        return match chars.len() {
            3 => {
                let [r, g, b] = [nib(chars[0])?, nib(chars[1])?, nib(chars[2])?];
                Some(Color::new(r * 17, g * 17, b * 17, 1.0))
            }
            6 | 8 => {
                let byte =
                    |i: usize| -> Option<u8> { Some(nib(chars[i])? * 16 + nib(chars[i + 1])?) };
                let a = if chars.len() == 8 {
                    byte(6)? as f32 / 255.0
                } else {
                    1.0
                };
                Some(Color::new(byte(0)?, byte(2)?, byte(4)?, a))
            }
            _ => None,
        };
    }
    let inner = v
        .strip_prefix("rgba(")
        .or_else(|| v.strip_prefix("rgb("))?
        .strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
    if !(parts.len() == 3 || parts.len() == 4) {
        return None;
    }
    let channel = |s: &str| s.parse::<u8>().ok();
    let a = match parts.get(3) {
        Some(s) => s.parse::<f32>().ok()?,
        None => 1.0,
    };
    Some(Color::new(
        channel(parts[0])?,
        channel(parts[1])?,
        channel(parts[2])?,
        a,
    ))
}

/// Parses a plain number with an optional `px` suffix, for example `36px`, `1.45`.
pub fn parse_number(value: &str) -> Option<f32> {
    let v = value.trim();
    let v = v.strip_suffix("px").unwrap_or(v);
    if v.is_empty()
        || !v
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    {
        return None;
    }
    v.parse::<f32>().ok()
}

/// Parses a duration in milliseconds, for example `120ms`.
pub fn parse_ms(value: &str) -> Option<u64> {
    let digits = value.trim().strip_suffix("ms")?;
    digits.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_comments() {
        assert_eq!(strip_comments("a/* x */b"), "ab");
        assert_eq!(strip_comments("a/* open"), "a");
    }

    #[test]
    fn reads_only_the_requested_selector() {
        let css = ":root{--a:#fff;}[data-theme=\"light\"]{--a:#000;}";
        let root = declarations_in(css, ":root");
        assert_eq!(root.len(), 1);
        assert_eq!(root[0].value, "#fff");
        let light = declarations_in(css, "[data-theme=\"light\"]");
        assert_eq!(light[0].value, "#000");
    }

    #[test]
    fn ignores_a_leading_at_import() {
        let css = "@import url('https://example.com/a.css');:root{--a:1;}";
        assert_eq!(declarations_in(css, ":root").len(), 1);
    }

    #[test]
    fn keeps_commas_and_parens_inside_values() {
        let css = ":root{--s:0 24px 48px rgba(0,0,0,.6),0 0 0 1px #fff;}";
        let d = declarations_in(css, ":root");
        assert_eq!(d[0].value, "0 24px 48px rgba(0,0,0,.6),0 0 0 1px #fff");
    }

    #[test]
    fn resolves_nested_references() {
        let css = ":root{--a:#2f7de1;--b:var(--a);--c:var(--b);}";
        let resolved = resolve(&declarations_in(css, ":root")).unwrap();
        assert_eq!(resolved[2].value, "#2f7de1");
    }

    #[test]
    fn reports_unknown_reference() {
        let css = ":root{--b:var(--missing);}";
        assert!(resolve(&declarations_in(css, ":root")).is_err());
    }

    #[test]
    fn parses_colours() {
        assert_eq!(
            parse_color("#1a1f29"),
            Some(Color::new(0x1a, 0x1f, 0x29, 1.0))
        );
        assert_eq!(parse_color("#fff"), Some(Color::new(255, 255, 255, 1.0)));
        assert_eq!(
            parse_color("rgba(26,31,41,.78)"),
            Some(Color::new(26, 31, 41, 0.78))
        );
        assert_eq!(parse_color("transparent"), None);
        assert_eq!(parse_color("radial-gradient(#fff, #000)"), None);
    }

    #[test]
    fn parses_numbers_and_durations() {
        assert_eq!(parse_number("36px"), Some(36.0));
        assert_eq!(parse_number("1.45"), Some(1.45));
        assert_eq!(parse_number("22%"), None);
        assert_eq!(parse_ms("180ms"), Some(180));
        assert_eq!(parse_ms("180"), None);
    }
}
