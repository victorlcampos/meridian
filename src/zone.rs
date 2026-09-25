//! Where the displayed time comes from: the computer's clock or a city's zone.

use chrono::{DateTime, FixedOffset, Local, NaiveDateTime, TimeZone, Utc};
use chrono_tz::{OffsetName, Tz};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Zone {
    /// The computer's clock, with its IANA zone when it could be detected.
    System(Option<Tz>),
    /// The zone of a city picked by the user.
    City(Tz),
}

impl Zone {
    pub fn system() -> Self {
        Self::System(
            iana_time_zone::get_timezone()
                .ok()
                .and_then(|name| name.parse().ok()),
        )
    }

    pub fn tz(self) -> Option<Tz> {
        match self {
            Self::System(tz) => tz,
            Self::City(tz) => Some(tz),
        }
    }

    pub fn local_time(self, now: DateTime<Utc>) -> DateTime<FixedOffset> {
        match self.tz() {
            Some(tz) => now.with_timezone(&tz).fixed_offset(),
            None => now.with_timezone(&Local).fixed_offset(),
        }
    }

    /// The instant a wall-clock time happens in this zone: the earliest one when a
    /// DST change repeats it, `None` when a DST change skips it.
    pub fn instant(self, wall: NaiveDateTime) -> Option<DateTime<Utc>> {
        match self.tz() {
            Some(tz) => tz.from_local_datetime(&wall).earliest().map(|t| t.to_utc()),
            None => Local
                .from_local_datetime(&wall)
                .earliest()
                .map(|t| t.to_utc()),
        }
    }

    /// Letter abbreviation such as "JST" or "CEST". Zones whose abbreviation is
    /// numeric ("-03") return `None`, since the UTC offset already says that.
    pub fn abbreviation(self, now: DateTime<Utc>) -> Option<String> {
        let tz = self.tz()?;
        let local = now.with_timezone(&tz);
        let abbreviation = local.offset().abbreviation()?;
        abbreviation
            .chars()
            .all(|c| c.is_ascii_alphabetic())
            .then(|| abbreviation.to_owned())
    }
}

/// "UTC-03:00", "UTC+05:45" or plain "UTC".
pub fn format_offset(offset: FixedOffset) -> String {
    let seconds = offset.local_minus_utc();
    if seconds == 0 {
        return "UTC".to_owned();
    }
    let sign = if seconds < 0 { '-' } else { '+' };
    let minutes = seconds.unsigned_abs() / 60;
    format!("UTC{sign}{:02}:{:02}", minutes / 60, minutes % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
            .and_utc()
    }

    #[test]
    fn converts_to_the_city_wall_clock() {
        let tokyo = Zone::City(chrono_tz::Asia::Tokyo);
        let local = tokyo.local_time(utc(2026, 9, 25, 16, 0));
        assert_eq!(
            local.format("%Y-%m-%d %H:%M").to_string(),
            "2026-09-26 01:00"
        );
        assert_eq!(format_offset(*local.offset()), "UTC+09:00");
    }

    #[test]
    fn formats_offsets() {
        let offset = |s| FixedOffset::east_opt(s).unwrap();
        assert_eq!(format_offset(offset(0)), "UTC");
        assert_eq!(format_offset(offset(-3 * 3600)), "UTC-03:00");
        assert_eq!(format_offset(offset(5 * 3600 + 45 * 60)), "UTC+05:45");
    }

    #[test]
    fn keeps_only_letter_abbreviations() {
        let now = utc(2026, 7, 1, 12, 0);
        assert_eq!(
            Zone::City(chrono_tz::Europe::Berlin).abbreviation(now),
            Some("CEST".to_owned())
        );
        assert_eq!(
            Zone::City(chrono_tz::America::Sao_Paulo).abbreviation(now),
            None
        );
    }

    #[test]
    fn resolves_wall_times_across_dst_changes() {
        let new_york = Zone::City(chrono_tz::America::New_York);
        let day = NaiveDate::from_ymd_opt(2026, 3, 8).unwrap();
        assert_eq!(new_york.instant(day.and_hms_opt(2, 30, 0).unwrap()), None);

        let fall = NaiveDate::from_ymd_opt(2026, 11, 1).unwrap();
        let repeated = new_york.instant(fall.and_hms_opt(1, 30, 0).unwrap());
        assert_eq!(repeated, Some(utc(2026, 11, 1, 5, 30)));
    }
}
