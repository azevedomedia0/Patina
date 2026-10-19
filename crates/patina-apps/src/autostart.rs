//! XDG Autostart: programs to start when the session starts, from `autostart/*.desktop` in
//! `$XDG_CONFIG_HOME` (default `~/.config`) and `$XDG_CONFIG_DIRS` (default `/etc/xdg`). An entry
//! in the user's directory replaces the system one with the same name, so a user can turn one
//! off with `Hidden=true`.

use std::path::PathBuf;

use crate::{desktop_entry::DesktopEntry, App};

fn config_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    match std::env::var_os("XDG_CONFIG_HOME") {
        Some(d) if !d.is_empty() => dirs.push(PathBuf::from(d)),
        _ => {
            if let Some(home) = std::env::var_os("HOME") {
                dirs.push(PathBuf::from(home).join(".config"));
            }
        }
    }
    let system = std::env::var("XDG_CONFIG_DIRS")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/etc/xdg".into());
    dirs.extend(
        system
            .split(':')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
    );
    dirs
}

/// The programs to start, for desktops `desktops` (`XDG_CURRENT_DESKTOP`), from `dirs`.
pub fn entries_in(dirs: &[PathBuf], desktops: &[String]) -> Vec<App> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(read) = std::fs::read_dir(dir.join("autostart")) else {
            continue;
        };
        let mut files: Vec<PathBuf> = read
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "desktop"))
            .collect();
        files.sort();
        for path in files {
            let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
                continue;
            };
            // The first directory to have a file of this name decides, even to turn it off.
            if !seen.insert(name.clone()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let e = DesktopEntry::parse(&text);
            if !e.autostarts(desktops) {
                continue;
            }
            out.push(App {
                id: name.trim_end_matches(".desktop").into(),
                name: e.get("Name").unwrap_or(&name).into(),
                exec: e.get("Exec").unwrap_or_default().into(),
                icon: None,
                terminal: e.get("Terminal") == Some("true"),
                wm_class: None,
                path,
            });
        }
    }
    out
}

pub fn entries() -> Vec<App> {
    let desktops: Vec<String> = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_else(|_| "Patina".into())
        .split(':')
        .map(str::to_owned)
        .collect();
    entries_in(&config_dirs(), &desktops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_entries_override_and_hidden_turns_off() {
        let base = std::env::temp_dir().join(format!("patina-autostart-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (user, sys) = (base.join("user"), base.join("sys"));
        for d in [&user, &sys] {
            std::fs::create_dir_all(d.join("autostart")).unwrap();
        }
        let w =
            |d: &PathBuf, n: &str, t: &str| std::fs::write(d.join("autostart").join(n), t).unwrap();
        w(
            &sys,
            "a.desktop",
            "[Desktop Entry]\nType=Application\nName=A\nExec=a\n",
        );
        w(
            &sys,
            "b.desktop",
            "[Desktop Entry]\nType=Application\nName=B\nExec=b\nNoDisplay=true\n",
        );
        w(
            &sys,
            "c.desktop",
            "[Desktop Entry]\nType=Application\nName=C\nExec=c\nOnlyShowIn=GNOME;\n",
        );
        w(
            &sys,
            "d.desktop",
            "[Desktop Entry]\nType=Application\nName=D\nExec=d\nX-GNOME-Autostart-enabled=false\n",
        );
        w(
            &user,
            "a.desktop",
            "[Desktop Entry]\nType=Application\nName=A\nExec=a\nHidden=true\n",
        );
        let got: Vec<String> = entries_in(&[user, sys], &["Patina".into()])
            .into_iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(got, vec!["b"]);
        let _ = std::fs::remove_dir_all(base);
    }
}
