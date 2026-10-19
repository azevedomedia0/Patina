//! The panel clock, formatted as in the design: `Sat 17 Oct · 09:41` (24-hour, local time).

use std::time::Duration;

use chrono::{DateTime, Local, TimeZone, Timelike};

/// Formats `time` as the panel shows it. The weekday and month names are English for now;
/// localisation belongs to the i18n milestone.
pub fn format<Tz: TimeZone>(time: &DateTime<Tz>) -> String
where
    Tz::Offset: std::fmt::Display,
{
    time.format("%a %-d %b \u{b7} %H:%M").to_string()
}

/// The clock text for right now, in the local time zone.
pub fn now() -> String {
    format(&Local::now())
}

/// How long to wait from `time` until the displayed minute changes, so the panel redraws once a
/// minute, on the minute, instead of polling.
pub fn until_next_minute<Tz: TimeZone>(time: &DateTime<Tz>) -> Duration {
    let into_minute = Duration::new(time.second() as u64, time.nanosecond() % 1_000_000_000);
    // A leap second can report 60 s; never return zero, which would busy-loop.
    Duration::from_secs(60)
        .saturating_sub(into_minute)
        .max(Duration::from_millis(50))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap()
    }

    #[test]
    fn matches_the_design_sample() {
        assert_eq!(
            format(&at(2026, 10, 17, 9, 41, 0)),
            "Sat 17 Oct \u{b7} 09:41"
        );
    }

    #[test]
    fn day_is_not_zero_padded_and_hours_are_24h() {
        assert_eq!(format(&at(2026, 10, 5, 0, 7, 59)), "Mon 5 Oct \u{b7} 00:07");
        assert_eq!(
            format(&at(2026, 12, 31, 23, 59, 0)),
            "Thu 31 Dec \u{b7} 23:59"
        );
    }

    #[test]
    fn waits_until_the_minute_changes() {
        assert_eq!(
            until_next_minute(&at(2026, 10, 17, 9, 41, 0)),
            Duration::from_secs(60)
        );
        assert_eq!(
            until_next_minute(&at(2026, 10, 17, 9, 41, 45)),
            Duration::from_secs(15)
        );
        let almost = at(2026, 10, 17, 9, 41, 59);
        assert!(until_next_minute(&almost) <= Duration::from_secs(1));
    }

    #[test]
    fn never_asks_for_a_zero_wait() {
        let leap = Utc.with_ymd_and_hms(2026, 10, 17, 9, 41, 59).unwrap()
            + chrono::Duration::milliseconds(999);
        assert!(until_next_minute(&leap) >= Duration::from_millis(50));
    }
}
