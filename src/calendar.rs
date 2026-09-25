//! The next events of an iCalendar feed: Google Calendar's secret iCal
//! address, or any other .ics. Recurring events are expanded; all-day,
//! cancelled and declined events are left out.

use std::collections::HashSet;

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use chrono_tz::Tz;

use crate::zone::Zone;

/// How far ahead events are read.
pub const WINDOW_DAYS: i64 = 7;

/// Download key of a calendar, so an answer for an address since replaced is ignored.
pub fn key(address: &str) -> String {
    format!("calendar:{address}")
}

#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub uid: String,
    pub title: String,
    pub start: DateTime<Utc>,
}

impl Event {
    /// Names one occurrence, so its reminder rings once.
    pub fn key(&self) -> String {
        format!("{}@{}", self.uid, self.start.timestamp())
    }
}

/// `DTSTART;TZID=Europe/Paris:20260101T090000` split into name, parameters and value.
struct Property {
    name: String,
    params: Vec<(String, String)>,
    value: String,
}

impl Property {
    fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(key))
            .map(|(_, value)| value.as_str())
    }
}

#[derive(Default)]
struct Component {
    uid: String,
    summary: String,
    start: Option<Property>,
    rule: Option<String>,
    exdates: Vec<Property>,
    recurrence_id: Option<Property>,
    cancelled: bool,
    /// Attendees who declined, by lower-case e-mail.
    declined: Vec<String>,
}

#[derive(Clone, Copy)]
enum Moment {
    AllDay,
    At(DateTime<Utc>, Option<Tz>),
}

/// Events starting in `[from, until)`, earliest first. Times without a zone
/// follow the calendar's own zone, or else `system`.
pub fn upcoming(ics: &str, from: DateTime<Utc>, until: DateTime<Utc>, system: Zone) -> Vec<Event> {
    let mut components = Vec::new();
    let mut owner = None;
    let mut floating = system;
    let mut current: Option<Component> = None;
    let mut nested = 0;
    for line in unfold(ics) {
        let Some(property) = parse_property(&line) else {
            continue;
        };
        let (name, value) = (property.name.clone(), property.value.clone());
        let value = value.as_str();
        match (name.as_str(), current.as_mut()) {
            ("BEGIN", None) if value.eq_ignore_ascii_case("VEVENT") => {
                current = Some(Component::default());
            }
            ("BEGIN", Some(_)) => nested += 1,
            ("END", Some(_)) if nested > 0 => nested -= 1,
            ("END", Some(_)) => components.extend(current.take()),
            (_, Some(_)) if nested > 0 => {}
            (name, Some(event)) => read_event_property(event, name, property),
            ("X-WR-CALNAME", None) if value.contains('@') => owner = Some(value.to_lowercase()),
            ("X-WR-TIMEZONE", None) => {
                if let Ok(tz) = value.parse() {
                    floating = Zone::City(tz);
                }
            }
            _ => {}
        }
    }

    let declined = |event: &Component| {
        owner
            .as_ref()
            .is_some_and(|owner| event.declined.contains(owner))
    };
    // Occurrences of a series that were moved or cancelled one by one.
    let replaced: HashSet<(String, i64)> = components
        .iter()
        .filter_map(|event| {
            let Some(Moment::At(original, _)) =
                parse_moment(event.recurrence_id.as_ref()?, floating)
            else {
                return None;
            };
            Some((event.uid.clone(), original.timestamp()))
        })
        .collect();

    let mut events = Vec::new();
    for component in &components {
        let Some(Moment::At(start, zone)) = component
            .start
            .as_ref()
            .and_then(|start| parse_moment(start, floating))
        else {
            continue;
        };
        if component.cancelled || declined(component) {
            continue;
        }
        let starts = match (&component.rule, &component.recurrence_id) {
            (Some(rule), None) => {
                expand(rule, start, zone, &component.exdates, floating, from, until)
            }
            _ => vec![start],
        };
        for start in starts {
            let moved = component.recurrence_id.is_none()
                && replaced.contains(&(component.uid.clone(), start.timestamp()));
            if start >= from && start < until && !moved {
                events.push(Event {
                    uid: component.uid.clone(),
                    title: component.summary.clone(),
                    start,
                });
            }
        }
    }
    events.sort_by(|a, b| a.start.cmp(&b.start).then_with(|| a.title.cmp(&b.title)));
    events
}

fn read_event_property(event: &mut Component, name: &str, property: Property) {
    match name {
        "UID" => event.uid = property.value,
        "SUMMARY" => event.summary = unescape(&property.value),
        "DTSTART" => event.start = Some(property),
        "RRULE" => event.rule = Some(property.value),
        "EXDATE" => event.exdates.push(property),
        "RECURRENCE-ID" => event.recurrence_id = Some(property),
        "STATUS" => event.cancelled = property.value.eq_ignore_ascii_case("CANCELLED"),
        "ATTENDEE" => {
            let declined = property
                .param("PARTSTAT")
                .is_some_and(|status| status.eq_ignore_ascii_case("DECLINED"));
            let lower = property.value.to_lowercase();
            if declined && let Some(email) = lower.strip_prefix("mailto:") {
                event.declined.push(email.to_owned());
            }
        }
        _ => {}
    }
}

/// Occurrences of a recurring event in `[from, until)`, in its own zone so
/// they keep their wall-clock time across DST changes. A rule that cannot be
/// read leaves just the first occurrence.
fn expand(
    rule: &str,
    start: DateTime<Utc>,
    zone: Option<Tz>,
    exdates: &[Property],
    floating: Zone,
    from: DateTime<Utc>,
    until: DateTime<Utc>,
) -> Vec<DateTime<Utc>> {
    let tz = zone.map_or(rrule::Tz::LOCAL, rrule::Tz::Tz);
    let set = rule
        .parse::<rrule::RRule<rrule::Unvalidated>>()
        .and_then(|rule| rule.build(start.with_timezone(&tz)));
    let Ok(mut set) = set else {
        return vec![start];
    };
    for exdate in exdates {
        for value in exdate.value.split(',') {
            let single = Property {
                name: exdate.name.clone(),
                params: exdate.params.clone(),
                value: value.to_owned(),
            };
            if let Some(Moment::At(excluded, _)) = parse_moment(&single, floating) {
                set = set.exdate(excluded.with_timezone(&tz));
            }
        }
    }
    set.after((from - Duration::seconds(1)).with_timezone(&tz))
        .before(until.with_timezone(&tz))
        .all(1000)
        .dates
        .into_iter()
        .map(|date| date.with_timezone(&Utc))
        .collect()
}

/// `20260925T140000Z` (UTC), `20260925T140000` with a TZID or floating, or an all-day date.
fn parse_moment(property: &Property, floating: Zone) -> Option<Moment> {
    let value = property.value.trim();
    let all_day = property
        .param("VALUE")
        .is_some_and(|kind| kind.eq_ignore_ascii_case("DATE"));
    if all_day || value.len() == 8 {
        return Some(Moment::AllDay);
    }
    if let Some(utc) = value.strip_suffix('Z') {
        let time = NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S").ok()?;
        return Some(Moment::At(time.and_utc(), Some(chrono_tz::UTC)));
    }
    let time = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()?;
    let zone = property
        .param("TZID")
        .and_then(|name| name.parse().ok())
        .map_or(floating, Zone::City);
    // A time skipped by a DST jump happens an hour later.
    let at = zone
        .instant(time)
        .or_else(|| zone.instant(time + Duration::hours(1)))?;
    Some(Moment::At(at, zone.tz()))
}

/// Lines with the long ones put back together (RFC 5545 folds them with a
/// leading space).
fn unfold(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        match (line.strip_prefix([' ', '\t']), lines.last_mut()) {
            (Some(rest), Some(last)) => last.push_str(rest),
            _ if !line.is_empty() => lines.push(line.to_owned()),
            _ => {}
        }
    }
    lines
}

fn parse_property(line: &str) -> Option<Property> {
    let (head, value) = split_once_unquoted(line, ':')?;
    let mut parts = split_unquoted(head, ';').into_iter();
    let name = parts.next()?.to_ascii_uppercase();
    let params = parts
        .filter_map(|part| part.split_once('='))
        .map(|(key, value)| (key.to_ascii_uppercase(), value.trim_matches('"').to_owned()))
        .collect();
    Some(Property {
        name,
        params,
        value: value.to_owned(),
    })
}

fn split_once_unquoted(text: &str, separator: char) -> Option<(&str, &str)> {
    let mut quoted = false;
    let at = text.char_indices().find(|&(_, ch)| {
        if ch == '"' {
            quoted = !quoted;
        }
        ch == separator && !quoted
    })?;
    Some((&text[..at.0], &text[at.0 + 1..]))
}

fn split_unquoted(text: &str, separator: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = text;
    while let Some((part, tail)) = split_once_unquoted(rest, separator) {
        parts.push(part);
        rest = tail;
    }
    parts.push(rest);
    parts
}

/// `\,` `\;` `\n` and `\\` escapes of text values.
fn unescape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n' | 'N') => out.push(' '),
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

/// A calendar address safe to show: host and file name, not the secret in between.
pub fn describe_address(address: &str) -> String {
    let rest = address.split_once("://").map_or(address, |(_, rest)| rest);
    let mut parts = rest.split('/').filter(|part| !part.is_empty());
    let first = parts.next().unwrap_or_default();
    match parts.next_back() {
        Some(last) => format!("{first}/…/{last}"),
        None => first.to_owned(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use chrono::NaiveDate;

    pub fn utc(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, 0)
            .unwrap()
            .and_utc()
    }

    /// A feed shaped like Google Calendar's export.
    pub const SAMPLE: &str = "BEGIN:VCALENDAR\r
PRODID:-//Google Inc//Google Calendar 70.9054//EN\r
VERSION:2.0\r
X-WR-CALNAME:ana@example.com\r
X-WR-TIMEZONE:America/Sao_Paulo\r
BEGIN:VEVENT\r
DTSTART;TZID=America/Sao_Paulo:20260925T140000\r
DTEND;TZID=America/Sao_Paulo:20260925T150000\r
UID:planning@google.com\r
SUMMARY:Planejamento\\, sprint 42\r
BEGIN:VALARM\r
ACTION:DISPLAY\r
SUMMARY:ignored alarm text\r
END:VALARM\r
END:VEVENT\r
BEGIN:VEVENT\r
DTSTART:20260925T190000Z\r
UID:call@google.com\r
SUMMARY:Call with a very long title that Google folds across\r
  two lines\r
END:VEVENT\r
BEGIN:VEVENT\r
DTSTART;VALUE=DATE:20260926\r
UID:holiday@google.com\r
SUMMARY:Holiday\r
END:VEVENT\r
BEGIN:VEVENT\r
DTSTART:20260925T200000Z\r
UID:cancelled@google.com\r
STATUS:CANCELLED\r
SUMMARY:Cancelled\r
END:VEVENT\r
BEGIN:VEVENT\r
DTSTART:20260925T210000Z\r
UID:declined@google.com\r
SUMMARY:Declined\r
ATTENDEE;CN=Ana;PARTSTAT=DECLINED:mailto:Ana@Example.com\r
END:VEVENT\r
BEGIN:VEVENT\r
DTSTART;TZID=America/New_York:20261026T090000\r
RRULE:FREQ=WEEKLY;BYDAY=MO;UNTIL=20261124T000000Z\r
EXDATE;TZID=America/New_York:20261109T090000\r
UID:standup@google.com\r
SUMMARY:Standup\r
END:VEVENT\r
BEGIN:VEVENT\r
DTSTART;TZID=America/New_York:20261117T100000\r
RECURRENCE-ID;TZID=America/New_York:20261116T090000\r
UID:standup@google.com\r
SUMMARY:Standup (moved)\r
END:VEVENT\r
BEGIN:VEVENT\r
DTSTART:20261001T120000\r
UID:floating@google.com\r
SUMMARY:Floating\r
END:VEVENT\r
END:VCALENDAR\r
";

    fn titles(events: &[Event]) -> Vec<&str> {
        events.iter().map(|event| event.title.as_str()).collect()
    }

    #[test]
    fn reads_single_events() {
        let system = Zone::City(chrono_tz::UTC);
        let events = upcoming(
            SAMPLE,
            utc(2026, 9, 25, 0, 0),
            utc(2026, 9, 26, 0, 0),
            system,
        );
        assert_eq!(
            titles(&events),
            [
                "Planejamento, sprint 42",
                "Call with a very long title that Google folds across two lines"
            ]
        );
        // 14:00 in São Paulo is 17:00 UTC.
        assert_eq!(events[0].start, utc(2026, 9, 25, 17, 0));
        assert_eq!(
            events[0].key(),
            format!(
                "planning@google.com@{}",
                utc(2026, 9, 25, 17, 0).timestamp()
            )
        );
    }

    #[test]
    fn expands_series_across_dst_with_exceptions() {
        let system = Zone::City(chrono_tz::UTC);
        let events = upcoming(
            SAMPLE,
            utc(2026, 10, 20, 0, 0),
            utc(2026, 12, 1, 0, 0),
            system,
        );
        let standups: Vec<_> = events
            .iter()
            .filter(|event| event.uid == "standup@google.com")
            .map(|event| (event.title.as_str(), event.start))
            .collect();
        assert_eq!(
            standups,
            [
                // 09:00 in New York: EDT until November 1st, EST after.
                ("Standup", utc(2026, 10, 26, 13, 0)),
                ("Standup", utc(2026, 11, 2, 14, 0)),
                // November 9th was removed, November 16th moved to the 17th.
                ("Standup (moved)", utc(2026, 11, 17, 15, 0)),
                ("Standup", utc(2026, 11, 23, 14, 0)),
            ]
        );
    }

    #[test]
    fn floating_times_follow_the_calendar_zone() {
        let events = upcoming(
            SAMPLE,
            utc(2026, 10, 1, 0, 0),
            utc(2026, 10, 2, 0, 0),
            Zone::City(chrono_tz::UTC),
        );
        assert_eq!(titles(&events), ["Floating"]);
        assert_eq!(events[0].start, utc(2026, 10, 1, 15, 0));
    }

    #[test]
    fn ignores_what_it_cannot_read() {
        let system = Zone::City(chrono_tz::UTC);
        assert!(
            upcoming(
                "not a calendar",
                utc(2026, 1, 1, 0, 0),
                utc(2027, 1, 1, 0, 0),
                system
            )
            .is_empty()
        );
        let broken_rule = "BEGIN:VEVENT\nDTSTART:20260101T100000Z\nRRULE:FREQ=SOMETIMES\nUID:x\nSUMMARY:Once\nEND:VEVENT\n";
        let events = upcoming(
            broken_rule,
            utc(2026, 1, 1, 0, 0),
            utc(2027, 1, 1, 0, 0),
            system,
        );
        assert_eq!(titles(&events), ["Once"]);
    }

    #[test]
    fn hides_the_secret_part_of_the_address() {
        assert_eq!(
            describe_address(
                "https://calendar.google.com/calendar/ical/ana%40example.com/private-0123abc/basic.ics"
            ),
            "calendar.google.com/…/basic.ics"
        );
        assert_eq!(describe_address("/home/ana/work.ics"), "home/…/work.ics");
        assert_eq!(describe_address("webcal://example.com"), "example.com");
    }
}
