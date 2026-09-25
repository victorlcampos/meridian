//! Alarms: reading what the user types, scheduling and ringing.

use chrono::{DateTime, Duration, NaiveTime, SubsecRound, Utc};

use crate::zone::Zone;

#[derive(Clone, Debug, PartialEq)]
pub struct Alarm {
    pub id: u32,
    /// Wall-clock time in `zone`.
    pub time: NaiveTime,
    pub zone: Zone,
    /// The zone as shown to the user: a city name or "local".
    pub place: String,
    pub label: String,
    pub daily: bool,
    /// Set from a delay ("+10m") or a snooze: gone once it rings.
    pub timer: bool,
    /// When it rings next; `None` while switched off.
    pub next: Option<DateTime<Utc>>,
}

impl Alarm {
    pub fn enabled(&self) -> bool {
        self.next.is_some()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum When {
    /// The next time the clock shows this time.
    At(NaiveTime),
    /// A delay from now.
    In(Duration),
}

#[derive(Debug, PartialEq, Eq)]
pub struct InvalidTime;

/// Reads a time ("07:30", "7h30", "730", "7:30pm") or a delay ("+10", "+10m",
/// "+1h30", "+90s"), optionally followed by a label.
pub fn parse(input: &str) -> Result<(When, String), InvalidTime> {
    let input = input.trim();
    let (token, rest) = input.split_once(char::is_whitespace).unwrap_or((input, ""));
    let mut label = rest.trim_start();
    let when = if let Some(delay) = token.strip_prefix('+') {
        When::In(parse_delay(delay).ok_or(InvalidTime)?)
    } else {
        // "7:30 pm": the meridiem may come as a word of its own.
        let meridiem = label
            .split_whitespace()
            .next()
            .filter(|word| word.eq_ignore_ascii_case("am") || word.eq_ignore_ascii_case("pm"));
        let token = match meridiem {
            Some(word) => {
                label = label[word.len()..].trim_start();
                format!("{token}{word}")
            }
            None => token.to_owned(),
        };
        When::At(parse_time(&token).ok_or(InvalidTime)?)
    };
    Ok((when, label.trim_end().to_owned()))
}

fn parse_time(token: &str) -> Option<NaiveTime> {
    let token = token.to_ascii_lowercase();
    let (token, pm) = match (token.strip_suffix("am"), token.strip_suffix("pm")) {
        (Some(time), _) => (time, Some(false)),
        (_, Some(time)) => (time, Some(true)),
        _ => (token.as_str(), None),
    };
    let separators = [':', '.', 'h'];
    let parts: Vec<&str> = if token.contains(separators) {
        token.split(separators).collect()
    } else if token.len() <= 2 {
        vec![token]
    } else if token.len() <= 4 && token.is_char_boundary(token.len() - 2) {
        let (hour, minute) = token.split_at(token.len() - 2);
        vec![hour, minute]
    } else {
        return None;
    };
    let number = |text: &str| match text {
        "" => Some(0),
        _ if text.len() <= 2 && text.bytes().all(|b| b.is_ascii_digit()) => text.parse().ok(),
        _ => None,
    };
    let (hour, minute, second) = match parts.as_slice() {
        [hour] => (*hour, "", ""),
        [hour, minute] => (*hour, *minute, ""),
        [hour, minute, second] => (*hour, *minute, *second),
        _ => return None,
    };
    if hour.is_empty() {
        return None;
    }
    let mut hour: u32 = number(hour)?;
    if let Some(pm) = pm {
        if !(1..=12).contains(&hour) {
            return None;
        }
        hour = hour % 12 + if pm { 12 } else { 0 };
    }
    NaiveTime::from_hms_opt(hour, number(minute)?, number(second)?)
}

/// "10" (minutes), "90s", "10m", "1h", "1h30", "2m30s". Up to a week.
fn parse_delay(text: &str) -> Option<Duration> {
    let mut total: i64 = 0;
    let mut digits = String::new();
    let mut last_unit = None;
    for ch in text.to_ascii_lowercase().chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        let unit = match ch {
            'h' => 3600,
            'm' => 60,
            's' => 1,
            _ => return None,
        };
        total = total.checked_add(digits.parse::<i64>().ok()?.checked_mul(unit)?)?;
        digits.clear();
        last_unit = Some(unit);
    }
    if !digits.is_empty() {
        // A trailing bare number is in the unit below the last one ("1h30",
        // "2m30"); on its own it counts minutes.
        let unit = match last_unit {
            None | Some(3600) => 60,
            Some(60) => 1,
            _ => return None,
        };
        total = total.checked_add(digits.parse::<i64>().ok()?.checked_mul(unit)?)?;
    }
    (1..=7 * 86_400)
        .contains(&total)
        .then(|| Duration::seconds(total))
}

/// The first instant after `after` when the clock in `zone` shows `time`. A
/// time skipped by a DST jump rings at the first valid quarter hour after it.
pub fn next_occurrence(time: NaiveTime, zone: Zone, after: DateTime<Utc>) -> DateTime<Utc> {
    let mut day = zone.local_time(after).date_naive();
    for _ in 0..3 {
        let wall = day.and_time(time);
        let at = (0..=8).find_map(|quarter| zone.instant(wall + Duration::minutes(15 * quarter)));
        if let Some(at) = at.filter(|at| *at > after) {
            return at;
        }
        day = day.succ_opt().unwrap_or(day);
    }
    after + Duration::days(1)
}

#[derive(Default)]
pub struct Alarms {
    items: Vec<Alarm>,
    last_id: u32,
}

impl Alarms {
    pub fn add(
        &mut self,
        when: When,
        label: String,
        zone: Zone,
        place: String,
        now: DateTime<Utc>,
    ) -> u32 {
        let (time, next) = match when {
            When::At(time) => (time, next_occurrence(time, zone, now)),
            When::In(delay) => {
                let at = (now + delay).trunc_subsecs(0);
                (zone.local_time(at).time(), at)
            }
        };
        self.last_id += 1;
        self.items.push(Alarm {
            id: self.last_id,
            time,
            zone,
            place,
            label,
            daily: false,
            timer: matches!(when, When::In(_)),
            next: Some(next),
        });
        self.items.sort_by_key(|alarm| (alarm.time, alarm.id));
        self.last_id
    }

    pub fn list(&self) -> &[Alarm] {
        &self.items
    }

    pub fn remove(&mut self, id: u32) {
        self.items.retain(|alarm| alarm.id != id);
    }

    pub fn toggle(&mut self, id: u32, now: DateTime<Utc>) {
        if let Some(alarm) = self.items.iter_mut().find(|alarm| alarm.id == id) {
            alarm.next = match alarm.next {
                Some(_) => None,
                None => Some(next_occurrence(alarm.time, alarm.zone, now)),
            };
        }
    }

    pub fn toggle_daily(&mut self, id: u32) {
        if let Some(alarm) = self.items.iter_mut().find(|alarm| alarm.id == id) {
            alarm.daily = !alarm.daily;
        }
    }

    /// The alarms that ring at `now`. Daily ones move on to their next day,
    /// timers go away and other alarms switch off.
    pub fn take_due(&mut self, now: DateTime<Utc>) -> Vec<Alarm> {
        let mut due = Vec::new();
        self.items.retain_mut(|alarm| {
            if alarm.next.is_none_or(|next| next > now) {
                return true;
            }
            due.push(alarm.clone());
            alarm.next = alarm
                .daily
                .then(|| next_occurrence(alarm.time, alarm.zone, now));
            alarm.daily || !alarm.timer
        });
        due
    }

    /// The enabled alarm that rings first.
    pub fn upcoming(&self) -> Option<&Alarm> {
        self.items
            .iter()
            .filter(|alarm| alarm.enabled())
            .min_by_key(|alarm| alarm.next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn utc(y: i32, m: u32, d: u32, h: u32, min: u32, s: u32) -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(y, m, d)
            .unwrap()
            .and_hms_opt(h, min, s)
            .unwrap()
            .and_utc()
    }

    fn hm(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    fn at(input: &str) -> NaiveTime {
        match parse(input) {
            Ok((When::At(time), _)) => time,
            other => panic!("{input:?} gave {other:?}"),
        }
    }

    const SAO_PAULO: Zone = Zone::City(chrono_tz::America::Sao_Paulo);

    #[test]
    fn reads_clock_times() {
        for input in [
            "07:30", "7:30", "7h30", "7.30", "0730", "730", "7:30am", "7:30 AM",
        ] {
            assert_eq!(at(input), hm(7, 30), "{input}");
        }
        assert_eq!(at("7"), hm(7, 0));
        assert_eq!(at("7h"), hm(7, 0));
        assert_eq!(at("19"), hm(19, 0));
        assert_eq!(at("7:30pm"), hm(19, 30));
        assert_eq!(at("12am"), hm(0, 0));
        assert_eq!(at("12 pm"), hm(12, 0));
        assert_eq!(at("23:59:30"), NaiveTime::from_hms_opt(23, 59, 30).unwrap());
    }

    #[test]
    fn rejects_impossible_times() {
        for input in [
            "",
            "24:00",
            "7:60",
            "13pm",
            "0am",
            "abc",
            "h30",
            "12345",
            "7:30:15:01",
            "1:2x",
        ] {
            assert_eq!(parse(input), Err(InvalidTime), "{input}");
        }
    }

    #[test]
    fn reads_delays() {
        let delay = |input: &str| match parse(input) {
            Ok((When::In(delay), _)) => delay.num_seconds(),
            other => panic!("{input:?} gave {other:?}"),
        };
        assert_eq!(delay("+10"), 600);
        assert_eq!(delay("+10m"), 600);
        assert_eq!(delay("+90s"), 90);
        assert_eq!(delay("+1h"), 3600);
        assert_eq!(delay("+1h30"), 5400);
        assert_eq!(delay("+2m30"), 150);
        assert_eq!(delay("+1H30M"), 5400);
        for input in ["+", "+0", "+5x", "+1s30", "+8d", "+999999999999999999999"] {
            assert_eq!(parse(input), Err(InvalidTime), "{input}");
        }
    }

    #[test]
    fn keeps_the_label() {
        assert_eq!(
            parse("  7:30 pm  Call  mom "),
            Ok((When::At(hm(19, 30)), "Call  mom".to_owned()))
        );
        assert_eq!(
            parse("+25m Pomodoro"),
            Ok((When::In(Duration::minutes(25)), "Pomodoro".to_owned()))
        );
        assert_eq!(parse("7h Acordar").unwrap().1, "Acordar");
        assert_eq!(parse("8 Amanda").unwrap().1, "Amanda");
    }

    #[test]
    fn rings_at_the_next_matching_wall_time() {
        // 10:00 in São Paulo (UTC-3) is 13:00 UTC.
        let now = utc(2026, 9, 25, 12, 0, 0);
        assert_eq!(
            next_occurrence(hm(10, 0), SAO_PAULO, now),
            utc(2026, 9, 25, 13, 0, 0)
        );
        assert_eq!(
            next_occurrence(hm(8, 0), SAO_PAULO, now),
            utc(2026, 9, 26, 11, 0, 0)
        );
        // Exactly now means tomorrow.
        assert_eq!(
            next_occurrence(hm(9, 0), SAO_PAULO, now),
            utc(2026, 9, 26, 12, 0, 0)
        );
    }

    #[test]
    fn moves_times_skipped_by_dst_forward() {
        // New York skips 02:00-03:00 on 2026-03-08; 03:00 EDT is 07:00 UTC.
        let new_york = Zone::City(chrono_tz::America::New_York);
        let now = utc(2026, 3, 8, 5, 0, 0);
        assert_eq!(
            next_occurrence(hm(2, 30), new_york, now),
            utc(2026, 3, 8, 7, 0, 0)
        );
    }

    #[test]
    fn one_off_alarms_ring_once() {
        let mut alarms = Alarms::default();
        let now = utc(2026, 9, 25, 12, 0, 0);
        let id = alarms.add(
            When::At(hm(9, 30)),
            "Coffee".into(),
            SAO_PAULO,
            "São Paulo".into(),
            now,
        );
        assert!(alarms.take_due(utc(2026, 9, 25, 12, 29, 59)).is_empty());

        let due = alarms.take_due(utc(2026, 9, 25, 12, 30, 0));
        assert_eq!(due.len(), 1);
        assert_eq!((due[0].id, due[0].label.as_str()), (id, "Coffee"));
        assert!(!alarms.list()[0].enabled());
        assert!(alarms.take_due(utc(2026, 9, 26, 12, 30, 0)).is_empty());
    }

    #[test]
    fn daily_alarms_move_to_the_next_day() {
        let mut alarms = Alarms::default();
        let now = utc(2026, 9, 25, 12, 0, 0);
        let id = alarms.add(
            When::At(hm(9, 30)),
            String::new(),
            SAO_PAULO,
            String::new(),
            now,
        );
        alarms.toggle_daily(id);
        // Rings even when checked late (e.g. after the computer slept).
        assert_eq!(alarms.take_due(utc(2026, 9, 25, 14, 0, 0)).len(), 1);
        assert_eq!(alarms.list()[0].next, Some(utc(2026, 9, 26, 12, 30, 0)));
    }

    #[test]
    fn delays_count_from_now() {
        let mut alarms = Alarms::default();
        let now = utc(2026, 9, 25, 12, 0, 0) + Duration::milliseconds(700);
        alarms.add(
            When::In(Duration::minutes(10)),
            String::new(),
            SAO_PAULO,
            String::new(),
            now,
        );
        let alarm = &alarms.list()[0];
        assert_eq!(alarm.next, Some(utc(2026, 9, 25, 12, 10, 0)));
        assert_eq!(alarm.time, hm(9, 10));
    }

    #[test]
    fn timers_go_away_after_ringing() {
        let mut alarms = Alarms::default();
        let now = utc(2026, 9, 25, 12, 0, 0);
        alarms.add(
            When::In(Duration::minutes(10)),
            String::new(),
            SAO_PAULO,
            String::new(),
            now,
        );
        alarms.add(
            When::At(hm(9, 10)),
            String::new(),
            SAO_PAULO,
            String::new(),
            now,
        );
        assert_eq!(alarms.take_due(utc(2026, 9, 25, 12, 10, 0)).len(), 2);
        assert_eq!(alarms.list().len(), 1);
        assert!(!alarms.list()[0].timer);
    }

    #[test]
    fn switches_alarms_off_and_on() {
        let mut alarms = Alarms::default();
        let now = utc(2026, 9, 25, 12, 0, 0);
        let early = alarms.add(
            When::At(hm(9, 30)),
            String::new(),
            SAO_PAULO,
            String::new(),
            now,
        );
        let late = alarms.add(
            When::At(hm(22, 0)),
            String::new(),
            SAO_PAULO,
            String::new(),
            now,
        );
        assert_eq!(alarms.upcoming().map(|a| a.id), Some(early));

        alarms.toggle(early, now);
        assert_eq!(alarms.upcoming().map(|a| a.id), Some(late));
        alarms.toggle(early, now);
        assert_eq!(alarms.upcoming().map(|a| a.id), Some(early));

        alarms.remove(early);
        alarms.remove(late);
        assert!(alarms.upcoming().is_none());
    }
}
