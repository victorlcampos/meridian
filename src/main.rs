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
mod pager;
mod solar;
mod state;
mod theme;
mod ui;
mod weather;
mod weather_fx;
mod worldmap;
mod zone;

use std::fmt;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use chrono::{Duration, Utc};
use clap::{Parser, Subcommand};
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{self, Event, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::crossterm::{self, execute};
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
#[command(version, about, args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

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

#[derive(Subcommand)]
enum Command {
    /// Update meridian to the latest release
    Update {
        /// Only tell whether a newer version is out
        #[arg(long)]
        check: bool,
    },
    /// Send a message to the meridian running, like to a pager: it beeps and
    /// the message scrolls across its screen until any key dismisses it
    ///
    /// An emoticon on its own, such as :) <3 (y) or 👍, passes by as ASCII
    /// art. With --url, a click on the message, or o, opens the address in
    /// the browser and dismisses the message. Copies of meridian side by
    /// side all show the message, and one started later still finds it.
    #[command(visible_alias = "bip")]
    Page {
        /// The message; read from the standard input when left out. One
        /// starting with a hyphen, such as -_-, goes after --
        #[arg(value_name = "MESSAGE", allow_negative_numbers = true)]
        message: Vec<String>,

        /// Take the message down by itself after a while: 10m, 1h30, 90s
        #[arg(long = "for", value_name = "DURATION", value_parser = parse_duration)]
        duration: Option<Duration>,

        /// Web address (http or https) that a click on the message, or o,
        /// opens, dismissing the message
        #[arg(long, value_name = "URL", value_parser = parse_url)]
        url: Option<String>,

        /// Take down the message on screen
        #[arg(long, conflicts_with_all = ["message", "duration", "url"])]
        clear: bool,

        /// State file of the meridian to page
        /// [default: ~/.config/meridian/state.json]
        #[arg(long, value_name = "FILE")]
        state: Option<PathBuf>,
    },
}

fn parse_color(name: &str) -> Result<Color, String> {
    name.parse().map_err(|_| format!("unknown color {name:?}"))
}

fn parse_duration(text: &str) -> Result<Duration, String> {
    alarm::parse_delay(text.strip_prefix('+').unwrap_or(text))
        .ok_or_else(|| format!("invalid duration {text:?}; try 10m, 1h30 or 90s"))
}

fn parse_url(text: &str) -> Result<String, String> {
    pager::web_link(text).ok_or_else(|| {
        format!("invalid URL {text:?}; give one that starts with https:// or http://")
    })
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Update { check }) => {
            let version = env!("CARGO_PKG_VERSION");
            return cerne::update::run("victorlcampos/meridian", "meridian", version, check);
        }
        Some(Command::Page {
            message,
            duration,
            url,
            clear,
            state,
        }) => return page(&message, duration, url, clear, state),
        None => {}
    }
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

/// `meridian page`: leaves the message, or its removal, in the state file,
/// where the meridian running picks it up within a second.
fn page(
    words: &[String],
    duration: Option<Duration>,
    link: Option<String>,
    clear: bool,
    state: Option<PathBuf>,
) -> ExitCode {
    let Some(path) = state.or_else(state::default_path) else {
        eprintln!("meridian: no home folder for the state file; give one with --state");
        return ExitCode::FAILURE;
    };
    let page = if clear {
        None
    } else {
        let text = if !words.is_empty() {
            words.join(" ")
        } else if io::stdin().is_terminal() {
            eprintln!("meridian: no message; write it after `meridian page`, or pipe it in");
            return ExitCode::from(2);
        } else {
            let mut text = String::new();
            if let Err(error) = io::stdin().read_to_string(&mut text) {
                eprintln!("meridian: {error}");
                return ExitCode::FAILURE;
            }
            text
        };
        let text = pager::clean(&text);
        if text.is_empty() {
            eprintln!("meridian: the message is empty");
            return ExitCode::from(2);
        }
        if !path.exists() {
            eprintln!(
                "meridian: no meridian has used {} yet; the message waits there for one",
                path.display()
            );
        }
        let sent = Utc::now();
        Some(pager::Page {
            text,
            sent,
            until: duration.map(|duration| sent + duration),
            link,
        })
    };
    match state::send_page(&path, page.as_ref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("meridian: {}: {error}", path.display());
            ExitCode::FAILURE
        }
    }
}

/// Asks the terminal to report clicks, or to stop: presses and releases
/// only (mode 1000), in SGR coordinates (1006) for windows wider than 223
/// columns. crossterm's `EnableMouseCapture` asks for every move of the
/// mouse as well, which would wake the clock, and read the state file, for
/// nothing.
struct ReportClicks(bool);

impl crossterm::Command for ReportClicks {
    fn write_ansi(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str(match self.0 {
            true => "\x1b[?1000h\x1b[?1006h",
            false => "\x1b[?1006l\x1b[?1000l",
        })
    }

    /// The Windows console has a switch of its own for the mouse.
    #[cfg(windows)]
    fn execute_winapi(&self) -> io::Result<()> {
        use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
        match self.0 {
            true => crossterm::Command::execute_winapi(&EnableMouseCapture),
            false => crossterm::Command::execute_winapi(&DisableMouseCapture),
        }
    }

    #[cfg(windows)]
    fn is_ansi_code_supported(&self) -> bool {
        false
    }
}

/// Whether the terminal reports clicks to meridian: only while a page's
/// link is on screen to click, so the rest of the time the mouse stays the
/// terminal's, to select and copy text.
#[derive(Default)]
struct Mouse {
    clicks: bool,
}

impl Mouse {
    fn report_clicks(&mut self, on: bool) -> io::Result<()> {
        if self.clicks != on {
            execute!(io::stdout(), ReportClicks(on))?;
            self.clicks = on;
        }
        Ok(())
    }
}

impl Drop for Mouse {
    /// However meridian ends, the terminal gets its mouse back.
    fn drop(&mut self) {
        let _ = self.report_clicks(false);
    }
}

fn run(terminal: &mut DefaultTerminal, app: &mut App, store: &mut Option<Store>) -> io::Result<()> {
    let mut mouse = Mouse::default();
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
        let mut drawn = ui::Drawn::default();
        terminal.draw(|frame| drawn = ui::render(frame, app, now))?;
        mouse.report_clicks(drawn.link.is_some())?;
        if !event::poll(app.next_wakeup(now, drawn.animation))? {
            continue;
        }
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => app.on_key(key, Utc::now()),
            Event::Mouse(click)
                if click.kind == MouseEventKind::Down(MouseButton::Left)
                    && drawn.on_link(click.column, click.row) =>
            {
                app.open_page(Utc::now());
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn the_command_line_is_well_formed() {
        Cli::command().debug_assert();
    }

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(["meridian"].iter().chain(args))
    }

    /// The message, how long it stays and whether it is a `--clear`.
    fn page(args: &[&str]) -> (Vec<String>, Option<Duration>, bool) {
        let cli = parse(&[&["page"], args].concat()).unwrap();
        let Some(Command::Page {
            message,
            duration,
            clear,
            ..
        }) = cli.command
        else {
            panic!("not a page")
        };
        (message, duration, clear)
    }

    #[test]
    fn takes_the_message_quoted_or_not() {
        assert_eq!(page(&["Lunch", "is", "ready"]).0, ["Lunch", "is", "ready"]);
        assert_eq!(page(&["Lunch is ready"]).0, ["Lunch is ready"]);
        assert!(page(&[]).0.is_empty(), "then it comes from a pipe");
        assert!(parse(&["bip", "Oi"]).is_ok(), "bip is page too");
    }

    #[test]
    fn takes_emoticons_that_start_with_a_hyphen() {
        assert_eq!(page(&["-1"]).0, ["-1"]);
        assert_eq!(page(&["--", "-_-"]).0, ["-_-"]);
        let error = parse(&["page", "-_-"]).err().unwrap().to_string();
        assert!(error.contains("use '-- "), "clap says how: {error}");
    }

    #[test]
    fn takes_how_long_the_message_stays() {
        let coffee = vec!["Coffee".to_owned()];
        let (message, duration, _) = page(&["--for", "10m", "Coffee"]);
        assert_eq!(
            (message, duration),
            (coffee.clone(), Some(Duration::minutes(10)))
        );
        let (message, duration, _) = page(&["Coffee", "--for", "+1h30"]);
        assert_eq!((message, duration), (coffee, Some(Duration::minutes(90))));
        assert!(parse(&["page", "--for", "soon", "Hi"]).is_err());
    }

    #[test]
    fn clears_without_a_message() {
        assert!(page(&["--clear"]).2);
        assert!(parse(&["page", "--clear", "Hi"]).is_err());
        assert!(parse(&["page", "--clear", "--for", "5m"]).is_err());
        let run = "https://example.com/run/1";
        assert!(parse(&["page", "--clear", "--url", run]).is_err());
    }

    #[test]
    fn takes_the_web_address_a_click_opens() {
        let url = |args: &[&str]| match parse(&[&["page"], args].concat()) {
            Ok(Cli {
                command: Some(Command::Page { url, .. }),
                ..
            }) => Ok(url),
            Ok(_) => panic!("not a page"),
            Err(error) => Err(error.to_string()),
        };
        let run = "https://github.com/acme/app/actions/runs/1";
        assert_eq!(
            url(&["--url", run, "Deploy", "finished"]),
            Ok(Some(run.into()))
        );
        assert_eq!(
            url(&["Deploy finished", "--url", run]),
            Ok(Some(run.into()))
        );
        assert_eq!(url(&["Deploy finished"]), Ok(None));
        let error = url(&["--url", "github.com/acme/app", "Hi"]).unwrap_err();
        assert!(error.contains("starts with https://"), "{error}");
        assert!(url(&["--url", "file:///etc/hosts", "Hi"]).is_err());
    }

    #[test]
    fn asks_the_terminal_for_clicks_and_nothing_more() {
        let ansi = |on| {
            let mut codes = String::new();
            crossterm::Command::write_ansi(&ReportClicks(on), &mut codes).unwrap();
            codes
        };
        let on = ansi(true);
        assert!(on.contains("[?1000h") && on.contains("[?1006h"), "{on:?}");
        assert!(!on.contains("[?1003h"), "not every move of the mouse");
        assert_eq!(ansi(false), "\x1b[?1006l\x1b[?1000l", "and undoes it");
    }
}
