//! Application state and keyboard handling.

use chrono::{DateTime, Duration, Utc};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;

use crate::alarm::{self, Alarm, Alarms, When};
use crate::cities::{self, City};
use crate::i18n::Lang;
use crate::worldmap::MapStyle;
use crate::zone::Zone;

const SNOOZE_MINUTES: i64 = 5;
const SEARCH_RESULTS: usize = 100;
const PAGE: usize = 10;

pub enum Mode {
    Clock,
    Search(Search),
    Alarms(AlarmPanel),
    Help,
}

#[derive(Default)]
pub struct Search {
    pub query: String,
    pub results: Vec<City>,
    pub selected: usize,
}

impl Search {
    fn update(&mut self) {
        self.results = cities::db().search(&self.query, SEARCH_RESULTS);
        self.selected = 0;
    }
}

#[derive(Default)]
pub struct AlarmPanel {
    pub selected: usize,
    /// Text of the alarm being typed.
    pub input: Option<String>,
    /// The last attempt to save `input` failed.
    pub invalid: bool,
}

pub struct App {
    pub lang: Lang,
    /// The computer's own zone, restored with `l`.
    pub system: Zone,
    pub zone: Zone,
    pub city: Option<City>,
    pub show_map: bool,
    pub map_style: MapStyle,
    pub show_seconds: bool,
    pub color: Color,
    pub alarms: Alarms,
    pub ringing: Vec<Alarm>,
    pub mode: Mode,
    pub quit: bool,
    bell: bool,
}

impl App {
    pub fn new(lang: Lang, system: Zone) -> Self {
        Self {
            lang,
            system,
            zone: system,
            city: None,
            show_map: false,
            map_style: MapStyle::default(),
            show_seconds: true,
            color: Color::Cyan,
            alarms: Alarms::default(),
            ringing: Vec::new(),
            mode: Mode::Clock,
            quit: false,
            bell: false,
        }
    }

    pub fn select_city(&mut self, city: City) {
        self.city = Some(city);
        self.zone = Zone::City(city.tz);
        self.show_map = true;
    }

    pub fn use_system_time(&mut self) {
        self.city = None;
        self.zone = self.system;
        self.show_map = false;
    }

    /// Adds an alarm in the zone on screen.
    pub fn add_alarm(&mut self, when: When, label: String, now: DateTime<Utc>) -> u32 {
        let place = match self.city {
            Some(city) => city.name.to_owned(),
            None => self.lang.text().local.to_owned(),
        };
        self.alarms.add(when, label, self.zone, place, now)
    }

    /// Starts ringing the alarms that are due.
    pub fn tick(&mut self, now: DateTime<Utc>) {
        let due = self.alarms.take_due(now);
        if !due.is_empty() {
            self.bell |= self.ringing.is_empty();
            self.ringing.extend(due);
            self.mode = Mode::Clock;
        }
    }

    /// Whether a terminal bell is owed, clearing it.
    pub fn take_bell(&mut self) -> bool {
        std::mem::take(&mut self.bell)
    }

    /// While an alarm rings the screen flashes twice a second.
    pub fn blink_on(&self, now: DateTime<Utc>) -> bool {
        !self.ringing.is_empty() && now.timestamp_subsec_millis() < 500
    }

    /// How long to wait for a key before the screen needs redrawing: until the
    /// next second, or the next blink while an alarm rings.
    pub fn next_wakeup(&self, now: DateTime<Utc>) -> std::time::Duration {
        let period = if self.ringing.is_empty() { 1000 } else { 500 };
        let elapsed = now.timestamp_subsec_millis() % period;
        std::time::Duration::from_millis(u64::from(period - elapsed) + 5)
    }

    pub fn on_key(&mut self, key: KeyEvent, now: DateTime<Utc>) {
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if ctrl && key.code == KeyCode::Char('c') {
            self.quit = true;
            return;
        }
        if !self.ringing.is_empty() {
            if key.code == KeyCode::Char('z') {
                self.snooze(now);
            }
            self.ringing.clear();
            return;
        }
        self.mode = match std::mem::replace(&mut self.mode, Mode::Clock) {
            Mode::Clock => self.clock_key(key),
            Mode::Search(search) => self.search_key(search, key, ctrl),
            Mode::Alarms(panel) => self.alarms_key(panel, key, ctrl, now),
            Mode::Help => Mode::Clock,
        };
    }

    fn clock_key(&mut self, key: KeyEvent) -> Mode {
        let KeyCode::Char(ch) = key.code else {
            return Mode::Clock;
        };
        match ch.to_ascii_lowercase() {
            'q' => self.quit = true,
            'c' | '/' => return Mode::Search(Search::default()),
            'l' => self.use_system_time(),
            'a' => {
                let input = self.alarms.list().is_empty().then(String::new);
                return Mode::Alarms(AlarmPanel {
                    input,
                    ..AlarmPanel::default()
                });
            }
            'm' => self.show_map = !self.show_map,
            'v' => self.map_style = self.map_style.next(),
            's' => self.show_seconds = !self.show_seconds,
            '?' | 'h' => return Mode::Help,
            _ => {}
        }
        Mode::Clock
    }

    fn search_key(&mut self, mut search: Search, key: KeyEvent, ctrl: bool) -> Mode {
        let last = search.results.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc => return Mode::Clock,
            KeyCode::Enter => {
                if let Some(city) = search.results.get(search.selected) {
                    self.select_city(*city);
                    return Mode::Clock;
                }
            }
            KeyCode::Up => search.selected = search.selected.saturating_sub(1),
            KeyCode::Down => search.selected = (search.selected + 1).min(last),
            KeyCode::PageUp => search.selected = search.selected.saturating_sub(PAGE),
            KeyCode::PageDown => search.selected = (search.selected + PAGE).min(last),
            KeyCode::Char('p') if ctrl => search.selected = search.selected.saturating_sub(1),
            KeyCode::Char('n') if ctrl => search.selected = (search.selected + 1).min(last),
            KeyCode::Char('u') if ctrl => {
                search.query.clear();
                search.update();
            }
            KeyCode::Char('w') if ctrl => {
                let kept = search
                    .query
                    .trim_end()
                    .rfind(' ')
                    .map_or(0, |space| space + 1);
                search.query.truncate(kept);
                search.update();
            }
            KeyCode::Backspace => {
                search.query.pop();
                search.update();
            }
            KeyCode::Char(ch) if !ctrl => {
                search.query.push(ch);
                search.update();
            }
            _ => {}
        }
        Mode::Search(search)
    }

    fn alarms_key(
        &mut self,
        mut panel: AlarmPanel,
        key: KeyEvent,
        ctrl: bool,
        now: DateTime<Utc>,
    ) -> Mode {
        if let Some(input) = &mut panel.input {
            match key.code {
                KeyCode::Esc => panel.input = None,
                KeyCode::Enter => match alarm::parse(input) {
                    Ok((when, label)) => {
                        let id = self.add_alarm(when, label, now);
                        panel.selected = self.position(id);
                        panel.input = None;
                    }
                    Err(_) => {
                        panel.invalid = true;
                        return Mode::Alarms(panel);
                    }
                },
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char('u') if ctrl => input.clear(),
                KeyCode::Char(ch) if !ctrl => input.push(ch),
                _ => {}
            }
            panel.invalid = false;
            return Mode::Alarms(panel);
        }

        let count = self.alarms.list().len();
        let selected = self.alarms.list().get(panel.selected).map(|alarm| alarm.id);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q' | 'a') => return Mode::Clock,
            KeyCode::Char('n' | '+') | KeyCode::Insert => panel.input = Some(String::new()),
            KeyCode::Up | KeyCode::Char('k') => panel.selected = panel.selected.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => {
                panel.selected = (panel.selected + 1).min(count.saturating_sub(1));
            }
            KeyCode::Char(' ') | KeyCode::Enter => {
                if let Some(id) = selected {
                    self.alarms.toggle(id, now);
                }
            }
            KeyCode::Char('r') => {
                if let Some(id) = selected {
                    self.alarms.toggle_daily(id);
                }
            }
            KeyCode::Char('d') | KeyCode::Delete | KeyCode::Backspace => {
                if let Some(id) = selected {
                    self.alarms.remove(id);
                    panel.selected = panel.selected.min(count.saturating_sub(2));
                }
            }
            _ => {}
        }
        Mode::Alarms(panel)
    }

    fn position(&self, id: u32) -> usize {
        self.alarms
            .list()
            .iter()
            .position(|alarm| alarm.id == id)
            .unwrap_or_default()
    }

    /// Rings every ringing alarm again in five minutes.
    fn snooze(&mut self, now: DateTime<Utc>) {
        let word = self.lang.text().snooze;
        for alarm in std::mem::take(&mut self.ringing) {
            let label = match alarm.label.as_str() {
                "" => word.to_owned(),
                label if label == word || label.ends_with(&format!("({word})")) => alarm.label,
                label => format!("{label} ({word})"),
            };
            self.alarms.add(
                When::In(Duration::minutes(SNOOZE_MINUTES)),
                label,
                alarm.zone,
                alarm.place,
                now,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use ratatui::crossterm::event::KeyEvent;

    fn now() -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(2026, 9, 25)
            .unwrap()
            .and_hms_opt(12, 0, 0)
            .unwrap()
            .and_utc()
    }

    fn app() -> App {
        App::new(Lang::En, Zone::City(chrono_tz::UTC))
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::from(code), now());
    }

    fn type_text(app: &mut App, text: &str) {
        for ch in text.chars() {
            press(app, KeyCode::Char(ch));
        }
    }

    #[test]
    fn picks_a_city_by_typing_its_name() {
        let mut app = app();
        press(&mut app, KeyCode::Char('c'));
        type_text(&mut app, "toquio");
        let Mode::Search(search) = &app.mode else {
            panic!("search not open")
        };
        assert_eq!(search.results[0].name, "Tokyo");
        press(&mut app, KeyCode::Enter);

        assert!(matches!(app.mode, Mode::Clock));
        assert_eq!(app.city.map(|c| c.name), Some("Tokyo"));
        assert_eq!(app.zone, Zone::City(chrono_tz::Asia::Tokyo));
        assert!(app.show_map, "a city brings up the map");

        press(&mut app, KeyCode::Char('l'));
        assert_eq!(app.zone, Zone::City(chrono_tz::UTC));
        assert!(app.city.is_none() && !app.show_map);
    }

    #[test]
    fn moves_through_the_results() {
        let mut app = app();
        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "portland");
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        let city = app.city.unwrap();
        assert_eq!((city.name, city.region), ("Portland", "Maine"));
    }

    #[test]
    fn escape_keeps_the_current_zone() {
        let mut app = app();
        press(&mut app, KeyCode::Char('c'));
        type_text(&mut app, "paris");
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.mode, Mode::Clock));
        assert!(app.city.is_none());
    }

    #[test]
    fn enter_without_results_keeps_searching() {
        let mut app = app();
        press(&mut app, KeyCode::Char('c'));
        type_text(&mut app, "zzqqxx");
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Search(_)));
    }

    #[test]
    fn edits_the_query() {
        let mut app = app();
        press(&mut app, KeyCode::Char('c'));
        type_text(&mut app, "rio de x");
        press(&mut app, KeyCode::Backspace);
        let ctrl = |ch| KeyEvent::new(KeyCode::Char(ch), KeyModifiers::CONTROL);
        app.on_key(ctrl('w'), now());
        let Mode::Search(search) = &app.mode else {
            panic!()
        };
        assert_eq!(search.query, "rio ");
        app.on_key(ctrl('u'), now());
        let Mode::Search(search) = &app.mode else {
            panic!()
        };
        assert!(search.query.is_empty() && search.results.is_empty());
    }

    #[test]
    fn sets_an_alarm_that_blinks_until_a_key_is_pressed() {
        let mut app = app();
        press(&mut app, KeyCode::Char('a'));
        // With no alarms the panel opens ready to type one.
        type_text(&mut app, "12:30 Lunch");
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.alarms.list()[0].label, "Lunch");

        let ring_time = now() + Duration::minutes(30);
        app.tick(ring_time - Duration::seconds(1));
        assert!(app.ringing.is_empty());
        app.tick(ring_time);
        assert_eq!(app.ringing.len(), 1);
        assert!(app.take_bell() && !app.take_bell(), "one bell per alarm");
        assert!(app.blink_on(ring_time));
        assert!(!app.blink_on(ring_time + Duration::milliseconds(600)));
        assert!(app.blink_on(ring_time + Duration::milliseconds(1100)));

        // Any key stops it without doing anything else.
        press(&mut app, KeyCode::Char('q'));
        assert!(app.ringing.is_empty() && !app.quit);
        assert!(!app.blink_on(ring_time));
    }

    #[test]
    fn snoozes_for_five_minutes() {
        let mut app = app();
        app.add_alarm(When::In(Duration::minutes(1)), "Tea".into(), now());
        app.tick(now() + Duration::minutes(1));
        press(&mut app, KeyCode::Char('z'));
        assert!(app.ringing.is_empty());
        let snoozed = app.alarms.upcoming().unwrap();
        assert_eq!(snoozed.label, "Tea (snooze)");
        assert_eq!(snoozed.next, Some(now() + Duration::minutes(5)));
    }

    #[test]
    fn rejects_invalid_alarms_and_keeps_the_text() {
        let mut app = app();
        press(&mut app, KeyCode::Char('a'));
        type_text(&mut app, "25:00");
        press(&mut app, KeyCode::Enter);
        let Mode::Alarms(panel) = &app.mode else {
            panic!()
        };
        assert!(panel.invalid);
        assert_eq!(panel.input.as_deref(), Some("25:00"));
        assert!(app.alarms.list().is_empty());
    }

    #[test]
    fn manages_alarms_from_the_panel() {
        let mut app = app();
        app.add_alarm(
            When::At(chrono::NaiveTime::from_hms_opt(8, 0, 0).unwrap()),
            String::new(),
            now(),
        );
        press(&mut app, KeyCode::Char('a'));
        press(&mut app, KeyCode::Char(' '));
        assert!(!app.alarms.list()[0].enabled());
        press(&mut app, KeyCode::Char('r'));
        assert!(app.alarms.list()[0].daily);
        press(&mut app, KeyCode::Char('d'));
        assert!(app.alarms.list().is_empty());
    }

    #[test]
    fn toggles_display_options() {
        let mut app = app();
        press(&mut app, KeyCode::Char('m'));
        press(&mut app, KeyCode::Char('v'));
        press(&mut app, KeyCode::Char('s'));
        assert!(app.show_map && !app.show_seconds);
        assert_eq!(app.map_style, MapStyle::Braille);
        press(&mut app, KeyCode::Char('?'));
        assert!(matches!(app.mode, Mode::Help));
        press(&mut app, KeyCode::Char('x'));
        assert!(matches!(app.mode, Mode::Clock));
        press(&mut app, KeyCode::Char('q'));
        assert!(app.quit);
    }

    #[test]
    fn wakes_up_on_the_next_second_or_blink() {
        let mut app = app();
        let at = now() + Duration::milliseconds(300);
        assert_eq!(app.next_wakeup(at).as_millis(), 705);
        app.add_alarm(When::In(Duration::seconds(1)), String::new(), now());
        app.tick(now() + Duration::seconds(1));
        assert_eq!(app.next_wakeup(at).as_millis(), 205);
    }
}
