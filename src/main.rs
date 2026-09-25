mod alarm;
mod app;
mod calendar;
mod cities;
mod clockface;
mod desktop;
mod i18n;
mod layout;
mod maccal;
mod net;
mod solar;
mod state;
mod theme;
mod ui;
mod weather;
mod worldmap;
mod zone;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::Utc;
use clap::Parser;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::style::Color;

use crate::app::App;
use crate::i18n::Lang;
use crate::state::Store;
use crate::theme::THEMES;
use crate::worldmap::MapStyle;
use crate::zone::Zone;

/// Terminal clock with a tab per city, a world map, themes and blinking alarms.
/// Everything you set is saved, and the layout fits anything from a full
/// screen to a small tiling pane.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Open a tab with a city's time (repeatable): "Tokyo", "São Paulo", "Portland, Maine".
    /// The first one given is shown first
    #[arg(short = 'c', long = "city", value_name = "CITY")]
    cities: Vec<String>,

    /// Set an alarm, optionally followed by a label: "07:30", "7:30pm Call", "+25m Tea" (repeatable)
    #[arg(short, long = "alarm", value_name = "WHEN")]
    alarms: Vec<String>,

    /// Clock theme, such as "Tokyo Night", "dracula" or "gruvbox" (see --list-themes)
    #[arg(short, long)]
    theme: Option<String>,

    /// List the themes and exit
    #[arg(long)]
    list_themes: bool,

    /// Show the world map, on every tab
    #[arg(short, long)]
    map: bool,

    /// Look of the world map
    #[arg(long, value_enum)]
    map_style: Option<MapStyle>,

    /// Hide the seconds
    #[arg(long)]
    no_seconds: bool,

    /// Color of the digits, over the theme's: a name ("green", "lightred"), an index (0-255) or "#rrggbb"
    #[arg(long, value_parser = parse_color)]
    color: Option<Color>,

    /// Connect a calendar: its secret iCal address (Google Calendar › Settings ›
    /// Integrate calendar) or an .ics file
    #[arg(long, value_name = "ADDRESS")]
    calendar: Option<String>,

    /// Minutes before an event its reminder rings; 0 turns reminders off [default: 5]
    #[arg(long, value_name = "MINUTES")]
    reminder: Option<u32>,

    /// Interface language [default: from $LANG]
    #[arg(long, value_enum)]
    lang: Option<Lang>,

    /// File that keeps tabs, theme and alarms between runs
    /// [default: ~/.config/meridian/state.json]
    #[arg(long, value_name = "FILE")]
    state: Option<PathBuf>,

    /// Reads the macOS Calendar for the running app (internal)
    #[arg(long, hide = true, num_args = 1.., allow_hyphen_values = true)]
    calendar_helper: Vec<String>,
}

fn parse_color(name: &str) -> Result<Color, String> {
    name.parse().map_err(|_| format!("unknown color {name:?}"))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if !cli.calendar_helper.is_empty() {
        return maccal::serve(&cli.calendar_helper);
    }
    if cli.list_themes {
        for theme in &THEMES {
            println!("{}", theme.name);
        }
        return ExitCode::SUCCESS;
    }
    let now = Utc::now();
    let mut app = App::new(cli.lang.unwrap_or_else(Lang::from_env), Zone::system());
    app.truecolor = theme::truecolor_from_env();
    app.digits_color = cli.color;
    app.net = Some(net::Net::start());

    let mut store = cli
        .state
        .clone()
        .or_else(state::default_path)
        .map(Store::new);
    if let Some(store) = &mut store {
        store.load(&mut app);
    }

    // Options act as if done in the app, so they are saved as well.
    let mut first_city = None;
    for query in &cli.cities {
        let Some(city) = cities::db().search(query, 1).first().copied() else {
            eprintln!("meridian: no city matches {query:?}");
            return ExitCode::from(2);
        };
        app.open_city(city);
        first_city.get_or_insert(app.active);
    }
    if let Some(tab) = first_city {
        app.active = tab;
    }
    if let Some(name) = &cli.theme {
        let Some(index) = theme::find(name) else {
            eprintln!("meridian: unknown theme {name:?}; see --list-themes");
            return ExitCode::from(2);
        };
        app.theme = index;
    }
    if let Some(style) = cli.map_style {
        app.map_style = style;
    }
    if cli.no_seconds {
        app.show_seconds = false;
    }
    if cli.map {
        app.show_map = true;
    }
    if let Some(address) = &cli.calendar {
        app.set_calendar(Some(address.clone()));
    }
    if let Some(minutes) = cli.reminder {
        app.calendar.reminder = minutes;
    }
    for spec in &cli.alarms {
        let Ok((when, label)) = alarm::parse(spec) else {
            eprintln!("meridian: invalid alarm {spec:?}; try 07:30, 7:30pm or +10m");
            return ExitCode::from(2);
        };
        app.add_alarm(when, label, now);
    }
    // Build the city index in the background so the first search is instant.
    drop(std::thread::spawn(cities::db));

    match ratatui::run(|terminal| run(terminal, &mut app, &mut store)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("meridian: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App, store: &mut Option<Store>) -> io::Result<()> {
    loop {
        let now = Utc::now();
        app.tick(now);
        // Saved on every change, not on exit, so a power cut loses nothing.
        if let Some(store) = store.as_mut() {
            app.unsaved = store.sync(app).is_err();
        }
        if app.quit {
            return Ok(());
        }
        if app.take_bell() {
            // Tiling window managers flag the window as urgent on a bell.
            io::stdout().write_all(b"\x07")?;
            io::stdout().flush()?;
        }
        terminal.draw(|frame| ui::render(frame, app, now))?;
        if event::poll(app.next_wakeup(now))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key, Utc::now());
        }
    }
}
