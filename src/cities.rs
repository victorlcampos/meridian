//! Embedded city database (GeoNames cities with 15 000+ inhabitants) and its search.

use std::cmp::Reverse;
use std::collections::HashMap;
use std::sync::OnceLock;

use chrono_tz::Tz;

const CITIES: &str = include_str!("../assets/cities.tsv");
const COUNTRIES: &str = include_str!("../assets/countries.tsv");

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct City {
    pub name: &'static str,
    /// State, province or equivalent ("admin1" in GeoNames).
    pub region: &'static str,
    pub country: &'static str,
    pub lat: f64,
    pub lon: f64,
    pub tz: Tz,
    pub population: u32,
}

impl City {
    /// "Minas Gerais, Brazil"; the region is left out when it repeats the city name.
    pub fn place(&self) -> String {
        if self.region.is_empty() || self.region == self.name {
            self.country.to_owned()
        } else {
            format!("{}, {}", self.region, self.country)
        }
    }
}

struct Entry {
    city: City,
    name: String,
    alternates: Vec<String>,
    region: String,
    country: String,
    code: String,
}

pub struct CityDb {
    entries: Vec<Entry>,
}

/// The embedded database, parsed on first use.
pub fn db() -> &'static CityDb {
    static DB: OnceLock<CityDb> = OnceLock::new();
    DB.get_or_init(CityDb::load)
}

// Score of a match by where it is found and how (exact, prefix, word prefix,
// anywhere), best first. The city name beats its alternate names ("Londres"
// for London); naming a whole region or country ("Japan") beats letters found
// in the middle of a word ("Sojapango").
const NAME_TIERS: [u8; 4] = [0, 1, 3, 7];
const ALTERNATE_TIERS: [u8; 4] = [2, 4, 5, 8];
const AREA_TIERS: [u8; 3] = [6, 9, 9];

impl CityDb {
    fn load() -> Self {
        let countries: HashMap<&str, &'static str> = COUNTRIES
            .lines()
            .filter_map(|line| line.split_once('\t'))
            .collect();
        let entries = CITIES
            .lines()
            .filter_map(|line| parse_line(line, &countries))
            .collect();
        Self { entries }
    }

    /// Cities matching `query`, best first; ties go to the most populous.
    /// "City, area" keeps only cities whose country, country code or region
    /// matches the part after the comma ("Portland, Maine", "Paris, US").
    pub fn search(&self, query: &str, limit: usize) -> Vec<City> {
        let (place, area) = match query.split_once(',') {
            Some((place, area)) => (fold(place), fold(area)),
            None => (fold(query), String::new()),
        };
        if place.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<(u8, Reverse<u32>, usize)> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| area.is_empty() || entry.in_area(&area))
            .filter_map(|(index, entry)| {
                let score = entry.score(&place)?;
                Some((score, Reverse(entry.city.population), index))
            })
            .collect();
        hits.sort_unstable();
        hits.truncate(limit);
        hits.into_iter()
            .map(|(_, _, index)| self.entries[index].city)
            .collect()
    }
}

impl CityDb {
    /// The city called `name` nearest to where it was saved, within a degree.
    pub fn find_saved(&self, name: &str, lat: f64, lon: f64) -> Option<City> {
        self.entries
            .iter()
            .map(|entry| entry.city)
            .filter(|city| city.name == name)
            .map(|city| ((city.lat - lat).powi(2) + (city.lon - lon).powi(2), city))
            .filter(|(distance, _)| *distance <= 1.0)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, city)| city)
    }
}

impl Entry {
    fn score(&self, query: &str) -> Option<u8> {
        let name = rank(&self.name, query).map(|r| NAME_TIERS[r]);
        let alternate = self
            .alternates
            .iter()
            .filter_map(|alternate| rank(alternate, query))
            .min()
            .map(|r| ALTERNATE_TIERS[r]);
        let area = [rank(&self.country, query), rank(&self.region, query)]
            .into_iter()
            .flatten()
            .filter(|r| *r < AREA_TIERS.len() && query.len() >= 3)
            .min()
            .map(|r| AREA_TIERS[r]);
        [name, alternate, area].into_iter().flatten().min()
    }

    fn in_area(&self, area: &str) -> bool {
        self.code == area
            || rank(&self.region, area).is_some_and(|r| r <= 2)
            || rank(&self.country, area).is_some_and(|r| r <= 2)
    }
}

/// 0 exact, 1 prefix, 2 prefix of a later word, 3 anywhere (3+ letters only).
fn rank(key: &str, query: &str) -> Option<usize> {
    if key == query {
        Some(0)
    } else if key.starts_with(query) {
        Some(1)
    } else if key
        .match_indices(query)
        .any(|(at, _)| key.as_bytes()[at - 1] == b' ')
    {
        Some(2)
    } else if query.len() >= 3 && key.contains(query) {
        Some(3)
    } else {
        None
    }
}

fn parse_line(line: &'static str, countries: &HashMap<&str, &'static str>) -> Option<Entry> {
    let mut fields = line.split('\t');
    let name = fields.next()?;
    let code = fields.next()?;
    let region = fields.next()?;
    let lat = fields.next()?.parse().ok()?;
    let lon = fields.next()?.parse().ok()?;
    let tz = fields.next()?.parse().ok()?;
    let population = fields.next()?.parse().ok()?;
    let alternates = fields.next().unwrap_or_default();
    let country = countries.get(code).copied().unwrap_or(code);
    Some(Entry {
        city: City {
            name,
            region,
            country,
            lat,
            lon,
            tz,
            population,
        },
        name: fold(name),
        alternates: alternates
            .split('|')
            .filter(|alternate| !alternate.is_empty())
            .map(fold)
            .collect(),
        region: fold(region),
        country: fold(country),
        code: code.to_ascii_lowercase(),
    })
}

/// Lower-case ASCII words separated by single spaces: "Saint-Étienne" → "saint etienne".
/// Apostrophes join words ("N'Djamena" → "ndjamena").
pub fn fold(text: &str) -> String {
    let ascii = deunicode::deunicode(text).replace('\'', "");
    ascii
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn first(query: &str) -> City {
        db().search(query, 1)[0]
    }

    #[test]
    fn loads_every_embedded_city() {
        assert_eq!(db().entries.len(), CITIES.lines().count());
    }

    #[test]
    fn folds_accents_case_and_punctuation() {
        assert_eq!(fold("São Paulo"), "sao paulo");
        assert_eq!(fold("  Saint-Étienne "), "saint etienne");
        assert_eq!(fold("N'Djamena"), "ndjamena");
        assert_eq!(fold("Zürich"), "zurich");
    }

    #[test]
    fn finds_cities_without_accents() {
        let city = first("sao paulo");
        assert_eq!((city.name, city.country), ("São Paulo", "Brazil"));
        assert_eq!(city.tz, chrono_tz::America::Sao_Paulo);
    }

    #[test]
    fn prefers_the_most_populous_exact_match() {
        assert_eq!(first("Paris").country, "France");
        assert_eq!(first("London").country, "United Kingdom");
    }

    #[test]
    fn finds_cities_by_alternate_names() {
        assert_eq!(first("Londres").name, "London");
        assert_eq!(first("Nova Iorque").name, "New York City");
        assert_eq!(first("Tóquio").name, "Tokyo");
        assert_eq!(first("Munique").name, "Munich");
    }

    #[test]
    fn completes_prefixes() {
        let names: Vec<_> = db().search("belo hor", 3).iter().map(|c| c.name).collect();
        assert_eq!(names[0], "Belo Horizonte");
    }

    #[test]
    fn filters_by_region_or_country_after_a_comma() {
        let city = first("Portland, Maine");
        assert_eq!((city.name, city.region), ("Portland", "Maine"));
        assert_eq!(city.tz, chrono_tz::America::New_York);
        assert_eq!(first("Paris, US").country, "United States");
        assert_eq!(first("london, ca").region, "Ontario");
    }

    #[test]
    fn lists_the_cities_of_a_country() {
        assert_eq!(first("Japan").name, "Tokyo");
    }

    #[test]
    fn ignores_blank_queries_and_nonsense() {
        assert!(db().search("   ", 10).is_empty());
        assert!(db().search("qqqzzzxxx", 10).is_empty());
    }

    #[test]
    fn finds_a_saved_city_by_name_and_position() {
        let maine = db().find_saved("Portland", 43.66, -70.26).unwrap();
        assert_eq!(maine.region, "Maine");
        let oregon = db().find_saved("Portland", 45.52, -122.68).unwrap();
        assert_eq!(oregon.region, "Oregon");
        assert!(db().find_saved("Portland", 0.0, 0.0).is_none());
        assert!(db().find_saved("Atlantis", 45.52, -122.68).is_none());
    }

    #[test]
    fn describes_the_place() {
        assert_eq!(first("Belo Horizonte").place(), "Minas Gerais, Brazil");
        assert_eq!(first("Tokyo").place(), "Japan");
    }
}
