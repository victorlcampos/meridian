//! What meridian remembers between runs: city tabs, theme, map, seconds and
//! alarms, in one JSON file saved on every change. Copies running side by side
//! share the file and pick up each other's changes.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, NaiveTime, Utc};
use serde::{Deserialize, Serialize};

use crate::app::App;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub theme: String,
    pub map_style: String,
    pub seconds: bool,
    pub weather: bool,
    /// City of the local time tab, for its weather.
    pub home: Option<SavedCity>,
    /// Secret iCal address (or .ics file) of the calendar.
    pub calendar: Option<String>,
    /// Events come from the macOS Calendar app.
    pub mac_calendar: bool,
    /// Minutes before an event its reminder rings; 0 is off.
    pub reminder: u32,
    /// Event occurrences whose reminder already rang.
    pub rung: Vec<String>,
    /// Whether the map shows, on every tab. Absent from files saved while
    /// each tab had its own switch: see `State::shows_map`.
    pub map: Option<bool>,
    /// The local time tab's own map switch, in those older files.
    #[serde(skip_serializing)]
    pub local_map: Option<bool>,
    /// Tab on screen: 0 is the local time, then one per city.
    pub active: usize,
    pub cities: Vec<SavedCity>,
    pub alarms: Vec<SavedAlarm>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            theme: String::new(),
            map_style: String::new(),
            seconds: true,
            weather: true,
            home: None,
            calendar: None,
            mac_calendar: false,
            reminder: 5,
            rung: Vec::new(),
            map: None,
            local_map: None,
            active: 0,
            cities: Vec::new(),
            alarms: Vec::new(),
        }
    }
}

impl State {
    /// Whether the map shows. Files saved while each tab had its own switch
    /// keep the one of the tab that was on screen.
    pub fn shows_map(&self) -> bool {
        self.map
            .unwrap_or_else(|| match self.active.checked_sub(1) {
                None => self.local_map.unwrap_or(true),
                Some(city) => self
                    .cities
                    .get(city)
                    .and_then(|saved| saved.map)
                    .unwrap_or(true),
            })
    }
}

/// A city tab. The name and position find the city again; region and
/// country are there for whoever reads the file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedCity {
    pub name: String,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub country: String,
    pub lat: f64,
    pub lon: f64,
    /// The tab's own map switch, in files saved before there was one for all.
    #[serde(default, skip_serializing)]
    pub map: Option<bool>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SavedAlarm {
    pub time: NaiveTime,
    /// IANA zone name; absent for the computer's own time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
    #[serde(default)]
    pub place: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub daily: bool,
    #[serde(default)]
    pub timer: bool,
    /// When it rings next; absent while switched off.
    #[serde(default)]
    pub next: Option<DateTime<Utc>>,
}

/// `$XDG_CONFIG_HOME/meridian/state.json`, or `~/.config/...`, or `%APPDATA%\...`.
pub fn default_path() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|dir| dir.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))?;
    Some(base.join("meridian").join("state.json"))
}

/// Keeps the state file and the app in step.
pub struct Store {
    path: PathBuf,
    /// The file's text when last read or written here.
    seen: Option<String>,
    /// What this copy last wrote, to tell its own writes from the others'.
    written: Option<String>,
    /// The app's state as last saved or loaded, to notice changes.
    synced: Option<State>,
}

impl Store {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            seen: None,
            written: None,
            synced: None,
        }
    }

    /// Restores the saved state, including the tab that was on screen. A file
    /// that cannot be read as a state is set aside as `*.broken`, not lost.
    pub fn load(&mut self, app: &mut App) {
        let Ok(text) = fs::read_to_string(&self.path) else {
            return;
        };
        match serde_json::from_str::<State>(&text) {
            Ok(state) => {
                app.restore(&state, true);
                self.seen = Some(text);
                self.synced = Some(app.state());
            }
            Err(_) => {
                let _ = fs::rename(&self.path, self.path.with_extension("json.broken"));
            }
        }
    }

    /// Takes in what another copy saved, then saves what changed here. Each
    /// copy keeps its own tab on screen.
    pub fn sync(&mut self, app: &mut App) -> io::Result<()> {
        if let Ok(text) = fs::read_to_string(&self.path)
            && self.seen.as_ref() != Some(&text)
        {
            if self.written.as_ref() != Some(&text)
                && let Ok(state) = serde_json::from_str::<State>(&text)
            {
                app.restore(&state, false);
                self.synced = Some(app.state());
            }
            self.seen = Some(text);
        }

        let state = app.state();
        if self.synced.as_ref() == Some(&state) {
            return Ok(());
        }
        let text = serde_json::to_string_pretty(&state).map_err(io::Error::other)? + "\n";
        write_atomically(&self.path, &text)?;
        self.seen = Some(text.clone());
        self.written = Some(text);
        self.synced = Some(state);
        Ok(())
    }
}

/// A crash or power cut leaves either the old file or the new one, whole.
fn write_atomically(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let temporary = path.with_extension(format!("json.{}.tmp", std::process::id()));
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    // The file holds the calendar's secret address: readable by its owner only.
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    let mut file = options.open(&temporary)?;
    file.write_all(text.as_bytes())?;
    file.sync_all()?;
    fs::rename(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alarm::When;
    use crate::cities;
    use crate::i18n::Lang;
    use crate::theme;
    use crate::zone::Zone;
    use chrono::{Duration, NaiveDate};

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

    /// A state file path in a fresh directory of its own.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("meridian-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("state.json")
    }

    fn open(path: &Path) -> (App, Store) {
        let mut app = app();
        let mut store = Store::new(path.to_owned());
        store.load(&mut app);
        (app, store)
    }

    fn city(query: &str) -> crate::cities::City {
        cities::db().search(query, 1)[0]
    }

    #[test]
    fn everything_comes_back_after_a_restart() {
        let path = scratch("restart");
        let (mut first, mut store) = open(&path);
        first.open_city(city("Tokyo"));
        first.open_city(city("London"));
        first.theme = theme::find("Tokyo Night").unwrap();
        first.add_alarm(When::In(Duration::minutes(5)), "Tea".into(), now());
        store.sync(&mut first).unwrap();

        let (second, _) = open(&path);
        assert_eq!(second.state(), first.state());
        assert_eq!(second.city().map(|c| c.name), Some("London"));
        assert_eq!(second.alarms.list()[0].label, "Tea");
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn copies_running_side_by_side_share_their_changes() {
        let path = scratch("share");
        let (mut left, mut left_store) = open(&path);
        let (mut right, mut right_store) = open(&path);
        left_store.sync(&mut left).unwrap();
        right_store.sync(&mut right).unwrap();

        left.open_city(city("Lima"));
        left_store.sync(&mut left).unwrap();
        right_store.sync(&mut right).unwrap();
        assert_eq!(
            right.city().map(|c| c.name),
            Some("Lima"),
            "the city opened on the left shows up on the right, in place of local time"
        );
        left.open_city(city("Tokyo"));
        left_store.sync(&mut left).unwrap();
        right_store.sync(&mut right).unwrap();
        assert_eq!(right.tabs.len(), 3);
        assert_eq!(
            right.city().map(|c| c.name),
            Some("Lima"),
            "but each copy keeps its own city on screen"
        );
        left.active = 1;

        right.theme = theme::find("Dracula").unwrap();
        right.add_alarm(When::In(Duration::minutes(1)), "Call".into(), now());
        right_store.sync(&mut right).unwrap();
        left_store.sync(&mut left).unwrap();
        assert_eq!(theme::THEMES[left.theme].name, "Dracula");
        assert_eq!(left.alarms.list()[0].label, "Call");
        assert_eq!(left.city().map(|c| c.name), Some("Lima"));
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn writes_only_when_something_changed() {
        let path = scratch("quiet");
        let (mut app, mut store) = open(&path);
        store.sync(&mut app).unwrap();
        assert!(path.exists());
        fs::remove_file(&path).unwrap();
        store.sync(&mut app).unwrap();
        assert!(!path.exists(), "nothing changed, nothing written");
        app.show_seconds = false;
        store.sync(&mut app).unwrap();
        assert!(path.exists());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn sets_a_broken_file_aside_instead_of_losing_it() {
        let path = scratch("broken");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "{ not json").unwrap();
        let (mut app, mut store) = open(&path);
        assert_eq!(app.tabs.len(), 1);
        let broken = path.with_extension("json.broken");
        assert_eq!(fs::read_to_string(&broken).unwrap(), "{ not json");
        store.sync(&mut app).unwrap();
        assert!(serde_json::from_str::<State>(&fs::read_to_string(&path).unwrap()).is_ok());
        let _ = fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn reads_files_missing_newer_fields() {
        let state: State =
            serde_json::from_str(r#"{"cities": [{"name": "Tokyo", "lat": 35.69, "lon": 139.69}]}"#)
                .unwrap();
        assert!(state.seconds && state.shows_map());
        let mut app = app();
        app.restore(&state, true);
        assert_eq!(app.tabs[1].city.map(|c| c.name), Some("Tokyo"));
    }

    #[test]
    fn files_with_a_map_switch_per_tab_keep_the_one_on_screen() {
        let old = |active: usize, local: bool, tokyo: bool| -> State {
            let text = format!(
                r#"{{"active": {active}, "local_map": {local}, "cities":
                    [{{"name": "Tokyo", "lat": 35.69, "lon": 139.69, "map": {tokyo}}}]}}"#
            );
            serde_json::from_str(&text).unwrap()
        };
        assert!(!old(1, true, false).shows_map());
        assert!(old(1, false, true).shows_map());
        assert!(!old(0, false, true).shows_map());
        let mut app = app();
        app.restore(&old(1, true, false), true);
        assert!(!app.show_map);
        let text = serde_json::to_string(&app.state()).unwrap();
        assert!(
            text.contains(r#""map":false"#) && !text.contains("local_map"),
            "{text}"
        );
    }
}
