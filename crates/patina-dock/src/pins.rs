//! Apps kept in the dock, one desktop file id per line in `$XDG_CONFIG_HOME/patina/dock-pins`
//! (default `~/.config/patina/dock-pins`).
//!
//! Before the user changes anything, the dock keeps a terminal, a file manager and a web
//! browser, the first of each kind that is installed: Patina's own apps first once they exist.

use std::path::PathBuf;

use patina_apps::App;

const TERMINALS: &[&str] = &[
    "patina-terminal",
    "org.gnome.Console",
    "org.gnome.Terminal",
    "org.kde.konsole",
    "xfce4-terminal",
    "org.codeberg.dnkl.foot",
    "foot",
    "xterm",
];
const FILE_MANAGERS: &[&str] = &[
    "patina-files",
    "org.gnome.Nautilus",
    "nemo",
    "org.kde.dolphin",
    "thunar",
    "pcmanfm",
];
const BROWSERS: &[&str] = &[
    "firefox-esr",
    "firefox",
    "org.mozilla.firefox",
    "chromium",
    "google-chrome",
    "brave-browser",
];

fn path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("patina/dock-pins"))
}

pub fn parse(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

pub fn defaults(apps: &[App]) -> Vec<String> {
    [TERMINALS, FILE_MANAGERS, BROWSERS]
        .iter()
        .filter_map(|kind| {
            kind.iter()
                .find(|id| apps.iter().any(|a| a.id == **id))
                .map(|id| id.to_string())
        })
        .collect()
}

pub fn load(apps: &[App]) -> Vec<String> {
    match path().and_then(|p| std::fs::read_to_string(p).ok()) {
        Some(text) => parse(&text),
        None => defaults(apps),
    }
}

pub fn save(pins: &[String]) {
    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let text = format!(
        "# Apps kept in the Patina dock, top to bottom.\n{}\n",
        pins.join("\n")
    );
    if let Err(e) = std::fs::write(&p, text) {
        eprintln!("patina-dock: cannot save {}: {e}", p.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str) -> App {
        App {
            id: id.into(),
            name: id.into(),
            exec: id.into(),
            icon: None,
            terminal: false,
            wm_class: None,
            path: PathBuf::new(),
        }
    }

    #[test]
    fn defaults_pick_one_of_each_kind_that_is_installed() {
        let apps = vec![
            app("foot"),
            app("org.gnome.Nautilus"),
            app("firefox-esr"),
            app("xterm"),
        ];
        assert_eq!(
            defaults(&apps),
            vec!["foot", "org.gnome.Nautilus", "firefox-esr"]
        );
        assert_eq!(defaults(&[app("xterm")]), vec!["xterm"]);
        assert!(defaults(&[]).is_empty());
    }

    #[test]
    fn the_file_is_one_id_per_line() {
        assert_eq!(
            parse("# comment\nfoot\n\n firefox-esr \n"),
            vec!["foot", "firefox-esr"]
        );
    }
}
