//! Installed applications, as the shell sees them: the desktop entries in the XDG data
//! directories (`*/applications/*.desktop`), their icons from the icon theme, starting them, and
//! telling which entry a window belongs to.
//!
//! No Wayland or GUI dependency, so it is tested on any OS.

pub mod autostart;
pub mod desktop_entry;
pub mod exec;
pub mod icons;

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub use desktop_entry::DesktopEntry;

/// One application the shell can show and start.
#[derive(Debug, Clone, PartialEq)]
pub struct App {
    /// Desktop file id: the file name without `.desktop`, such as `org.gnome.Nautilus` or `foot`.
    pub id: String,
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub terminal: bool,
    /// `StartupWMClass`: the app id its windows use when it differs from the desktop file id.
    pub wm_class: Option<String>,
    pub path: PathBuf,
}

/// The directories with `applications/` subfolders, most important first: `$XDG_DATA_HOME`
/// (default `~/.local/share`), then `$XDG_DATA_DIRS` (default `/usr/local/share:/usr/share`).
pub fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    match std::env::var_os("XDG_DATA_HOME") {
        Some(d) if !d.is_empty() => dirs.push(PathBuf::from(d)),
        _ => {
            if let Some(home) = std::env::var_os("HOME") {
                dirs.push(PathBuf::from(home).join(".local/share"));
            }
        }
    }
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.extend(
        system
            .split(':')
            .filter(|s| !s.is_empty())
            .map(PathBuf::from),
    );
    dirs
}

/// The current desktop names, for `OnlyShowIn` and `NotShowIn`.
fn current_desktops() -> Vec<String> {
    std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_else(|_| "Patina".into())
        .split(':')
        .map(str::to_owned)
        .collect()
}

/// Every application to show in the launcher, sorted by name (case-insensitively). An entry in
/// a more important directory hides one with the same id further down, as the spec says.
pub fn all_apps() -> Vec<App> {
    apps_in(&data_dirs(), &current_desktops(), &locale())
}

/// Like [`all_apps`], for given directories, desktops and locale (for tests).
pub fn apps_in(dirs: &[PathBuf], desktops: &[String], locale: &str) -> Vec<App> {
    let mut seen = HashSet::new();
    let mut apps = Vec::new();
    for dir in dirs {
        let root = dir.join("applications");
        let mut files = Vec::new();
        collect_desktop_files(&root, &mut files);
        files.sort();
        for path in files {
            let Some(id) = desktop_file_id(&root, &path) else {
                continue;
            };
            if !seen.insert(id.clone()) {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let entry = DesktopEntry::parse(&text);
            if let Some(app) = entry.to_app(&id, &path, desktops, locale) {
                apps.push(app);
            }
        }
    }
    apps.sort_by_key(|a| a.name.to_lowercase());
    apps
}

fn collect_desktop_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_desktop_files(&p, out);
        } else if p.extension().is_some_and(|x| x == "desktop") {
            out.push(p);
        }
    }
}

/// The desktop file id: the path below `applications/` with `/` turned into `-`, without
/// `.desktop` (`kde/foo.desktop` is `kde-foo`).
fn desktop_file_id(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let s = rel.to_str()?.strip_suffix(".desktop")?;
    Some(s.replace('/', "-"))
}

/// The messages locale, such as `de_DE`, for localised names.
fn locale() -> String {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()))
        .unwrap_or_default()
}

/// The application a window with `app_id` belongs to. Tries, in order: the desktop file id, the
/// same ignoring case, `StartupWMClass`, the last part of a reverse-DNS id (`org.gnome.Nautilus`
/// for `nautilus`), and the program name in `Exec`.
pub fn app_for_window<'a>(apps: &'a [App], app_id: &str) -> Option<&'a App> {
    if app_id.is_empty() {
        return None;
    }
    let lower = app_id.to_lowercase();
    apps.iter()
        .find(|a| a.id == app_id)
        .or_else(|| apps.iter().find(|a| a.id.to_lowercase() == lower))
        .or_else(|| {
            apps.iter().find(|a| {
                a.wm_class
                    .as_deref()
                    .is_some_and(|c| c.to_lowercase() == lower)
            })
        })
        .or_else(|| {
            apps.iter()
                .find(|a| a.id.rsplit('.').next().map(str::to_lowercase) == Some(lower.clone()))
        })
        .or_else(|| {
            apps.iter().find(|a| {
                exec::program(&a.exec).and_then(|p| {
                    Path::new(&p)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                }) == Some(lower.clone())
            })
        })
}

/// The apps whose names (or ids) contain `query`, ignoring case; names that start with it first.
pub fn search<'a>(apps: &'a [App], query: &str) -> Vec<&'a App> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return apps.iter().collect();
    }
    let mut hits: Vec<(&App, bool)> = apps
        .iter()
        .filter_map(|a| {
            let name = a.name.to_lowercase();
            if name.starts_with(&q) {
                Some((a, true))
            } else if name.contains(&q) || a.id.to_lowercase().contains(&q) {
                Some((a, false))
            } else {
                None
            }
        })
        .collect();
    hits.sort_by_key(|(_, starts)| !starts);
    hits.into_iter().map(|(a, _)| a).collect()
}

/// Starts `app` in its own session (so it outlives the shell process that started it).
/// Terminal apps run in `x-terminal-emulator` (Debian's alternatives), else `foot`.
pub fn launch(app: &App) -> std::io::Result<()> {
    use std::os::unix::process::CommandExt;
    let mut argv = exec::expand(&app.exec, app.icon.as_deref(), &app.name, &app.path);
    if argv.is_empty() {
        return Err(std::io::Error::other("empty Exec"));
    }
    if app.terminal {
        let term = ["x-terminal-emulator", "foot"]
            .into_iter()
            .find(|t| exec::in_path(t))
            .unwrap_or("x-terminal-emulator");
        let mut wrapped = vec![term.to_string(), "-e".into()];
        wrapped.append(&mut argv);
        argv = wrapped;
    }
    let home = std::env::var_os("HOME").unwrap_or_else(|| "/".into());
    std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .current_dir(home)
        .stdin(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("patina-apps-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("applications/kde")).unwrap();
        d
    }

    fn write(dir: &Path, rel: &str, text: &str) {
        std::fs::write(dir.join("applications").join(rel), text).unwrap();
    }

    fn app(id: &str, name: &str, exec: &str, wm: Option<&str>) -> App {
        App {
            id: id.into(),
            name: name.into(),
            exec: exec.into(),
            icon: None,
            terminal: false,
            wm_class: wm.map(Into::into),
            path: PathBuf::new(),
        }
    }

    #[test]
    fn reads_entries_and_hides_what_should_be_hidden() {
        let user = tmp("user");
        let sys = tmp("sys");
        write(
            &sys,
            "foot.desktop",
            "[Desktop Entry]\nType=Application\nName=Foot\nExec=foot\nIcon=foot\n",
        );
        write(
            &sys,
            "hidden.desktop",
            "[Desktop Entry]\nType=Application\nName=Hidden\nExec=x\nNoDisplay=true\n",
        );
        write(
            &sys,
            "gnome-only.desktop",
            "[Desktop Entry]\nType=Application\nName=G\nExec=g\nOnlyShowIn=GNOME;\n",
        );
        write(
            &sys,
            "link.desktop",
            "[Desktop Entry]\nType=Link\nName=L\nURL=x\n",
        );
        write(
            &sys,
            "kde/tool.desktop",
            "[Desktop Entry]\nType=Application\nName=Tool\nExec=tool %f\n",
        );
        // The user's copy of foot wins over the system one.
        write(
            &user,
            "foot.desktop",
            "[Desktop Entry]\nType=Application\nName=My Foot\nExec=foot -f mono\n",
        );
        let apps = apps_in(&[user.clone(), sys.clone()], &["Patina".into()], "");
        let names: Vec<_> = apps
            .iter()
            .map(|a| (a.id.as_str(), a.name.as_str()))
            .collect();
        assert_eq!(names, vec![("foot", "My Foot"), ("kde-tool", "Tool")]);
        let _ = std::fs::remove_dir_all(user);
        let _ = std::fs::remove_dir_all(sys);
    }

    #[test]
    fn windows_find_their_app() {
        let apps = vec![
            app("org.gnome.Nautilus", "Files", "nautilus --new-window", None),
            app(
                "firefox-esr",
                "Firefox ESR",
                "/usr/lib/firefox-esr/firefox-esr %u",
                Some("firefox-esr"),
            ),
            app(
                "code",
                "Visual Studio Code",
                "/usr/share/code/code --unity-launch %F",
                Some("Code"),
            ),
            app("foot", "Foot", "foot", None),
        ];
        let id = |w: &str| app_for_window(&apps, w).map(|a| a.id.as_str());
        assert_eq!(id("foot"), Some("foot"));
        assert_eq!(id("org.gnome.nautilus"), Some("org.gnome.Nautilus"));
        assert_eq!(id("nautilus"), Some("org.gnome.Nautilus"));
        assert_eq!(id("Code"), Some("code"));
        assert_eq!(id("firefox-esr"), Some("firefox-esr"));
        assert_eq!(id("unknown"), None);
        assert_eq!(id(""), None);
    }

    #[test]
    fn search_puts_prefix_matches_first() {
        let apps = vec![
            app("a", "Text Editor", "x", None),
            app("b", "Terminal", "x", None),
            app("c", "Files", "x", None),
        ];
        let names: Vec<_> = search(&apps, "te")
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(names, vec!["Text Editor", "Terminal"]);
        let names: Vec<_> = search(&apps, "edit")
            .iter()
            .map(|a| a.name.as_str())
            .collect();
        assert_eq!(names, vec!["Text Editor"]);
        assert_eq!(search(&apps, "  ").len(), 3);
    }
}
