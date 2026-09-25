mod alarm;
mod app;
mod cities;
mod clockface;
mod i18n;
mod layout;
mod solar;
mod ui;
mod worldmap;
mod zone;

use std::io::{self, Write};
use std::process::ExitCode;

use chrono::Utc;
use clap::Parser;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind};
use ratatui::style::Color;

use crate::app::App;
use crate::i18n::Lang;
use crate::worldmap::MapStyle;
use crate::zone::Zone;

/// Terminal clock with a world map, city time zones and blinking alarms.
/// Resizes to fit anything from a full screen to a small tiling pane.
#[derive(Parser)]
#[command(version, about)]
struct Cli {
    /// Show the time of a city instead of the computer's: "Tokyo", "São Paulo", "Portland, Maine"
    #[arg(short, long)]
    city: Option<String>,

    /// Set an alarm, optionally followed by a label: "07:30", "7:30pm Call", "+25m Tea" (repeatable)
    #[arg(short, long = "alarm", value_name = "WHEN")]
    alarms: Vec<String>,

    /// Show the world map even without a city
    #[arg(short, long)]
    map: bool,

    /// Look of the world map
    #[arg(long, value_enum, default_value_t)]
    map_style: MapStyle,

    /// Hide the seconds
    #[arg(long)]
    no_seconds: bool,

    /// Color of the digits: a name ("green", "lightred"), an index (0-255) or "#rrggbb"
    #[arg(long, default_value = "cyan", value_parser = parse_color)]
    color: Color,

    /// Interface language [default: from $LANG]
    #[arg(long, value_enum)]
    lang: Option<Lang>,
}

fn parse_color(name: &str) -> Result<Color, String> {
    name.parse().map_err(|_| format!("unknown color {name:?}"))
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let now = Utc::now();
    let mut app = App::new(cli.lang.unwrap_or_else(Lang::from_env), Zone::system());
    app.map_style = cli.map_style;
    app.show_seconds = !cli.no_seconds;
    app.color = cli.color;

    match &cli.city {
        Some(query) => match cities::db().search(query, 1).first() {
            Some(city) => app.select_city(*city),
            None => {
                eprintln!("meridian: no city matches {query:?}");
                return ExitCode::from(2);
            }
        },
        // Build the city index in the background so the first search is instant.
        None => drop(std::thread::spawn(cities::db)),
    }
    app.show_map |= cli.map;

    for spec in &cli.alarms {
        let Ok((when, label)) = alarm::parse(spec) else {
            eprintln!("meridian: invalid alarm {spec:?}; try 07:30, 7:30pm or +10m");
            return ExitCode::from(2);
        };
        app.add_alarm(when, label, now);
    }

    match ratatui::run(|terminal| run(terminal, &mut app)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("meridian: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> io::Result<()> {
    while !app.quit {
        let now = Utc::now();
        app.tick(now);
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
    Ok(())
}
