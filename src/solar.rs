//! Where the Sun is overhead, to shade the night side of the map.
//!
//! Low-precision formulas from the Astronomical Almanac (good to about 0.01°
//! between 1950 and 2050), far more than a terminal cell can show.

use chrono::{DateTime, Timelike, Utc};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Light {
    Day,
    /// Civil twilight: the Sun is less than 6° below the horizon.
    Twilight,
    Night,
}

/// Latitude and longitude, in degrees, of the point where the Sun is at the zenith.
pub fn subsolar_point(at: DateTime<Utc>) -> (f64, f64) {
    let days = (at.timestamp() as f64 + f64::from(at.timestamp_subsec_millis()) / 1000.0)
        / 86_400.0
        - 10_957.5; // days since J2000.0 (2000-01-01 12:00 UTC)
    let mean_longitude = (280.460 + 0.985_647_4 * days).rem_euclid(360.0);
    let mean_anomaly = (357.528 + 0.985_600_3 * days)
        .rem_euclid(360.0)
        .to_radians();
    let ecliptic_longitude =
        (mean_longitude + 1.915 * mean_anomaly.sin() + 0.020 * (2.0 * mean_anomaly).sin())
            .to_radians();
    let obliquity = (23.439 - 0.000_000_4 * days).to_radians();

    let declination = (obliquity.sin() * ecliptic_longitude.sin()).asin();
    let right_ascension = (obliquity.cos() * ecliptic_longitude.sin())
        .atan2(ecliptic_longitude.cos())
        .to_degrees();
    // Equation of time, in degrees of rotation (4 minutes each).
    let equation_of_time = wrap_degrees(mean_longitude - right_ascension);

    let hours = f64::from(at.num_seconds_from_midnight()) / 3600.0;
    let longitude = wrap_degrees(15.0 * (12.0 - hours) - equation_of_time);
    (declination.to_degrees(), longitude)
}

/// Daylight at a point, given the subsolar point.
pub fn light(sun: (f64, f64), lat: f64, lon: f64) -> Light {
    let (sun_lat, sun_lon) = (sun.0.to_radians(), sun.1.to_radians());
    let (lat, lon) = (lat.to_radians(), lon.to_radians());
    let elevation_sin =
        lat.sin() * sun_lat.sin() + lat.cos() * sun_lat.cos() * (lon - sun_lon).cos();
    if elevation_sin > 0.0 {
        Light::Day
    } else if elevation_sin > -(6.0_f64.to_radians().sin()) {
        Light::Twilight
    } else {
        Light::Night
    }
}

fn wrap_degrees(degrees: f64) -> f64 {
    (degrees + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
            .and_utc()
    }

    fn assert_near(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{actual} is not within {tolerance} of {expected}"
        );
    }

    #[test]
    fn follows_the_seasons() {
        assert_near(subsolar_point(at(2026, 6, 21, 12, 0)).0, 23.44, 0.05);
        assert_near(subsolar_point(at(2026, 12, 21, 12, 0)).0, -23.44, 0.05);
        // March equinox of 2026: 14:46 UTC on the 20th.
        assert_near(subsolar_point(at(2026, 3, 20, 14, 46)).0, 0.0, 0.05);
    }

    #[test]
    fn follows_the_equation_of_time() {
        // Early November the Sun runs about 16.4 minutes ahead of the clock,
        // so at 12:00 UTC it is overhead about 4.1° west of Greenwich.
        assert_near(subsolar_point(at(2026, 11, 3, 12, 0)).1, -4.1, 0.1);
        // Each hour moves the Sun 15° west.
        assert_near(subsolar_point(at(2026, 11, 3, 18, 0)).1, -94.1, 0.2);
    }

    #[test]
    fn shades_the_side_facing_away_from_the_sun() {
        let sun = subsolar_point(at(2026, 9, 25, 12, 0));
        assert_eq!(light(sun, 0.0, 10.0), Light::Day);
        assert_eq!(light(sun, 0.0, 170.0), Light::Night);
        assert_eq!(light(sun, 0.0, sun.1 + 93.0), Light::Twilight);
    }
}
