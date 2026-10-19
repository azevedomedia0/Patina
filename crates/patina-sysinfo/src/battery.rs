//! Battery level from `/sys/class/power_supply`.

use std::{fs, path::Path};

pub const SYSFS: &str = "/sys/class/power_supply";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Battery {
    /// 0 to 100.
    pub percent: u8,
    pub charging: bool,
}

/// The system battery under `root` (normally [`SYSFS`]), or `None` on a machine without one.
///
/// Batteries of peripherals such as mice and headsets show up there too, with `scope` set to
/// `Device`; they are not the system battery and are skipped.
pub fn read(root: &Path) -> Option<Battery> {
    let mut dirs: Vec<_> = fs::read_dir(root)
        .ok()?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    dirs.sort();
    dirs.iter().find_map(|dir| read_one(dir))
}

fn read_one(dir: &Path) -> Option<Battery> {
    let text = |name: &str| {
        fs::read_to_string(dir.join(name))
            .ok()
            .map(|s| s.trim().to_string())
    };
    if text("type")? != "Battery" || text("scope").as_deref() == Some("Device") {
        return None;
    }
    let percent = text("capacity")?.parse::<u32>().ok()?.min(100) as u8;
    let charging = matches!(text("status").as_deref(), Some("Charging"));
    Some(Battery { percent, charging })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supply(root: &Path, name: &str, files: &[(&str, &str)]) {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        for (file, content) in files {
            fs::write(dir.join(file), format!("{content}\n")).unwrap();
        }
    }

    #[test]
    fn reads_the_system_battery() {
        let root = tempfile::tempdir().unwrap();
        supply(root.path(), "AC", &[("type", "Mains")]);
        supply(
            root.path(),
            "BAT0",
            &[
                ("type", "Battery"),
                ("capacity", "87"),
                ("status", "Discharging"),
            ],
        );
        assert_eq!(
            read(root.path()),
            Some(Battery {
                percent: 87,
                charging: false
            })
        );
    }

    #[test]
    fn reports_charging() {
        let root = tempfile::tempdir().unwrap();
        supply(
            root.path(),
            "BAT1",
            &[
                ("type", "Battery"),
                ("capacity", "42"),
                ("status", "Charging"),
            ],
        );
        assert_eq!(
            read(root.path()),
            Some(Battery {
                percent: 42,
                charging: true
            })
        );
    }

    #[test]
    fn no_battery_means_none() {
        let root = tempfile::tempdir().unwrap();
        supply(root.path(), "AC", &[("type", "Mains"), ("online", "1")]);
        assert_eq!(read(root.path()), None);
        assert_eq!(read(Path::new("/definitely/not/here")), None);
    }

    #[test]
    fn skips_peripheral_batteries() {
        let root = tempfile::tempdir().unwrap();
        supply(
            root.path(),
            "hid-mouse",
            &[
                ("type", "Battery"),
                ("scope", "Device"),
                ("capacity", "15"),
                ("status", "Discharging"),
            ],
        );
        supply(
            root.path(),
            "BAT0",
            &[("type", "Battery"), ("capacity", "60"), ("status", "Full")],
        );
        assert_eq!(
            read(root.path()),
            Some(Battery {
                percent: 60,
                charging: false
            })
        );
    }

    #[test]
    fn clamps_and_tolerates_bad_data() {
        let root = tempfile::tempdir().unwrap();
        supply(
            root.path(),
            "BAT0",
            &[("type", "Battery"), ("capacity", "250"), ("status", "Full")],
        );
        assert_eq!(read(root.path()).map(|b| b.percent), Some(100));
        let broken = tempfile::tempdir().unwrap();
        supply(
            broken.path(),
            "BAT0",
            &[("type", "Battery"), ("capacity", "n/a")],
        );
        assert_eq!(read(broken.path()), None);
    }
}
