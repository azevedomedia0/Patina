//! Keyboard layout and repeat.
//!
//! The layout is the one the system is set up with, so a German keyboard types German from the
//! first login: `/etc/default/keyboard` (Debian, Ubuntu, Linux Mint) or the file `localectl`
//! writes (`/etc/X11/xorg.conf.d/00-keyboard.conf`, Fedora and Arch). `XKB_DEFAULT_LAYOUT` and
//! the other `XKB_DEFAULT_*` variables override both, as in other compositors. Choosing a
//! layout in Settings comes later; the prototype's Settings has a Layout control.

use smithay::input::keyboard::XkbConfig;

/// Repeat delay from the prototype's keyboard settings (`repeatDelay: 400`, in ms).
pub const REPEAT_DELAY_MS: i32 = 400;
/// Keys repeat this many times per second once repeating.
pub const REPEAT_RATE: i32 = 25;

/// An XKB rule set: model, layout, variant, options. Empty fields mean the xkbcommon default.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Xkb {
    pub model: String,
    pub layout: String,
    pub variant: String,
    pub options: String,
}

impl Xkb {
    /// What to use: empty (so xkbcommon reads `XKB_DEFAULT_*`) when any such variable is set,
    /// else the system's configuration, else xkbcommon's default (US).
    pub fn detect() -> Self {
        let env_set = ["MODEL", "LAYOUT", "VARIANT", "OPTIONS", "RULES"]
            .iter()
            .any(|k| std::env::var_os(format!("XKB_DEFAULT_{k}")).is_some());
        if env_set {
            return Self::default();
        }
        let read = |p: &str| std::fs::read_to_string(p).ok();
        read("/etc/default/keyboard")
            .map(|s| parse_default_keyboard(&s))
            .filter(|x| !x.layout.is_empty())
            .or_else(|| {
                read("/etc/X11/xorg.conf.d/00-keyboard.conf")
                    .map(|s| parse_xorg_conf(&s))
                    .filter(|x| !x.layout.is_empty())
            })
            .unwrap_or_default()
    }

    pub fn config(&self) -> XkbConfig<'_> {
        XkbConfig {
            rules: "",
            model: &self.model,
            layout: &self.layout,
            variant: &self.variant,
            options: if self.options.is_empty() {
                None
            } else {
                Some(self.options.clone())
            },
        }
    }
}

fn unquote(v: &str) -> String {
    v.trim().trim_matches('"').trim_matches('\'').to_string()
}

/// `/etc/default/keyboard`: shell assignments such as `XKBLAYOUT="de"`.
pub fn parse_default_keyboard(text: &str) -> Xkb {
    let mut x = Xkb::default();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = unquote(value);
        match key.trim() {
            "XKBMODEL" => x.model = value,
            "XKBLAYOUT" => x.layout = value,
            "XKBVARIANT" => x.variant = value,
            "XKBOPTIONS" => x.options = value,
            _ => {}
        }
    }
    x
}

/// `00-keyboard.conf`: `Option "XkbLayout" "de"` lines in an InputClass section.
pub fn parse_xorg_conf(text: &str) -> Xkb {
    let mut x = Xkb::default();
    for line in text.lines() {
        let words: Vec<&str> = line.split('"').collect();
        // Option "Name" "value" splits into ["Option ", "Name", " ", "value", ""].
        if !line.trim_start().starts_with("Option") || words.len() < 4 {
            continue;
        }
        let value = words[3].to_string();
        match words[1] {
            "XkbModel" => x.model = value,
            "XkbLayout" => x.layout = value,
            "XkbVariant" => x.variant = value,
            "XkbOptions" => x.options = value,
            _ => {}
        }
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_debian_keyboard_settings() {
        let x = parse_default_keyboard(
            "# KEYBOARD CONFIGURATION FILE\nXKBMODEL=\"pc105\"\nXKBLAYOUT=\"de\"\nXKBVARIANT=\"nodeadkeys\"\nXKBOPTIONS=\"\"\n\nBACKSPACE=\"guess\"\n",
        );
        assert_eq!(
            x,
            Xkb {
                model: "pc105".into(),
                layout: "de".into(),
                variant: "nodeadkeys".into(),
                options: String::new(),
            }
        );
    }

    #[test]
    fn reads_localectl_xorg_settings() {
        let x = parse_xorg_conf(
            "Section \"InputClass\"\n        Identifier \"system-keyboard\"\n        MatchIsKeyboard \"on\"\n        Option \"XkbLayout\" \"fr,us\"\n        Option \"XkbOptions\" \"grp:alt_shift_toggle\"\nEndSection\n",
        );
        assert_eq!(x.layout, "fr,us");
        assert_eq!(x.options, "grp:alt_shift_toggle");
        assert_eq!(x.model, "");
    }

    #[test]
    fn comments_and_noise_are_ignored() {
        let x = parse_default_keyboard("#XKBLAYOUT=\"de\"\nnot an assignment\n");
        assert_eq!(x, Xkb::default());
    }

    #[test]
    fn the_repeat_delay_is_the_designs() {
        assert_eq!(REPEAT_DELAY_MS, 400);
    }
}
