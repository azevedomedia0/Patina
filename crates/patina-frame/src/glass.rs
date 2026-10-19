//! The "glass" effect behind translucent surfaces, read from the design tokens
//! `--blur-glass` (windows) and `--blur-panel` (the panel).

use patina_tokens as tok;

/// A backdrop blur: CSS `blur(<sigma>px) saturate(<saturation>)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glass {
    /// Standard deviation of the blur in pixels.
    pub sigma: f32,
    /// 1.0 leaves colours alone.
    pub saturation: f32,
}

/// Parses `blur(Npx)` with an optional `saturate(X)`. Anything else gives `None`.
pub fn parse(css: &str) -> Option<Glass> {
    let arg = |name: &str| -> Option<&str> {
        let rest = css.split_once(&format!("{name}("))?.1;
        Some(rest.split_once(')')?.0.trim())
    };
    let sigma = arg("blur")?
        .strip_suffix("px")?
        .trim()
        .parse::<f32>()
        .ok()?;
    let saturation = match arg("saturate") {
        Some(v) => v.parse::<f32>().ok()?,
        None => 1.0,
    };
    Some(Glass { sigma, saturation })
}

/// Windows, launcher and popovers (`--blur-glass`).
pub fn window() -> Glass {
    parse(tok::BLUR_GLASS).unwrap_or(Glass {
        sigma: 24.0,
        saturation: 1.4,
    })
}

/// The top panel (`--blur-panel`).
pub fn panel() -> Glass {
    parse(tok::BLUR_PANEL).unwrap_or(Glass {
        sigma: 16.0,
        saturation: 1.0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_design_tokens() {
        assert_eq!(
            parse(tok::BLUR_GLASS),
            Some(Glass {
                sigma: 24.0,
                saturation: 1.4
            })
        );
        assert_eq!(
            parse(tok::BLUR_PANEL),
            Some(Glass {
                sigma: 16.0,
                saturation: 1.0
            })
        );
    }

    #[test]
    fn matches_the_documented_values() {
        assert_eq!(
            window(),
            Glass {
                sigma: 24.0,
                saturation: 1.4
            }
        );
        assert_eq!(
            panel(),
            Glass {
                sigma: 16.0,
                saturation: 1.0
            }
        );
    }

    #[test]
    fn rejects_other_filters() {
        assert_eq!(parse("brightness(1.2)"), None);
        assert_eq!(parse("blur(2em)"), None);
        assert_eq!(parse(""), None);
    }

    #[test]
    fn saturate_is_optional_and_order_does_not_matter() {
        assert_eq!(
            parse("blur(8px)"),
            Some(Glass {
                sigma: 8.0,
                saturation: 1.0
            })
        );
        assert_eq!(
            parse("saturate(2) blur(8px)"),
            Some(Glass {
                sigma: 8.0,
                saturation: 2.0
            })
        );
    }
}
