//! Hourly forecast from Open-Meteo (free, no key needed): temperature and
//! chance of rain.

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;

use crate::cities::City;
use crate::i18n::Lang;

/// How far ahead the forecast goes, in hours.
pub const HOURS: usize = 24;

#[derive(Clone, Debug, PartialEq)]
pub struct Forecast {
    pub temperature: f64,
    /// WMO weather code of the current conditions.
    pub code: u8,
    pub hours: Vec<Hour>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hour {
    pub at: DateTime<Utc>,
    pub temperature: f64,
    /// Chance of rain, in percent.
    pub rain: u8,
}

/// Download key of a city's forecast.
pub fn key(city: &City) -> String {
    format!("weather:{:.3},{:.3}", city.lat, city.lon)
}

pub fn url(city: &City) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={:.3}&longitude={:.3}\
         &current=temperature_2m,weather_code\
         &hourly=temperature_2m,precipitation_probability\
         &forecast_hours={HOURS}&timeformat=unixtime&timezone=UTC",
        city.lat, city.lon
    )
}

#[derive(Deserialize)]
struct Response {
    current: Current,
    hourly: Hourly,
}

#[derive(Deserialize)]
struct Current {
    temperature_2m: f64,
    weather_code: Option<u8>,
}

#[derive(Deserialize)]
struct Hourly {
    time: Vec<i64>,
    temperature_2m: Vec<Option<f64>>,
    precipitation_probability: Vec<Option<u8>>,
}

pub fn parse(json: &str) -> Result<Forecast, String> {
    let response: Response = serde_json::from_str(json).map_err(|error| error.to_string())?;
    let hourly = response.hourly;
    let hours = hourly
        .time
        .iter()
        .zip(&hourly.temperature_2m)
        .zip(&hourly.precipitation_probability)
        .filter_map(|((&time, &temperature), &rain)| {
            Some(Hour {
                at: DateTime::from_timestamp(time, 0)?,
                temperature: temperature?,
                rain: rain.unwrap_or(0),
            })
        })
        .collect();
    Ok(Forecast {
        temperature: response.current.temperature_2m,
        code: response.current.weather_code.unwrap_or(0),
        hours,
    })
}

impl Forecast {
    /// The hours from the one under way on.
    pub fn ahead(&self, now: DateTime<Utc>) -> impl Iterator<Item = &Hour> {
        self.hours
            .iter()
            .filter(move |hour| hour.at + Duration::hours(1) > now)
    }

    /// The first of the rainiest hours among the next `hours`, when rain is
    /// likely enough to mention (30% or more).
    pub fn rain_peak(&self, now: DateTime<Utc>, hours: usize) -> Option<Hour> {
        let mut peak: Option<Hour> = None;
        for hour in self.ahead(now).take(hours) {
            if hour.rain >= 30 && peak.is_none_or(|peak| hour.rain > peak.rain) {
                peak = Some(*hour);
            }
        }
        peak
    }
}

/// Short description of a WMO weather code.
pub fn describe(code: u8, lang: Lang) -> &'static str {
    let (en, pt) = match code {
        0 => ("Clear sky", "Céu limpo"),
        1 => ("Mostly clear", "Predomínio de sol"),
        2 => ("Partly cloudy", "Parcialmente nublado"),
        3 => ("Overcast", "Nublado"),
        45 | 48 => ("Fog", "Neblina"),
        51..=55 => ("Drizzle", "Garoa"),
        56 | 57 => ("Freezing drizzle", "Garoa congelante"),
        61 => ("Light rain", "Chuva fraca"),
        63 => ("Rain", "Chuva"),
        65 => ("Heavy rain", "Chuva forte"),
        66 | 67 => ("Freezing rain", "Chuva congelante"),
        71..=77 => ("Snow", "Neve"),
        80 | 81 => ("Rain showers", "Pancadas de chuva"),
        82 => ("Violent showers", "Pancadas fortes"),
        85 | 86 => ("Snow showers", "Pancadas de neve"),
        95 => ("Thunderstorm", "Trovoadas"),
        96..=99 => ("Thunderstorm, hail", "Trovoadas com granizo"),
        _ => ("", ""),
    };
    match lang {
        Lang::En => en,
        Lang::Pt => pt,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A trimmed-down answer from the Open-Meteo API, starting at 2026-09-25 16:00 UTC.
    pub const SAMPLE: &str = r#"{
        "latitude": 35.7, "longitude": 139.7,
        "current": {"time": 1790355600, "interval": 900, "temperature_2m": 21.0, "weather_code": 55},
        "hourly": {
            "time": [1790355600, 1790359200, 1790362800, 1790366400, 1790370000],
            "temperature_2m": [21.1, 20.8, null, 20.4, 19.9],
            "precipitation_probability": [20, 64, 97, 97, null]
        }
    }"#;

    pub fn at(timestamp: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(timestamp, 0).unwrap()
    }

    #[test]
    fn reads_the_forecast() {
        let forecast = parse(SAMPLE).unwrap();
        assert_eq!((forecast.temperature, forecast.code), (21.0, 55));
        assert_eq!(
            forecast.hours.len(),
            4,
            "hours without a temperature are skipped"
        );
        assert_eq!(
            forecast.hours[3].rain, 0,
            "a missing chance of rain counts as none"
        );
        assert!(parse("{}").is_err());
    }

    #[test]
    fn looks_ahead_from_the_hour_under_way() {
        let forecast = parse(SAMPLE).unwrap();
        let now = at(1790355600 + 1800);
        assert_eq!(forecast.ahead(now).next().unwrap().at, at(1790355600));
        assert_eq!(forecast.ahead(at(1790370000 + 3600)).count(), 0);
    }

    #[test]
    fn finds_the_first_rainiest_hour() {
        let forecast = parse(SAMPLE).unwrap();
        let now = at(1790355600);
        assert_eq!(
            forecast.rain_peak(now, 12).map(|hour| hour.at),
            Some(at(1790366400))
        );
        assert_eq!(
            forecast.rain_peak(now, 1),
            None,
            "20% is not worth a mention"
        );
    }

    #[test]
    fn describes_the_weather() {
        assert_eq!(describe(55, Lang::Pt), "Garoa");
        assert_eq!(describe(95, Lang::En), "Thunderstorm");
        assert_eq!(describe(200, Lang::En), "");
    }
}
