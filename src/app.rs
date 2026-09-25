//! Application state and keyboard handling.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use clap::ValueEnum;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Color;

use crate::alarm::{self, Alarm, Alarms, When};
use crate::calendar::{self, Event};
use crate::cities::{self, City, fold};
use crate::i18n::Lang;
use crate::maccal::{self, Access};
use crate::net::{Answer, Net};
use crate::state::{SavedAlarm, SavedCity, State};
use crate::theme::{self, Palette, THEMES};
use crate::weather::{self, Forecast};
use crate::worldmap::MapStyle;
use crate::zone::Zone;

const SNOOZE_MINUTES: i64 = 5;
const SEARCH_RESULTS: usize = 100;
const PAGE: usize = 10;

pub enum Mode {
    Clock,
    Search(Search),
    Alarms(AlarmPanel),
    Themes(ThemePicker),
    Calendar(CalendarPanel),
    Help,
}

/// Where calendar events come from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalendarSource {
    /// The macOS Calendar app, with whatever accounts it syncs.
    Mac,
    /// An iCal (.ics) address or file.
    Ical,
}

/// The sources this system offers, in the order they are listed.
pub fn calendar_sources() -> &'static [CalendarSource] {
    if maccal::SUPPORTED {
        &[CalendarSource::Mac, CalendarSource::Ical]
    } else {
        &[CalendarSource::Ical]
    }
}

/// Download key of the macOS Calendar.
const MAC_KEY: &str = "calendar:mac";

/// The calendar pop-up.
pub enum CalendarPanel {
    /// Nothing connected yet: pick a source.
    Choose(usize),
    /// Typing an iCal address.
    Address(String),
    /// Connected: the source, the reminder and the next events.
    Status,
}

/// The calendar feed, its next events and the reminders already rung.
pub struct CalendarSlot {
    /// Secret iCal address or .ics file; `None` when not connected.
    pub address: Option<String>,
    /// Minutes before an event its reminder rings; 0 switches reminders off.
    pub reminder: u32,
    pub events: Vec<Event>,
    /// Last successful download.
    pub updated: Option<DateTime<Utc>>,
    pub error: Option<String>,
    /// Occurrences whose reminder already rang, by `Event::key`.
    pub rung: Vec<String>,
    /// The macOS Calendar is the source.
    pub mac: bool,
    /// What macOS last said about access to the calendars.
    pub access: Option<Access>,
    /// Waiting for the user to answer macOS's prompt.
    pub asking: bool,
    asked: Option<DateTime<Utc>>,
    pending: bool,
}

impl Default for CalendarSlot {
    fn default() -> Self {
        Self {
            mac: false,
            access: None,
            asking: false,
            address: None,
            reminder: 5,
            events: Vec::new(),
            updated: None,
            error: None,
            rung: Vec::new(),
            asked: None,
            pending: false,
        }
    }
}

impl CalendarSlot {
    /// The next event that has not started yet.
    pub fn next(&self, now: DateTime<Utc>) -> Option<&Event> {
        self.events.iter().find(|event| event.start > now)
    }

    pub fn connected(&self) -> bool {
        self.mac || self.address.is_some()
    }
}

#[derive(Default)]
pub struct Search {
    pub query: String,
    pub results: Vec<City>,
    pub selected: usize,
    /// Picking the city of the local time tab rather than opening a tab.
    pub home: bool,
}

impl Search {
    /// The search for the city of the local time tab, started from the
    /// current one or the city the computer's time zone is named after.
    fn home(app: &App) -> Self {
        let query = match (app.home, app.system.tz()) {
            (Some(city), _) => city.name.to_owned(),
            (None, Some(tz)) => tz
                .name()
                .rsplit('/')
                .next()
                .unwrap_or_default()
                .replace('_', " "),
            (None, None) => String::new(),
        };
        let mut search = Self {
            query,
            home: true,
            ..Self::default()
        };
        search.update();
        search
    }

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

/// The theme list. Moving through it shows each theme at once; Esc goes back
/// to the one in use before.
pub struct ThemePicker {
    pub query: String,
    /// Indices into `THEMES` whose names contain the query.
    pub matches: Vec<usize>,
    pub selected: usize,
    original: usize,
}

impl ThemePicker {
    fn new(current: usize) -> Self {
        Self {
            query: String::new(),
            matches: (0..THEMES.len()).collect(),
            selected: current,
            original: current,
        }
    }

    fn filter(&mut self) {
        let wanted = fold(&self.query);
        self.matches = (0..THEMES.len())
            .filter(|&index| fold(THEMES[index].name).contains(&wanted))
            .collect();
        self.selected = 0;
    }

    pub fn current(&self) -> Option<usize> {
        self.matches.get(self.selected).copied()
    }
}

/// A city's forecast and when it was last asked for.
#[derive(Default)]
pub struct WeatherSlot {
    pub forecast: Option<Forecast>,
    /// Why the last download failed, if it did.
    pub failed: Option<String>,
    asked: Option<DateTime<Utc>>,
    pending: bool,
}

impl WeatherSlot {
    /// A download is under way.
    pub fn pending(&self) -> bool {
        self.pending
    }
}

#[cfg(test)]
impl WeatherSlot {
    pub fn ready(forecast: Forecast) -> Self {
        Self {
            forecast: Some(forecast),
            ..Self::default()
        }
    }
}

/// One clock: the computer's time or a city's, each with its own map switch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tab {
    /// `None` for the computer's time.
    pub city: Option<City>,
    pub show_map: bool,
}

pub struct App {
    pub lang: Lang,
    /// The computer's own zone.
    pub system: Zone,
    /// The first tab shows the computer's time, on offer only while no city
    /// is open; every city opened gets its own after it.
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub map_style: MapStyle,
    pub show_seconds: bool,
    /// Index into `THEMES`.
    pub theme: usize,
    /// Whether the terminal shows 24-bit color; otherwise themes use the 256-color palette.
    pub truecolor: bool,
    /// Digit color asked for on the command line, over the theme's.
    pub digits_color: Option<Color>,
    /// The last attempt to save the state failed.
    pub unsaved: bool,
    pub show_weather: bool,
    /// Where the local time tab is, for its weather.
    pub home: Option<City>,
    /// Forecasts by `weather::key`.
    pub weather: HashMap<String, WeatherSlot>,
    /// Downloads; `None` keeps the app offline.
    pub net: Option<Net>,
    pub calendar: CalendarSlot,
    /// Opens a web address or a System Settings page.
    pub open: Box<dyn Fn(&str) -> bool>,
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
            tabs: vec![Tab {
                city: None,
                show_map: false,
            }],
            active: 0,
            map_style: MapStyle::default(),
            show_seconds: true,
            theme: 0,
            truecolor: true,
            digits_color: None,
            unsaved: false,
            show_weather: true,
            home: None,
            weather: HashMap::new(),
            net: None,
            calendar: CalendarSlot::default(),
            open: Box::new(crate::desktop::open),
            alarms: Alarms::default(),
            ringing: Vec::new(),
            mode: Mode::Clock,
            quit: false,
            bell: false,
        }
    }

    pub fn palette(&self) -> Palette {
        let mut palette = THEMES[self.theme].palette(self.truecolor);
        if let Some(color) = self.digits_color {
            palette.digits = color;
        }
        palette
    }

    /// What is saved between runs. While a theme is being picked, the one in
    /// use before counts.
    pub fn state(&self) -> State {
        let theme = match &self.mode {
            Mode::Themes(picker) => picker.original,
            _ => self.theme,
        };
        State {
            theme: THEMES[theme].name.to_owned(),
            map_style: self
                .map_style
                .to_possible_value()
                .map(|value| value.get_name().to_owned())
                .unwrap_or_default(),
            seconds: self.show_seconds,
            weather: self.show_weather,
            home: self.home.map(|city| SavedCity {
                name: city.name.to_owned(),
                region: city.region.to_owned(),
                country: city.country.to_owned(),
                lat: city.lat,
                lon: city.lon,
                map: true,
            }),
            calendar: self.calendar.address.clone(),
            mac_calendar: self.calendar.mac,
            reminder: self.calendar.reminder,
            rung: self.calendar.rung.clone(),
            local_map: self.tabs[0].show_map,
            active: self.active,
            cities: self
                .tabs
                .iter()
                .filter_map(|tab| {
                    let city = tab.city?;
                    Some(SavedCity {
                        name: city.name.to_owned(),
                        region: city.region.to_owned(),
                        country: city.country.to_owned(),
                        lat: city.lat,
                        lon: city.lon,
                        map: tab.show_map,
                    })
                })
                .collect(),
            alarms: self
                .alarms
                .list()
                .iter()
                .map(|alarm| SavedAlarm {
                    time: alarm.time,
                    zone: match alarm.zone {
                        Zone::City(tz) => Some(tz.name().to_owned()),
                        Zone::System(_) => None,
                    },
                    place: alarm.place.clone(),
                    label: alarm.label.clone(),
                    daily: alarm.daily,
                    timer: alarm.timer,
                    next: alarm.next,
                })
                .collect(),
        }
    }

    /// Puts back a saved state. With `with_active` the tab on screen comes back
    /// too; otherwise the city on screen stays, wherever its tab moved.
    pub fn restore(&mut self, state: &State, with_active: bool) {
        if !matches!(self.mode, Mode::Themes(_))
            && let Some(theme) = theme::find(&state.theme)
        {
            self.theme = theme;
        }
        if let Ok(style) = MapStyle::from_str(&state.map_style, true) {
            self.map_style = style;
        }
        self.show_seconds = state.seconds;
        self.show_weather = state.weather;
        self.home = state
            .home
            .as_ref()
            .and_then(|saved| cities::db().find_saved(&saved.name, saved.lat, saved.lon));
        if self.calendar.address != state.calendar || self.calendar.mac != state.mac_calendar {
            self.set_calendar(state.calendar.clone());
            self.calendar.mac = state.mac_calendar;
        }
        self.calendar.reminder = state.reminder;
        self.calendar.rung = state.rung.clone();

        let on_screen = self.city();
        let mut tabs = vec![Tab {
            city: None,
            show_map: state.local_map,
        }];
        for saved in &state.cities {
            if let Some(city) = cities::db().find_saved(&saved.name, saved.lat, saved.lon)
                && !tabs.iter().any(|tab| tab.city == Some(city))
            {
                tabs.push(Tab {
                    city: Some(city),
                    show_map: saved.map,
                });
            }
        }
        let active = match (with_active, on_screen) {
            (true, _) => state.active,
            (false, None) => 0,
            (false, Some(city)) => tabs
                .iter()
                .position(|tab| tab.city == Some(city))
                .unwrap_or(self.active),
        };
        self.active = active.min(tabs.len() - 1);
        self.tabs = tabs;
        self.settle();

        self.alarms = Alarms::default();
        for saved in &state.alarms {
            let zone = match saved.zone.as_deref().map(str::parse) {
                Some(Ok(tz)) => Zone::City(tz),
                _ => self.system,
            };
            self.alarms.insert(Alarm {
                id: 0,
                time: saved.time,
                zone,
                place: saved.place.clone(),
                label: saved.label.clone(),
                daily: saved.daily,
                timer: saved.timer,
                next: saved.next,
            });
        }
    }

    pub fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    pub fn city(&self) -> Option<City> {
        self.tab().city
    }

    pub fn zone(&self) -> Zone {
        self.zone_of(self.tab())
    }

    pub fn zone_of(&self, tab: &Tab) -> Zone {
        tab.city.map_or(self.system, |city| Zone::City(city.tz))
    }

    /// Switches to the tab of `city`, opening one (with the map showing) if there is none.
    pub fn open_city(&mut self, city: City) {
        self.active = match self.tabs.iter().position(|tab| tab.city == Some(city)) {
            Some(index) => index,
            None => {
                self.tabs.push(Tab {
                    city: Some(city),
                    show_map: true,
                });
                self.tabs.len() - 1
            }
        };
    }

    /// The tabs on offer: the cities, or the computer's time when none is open.
    pub fn shown_tabs(&self) -> std::ops::Range<usize> {
        if self.tabs.len() > 1 {
            1..self.tabs.len()
        } else {
            0..1
        }
    }

    /// Keeps the tab on screen among the ones on offer.
    fn settle(&mut self) {
        let shown = self.shown_tabs();
        self.active = self.active.clamp(shown.start, shown.end - 1);
    }

    /// Closes the city on screen; closing the last one brings back the
    /// computer's time.
    fn close_tab(&mut self) {
        if self.active > 0 {
            self.tabs.remove(self.active);
            self.settle();
        }
    }

    /// Moves `step` tabs to the right (left when negative), wrapping around.
    fn switch_tab(&mut self, step: isize) {
        let shown = self.shown_tabs();
        let at = (self.active - shown.start) as isize;
        self.active = shown.start + (at + step).rem_euclid(shown.len() as isize) as usize;
    }

    /// Adds an alarm in the zone on screen.
    pub fn add_alarm(&mut self, when: When, label: String, now: DateTime<Utc>) -> u32 {
        let place = match self.city() {
            Some(city) => city.name.to_owned(),
            None => self.lang.text().local.to_owned(),
        };
        self.alarms.add(when, label, self.zone(), place, now)
    }

    /// Reads the macOS Calendar from now on, asking macOS for access.
    fn connect_mac(&mut self, now: DateTime<Utc>) {
        self.set_calendar(None);
        self.calendar.mac = true;
        self.ask_mac_access(now);
    }

    fn ask_mac_access(&mut self, now: DateTime<Utc>) {
        if let Some(net) = &self.net {
            net.mac_calendar(MAC_KEY.to_owned(), true);
            self.calendar.asking = true;
            self.calendar.pending = true;
            self.calendar.asked = Some(now);
        }
    }

    /// Connects to an iCal address, or disconnects with `None`.
    pub fn set_calendar(&mut self, address: Option<String>) {
        self.calendar = CalendarSlot {
            address,
            reminder: self.calendar.reminder,
            rung: std::mem::take(&mut self.calendar.rung),
            ..CalendarSlot::default()
        };
    }

    pub fn weather_of(&self, city: &City) -> Option<&WeatherSlot> {
        self.weather.get(&weather::key(city))
    }

    /// The forecast of a city, once downloaded.
    #[cfg(test)]
    pub fn forecast(&self, city: &City) -> Option<&Forecast> {
        self.weather_of(city)?.forecast.as_ref()
    }

    /// Takes in finished downloads and asks for the forecasts that are due:
    /// every 30 minutes, or 5 after a failure.
    fn download(&mut self, now: DateTime<Utc>) {
        let Some(net) = &self.net else {
            return;
        };
        let calendar_key = self.calendar.address.as_deref().map(calendar::key);
        for (key, answer) in net.finished() {
            let result = match answer {
                Answer::Text(result) => result,
                Answer::MacCalendar(result) => {
                    if key == MAC_KEY && self.calendar.mac {
                        let calendar = &mut self.calendar;
                        calendar.pending = false;
                        calendar.asking = false;
                        match result {
                            Ok(events) => {
                                calendar.events = events;
                                calendar.updated = Some(now);
                                calendar.error = None;
                                calendar.access = Some(Access::Granted);
                            }
                            Err(refusal) => {
                                calendar.events.clear();
                                calendar.access = Some(refusal.access);
                                calendar.error = match (refusal.access, refusal.detail) {
                                    // Simply not asked yet: nothing went wrong.
                                    (Access::NotAsked, None) => None,
                                    (_, Some(detail)) => Some(detail),
                                    (_, None) => Some("no access to the calendars".into()),
                                };
                            }
                        }
                    }
                    continue;
                }
            };
            if Some(&key) == calendar_key.as_ref() {
                self.calendar.pending = false;
                match result {
                    Ok(text) if text.contains("BEGIN:VCALENDAR") => {
                        let until = now + Duration::days(calendar::WINDOW_DAYS);
                        self.calendar.events = calendar::upcoming(&text, now, until, self.system);
                        self.calendar.updated = Some(now);
                        self.calendar.error = None;
                    }
                    Ok(_) => self.calendar.error = Some("not an iCal feed".into()),
                    Err(error) => self.calendar.error = Some(error),
                }
                continue;
            }
            if let Some(slot) = self.weather.get_mut(&key) {
                slot.pending = false;
                match result.and_then(|text| weather::parse(&text)) {
                    Ok(forecast) => {
                        slot.forecast = Some(forecast);
                        slot.failed = None;
                    }
                    Err(error) => slot.failed = Some(error),
                }
            }
        }
        if self.calendar.mac {
            // Local and quick to read: every minute.
            let calendar = &mut self.calendar;
            if !calendar.pending
                && calendar
                    .asked
                    .is_none_or(|asked| now - asked >= Duration::minutes(1))
            {
                net.mac_calendar(MAC_KEY.to_owned(), false);
                calendar.pending = true;
                calendar.asked = Some(now);
            }
        } else if let Some(address) = self.calendar.address.clone() {
            let wait = match self.calendar.error {
                Some(_) => Duration::minutes(2),
                None => Duration::minutes(5),
            };
            let calendar = &mut self.calendar;
            if !calendar.pending && calendar.asked.is_none_or(|asked| now - asked >= wait) {
                net.fetch(calendar::key(&address), address);
                calendar.pending = true;
                calendar.asked = Some(now);
            }
        }
        if !self.show_weather {
            return;
        }
        for city in self.tabs.iter().filter_map(|tab| tab.city).chain(self.home) {
            let key = weather::key(&city);
            let slot = self.weather.entry(key.clone()).or_default();
            let wait = match slot.forecast {
                Some(_) => Duration::minutes(30),
                None => Duration::minutes(5),
            };
            if !slot.pending && slot.asked.is_none_or(|asked| now - asked >= wait) {
                net.fetch(key, weather::url(&city));
                slot.pending = true;
                slot.asked = Some(now);
            }
        }
    }

    /// Reminders of the events about to start, each rung once.
    fn due_reminders(&mut self, now: DateTime<Utc>) -> Vec<Alarm> {
        let lead = Duration::minutes(i64::from(self.calendar.reminder));
        let calendar = &mut self.calendar;
        // Forget reminders of events more than a day old.
        calendar.rung.retain(|key| {
            key.rsplit_once('@')
                .and_then(|(_, start)| start.parse().ok())
                .and_then(|start| DateTime::from_timestamp(start, 0))
                .is_some_and(|start| now - start < Duration::days(1))
        });
        if lead.is_zero() {
            return Vec::new();
        }
        let mut due = Vec::new();
        for event in &calendar.events {
            let key = event.key();
            if event.start > now && now >= event.start - lead && !calendar.rung.contains(&key) {
                calendar.rung.push(key);
                due.push(Alarm {
                    id: 0,
                    time: self.system.local_time(event.start).time(),
                    zone: self.system,
                    place: self.lang.text().calendar.to_owned(),
                    label: event.title.clone(),
                    daily: false,
                    timer: true,
                    next: None,
                });
            }
        }
        due
    }

    /// Starts ringing the alarms and reminders that are due and keeps the
    /// downloads going.
    pub fn tick(&mut self, now: DateTime<Utc>) {
        self.download(now);
        let mut due = self.alarms.take_due(now);
        due.extend(self.due_reminders(now));
        if !due.is_empty() {
            self.bell |= self.ringing.is_empty();
            self.ringing.extend(due);
            if let Mode::Themes(picker) = &self.mode {
                self.theme = picker.original;
            }
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
            Mode::Themes(picker) => self.themes_key(picker, key, ctrl),
            Mode::Calendar(panel) => self.calendar_key(panel, key, ctrl, now),
            Mode::Help => Mode::Clock,
        };
    }

    fn clock_key(&mut self, key: KeyEvent) -> Mode {
        let ch = match key.code {
            KeyCode::Right | KeyCode::Tab => {
                self.switch_tab(1);
                return Mode::Clock;
            }
            KeyCode::Left | KeyCode::BackTab => {
                self.switch_tab(-1);
                return Mode::Clock;
            }
            // Shift+W changes the city of the local time tab.
            KeyCode::Char('W') => return Mode::Search(Search::home(self)),
            KeyCode::Char(ch) => ch.to_ascii_lowercase(),
            _ => return Mode::Clock,
        };
        match ch {
            'q' => self.quit = true,
            'c' | '/' => return Mode::Search(Search::default()),
            'x' => self.close_tab(),
            '1'..='9' => {
                let shown = self.shown_tabs();
                let index = shown.start + usize::from(ch as u8 - b'1');
                if index < shown.end {
                    self.active = index;
                }
            }
            'a' => {
                let input = self.alarms.list().is_empty().then(String::new);
                return Mode::Alarms(AlarmPanel {
                    input,
                    ..AlarmPanel::default()
                });
            }
            'm' => {
                let tab = &mut self.tabs[self.active];
                tab.show_map = !tab.show_map;
            }
            't' => return Mode::Themes(ThemePicker::new(self.theme)),
            'g' => {
                return Mode::Calendar(match self.calendar.connected() {
                    true => CalendarPanel::Status,
                    false => CalendarPanel::Choose(0),
                });
            }
            'v' => {
                // The style only shows on the map, so a hidden map comes up first.
                let tab = &mut self.tabs[self.active];
                if tab.show_map {
                    self.map_style = self.map_style.next();
                } else {
                    tab.show_map = true;
                }
            }
            's' => self.show_seconds = !self.show_seconds,
            // The local time tab needs a place for its weather: ask for it first.
            'w' if self.active == 0 && self.home.is_none() => {
                return Mode::Search(Search::home(self));
            }
            'w' => self.show_weather = !self.show_weather,
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
                    if search.home {
                        self.home = Some(*city);
                        self.show_weather = true;
                    } else {
                        self.open_city(*city);
                    }
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

    fn calendar_key(
        &mut self,
        panel: CalendarPanel,
        key: KeyEvent,
        ctrl: bool,
        now: DateTime<Utc>,
    ) -> Mode {
        let panel = match panel {
            CalendarPanel::Choose(selected) => {
                let sources = calendar_sources();
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q' | 'g') => return Mode::Clock,
                    KeyCode::Up => CalendarPanel::Choose(selected.saturating_sub(1)),
                    KeyCode::Down => CalendarPanel::Choose((selected + 1).min(sources.len() - 1)),
                    KeyCode::Enter => match sources[selected] {
                        CalendarSource::Mac => {
                            self.connect_mac(now);
                            CalendarPanel::Status
                        }
                        CalendarSource::Ical => CalendarPanel::Address(String::new()),
                    },
                    _ => CalendarPanel::Choose(selected),
                }
            }
            CalendarPanel::Address(mut input) => match key.code {
                KeyCode::Esc if self.calendar.connected() => CalendarPanel::Status,
                KeyCode::Esc => CalendarPanel::Choose(0),
                KeyCode::Enter => {
                    let address = input.trim().to_owned();
                    if address.is_empty() {
                        CalendarPanel::Choose(0)
                    } else {
                        self.set_calendar(Some(address));
                        CalendarPanel::Status
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                    CalendarPanel::Address(input)
                }
                KeyCode::Char('u') if ctrl => CalendarPanel::Address(String::new()),
                KeyCode::Char(ch) if !ctrl => {
                    input.push(ch);
                    CalendarPanel::Address(input)
                }
                _ => CalendarPanel::Address(input),
            },
            CalendarPanel::Status => {
                let reminder = self.calendar.reminder;
                let mac = self.calendar.mac;
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q' | 'g') => return Mode::Clock,
                    KeyCode::Char('e') if !mac => {
                        return Mode::Calendar(CalendarPanel::Address(
                            self.calendar.address.clone().unwrap_or_default(),
                        ));
                    }
                    // In steps of five minutes, down to 0 (off) and up to an hour.
                    KeyCode::Char('+' | '=') => {
                        self.calendar.reminder = ((reminder / 5 + 1) * 5).min(60);
                    }
                    KeyCode::Char('-') => {
                        self.calendar.reminder = reminder.saturating_sub(1) / 5 * 5
                    }
                    KeyCode::Char('r') | KeyCode::Enter
                        if mac && self.calendar.access != Some(Access::Granted) =>
                    {
                        self.ask_mac_access(now);
                    }
                    KeyCode::Char('r') => self.calendar.asked = None,
                    KeyCode::Char('o') if mac => {
                        (self.open)(maccal::PRIVACY_SETTINGS);
                    }
                    KeyCode::Char('a') if mac => {
                        (self.open)(maccal::ACCOUNTS_SETTINGS);
                    }
                    KeyCode::Char('d') => {
                        self.set_calendar(None);
                        return Mode::Clock;
                    }
                    _ => {}
                }
                CalendarPanel::Status
            }
        };
        Mode::Calendar(panel)
    }

    fn themes_key(&mut self, mut picker: ThemePicker, key: KeyEvent, ctrl: bool) -> Mode {
        let last = picker.matches.len().saturating_sub(1);
        match key.code {
            KeyCode::Esc => {
                self.theme = picker.original;
                return Mode::Clock;
            }
            KeyCode::Enter if picker.current().is_some() => return Mode::Clock,
            KeyCode::Up => picker.selected = picker.selected.saturating_sub(1),
            KeyCode::Down => picker.selected = (picker.selected + 1).min(last),
            KeyCode::PageUp => picker.selected = picker.selected.saturating_sub(PAGE),
            KeyCode::PageDown => picker.selected = (picker.selected + PAGE).min(last),
            KeyCode::Backspace => {
                picker.query.pop();
                picker.filter();
            }
            KeyCode::Char('u') if ctrl => {
                picker.query.clear();
                picker.filter();
            }
            KeyCode::Char(ch) if !ctrl => {
                picker.query.push(ch);
                picker.filter();
            }
            _ => {}
        }
        if let Some(theme) = picker.current() {
            self.theme = theme;
        }
        Mode::Themes(picker)
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
    use crate::net::Job;
    use chrono::NaiveDate;
    use ratatui::crossterm::event::KeyEvent;
    use std::sync::{Arc, Mutex};

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

    fn city(query: &str) -> City {
        cities::db().search(query, 1)[0]
    }

    fn tab_names(app: &App) -> Vec<&'static str> {
        app.tabs
            .iter()
            .map(|tab| tab.city.map_or("local", |city| city.name))
            .collect()
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
        assert_eq!(app.city().map(|c| c.name), Some("Tokyo"));
        assert_eq!(app.zone(), Zone::City(chrono_tz::Asia::Tokyo));
        assert!(app.tab().show_map, "a city brings up the map");

        press(&mut app, KeyCode::Char('x'));
        assert_eq!(
            app.zone(),
            Zone::City(chrono_tz::UTC),
            "closing the only city brings back local time"
        );
        assert!(app.city().is_none() && !app.tab().show_map);
    }

    #[test]
    fn moves_through_the_results() {
        let mut app = app();
        press(&mut app, KeyCode::Char('/'));
        type_text(&mut app, "portland");
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        let city = app.city().unwrap();
        assert_eq!((city.name, city.region), ("Portland", "Maine"));
    }

    #[test]
    fn escape_keeps_the_current_tab() {
        let mut app = app();
        press(&mut app, KeyCode::Char('c'));
        type_text(&mut app, "paris");
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.mode, Mode::Clock));
        assert_eq!(tab_names(&app), ["local"]);
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
    fn opens_a_tab_per_city_and_cycles_with_the_arrows() {
        let mut app = app();
        for query in ["Tokyo", "London", "São Paulo"] {
            app.open_city(city(query));
        }
        assert_eq!(tab_names(&app), ["local", "Tokyo", "London", "São Paulo"]);
        assert_eq!(
            app.shown_tabs(),
            1..4,
            "local time steps aside for the cities"
        );
        assert_eq!(app.active, 3, "the city just opened is on screen");

        press(&mut app, KeyCode::Right);
        assert_eq!(
            app.city().map(|c| c.name),
            Some("Tokyo"),
            "wraps around the cities"
        );
        press(&mut app, KeyCode::Left);
        press(&mut app, KeyCode::Left);
        assert_eq!(app.city().map(|c| c.name), Some("London"));
        press(&mut app, KeyCode::Tab);
        assert_eq!(app.active, 3);
        press(&mut app, KeyCode::BackTab);
        assert_eq!(app.active, 2);

        press(&mut app, KeyCode::Char('1'));
        assert_eq!(app.city().map(|c| c.name), Some("Tokyo"));
        press(&mut app, KeyCode::Char('3'));
        assert_eq!(app.city().map(|c| c.name), Some("São Paulo"));
        press(&mut app, KeyCode::Char('9'));
        assert_eq!(app.active, 3, "no ninth city");
    }

    #[test]
    fn opening_a_city_again_goes_to_its_tab() {
        let mut app = app();
        app.open_city(city("Tokyo"));
        app.open_city(city("London"));
        press(&mut app, KeyCode::Char('c'));
        type_text(&mut app, "tokyo");
        press(&mut app, KeyCode::Enter);
        assert_eq!(tab_names(&app), ["local", "Tokyo", "London"]);
        assert_eq!(app.active, 1);
    }

    #[test]
    fn closing_the_last_city_brings_back_local_time() {
        let mut app = app();
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(
            tab_names(&app),
            ["local"],
            "local time itself does not close"
        );

        for query in ["Tokyo", "London", "Lima"] {
            app.open_city(city(query));
        }
        press(&mut app, KeyCode::Char('2'));
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(tab_names(&app), ["local", "Tokyo", "Lima"]);
        assert_eq!(
            app.city().map(|c| c.name),
            Some("Lima"),
            "the next city takes its place"
        );
        press(&mut app, KeyCode::Char('x'));
        assert_eq!(
            app.city().map(|c| c.name),
            Some("Tokyo"),
            "the last city falls back left"
        );
        press(&mut app, KeyCode::Char('x'));
        assert_eq!((tab_names(&app), app.active), (vec!["local"], 0));
        assert_eq!(app.zone(), Zone::City(chrono_tz::UTC));
    }

    #[test]
    fn each_tab_shows_or_hides_its_own_map() {
        let mut app = app();
        press(&mut app, KeyCode::Char('m'));
        assert!(app.tab().show_map, "the map opens in local time");
        app.open_city(city("Tokyo"));
        app.open_city(city("London"));
        press(&mut app, KeyCode::Char('m'));
        assert!(!app.tab().show_map, "the map closes in the London tab");
        press(&mut app, KeyCode::Left);
        assert!(app.tab().show_map, "the Tokyo tab keeps its own");
        press(&mut app, KeyCode::Char('x'));
        press(&mut app, KeyCode::Char('x'));
        assert!(
            app.city().is_none() && app.tab().show_map,
            "and local time its own"
        );
    }

    #[test]
    fn alarms_follow_the_tab_on_screen() {
        let mut app = app();
        app.open_city(city("Tokyo"));
        app.add_alarm(When::In(Duration::minutes(5)), String::new(), now());
        let alarm = &app.alarms.list()[0];
        assert_eq!(alarm.zone, Zone::City(chrono_tz::Asia::Tokyo));
        assert_eq!(alarm.place, "Tokyo");
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
    fn the_style_key_brings_up_a_hidden_map_before_changing_its_style() {
        let mut app = app();
        press(&mut app, KeyCode::Char('v'));
        assert!(app.tab().show_map);
        assert_eq!(app.map_style, MapStyle::Ascii);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(app.map_style, MapStyle::Braille);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(app.map_style, MapStyle::Blocks);
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(app.map_style, MapStyle::Ascii);
    }

    #[test]
    fn toggles_display_options() {
        let mut app = app();
        press(&mut app, KeyCode::Char('s'));
        assert!(!app.show_seconds);
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

    fn theme_name(app: &App) -> &'static str {
        THEMES[app.theme].name
    }

    #[test]
    fn previews_themes_while_moving_through_the_list() {
        let mut app = app();
        press(&mut app, KeyCode::Char('t'));
        assert!(matches!(app.mode, Mode::Themes(_)));
        press(&mut app, KeyCode::Down);
        assert_eq!(theme_name(&app), "Tokyo Night");
        press(&mut app, KeyCode::Down);
        assert_eq!(theme_name(&app), "Tokyo Night Storm");
        assert_eq!(app.state().theme, "Terminal", "not saved while previewing");
        press(&mut app, KeyCode::Esc);
        assert_eq!(theme_name(&app), "Terminal", "Esc goes back");
    }

    #[test]
    fn picks_a_theme_by_typing_part_of_its_name() {
        let mut app = app();
        press(&mut app, KeyCode::Char('t'));
        type_text(&mut app, "drac");
        assert_eq!(theme_name(&app), "Dracula");
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Clock));
        assert_eq!(app.state().theme, "Dracula");

        press(&mut app, KeyCode::Char('t'));
        type_text(&mut app, "zzz");
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Themes(_)), "nothing to pick");
        press(&mut app, KeyCode::Esc);
        assert_eq!(theme_name(&app), "Dracula");
    }

    #[test]
    fn an_alarm_cancels_a_theme_preview() {
        let mut app = app();
        app.add_alarm(When::In(Duration::seconds(1)), String::new(), now());
        press(&mut app, KeyCode::Char('t'));
        press(&mut app, KeyCode::Down);
        app.tick(now() + Duration::seconds(1));
        assert!(matches!(app.mode, Mode::Clock));
        assert_eq!(theme_name(&app), "Terminal");
    }

    #[test]
    fn restores_everything_it_saves() {
        let mut app = app();
        press(&mut app, KeyCode::Char('m'));
        for query in ["Tokyo", "London"] {
            app.open_city(city(query));
        }
        press(&mut app, KeyCode::Char('v'));
        press(&mut app, KeyCode::Char('m'));
        press(&mut app, KeyCode::Char('s'));
        press(&mut app, KeyCode::Right);
        app.theme = theme::find("Nord").unwrap();
        app.add_alarm(
            When::At(chrono::NaiveTime::from_hms_opt(7, 30, 0).unwrap()),
            "Wake".into(),
            now(),
        );
        app.add_alarm(When::In(Duration::minutes(25)), "Tea".into(), now());
        let saved = app.state();

        let mut copy = App::new(Lang::En, Zone::City(chrono_tz::UTC));
        copy.restore(&saved, true);
        assert_eq!(copy.state(), saved);
        assert_eq!(tab_names(&copy), ["local", "Tokyo", "London"]);
        assert_eq!(copy.city().map(|c| c.name), Some("Tokyo"));
        assert!(copy.tabs[0].show_map && !copy.tabs[2].show_map);
        assert_eq!(copy.map_style, MapStyle::Braille);
        assert!(!copy.show_seconds);
        assert_eq!(theme_name(&copy), "Nord");
        assert_eq!(copy.alarms.list().len(), 2);
    }

    #[test]
    fn another_copy_does_not_move_the_tab_on_screen() {
        let mut here = app();
        here.open_city(city("Tokyo"));
        here.open_city(city("London"));
        press(&mut here, KeyCode::Char('2'));

        let mut there = app();
        there.open_city(city("Lima"));
        there.open_city(city("Tokyo"));
        there.active = 0;
        here.restore(&there.state(), false);
        assert_eq!(tab_names(&here), ["local", "Lima", "Tokyo"]);
        assert_eq!(here.city().map(|c| c.name), Some("Tokyo"));
    }

    #[test]
    fn skips_saved_cities_it_cannot_find() {
        let mut app = app();
        app.open_city(city("Tokyo"));
        let mut saved = app.state();
        saved.cities[0].name = "Atlantis".into();
        saved.active = 7;
        let mut copy = App::new(Lang::En, Zone::City(chrono_tz::UTC));
        copy.restore(&saved, true);
        assert_eq!((tab_names(&copy), copy.active), (vec!["local"], 0));
    }

    #[test]
    fn asks_for_the_forecast_of_each_city_tab() {
        let (net, jobs, answers) = Net::manual();
        let mut app = app();
        app.net = Some(net);
        app.tick(now());
        assert!(
            jobs.try_recv().is_err(),
            "the local tab has no place to ask about"
        );

        let tokyo = city("Tokyo");
        app.open_city(tokyo);
        app.tick(now());
        let (key, Job::Get(url)) = jobs.try_recv().unwrap() else {
            panic!("expected a download")
        };
        assert_eq!(key, weather::key(&tokyo));
        assert!(
            url.starts_with(
                "https://api.open-meteo.com/v1/forecast?latitude=35.690&longitude=139.692"
            ),
            "{url}"
        );
        app.tick(now());
        assert!(jobs.try_recv().is_err(), "one request at a time");

        answers
            .send((key, Answer::Text(Ok(weather::tests::SAMPLE.to_owned()))))
            .unwrap();
        app.tick(now());
        assert_eq!(app.forecast(&tokyo).unwrap().temperature, 21.0);
        app.tick(now() + Duration::minutes(29));
        assert!(jobs.try_recv().is_err(), "fresh for half an hour");
        app.tick(now() + Duration::minutes(30));
        assert!(jobs.try_recv().is_ok());

        press(&mut app, KeyCode::Char('w'));
        app.tick(now() + Duration::hours(2));
        assert!(jobs.try_recv().is_err(), "no requests with the weather off");
    }

    #[test]
    fn retries_a_failed_forecast_sooner() {
        let (net, jobs, answers) = Net::manual();
        let mut app = app();
        app.net = Some(net);
        app.open_city(city("Tokyo"));
        app.tick(now());
        let (key, _) = jobs.try_recv().unwrap();
        answers
            .send((key, Answer::Text(Err("offline".into()))))
            .unwrap();
        app.tick(now() + Duration::minutes(4));
        assert!(jobs.try_recv().is_err());
        app.tick(now() + Duration::minutes(5));
        assert!(jobs.try_recv().is_ok());
    }

    fn with_calendar() -> (App, std::sync::mpsc::Receiver<(String, Job)>) {
        let (net, jobs, answers) = Net::manual();
        let mut app = app();
        app.net = Some(net);
        app.set_calendar(Some("https://calendar.example/basic.ics".into()));
        app.tick(now());
        let (key, Job::Get(address)) = jobs.try_recv().unwrap() else {
            panic!("expected a download")
        };
        assert_eq!(address, "https://calendar.example/basic.ics");
        let feed = "BEGIN:VCALENDAR\nBEGIN:VEVENT\nUID:review\nSUMMARY:Review\nDTSTART:20260925T123000Z\nEND:VEVENT\nEND:VCALENDAR\n";
        answers.send((key, Answer::Text(Ok(feed.into())))).unwrap();
        app.tick(now());
        (app, jobs)
    }

    #[test]
    fn rings_five_minutes_before_an_event_once() {
        let (mut app, _jobs) = with_calendar();
        assert_eq!(
            app.calendar.next(now()).map(|event| event.title.as_str()),
            Some("Review")
        );
        app.tick(now() + Duration::minutes(24));
        assert!(app.ringing.is_empty());
        app.tick(now() + Duration::minutes(25));
        assert_eq!(app.ringing.len(), 1);
        assert_eq!(app.ringing[0].label, "Review");
        assert_eq!(
            app.ringing[0].time,
            chrono::NaiveTime::from_hms_opt(12, 30, 0).unwrap()
        );
        press(&mut app, KeyCode::Enter);
        app.tick(now() + Duration::minutes(26));
        assert!(app.ringing.is_empty(), "each reminder rings once");
        assert_eq!(
            app.state().rung.len(),
            1,
            "and remembers it across restarts"
        );
    }

    #[test]
    fn reminders_can_be_changed_or_switched_off() {
        let (mut app, _jobs) = with_calendar();
        press(&mut app, KeyCode::Char('g'));
        press(&mut app, KeyCode::Char('-'));
        assert_eq!(app.calendar.reminder, 0);
        app.tick(now() + Duration::minutes(29));
        assert!(app.ringing.is_empty(), "reminders off");
        press(&mut app, KeyCode::Char('+'));
        press(&mut app, KeyCode::Char('+'));
        assert_eq!(app.calendar.reminder, 10);
        app.calendar.reminder = 7;
        press(&mut app, KeyCode::Char('-'));
        assert_eq!(app.calendar.reminder, 5);
    }

    #[test]
    fn does_not_remind_of_events_already_started() {
        let (mut app, _jobs) = with_calendar();
        app.tick(now() + Duration::minutes(31));
        assert!(app.ringing.is_empty());
    }

    #[test]
    fn connects_by_pasting_the_address() {
        let (net, jobs, _answers) = Net::manual();
        let mut app = app();
        app.net = Some(net);
        press(&mut app, KeyCode::Char('g'));
        assert!(matches!(app.mode, Mode::Calendar(CalendarPanel::Choose(0))));
        // The iCal address is the last option on every system.
        press(&mut app, KeyCode::Down);
        press(&mut app, KeyCode::Enter);
        assert!(
            matches!(&app.mode, Mode::Calendar(CalendarPanel::Address(text)) if text.is_empty())
        );
        type_text(&mut app, "  webcal://example.com/basic.ics ");
        press(&mut app, KeyCode::Enter);
        assert_eq!(
            app.calendar.address.as_deref(),
            Some("webcal://example.com/basic.ics")
        );
        assert_eq!(
            app.state().calendar.as_deref(),
            Some("webcal://example.com/basic.ics")
        );
        app.tick(now());
        assert!(jobs.try_recv().is_ok());

        press(&mut app, KeyCode::Char('d'));
        assert!(app.calendar.address.is_none() && matches!(app.mode, Mode::Clock));
    }

    #[test]
    fn ignores_answers_for_a_replaced_address() {
        let (net, jobs, answers) = Net::manual();
        let mut app = app();
        app.net = Some(net);
        app.set_calendar(Some("old.ics".into()));
        app.tick(now());
        let (old_key, _) = jobs.try_recv().unwrap();
        app.set_calendar(Some("new.ics".into()));
        answers
            .send((
                old_key,
                Answer::Text(Ok("BEGIN:VCALENDAR\nEND:VCALENDAR".into())),
            ))
            .unwrap();
        app.tick(now());
        assert!(app.calendar.updated.is_none());
    }

    #[test]
    fn reports_a_feed_that_is_not_a_calendar() {
        let (net, jobs, answers) = Net::manual();
        let mut app = app();
        app.net = Some(net);
        app.set_calendar(Some("https://example.com/page".into()));
        app.tick(now());
        let (key, _) = jobs.try_recv().unwrap();
        answers
            .send((key, Answer::Text(Ok("<html>sign in</html>".into()))))
            .unwrap();
        app.tick(now());
        assert_eq!(app.calendar.error.as_deref(), Some("not an iCal feed"));
    }

    /// Records the pages the app asks to open instead of opening them.
    fn record_opened(app: &mut App) -> Arc<Mutex<Vec<String>>> {
        let opened = Arc::new(Mutex::new(Vec::new()));
        let log = opened.clone();
        app.open = Box::new(move |target| {
            log.lock().unwrap().push(target.to_owned());
            true
        });
        opened
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn connects_the_mac_calendar_asking_for_access_once() {
        let (net, jobs, answers) = Net::manual();
        let mut app = app();
        app.net = Some(net);
        press(&mut app, KeyCode::Char('g'));
        press(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Calendar(CalendarPanel::Status)));
        assert!(app.calendar.mac && app.calendar.asking);
        assert!(app.state().mac_calendar);
        let (key, job) = jobs.try_recv().unwrap();
        assert!(matches!(job, Job::MacCalendar { ask: true }));

        let review = Event {
            uid: "1".into(),
            title: "Review".into(),
            start: now() + Duration::minutes(40),
        };
        answers
            .send((key, Answer::MacCalendar(Ok(vec![review]))))
            .unwrap();
        app.tick(now());
        assert!(!app.calendar.asking);
        assert_eq!(app.calendar.access, Some(Access::Granted));
        assert_eq!(
            app.calendar.next(now()).map(|event| event.title.as_str()),
            Some("Review")
        );

        app.tick(now() + Duration::minutes(1));
        let (_, job) = jobs.try_recv().unwrap();
        assert!(
            matches!(job, Job::MacCalendar { ask: false }),
            "later reads never ask again"
        );
    }

    #[test]
    fn a_denied_mac_calendar_opens_the_privacy_settings() {
        let (net, jobs, answers) = Net::manual();
        let mut app = app();
        app.net = Some(net);
        let opened = record_opened(&mut app);
        app.connect_mac(now());
        let (key, _) = jobs.try_recv().unwrap();
        answers
            .send((
                key,
                Answer::MacCalendar(Err(maccal::Refusal {
                    access: Access::Denied,
                    detail: None,
                })),
            ))
            .unwrap();
        app.tick(now());
        assert_eq!(app.calendar.access, Some(Access::Denied));

        press(&mut app, KeyCode::Char('g'));
        press(&mut app, KeyCode::Char('o'));
        press(&mut app, KeyCode::Char('a'));
        assert_eq!(
            *opened.lock().unwrap(),
            [maccal::PRIVACY_SETTINGS, maccal::ACCOUNTS_SETTINGS]
        );
        press(&mut app, KeyCode::Char('r'));
        let (_, job) = jobs.try_recv().unwrap();
        assert!(
            matches!(job, Job::MacCalendar { ask: true }),
            "r asks again"
        );
    }

    #[test]
    fn the_mac_calendar_and_an_ical_address_replace_each_other() {
        let mut app = app();
        app.connect_mac(now());
        app.set_calendar(Some("work.ics".into()));
        assert!(!app.calendar.mac);
        app.connect_mac(now());
        assert!(app.calendar.address.is_none() && app.calendar.mac);

        let mut copy = self::app();
        copy.restore(&app.state(), true);
        assert!(copy.calendar.mac, "the source is saved");
    }

    #[test]
    fn the_weather_key_asks_for_your_city_in_the_local_tab() {
        let mut app = App::new(Lang::En, Zone::System(Some(chrono_tz::America::Sao_Paulo)));
        app.show_weather = false;
        press(&mut app, KeyCode::Char('w'));
        let Mode::Search(search) = &app.mode else {
            panic!("search not open")
        };
        assert!(search.home);
        assert_eq!(
            search.query, "Sao Paulo",
            "starts from the time zone's city"
        );
        assert_eq!(search.results[0].name, "São Paulo");

        press(&mut app, KeyCode::Char('u'));
        app.on_key(
            KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL),
            now(),
        );
        type_text(&mut app, "belo horizonte");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.home.map(|city| city.name), Some("Belo Horizonte"));
        assert!(app.show_weather && app.tabs.len() == 1, "no tab is opened");

        press(&mut app, KeyCode::Char('w'));
        assert!(!app.show_weather, "then w switches the weather off and on");
        press(&mut app, KeyCode::Char('W'));
        assert!(
            matches!(&app.mode, Mode::Search(search) if search.home && search.query == "Belo Horizonte")
        );
        press(&mut app, KeyCode::Esc);

        let mut copy = self::app();
        copy.restore(&app.state(), true);
        assert_eq!(
            copy.home.map(|city| city.name),
            Some("Belo Horizonte"),
            "saved"
        );
    }

    #[test]
    fn asks_for_the_forecast_of_your_city() {
        let (net, jobs, _answers) = Net::manual();
        let mut app = app();
        app.net = Some(net);
        app.home = Some(city("Belo Horizonte"));
        app.tick(now());
        let (key, _) = jobs.try_recv().unwrap();
        assert_eq!(key, weather::key(&city("Belo Horizonte")));
        assert!(app.weather_of(&city("Belo Horizonte")).unwrap().pending());
    }
}
