//! The `[Desktop Entry]` group of a `.desktop` file (freedesktop Desktop Entry Specification).

use std::{collections::HashMap, path::Path};

use crate::App;

/// Keys of the `[Desktop Entry]` group, values unescaped, localised keys kept as `Name[de]`.
#[derive(Debug, Default)]
pub struct DesktopEntry {
    keys: HashMap<String, String>,
}

impl DesktopEntry {
    pub fn parse(text: &str) -> Self {
        let mut keys = HashMap::new();
        let mut in_entry = false;
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') {
                in_entry = line == "[Desktop Entry]";
                continue;
            }
            if !in_entry {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                keys.entry(k.trim().to_string())
                    .or_insert_with(|| unescape(v.trim()));
            }
        }
        Self { keys }
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.keys.get(key).map(String::as_str)
    }

    /// A localised string: `Name[de_DE]`, `Name[de]`, then `Name` (for locale `de_DE.UTF-8`).
    pub fn localized(&self, key: &str, locale: &str) -> Option<&str> {
        let base = locale.split(['.', '@']).next().unwrap_or("");
        let lang = base.split('_').next().unwrap_or("");
        [base, lang]
            .iter()
            .filter(|l| !l.is_empty())
            .find_map(|l| self.get(&format!("{key}[{l}]")))
            .or_else(|| self.get(key))
    }

    fn flag(&self, key: &str) -> bool {
        self.get(key) == Some("true")
    }

    fn list(&self, key: &str) -> Vec<&str> {
        self.get(key)
            .map(|v| v.split(';').filter(|s| !s.is_empty()).collect())
            .unwrap_or_default()
    }

    /// Whether an autostart entry should start in this desktop (XDG Autostart spec): not
    /// `Hidden`, not turned off by GNOME's key, shown in this desktop, its `TryExec` present.
    /// (`NoDisplay` does not matter for autostart.)
    pub fn autostarts(&self, desktops: &[String]) -> bool {
        if self.get("Type").is_some_and(|t| t != "Application")
            || self.flag("Hidden")
            || self.get("X-GNOME-Autostart-enabled") == Some("false")
        {
            return false;
        }
        let only = self.list("OnlyShowIn");
        if !only.is_empty() && !only.iter().any(|d| desktops.iter().any(|c| c == d)) {
            return false;
        }
        if self
            .list("NotShowIn")
            .iter()
            .any(|d| desktops.iter().any(|c| c == d))
        {
            return false;
        }
        self.get("TryExec").is_none_or(crate::exec::in_path) && self.get("Exec").is_some()
    }

    /// The application this entry describes, if the launcher should show it.
    pub fn to_app(&self, id: &str, path: &Path, desktops: &[String], locale: &str) -> Option<App> {
        if self.get("Type") != Some("Application") || self.flag("NoDisplay") || self.flag("Hidden")
        {
            return None;
        }
        let only = self.list("OnlyShowIn");
        if !only.is_empty() && !only.iter().any(|d| desktops.iter().any(|c| c == d)) {
            return None;
        }
        if self
            .list("NotShowIn")
            .iter()
            .any(|d| desktops.iter().any(|c| c == d))
        {
            return None;
        }
        let exec = self.get("Exec")?.to_string();
        if let Some(try_exec) = self.get("TryExec") {
            if !crate::exec::in_path(try_exec) {
                return None;
            }
        }
        Some(App {
            id: id.to_string(),
            name: self.localized("Name", locale)?.to_string(),
            exec,
            icon: self
                .get("Icon")
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
            terminal: self.flag("Terminal"),
            wm_class: self.get("StartupWMClass").map(str::to_owned),
            path: path.to_path_buf(),
        })
    }
}

/// The spec's escapes in string values: `\s`, `\n`, `\t`, `\r`, `\\`.
fn unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('s') => out.push(' '),
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('\\') => out.push('\\'),
            // Anything else (such as `\;` in lists, or a quote in Exec) stays as it was.
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILES: &str = "[Desktop Entry]\nName=Files\nName[de]=Dateien\nName[pt_BR]=Arquivos\nExec=nautilus --new-window %U\nIcon=org.gnome.Nautilus\nType=Application\nStartupWMClass=org.gnome.Nautilus\n\n[Desktop Action new-window]\nName=New Window\nExec=nautilus --new-window\n";

    #[test]
    fn reads_the_main_group_only() {
        let e = DesktopEntry::parse(FILES);
        assert_eq!(e.get("Exec"), Some("nautilus --new-window %U"));
        assert_eq!(e.get("Name"), Some("Files"));
    }

    #[test]
    fn picks_the_best_localised_name() {
        let e = DesktopEntry::parse(FILES);
        assert_eq!(e.localized("Name", "de_DE.UTF-8"), Some("Dateien"));
        assert_eq!(e.localized("Name", "pt_BR.UTF-8"), Some("Arquivos"));
        assert_eq!(e.localized("Name", "fr_FR.UTF-8"), Some("Files"));
        assert_eq!(e.localized("Name", "C"), Some("Files"));
    }

    #[test]
    fn unescapes_values() {
        assert_eq!(unescape(r"a\sb\\c\nd"), "a b\\c\nd");
    }

    #[test]
    fn hides_entries_for_other_desktops() {
        let e = DesktopEntry::parse(
            "[Desktop Entry]\nType=Application\nName=X\nExec=x\nNotShowIn=Patina;KDE;\n",
        );
        assert!(e
            .to_app("x", Path::new("/x"), &["Patina".into()], "")
            .is_none());
        assert!(e
            .to_app("x", Path::new("/x"), &["GNOME".into()], "")
            .is_some());
    }
}
