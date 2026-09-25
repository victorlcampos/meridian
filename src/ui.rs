//! Drawing: the clock, the world map, the key hints and the pop-ups.

use chrono::{DateTime, FixedOffset, Utc};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Widget};
use unicode_width::UnicodeWidthStr;

use crate::alarm::Alarm;
use crate::app::{AlarmPanel, App, Mode, Search};
use crate::i18n::Keys;
use crate::worldmap::{Marker, WorldMap};
use crate::zone::format_offset;
use crate::{clockface, layout, solar};

pub fn render(frame: &mut Frame, app: &App, now: DateTime<Utc>) {
    let area = frame.area();
    let screen = layout::split(area, app.show_map);
    let buf = frame.buffer_mut();
    render_clock(app, now, screen.clock, buf);
    if let Some(map) = screen.map {
        WorldMap {
            style: app.map_style,
            sun: Some(solar::subsolar_point(now)),
            marker: app.city.map(|city| Marker {
                lat: city.lat,
                lon: city.lon,
                label: city.name,
            }),
        }
        .render(map, buf);
    }
    if let Some(footer) = screen.footer {
        render_keys(app, footer, buf);
    }
    match &app.mode {
        Mode::Clock => {}
        Mode::Search(search) => render_search(frame, app, search, now),
        Mode::Alarms(panel) => render_alarms(frame, app, panel, now),
        Mode::Help => render_help(frame, app),
    }
    if app.blink_on(now) {
        frame
            .buffer_mut()
            .set_style(area, Style::new().fg(Color::White).bg(Color::Red));
    }
}

/// A line of text next to the digits: the place above them, the rest below.
struct Info {
    line: Line<'static>,
    above: bool,
}

enum Row<'a> {
    Digits(&'a str, u16),
    Text(Line<'a>),
    Gap,
}

impl Row<'_> {
    fn height(&self) -> u16 {
        match self {
            Row::Digits(text, scale) => clockface::size(text, *scale).1,
            Row::Text(_) | Row::Gap => 1,
        }
    }
}

fn render_clock(app: &App, now: DateTime<Utc>, area: Rect, buf: &mut Buffer) {
    if area.is_empty() {
        return;
    }
    let local = app.zone.local_time(now);
    let full = local.format("%H:%M:%S").to_string();
    let short = local.format("%H:%M").to_string();
    let texts: Vec<&str> = if app.show_seconds {
        vec![&full, &short]
    } else {
        vec![&short]
    };
    let infos = infos(app, now, local, area.width);
    let rows = arrange(&texts, &infos, area);

    let total: u16 = rows.iter().map(Row::height).sum();
    let mut y = area.y + area.height.saturating_sub(total) / 2;
    for row in rows {
        let height = row.height();
        if y + height > area.bottom() {
            break;
        }
        match row {
            Row::Digits(text, scale) => {
                let width = clockface::size(text, scale).0;
                let x = area.x + (area.width - width) / 2;
                clockface::draw(text, scale, (x, y), Style::new().fg(app.color), buf);
            }
            Row::Text(line) => line.centered().render(
                Rect {
                    y,
                    height: 1,
                    ..area
                },
                buf,
            ),
            Row::Gap => {}
        }
        y += height;
    }
}

/// Big digits and as many info lines as fit, most important first; seconds are
/// dropped before the digits shrink to plain text.
fn arrange<'a>(texts: &[&'a str], infos: &'a [Info], area: Rect) -> Vec<Row<'a>> {
    let rows = |shown: &'a [Info], middle: Row<'a>, gaps: bool| {
        let (above, below): (Vec<_>, Vec<_>) = shown.iter().partition(|info| info.above);
        let mut rows: Vec<Row> = above
            .iter()
            .map(|info| Row::Text(info.line.clone()))
            .collect();
        if gaps && !above.is_empty() {
            rows.push(Row::Gap);
        }
        rows.push(middle);
        if gaps && !below.is_empty() {
            rows.push(Row::Gap);
        }
        rows.extend(below.iter().map(|info| Row::Text(info.line.clone())));
        rows
    };
    for shown in (0..=infos.len()).rev() {
        let lines = &infos[..shown];
        let gaps = u16::from(lines.iter().any(|info| info.above))
            + u16::from(lines.iter().any(|info| !info.above));
        let height = area.height.saturating_sub(shown as u16);
        for text in texts {
            let scale = clockface::fit_scale(text, area.width, height);
            if scale > 0 {
                let roomy =
                    clockface::fit_scale(text, area.width, height.saturating_sub(gaps)) == scale;
                return rows(lines, Row::Digits(text, scale), roomy);
            }
        }
    }
    let text = texts
        .iter()
        .find(|text| text.len() <= usize::from(area.width))
        .unwrap_or(&texts[texts.len() - 1]);
    let shown = infos.len().min(usize::from(area.height.saturating_sub(1)));
    rows(&infos[..shown], Row::Text(Line::from(*text).bold()), false)
}

fn infos(app: &App, now: DateTime<Utc>, local: DateTime<FixedOffset>, width: u16) -> Vec<Info> {
    let text = app.lang.text();
    let dim = Style::new().add_modifier(Modifier::DIM);
    let mut infos = Vec::new();

    if let Some(alarm) = app.ringing.first() {
        let mut spans = vec![
            Span::styled(
                format!("{} ", text.alarm),
                Style::new().fg(Color::Red).bold(),
            ),
            Span::styled(alarm_time(alarm), Style::new().bold()),
        ];
        if !alarm.label.is_empty() {
            spans.push(Span::raw(format!(" · {}", alarm.label)));
        }
        if app.ringing.len() > 1 {
            spans.push(Span::raw(format!(" +{}", app.ringing.len() - 1)));
        }
        infos.push(Info {
            line: Line::from(spans),
            above: false,
        });
    }

    let place = match app.city {
        Some(city) => vec![
            Line::from(vec![
                Span::styled(city.name, Style::new().bold()),
                Span::styled(format!(" · {}", city.place()), dim),
            ]),
            Line::from(city.name).bold(),
        ],
        None => {
            let zone = app
                .zone
                .tz()
                .map(|tz| format!(" · {}", tz.name()))
                .unwrap_or_default();
            vec![
                Line::from(vec![
                    Span::styled(text.local_time, Style::new().bold()),
                    Span::styled(zone, dim),
                ]),
                Line::from(text.local_time).bold(),
            ]
        }
    };
    infos.push(Info {
        line: widest_fitting(place, width),
        above: true,
    });

    let offset = format_offset(*local.offset());
    let zone = match app.zone.abbreviation(now) {
        Some(abbreviation) => format!("{abbreviation} · {offset}"),
        None => offset.clone(),
    };
    let date = local.date_naive();
    let dates = vec![
        Line::from(format!("{} · {zone}", app.lang.long_date(date))),
        Line::from(format!("{} · {offset}", app.lang.short_date(date))),
        Line::from(app.lang.short_date(date)),
    ];
    infos.push(Info {
        line: widest_fitting(dates, width),
        above: false,
    });

    if let Some(alarm) = app.alarms.upcoming().filter(|_| app.ringing.is_empty()) {
        let mut spans = vec![Span::raw(format!(
            "{} {}",
            text.next_alarm,
            alarm_time(alarm)
        ))];
        if !alarm.label.is_empty() {
            spans.push(Span::raw(format!(" {}", alarm.label)));
        }
        if let Some(next) = alarm.next {
            spans.push(Span::raw(format!(
                " · {} {}",
                text.in_,
                app.lang.countdown(next - now)
            )));
        }
        infos.push(Info {
            line: Line::from(spans).style(dim),
            above: false,
        });
    }
    infos
}

/// The first line that fits in `width`, or the last (shortest) one.
fn widest_fitting(mut lines: Vec<Line<'static>>, width: u16) -> Line<'static> {
    let position = lines
        .iter()
        .position(|line| line.width() <= usize::from(width))
        .unwrap_or(lines.len() - 1);
    lines.swap_remove(position)
}

fn alarm_time(alarm: &Alarm) -> String {
    let format = if alarm.time.format("%S").to_string() == "00" {
        "%H:%M"
    } else {
        "%H:%M:%S"
    };
    alarm.time.format(format).to_string()
}

fn render_keys(app: &App, area: Rect, buf: &mut Buffer) {
    let text = app.lang.text();
    let keys: Keys = if !app.ringing.is_empty() {
        text.ringing_keys
    } else {
        match &app.mode {
            Mode::Clock => text.clock_keys,
            Mode::Search(_) => text.search_keys,
            Mode::Alarms(panel) if panel.input.is_some() => text.input_keys,
            Mode::Alarms(_) => text.alarm_keys,
            Mode::Help => &[],
        }
    };
    let mut spans = Vec::new();
    let mut width = 0;
    for (key, action) in keys {
        let item = key.width() + action.width() + 3;
        if width + item > usize::from(area.width) {
            break;
        }
        width += item;
        spans.push(Span::styled(
            format!(" {key}"),
            Style::new().fg(app.color).bold(),
        ));
        spans.push(Span::styled(
            format!(" {action} "),
            Style::new().add_modifier(Modifier::DIM),
        ));
    }
    Line::from(spans).render(area, buf);
}

/// A bordered box centered in `area`, cleared, with the space inside it.
fn popup(frame: &mut Frame, app: &App, title: &str, size: (u16, u16)) -> Rect {
    let area = frame.area();
    let (width, height) = (size.0.min(area.width), size.1.min(area.height));
    let area = Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    let block = Block::bordered()
        .title(format!(" {title} "))
        .border_style(Style::new().fg(app.color));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);
    inner
}

/// A text field: the end of `value` after `prompt`, with the cursor after it.
fn render_input(frame: &mut Frame, prompt: &str, value: &str, area: Rect) {
    let room = usize::from(area.width).saturating_sub(prompt.width() + 1);
    let mut shown = value;
    while shown.width() > room {
        let mut chars = shown.chars();
        chars.next();
        shown = chars.as_str();
    }
    let line = Line::from(vec![Span::raw(prompt).bold(), Span::raw(shown)]);
    frame.render_widget(line, area);
    let x = area.x + (prompt.width() + shown.width()) as u16;
    frame.set_cursor_position((x.min(area.right().saturating_sub(1)), area.y));
}

fn render_search(frame: &mut Frame, app: &App, search: &Search, now: DateTime<Utc>) {
    let text = app.lang.text();
    let area = popup(frame, app, text.city, (72, 22));
    if area.is_empty() {
        return;
    }
    render_input(frame, "› ", &search.query, Rect { height: 1, ..area });
    let list = Rect {
        y: area.y + 1,
        height: area.height - 1,
        ..area
    };
    if list.is_empty() {
        return;
    }
    let dim = Style::new().add_modifier(Modifier::DIM);
    if search.results.is_empty() {
        let message = if search.query.trim().is_empty() {
            text.city_prompt
        } else {
            text.no_match
        };
        frame.render_widget(Line::styled(message, dim), Rect { height: 1, ..list });
        return;
    }
    let visible = usize::from(list.height);
    let first = search.selected.saturating_sub(visible - 1);
    let buf = frame.buffer_mut();
    for (row, (index, city)) in search
        .results
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
        .enumerate()
    {
        let local = now.with_timezone(&city.tz).fixed_offset();
        let time = format!(
            "{}  {:>9} ",
            local.format("%H:%M"),
            format_offset(*local.offset())
        );
        let place = format!("  {}", city.place());
        let name = format!(" {}", city.name);
        let width = usize::from(list.width);
        let mut spans = vec![Span::styled(name.clone(), Style::new().bold())];
        let used = name.width() + time.width();
        if used + place.width() <= width {
            spans.push(Span::styled(place.clone(), dim));
        }
        let left = Line::from(spans);
        let y = list.y + row as u16;
        let row_area = Rect {
            y,
            height: 1,
            ..list
        };
        let style = if index == search.selected {
            Style::new().add_modifier(Modifier::REVERSED)
        } else {
            Style::new()
        };
        buf.set_style(row_area, style);
        let left_width = left.width() as u16;
        left.render(row_area, buf);
        if usize::from(left_width) + time.width() <= width {
            let x = list.right() - time.width() as u16;
            buf.set_string(x, y, &time, Style::new());
        }
    }
}

fn render_alarms(frame: &mut Frame, app: &App, panel: &AlarmPanel, now: DateTime<Utc>) {
    let text = app.lang.text();
    let alarms = app.alarms.list();
    // One row per alarm (or for the "no alarms" note), then the new alarm being
    // typed and its hint, below a blank row when there is a list above.
    let list_rows = match (alarms.len(), &panel.input) {
        (0, Some(_)) => 0,
        (count, _) => count.max(1) as u16,
    };
    let input_rows = match (&panel.input, list_rows) {
        (None, _) => 0,
        (Some(_), 0) => 2,
        (Some(_), _) => 3,
    };
    let area = popup(frame, app, text.alarms, (68, list_rows + input_rows + 2));
    if area.is_empty() {
        return;
    }
    let dim = Style::new().add_modifier(Modifier::DIM);
    let list = Rect {
        height: area.height.saturating_sub(input_rows),
        ..area
    };
    if alarms.is_empty() {
        if panel.input.is_none() {
            let note = Line::styled(format!(" {}", text.no_alarms), dim);
            frame.render_widget(note, Rect { height: 1, ..list });
        }
    } else if !list.is_empty() {
        let visible = usize::from(list.height.max(1));
        let first = panel.selected.saturating_sub(visible - 1);
        for (row, (index, alarm)) in alarms
            .iter()
            .enumerate()
            .skip(first)
            .take(visible)
            .enumerate()
        {
            let state = match (alarm.enabled(), alarm.daily) {
                (false, _) => text.off,
                (true, true) => text.daily,
                (true, false) => text.once,
            };
            let mark = if alarm.enabled() {
                Span::styled(" ● ", Style::new().fg(Color::Green))
            } else {
                Span::styled(" ○ ", dim)
            };
            let countdown = alarm
                .next
                .map(|next| format!("{} {}", text.in_, app.lang.countdown(next - now)))
                .unwrap_or_default();
            let line = Line::from(vec![
                mark,
                Span::styled(format!("{:<9}", alarm_time(alarm)), Style::new().bold()),
                Span::styled(format!("{state:<10}"), dim),
                Span::raw(format!("{} ", alarm.label)),
                Span::styled(format!("({}) ", alarm.place), dim),
                Span::styled(countdown, dim),
            ]);
            let row_area = Rect {
                y: list.y + row as u16,
                height: 1,
                ..list
            };
            if row_area.bottom() > list.bottom() {
                break;
            }
            let selected = index == panel.selected && panel.input.is_none();
            let line = if selected { line.reversed() } else { line };
            frame.render_widget(line, row_area);
        }
    }
    if let Some(input) = &panel.input {
        let row = Rect {
            x: area.x + 1,
            width: area.width.saturating_sub(1),
            height: 1,
            ..area
        };
        if area.height >= 2 {
            let (message, style) = if panel.invalid {
                (text.invalid_time, Style::new().fg(Color::Red))
            } else {
                (text.alarm_example, dim)
            };
            let hint = Rect {
                y: area.bottom() - 1,
                ..row
            };
            frame.render_widget(Line::styled(message, style), hint);
        }
        let field = Rect {
            y: area.bottom().saturating_sub(2).max(area.y),
            ..row
        };
        render_input(frame, &format!("{}: ", text.new_alarm), input, field);
    }
}

fn render_help(frame: &mut Frame, app: &App) {
    let text = app.lang.text();
    let keys = text.help_keys;
    let area = popup(frame, app, text.help, (56, keys.len() as u16 + 2));
    let key_width = keys.iter().map(|(key, _)| key.width()).max().unwrap_or(0) + 2;
    for (row, (key, action)) in keys.iter().enumerate() {
        if row >= usize::from(area.height) {
            break;
        }
        let line = Line::from(vec![
            Span::styled(
                format!(" {key:<key_width$}"),
                Style::new().fg(app.color).bold(),
            ),
            Span::raw(*action),
        ]);
        frame.render_widget(
            line,
            Rect {
                y: area.y + row as u16,
                height: 1,
                ..area
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alarm::When;
    use crate::cities;
    use crate::i18n::Lang;
    use crate::zone::Zone;
    use chrono::{Duration, NaiveDate};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::crossterm::event::{KeyCode, KeyEvent};

    fn now() -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(2026, 9, 25)
            .unwrap()
            .and_hms_opt(16, 4, 5)
            .unwrap()
            .and_utc()
    }

    fn app() -> App {
        App::new(Lang::En, Zone::City(chrono_tz::America::Sao_Paulo))
    }

    fn draw(app: &App, width: u16, height: u16, at: DateTime<Utc>) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal.draw(|frame| render(frame, app, at)).unwrap();
        terminal.backend().buffer().clone()
    }

    fn screen(buf: &Buffer) -> String {
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol().to_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn with_city(query: &str) -> App {
        let mut app = app();
        app.select_city(cities::db().search(query, 1)[0]);
        app
    }

    #[test]
    fn never_panics_at_any_size_or_mode() {
        let mut apps = vec![app(), with_city("Tokyo")];
        let mut ringing = with_city("Reykjavik");
        ringing.add_alarm(When::In(Duration::seconds(1)), "Stretch".into(), now());
        ringing.tick(now() + Duration::seconds(1));
        apps.push(ringing);
        for mode in 0..3 {
            let mut app = with_city("Auckland");
            app.add_alarm(When::In(Duration::minutes(3)), "Tea".into(), now());
            let key = ['c', 'a', '?'][mode];
            app.on_key(KeyEvent::from(KeyCode::Char(key)), now());
            if mode == 0 {
                for ch in "sao".chars() {
                    app.on_key(KeyEvent::from(KeyCode::Char(ch)), now());
                }
            }
            apps.push(app);
        }
        for app in &apps {
            for (width, height) in [
                (1, 1),
                (5, 2),
                (12, 3),
                (20, 6),
                (27, 4),
                (40, 10),
                (80, 24),
                (120, 30),
                (250, 70),
            ] {
                draw(app, width, height, now());
            }
        }
    }

    #[test]
    fn shows_big_digits_date_and_place() {
        let buf = draw(&app(), 80, 24, now());
        let text = screen(&buf);
        assert!(text.contains('█'), "{text}");
        assert!(
            text.contains("Friday, September 25, 2026 · UTC-03:00"),
            "{text}"
        );
        assert!(text.contains("Local time · America/Sao_Paulo"), "{text}");
        assert!(text.contains("c city"), "{text}");
        assert!(!text.contains('●'), "no map without a city");
    }

    #[test]
    fn falls_back_to_plain_text_in_tiny_panes() {
        let text = screen(&draw(&app(), 12, 3, now()));
        assert!(text.contains("13:04:05"), "{text}");
        let text = screen(&draw(&app(), 6, 1, now()));
        assert_eq!(text, "13:04 ");
    }

    #[test]
    fn a_city_brings_up_the_map_and_its_zone() {
        let app = with_city("Tokyo");
        let text = screen(&draw(&app, 120, 40, now()));
        assert!(text.contains("Tokyo · Japan"), "{text}");
        assert!(
            text.contains("Saturday, September 26, 2026 · JST · UTC+09:00"),
            "{text}"
        );
        assert!(text.contains('●') && text.contains(" Tokyo "), "{text}");
        assert!(text.contains('#'), "ASCII map: {text}");
    }

    #[test]
    fn shows_the_next_alarm() {
        let mut app = app();
        app.add_alarm(When::In(Duration::minutes(90)), "Meeting".into(), now());
        let text = screen(&draw(&app, 80, 24, now()));
        assert!(
            text.contains("Alarm 14:34:05 Meeting · in 1h 30m"),
            "{text}"
        );
    }

    #[test]
    fn blinks_while_the_alarm_rings() {
        let mut app = app();
        app.add_alarm(When::In(Duration::seconds(1)), "Stretch".into(), now());
        let ring = now() + Duration::seconds(1);
        app.tick(ring);
        let on = draw(&app, 80, 24, ring);
        let off = draw(&app, 80, 24, ring + Duration::milliseconds(500));
        assert_eq!(on[(0, 0)].bg, Color::Red);
        assert_eq!(off[(0, 0)].bg, Color::Reset);
        let text = screen(&off);
        assert!(text.contains("ALARM 13:04:06 · Stretch"), "{text}");
        assert!(text.contains("any key stop"), "{text}");
    }

    #[test]
    fn lists_cities_with_their_local_time() {
        let mut app = app();
        app.on_key(KeyEvent::from(KeyCode::Char('c')), now());
        for ch in "lisboa".chars() {
            app.on_key(KeyEvent::from(KeyCode::Char(ch)), now());
        }
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(text.contains("› lisboa"), "{text}");
        assert!(text.contains("Lisbon"), "{text}");
        assert!(text.contains("17:04  UTC+01:00"), "{text}");
    }

    #[test]
    fn speaks_portuguese() {
        let lang = if std::env::var("UILANG").as_deref() == Ok("en") {
            Lang::En
        } else {
            Lang::Pt
        };
        let mut app = App::new(lang, Zone::City(chrono_tz::America::Sao_Paulo));
        app.add_alarm(When::In(Duration::minutes(5)), String::new(), now());
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(
            text.contains("sexta-feira, 25 de setembro de 2026"),
            "{text}"
        );
        assert!(text.contains("Hora local"), "{text}");
        assert!(text.contains("Alarme 13:09:05 · em 5min 00s"), "{text}");
        assert!(text.contains("c cidade"), "{text}");
    }
}
