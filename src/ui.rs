//! Drawing: the tab bar, the clock, the world map, the key hints and the pop-ups.

use std::ops::Range;

use chrono::{DateTime, FixedOffset, Utc};
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Widget};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::alarm::Alarm;
use crate::app::{AlarmPanel, App, CalendarPanel, CalendarSource, Mode, Search, ThemePicker};
use crate::calendar;
use crate::i18n::Keys;
use crate::maccal::Access;
use crate::theme::THEMES;
use crate::weather::{self, Hour};
use crate::weather_fx;
use crate::worldmap::{MapColors, Marker, WorldMap};
use crate::zone::format_offset;
use crate::{clockface, layout, solar};

/// Draws the whole screen. Returns how often, in milliseconds, the picture
/// changes by itself: the frame period of a live weather scene on screen.
pub fn render(frame: &mut Frame, app: &App, now: DateTime<Utc>) -> Option<u64> {
    let area = frame.area();
    let palette = app.palette();
    let screen = layout::split(area, app.show_map, app.shown_tabs().len() > 1);
    let buf = frame.buffer_mut();
    buf.set_style(area, palette.base());
    if let Some(tabs) = screen.tabs {
        render_tabs(app, now, tabs, buf);
    }
    // The tab wants its map but the window has no room for it.
    let map_no_room = app.show_map && screen.map.is_none();
    let animation = render_clock(app, now, screen.clock, map_no_room, buf);
    if let Some(map) = screen.map {
        // A city tab marks its city; the local tab marks every city with a tab.
        let cities = match app.city() {
            Some(city) => vec![city],
            None => {
                let mut cities: Vec<_> = app.tabs.iter().filter_map(|tab| tab.city).collect();
                cities.extend(app.home.filter(|home| !cities.contains(home)));
                cities
            }
        };
        WorldMap {
            style: app.map_style,
            colors: MapColors {
                day: palette.day,
                dusk: palette.dusk,
                night: palette.night,
                marker: palette.marker,
                sun: palette.sun,
            },
            sun: Some(solar::subsolar_point(now)),
            markers: cities
                .iter()
                .map(|city| Marker {
                    lat: city.lat,
                    lon: city.lon,
                    label: city.name,
                })
                .collect(),
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
        Mode::Themes(picker) => render_themes(frame, app, picker),
        Mode::Calendar(panel) => render_calendar(frame, app, panel, now),
        Mode::Help => render_help(frame, app),
    }
    if app.blink_on(now) {
        frame.buffer_mut().set_style(area, palette.flash());
    }
    animation
}

/// A tab per clock with its current time. When they do not all fit the times
/// go first, then the tabs farthest from the one on screen, with `‹` and `›`
/// marking the hidden ones.
fn render_tabs(app: &App, now: DateTime<Utc>, area: Rect, buf: &mut Buffer) {
    let text = app.lang.text();
    let offered = app.shown_tabs();
    let tabs = &app.tabs[offered.clone()];
    let active = app.active.saturating_sub(offered.start).min(tabs.len() - 1);
    let names: Vec<&str> = tabs
        .iter()
        .map(|tab| tab.city.map_or(text.local_tab, |city| city.name))
        .collect();
    let timed: Vec<String> = tabs
        .iter()
        .zip(&names)
        .map(|(tab, name)| {
            let time = app.zone_of(tab).local_time(now).format("%H:%M");
            format!(" {name} {time} ")
        })
        .collect();
    let labels = if row_width(&timed) <= usize::from(area.width) {
        timed
    } else {
        names.iter().map(|name| format!(" {name} ")).collect()
    };
    let shown = visible_tabs(&labels, active, usize::from(area.width));

    let dim = app.palette().muted();
    let mut x = area.x;
    if shown.start > 0 {
        buf.set_string(x, area.y, "‹", dim);
        x += 1;
    }
    for index in shown.clone() {
        if index > shown.start {
            buf.set_string(x, area.y, "│", dim);
            x += 1;
        }
        let style = if index == active {
            Style::new()
                .fg(app.palette().accent)
                .add_modifier(Modifier::REVERSED | Modifier::BOLD)
        } else {
            dim
        };
        let room = usize::from(area.right().saturating_sub(x));
        x = buf.set_stringn(x, area.y, &labels[index], room, style).0;
    }
    if shown.end < labels.len() && x < area.right() {
        buf.set_string(x, area.y, "›", dim);
    }
}

/// Width of the tabs side by side, one separator between each two.
fn row_width(labels: &[String]) -> usize {
    labels.iter().map(|label| label.width()).sum::<usize>() + labels.len().saturating_sub(1)
}

/// The tabs that fit in `width` around the active one, growing to both sides
/// in turn, with room left for the `‹` `›` marks when some stay hidden.
fn visible_tabs(labels: &[String], active: usize, width: usize) -> Range<usize> {
    if row_width(labels) <= width {
        return 0..labels.len();
    }
    let room = width.saturating_sub(2);
    let mut shown = active..active + 1;
    let mut used = labels[active].width();
    let mut grew = true;
    while grew {
        grew = false;
        if let Some(label) = labels.get(shown.end)
            && used + label.width() < room
        {
            used += label.width() + 1;
            shown.end += 1;
            grew = true;
        }
        if shown.start > 0 && used + labels[shown.start - 1].width() < room {
            used += labels[shown.start - 1].width() + 1;
            shown.start -= 1;
            grew = true;
        }
    }
    shown
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

/// Draws the clock, with the weather scene in its top right corner when it
/// fits. Returns the scene's frame period while a live one shows.
fn render_clock(
    app: &App,
    now: DateTime<Utc>,
    area: Rect,
    map_no_room: bool,
    buf: &mut Buffer,
) -> Option<u64> {
    if area.is_empty() {
        return None;
    }
    let local = app.zone().local_time(now);
    let full = local.format("%H:%M:%S").to_string();
    let short = local.format("%H:%M").to_string();
    let texts: Vec<&str> = if app.show_seconds {
        vec![&full, &short]
    } else {
        vec![&short]
    };
    let lines = |width, weather| infos(app, now, local, width, map_no_room, weather);
    let scene = app.fx_scene(now);
    let plan = plan(&texts, &lines, area, scene.is_some(), app.show_weather);
    let palette = app.palette();
    for (row, rect) in plan.rows.into_iter().zip(plan.rects) {
        match row {
            _ if rect.is_empty() => {}
            Row::Digits(text, scale) => clockface::draw(
                text,
                scale,
                (rect.x, rect.y),
                Style::new().fg(palette.digits),
                buf,
            ),
            Row::Text(line) => line.centered().render(rect, buf),
            Row::Gap => {}
        }
    }
    let (spot, scene) = (plan.scene?, scene?);
    weather_fx::draw(buf, spot, &palette, scene, now);
    scene.frame_ms()
}

/// The clock's rows, the rectangle each takes, and the weather scene's spot.
struct Plan<'a> {
    rows: Vec<Row<'a>>,
    rects: Vec<Rect>,
    scene: Option<Rect>,
}

/// Lays out the clock in `area`, and the weather scene too when `scene`;
/// `lines(width, weather)` gives the lines around the digits for a width,
/// the forecast's among them or not.
///
/// The scene takes the top right corner the clock leaves free, the clock
/// stepping down a few rows if that makes room. Failing that the scene keeps
/// the corner all the same and the clock makes do left of it, with smaller
/// digits, without seconds or as plain text, whichever fits: a pane shows
/// the scene whenever it has room for the smallest one, so a larger pane
/// never lacks what a smaller one shows. Only where not even that fits does
/// the weather it stood for show as text.
fn plan<'a>(
    texts: &[&'a str],
    lines: &dyn Fn(u16, bool) -> Vec<Info>,
    area: Rect,
    scene: bool,
    weather: bool,
) -> Plan<'a> {
    let rows = arrange(texts, &lines(area.width, weather), area);
    if scene {
        if let Some((rects, spot)) = settle(&rows, area) {
            return Plan {
                rows,
                rects,
                scene: Some(spot),
            };
        }
        if let Some(plan) = beside(texts, lines, area, weather) {
            return plan;
        }
    }
    let rows = if scene && !weather {
        arrange(texts, &lines(area.width, true), area)
    } else {
        rows
    };
    let rects = place(&rows, area, 0);
    Plan {
        rows,
        rects,
        scene: None,
    }
}

/// How a clock beside the scene ranks, the higher the better: every line
/// whole, then the scale of its digits, whether it keeps the seconds, a
/// column of margin at its left, and the rows of the scene.
type Rank = (bool, u16, bool, u16, u16);

/// The scene in the top right corner and the clock centered left of it,
/// two columns away: first with every line whole, then the clock as large
/// as it can be there (with seconds, if it can keep them), then a column of
/// margin at its left, then the larger scene; `None` when not even the
/// smallest scene fits.
fn beside<'a>(
    texts: &[&'a str],
    lines: &dyn Fn(u16, bool) -> Vec<Info>,
    area: Rect,
    weather: bool,
) -> Option<Plan<'a>> {
    let tallest = (area.height / 2).clamp(weather_fx::MIN_ROWS, weather_fx::MAX_ROWS);
    let mut best: Option<(Rank, Plan<'a>)> = None;
    for tall in weather_fx::MIN_ROWS..=tallest {
        let Some(spot) = spot(area, tall) else {
            continue;
        };
        for margin in [0, 1] {
            let left = Rect {
                x: area.x + margin,
                width: spot.x.saturating_sub(area.x + margin + 2),
                ..area
            };
            let rows = arrange(texts, &lines(left.width, weather), left);
            // Plain text counts as scale 0.
            let (scale, seconds) = rows
                .iter()
                .find_map(|row| match row {
                    Row::Digits(text, scale) => Some((*scale, text.len() > 5)),
                    _ => None,
                })
                .unwrap_or((0, false));
            let whole = rows.iter().all(|row| match row {
                Row::Text(line) => line.width() <= usize::from(left.width),
                _ => true,
            });
            let rank = (whole, scale, seconds, margin, tall);
            if best.as_ref().is_none_or(|(best, _)| rank > *best) {
                let rects = place(&rows, left, 0);
                let plan = Plan {
                    rows,
                    rects,
                    scene: Some(spot),
                };
                best = Some((rank, plan));
            }
        }
    }
    best.map(|(_, plan)| plan)
}

/// Where each row goes, stacked in the middle of `area` and `drop` rows
/// lower: the rectangle the digits or the line take, centered. A gap, or a
/// row past the bottom, takes none.
fn place(rows: &[Row], area: Rect, drop: u16) -> Vec<Rect> {
    let total: u16 = rows.iter().map(Row::height).sum();
    let mut y = area.y + area.height.saturating_sub(total) / 2 + drop;
    rows.iter()
        .map(|row| {
            let height = row.height();
            let top = y;
            y += height;
            let width = match row {
                _ if y > area.bottom() => return Rect::default(),
                Row::Digits(text, scale) => clockface::size(text, *scale).0,
                // A line too wide for the clock keeps the whole row, where
                // it gets cut.
                Row::Text(line) => u16::try_from(line.width())
                    .ok()
                    .filter(|&width| width <= area.width)
                    .unwrap_or(area.width),
                Row::Gap => return Rect::default(),
            };
            let x = area.x + area.width.saturating_sub(width) / 2;
            Rect::new(x, top, width, height)
        })
        .collect()
}

/// The rows' places with the weather scene in the clock's top right corner,
/// the clock stepping down as few rows as it takes to make room; `None`
/// when that is not enough.
fn settle(rows: &[Row], area: Rect) -> Option<(Vec<Rect>, Rect)> {
    let total: u16 = rows.iter().map(Row::height).sum();
    let spare = area.height.saturating_sub(total);
    (0..=spare - spare / 2).find_map(|drop| {
        let rects = place(rows, area, drop);
        corner(&rects, area).map(|spot| (rects, spot))
    })
}

/// Where the weather scene goes: the largest that fits in the top right
/// corner of the clock, a cell clear of the digits and the lines; `None` when
/// even the smallest would touch them.
fn corner(taken: &[Rect], area: Rect) -> Option<Rect> {
    (weather_fx::MIN_ROWS..=weather_fx::MAX_ROWS)
        .rev()
        .find_map(|rows| {
            let spot = spot(area, rows)?;
            let clear = Rect {
                x: spot.x - 1,
                width: spot.width + 1,
                height: spot.height + 1,
                ..spot
            };
            taken
                .iter()
                .all(|rect| !rect.intersects(clear))
                .then_some(spot)
        })
}

/// The weather scene `rows` tall in the top right corner of `area`, a cell
/// in from its edges and with a cell to spare to its left and below; `None`
/// when that does not fit.
fn spot(area: Rect, rows: u16) -> Option<Rect> {
    let width = weather_fx::width(rows);
    let spot = Rect::new(
        area.right().checked_sub(width + 1)?,
        area.y + 1,
        width,
        rows,
    );
    (spot.x > area.x && spot.bottom() < area.bottom()).then_some(spot)
}

/// Big digits and as many info lines as fit, most important first; seconds are
/// dropped before the digits shrink to plain text.
fn arrange<'a>(texts: &[&'a str], infos: &[Info], area: Rect) -> Vec<Row<'a>> {
    let rows = |shown: &[Info], middle: Row<'a>, gaps: bool| {
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

/// The lines around the digits; `weather` adds the forecast's.
fn infos(
    app: &App,
    now: DateTime<Utc>,
    local: DateTime<FixedOffset>,
    width: u16,
    map_no_room: bool,
    weather: bool,
) -> Vec<Info> {
    let text = app.lang.text();
    let palette = app.palette();
    let dim = palette.muted();
    let mut infos = Vec::new();

    if app.unsaved {
        infos.push(Info {
            line: Line::styled(text.unsaved, Style::new().fg(palette.marker)),
            above: false,
        });
    }

    if let Some(alarm) = app.ringing.first() {
        let mut spans = vec![
            Span::styled(
                format!("{} ", text.alarm),
                Style::new().fg(palette.marker).bold(),
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

    let place = match app.city() {
        Some(city) => vec![
            Line::from(vec![
                Span::styled(city.name, Style::new().bold()),
                Span::styled(format!(" · {}", city.place()), dim),
            ]),
            Line::from(city.name).bold(),
        ],
        None => {
            let zone = app
                .zone()
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
    let zone = match app.zone().abbreviation(now) {
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
    infos.extend(event_line(app, now, width));
    if weather {
        infos.extend(weather_lines(app, now, width));
    }
    if map_no_room {
        infos.push(Info {
            line: Line::styled(text.map_no_room, dim),
            above: false,
        });
    }
    infos
}

/// The next calendar event, in the time of the tab on screen.
fn event_line(app: &App, now: DateTime<Utc>, width: u16) -> Option<Info> {
    let text = app.lang.text();
    let dim = app.palette().muted();
    let calendar = &app.calendar;
    let Some(event) = calendar.next(now) else {
        // A calendar that stopped answering says so rather than going quiet:
        // macOS forgets meridian's access to the Mac Calendar when it cannot
        // recognize the binary, after an update signed the old way.
        let line = match (calendar.mac, calendar.access, &calendar.error) {
            (true, Some(Access::Denied | Access::NotAsked), _) => text.calendar_no_access,
            (_, _, Some(_)) if calendar.connected() => text.calendar_unavailable,
            _ => return None,
        };
        return Some(Info {
            line: Line::styled(line, dim),
            above: false,
        });
    };
    let zone = app.zone();
    let start = zone.local_time(event.start);
    let when = if start.date_naive() == zone.local_time(now).date_naive() {
        start.format("%H:%M").to_string()
    } else {
        format!(
            "{} {}",
            app.lang.short_weekday(start.date_naive()),
            start.format("%H:%M")
        )
    };
    let countdown = app.lang.countdown(event.start - now);
    let title = &event.title;
    // A title too long for the pane gives way, so the time stays in view.
    let short = shorten(title, usize::from(width).saturating_sub(when.width() + 1));
    let lines = [
        format!(
            "{} {when} {title} · {} {countdown}",
            text.next_event, text.in_
        ),
        format!("{when} {title} · {} {countdown}", text.in_),
        format!("{when} {title}"),
        format!("{when} {short}"),
    ];
    Some(Info {
        line: widest_fitting(
            lines
                .into_iter()
                .map(|line| Line::styled(line, dim))
                .collect(),
            width,
        ),
        above: false,
    })
}

/// For a city tab: the weather now with the rain to come, then the next hours
/// with their temperature and chance of rain, as many as fit.
fn weather_lines(app: &App, now: DateTime<Utc>, width: u16) -> Vec<Info> {
    // The local time tab shows the weather of your city, once you pick it.
    let place = app.city().or(app.home);
    let Some(slot) = place.and_then(|city| app.weather_of(&city)) else {
        return Vec::new();
    };
    let text = app.lang.text();
    let palette = app.palette();
    let Some(forecast) = &slot.forecast else {
        let lines = match (&slot.failed, slot.pending()) {
            // The reason, when it fits, says what to fix.
            (Some(error), _) => vec![
                format!("{}: {error}", text.weather_unavailable),
                text.weather_unavailable.to_owned(),
            ],
            (None, true) => vec![text.loading_weather.to_owned()],
            (None, false) => return Vec::new(),
        };
        let lines = lines
            .into_iter()
            .map(|line| Line::styled(line, palette.muted()))
            .collect();
        return vec![Info {
            line: widest_fitting(lines, width),
            above: false,
        }];
    };
    let name = match (app.city(), place) {
        (None, Some(home)) => format!("{}: ", home.name),
        _ => String::new(),
    };
    let zone = app.zone();
    let hour_of = |at: DateTime<Utc>| zone.local_time(at).format("%Hh").to_string();
    let degrees = |temperature: f64| format!("{}°", temperature.round() as i64);

    let rain = match forecast.rain_peak(now, 12) {
        Some(peak) => format!(
            "{} {}% {} {}",
            text.rain,
            peak.rain,
            text.at,
            hour_of(peak.at)
        ),
        None => text.no_rain.to_owned(),
    };
    let current = format!("{}C", degrees(forecast.temperature));
    let description = weather::describe(forecast.code, app.lang);
    let summary = widest_fitting(
        vec![
            Line::from(format!("{name}{description} {current} · {rain}")),
            Line::from(format!("{name}{current} · {rain}")),
            Line::from(format!("{name}{current}")),
        ],
        width,
    );
    let mut lines = vec![Info {
        line: summary,
        above: false,
    }];

    let rain_style = |hour: &Hour| {
        if hour.rain >= 30 {
            Style::new().fg(palette.rain)
        } else {
            palette.muted()
        }
    };
    let mut spans: Vec<Span> = Vec::new();
    let mut used = 0;
    for hour in forecast.ahead(now).filter(|hour| hour.at > now).take(12) {
        let label = format!("{} {}", hour_of(hour.at), degrees(hour.temperature));
        let chance = format!(" {}%", hour.rain);
        let gap = if spans.is_empty() { 0 } else { 2 };
        let item = gap + label.width() + chance.width();
        if used + item > usize::from(width) {
            break;
        }
        if gap > 0 {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::raw(label));
        spans.push(Span::styled(chance, rain_style(hour)));
        used += item;
    }
    if !spans.is_empty() {
        lines.push(Info {
            line: Line::from(spans),
            above: false,
        });
    }
    lines
}

/// The first line that fits in `width`, or the last (shortest) one.
fn widest_fitting(mut lines: Vec<Line<'static>>, width: u16) -> Line<'static> {
    let position = lines
        .iter()
        .position(|line| line.width() <= usize::from(width))
        .unwrap_or(lines.len() - 1);
    lines.swap_remove(position)
}

/// `text` cut down to `width` columns, ending in "…" when anything was cut.
fn shorten(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    let Some(room) = width.checked_sub(1) else {
        return String::new();
    };
    let mut used = 0;
    let kept: String = text
        .chars()
        .take_while(|character| {
            used += character.width().unwrap_or(0);
            used <= room
        })
        .collect();
    format!("{}…", kept.trim_end())
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
            Mode::Themes(_) => text.theme_keys,
            Mode::Calendar(CalendarPanel::Choose(_)) => text.choose_keys,
            Mode::Calendar(CalendarPanel::Address(_)) => text.input_keys,
            Mode::Calendar(CalendarPanel::Status) if app.calendar.mac => text.mac_calendar_keys,
            Mode::Calendar(CalendarPanel::Status) => text.calendar_keys,
            Mode::Help => &[],
        }
    };
    // Tab keys only where they do something.
    let applies = |key: &str| match key {
        "←→" => app.shown_tabs().len() > 1,
        "x" => app.active > 0,
        _ => true,
    };
    let mut spans = Vec::new();
    let mut width = 0;
    for (key, action) in keys.iter().filter(|(key, _)| applies(key)) {
        let item = key.width() + action.width() + 3;
        if width + item > usize::from(area.width) {
            break;
        }
        width += item;
        spans.push(Span::styled(
            format!(" {key}"),
            Style::new().fg(app.palette().accent).bold(),
        ));
        spans.push(Span::styled(format!(" {action} "), app.palette().muted()));
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
        .border_style(Style::new().fg(app.palette().accent));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.buffer_mut().set_style(area, app.palette().base());
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
    let title = if search.home {
        text.home_city
    } else {
        text.city
    };
    let area = popup(frame, app, title, (72, 22));
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
    let dim = app.palette().muted();
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
    let dim = app.palette().muted();
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
                Span::styled(" ● ", Style::new().fg(app.palette().day))
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
                (text.invalid_time, Style::new().fg(app.palette().marker))
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

/// The themes, each with a strip of its colors: background, digits, accent,
/// land and marker.
fn render_themes(frame: &mut Frame, app: &App, picker: &ThemePicker) {
    let text = app.lang.text();
    let area = popup(frame, app, text.themes, (40, 22));
    if area.is_empty() {
        return;
    }
    render_input(frame, "› ", &picker.query, Rect { height: 1, ..area });
    let list = Rect {
        y: area.y + 1,
        height: area.height - 1,
        ..area
    };
    if list.is_empty() {
        return;
    }
    if picker.matches.is_empty() {
        let note = Line::styled(text.no_theme, app.palette().muted());
        frame.render_widget(note, Rect { height: 1, ..list });
        return;
    }
    let visible = usize::from(list.height);
    let first = picker.selected.saturating_sub(visible - 1);
    for (row, (position, &index)) in picker
        .matches
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
        .enumerate()
    {
        let theme = THEMES[index].palette(app.truecolor);
        let mut spans = vec![Span::raw(" ")];
        for color in [
            theme.bg,
            theme.digits,
            theme.accent,
            theme.day,
            theme.marker,
        ] {
            spans.push(Span::styled("█", Style::new().fg(color)));
        }
        spans.push(Span::raw(" "));
        let name = Span::raw(format!(" {} ", THEMES[index].name));
        spans.push(if position == picker.selected {
            name.reversed().bold()
        } else {
            name
        });
        let row_area = Rect {
            y: list.y + row as u16,
            height: 1,
            ..list
        };
        frame.render_widget(Line::from(spans), row_area);
    }
}

/// Picking where events come from; typing an iCal address, with where to find
/// it; or the source, the reminder and the next events.
fn render_calendar(frame: &mut Frame, app: &App, panel: &CalendarPanel, now: DateTime<Utc>) {
    let text = app.lang.text();
    let palette = app.palette();
    let dim = palette.muted();
    let calendar = &app.calendar;
    let mut lines: Vec<Line> = Vec::new();
    let mut input = None;
    match panel {
        CalendarPanel::Choose(selected) => {
            for (index, source) in crate::app::calendar_sources().iter().enumerate() {
                let (name, about) = match source {
                    CalendarSource::Mac => (text.mac_calendar, text.mac_calendar_about),
                    CalendarSource::Ical => (text.calendar_address, text.ical_about),
                };
                let line = if index == *selected {
                    Line::styled(format!(" › {name}"), Style::new().fg(palette.accent).bold())
                } else {
                    Line::raw(format!("   {name}"))
                };
                lines.push(line);
                lines.push(Line::styled(format!("     {about}"), dim));
            }
        }
        CalendarPanel::Address(value) => {
            lines.extend(
                text.calendar_how
                    .iter()
                    .map(|line| Line::styled(format!(" {line}"), dim)),
            );
            lines.push(Line::raw(""));
            input = Some(value.as_str());
        }
        CalendarPanel::Status => {
            let source = match &calendar.address {
                _ if calendar.mac => text.mac_calendar.to_owned(),
                Some(address) => calendar::describe_address(address),
                None => String::new(),
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {}: ", text.source), dim),
                Span::raw(source),
            ]));
            let reminder = match calendar.reminder {
                0 => text.off.to_owned(),
                minutes => format!("{minutes} {}", text.minutes_before),
            };
            lines.push(Line::from(vec![
                Span::styled(format!(" {}: ", text.reminder), dim),
                Span::raw(reminder),
            ]));
            let warn = Style::new().fg(palette.marker);
            match (
                calendar.mac,
                calendar.access,
                &calendar.error,
                calendar.updated,
            ) {
                (true, _, _, _) if calendar.asking => {
                    lines.push(Line::styled(
                        format!(" {}", text.asking_access),
                        Style::new().fg(palette.accent),
                    ));
                }
                (true, Some(Access::Denied), _, _) => {
                    lines.push(Line::styled(format!(" {}", text.access_denied), warn))
                }
                (true, Some(Access::NotAsked), Some(detail), _) => {
                    lines.push(Line::styled(
                        format!(" {}: {detail}", text.access_not_shown),
                        warn,
                    ));
                    lines.push(Line::styled(format!(" {}", text.access_not_asked), dim));
                }
                (true, Some(Access::NotAsked), None, _) => {
                    lines.push(Line::styled(format!(" {}", text.access_not_asked), dim))
                }
                (_, _, Some(error), _) => lines.push(Line::styled(
                    format!(" {}: {error}", text.calendar_unavailable),
                    warn,
                )),
                (_, _, None, Some(at)) => lines.push(Line::styled(
                    format!(
                        " {} {}",
                        text.updated_at,
                        app.system.local_time(at).format("%H:%M")
                    ),
                    dim,
                )),
                _ => {}
            }
            lines.push(Line::raw(""));
            let upcoming: Vec<_> = calendar
                .events
                .iter()
                .filter(|event| event.start > now)
                .take(10)
                .collect();
            if upcoming.is_empty() && calendar.updated.is_some() {
                lines.push(Line::styled(format!(" {}", text.no_events), dim));
                if calendar.mac {
                    lines.push(Line::styled(format!(" {}", text.no_events_mac), dim));
                }
            }
            for event in upcoming {
                let start = app.system.local_time(event.start);
                lines.push(Line::from(vec![
                    Span::styled(
                        format!(
                            " {} {} ",
                            app.lang.short_weekday(start.date_naive()),
                            start.format("%H:%M")
                        ),
                        Style::new().fg(palette.accent),
                    ),
                    Span::raw(event.title.clone()),
                ]));
            }
        }
    }
    let input_rows = u16::from(input.is_some());
    let area = popup(
        frame,
        app,
        text.calendar,
        (76, lines.len() as u16 + input_rows + 2),
    );
    for (row, line) in lines.into_iter().enumerate() {
        let y = area.y + row as u16;
        if y >= area.bottom().saturating_sub(input_rows) {
            break;
        }
        frame.render_widget(
            line,
            Rect {
                y,
                height: 1,
                ..area
            },
        );
    }
    if let Some(input) = input
        && !area.is_empty()
    {
        let field = Rect {
            x: area.x + 1,
            y: area.bottom() - 1,
            width: area.width.saturating_sub(1),
            height: 1,
        };
        render_input(frame, &format!("{}: ", text.calendar_address), input, field);
    }
}

fn render_help(frame: &mut Frame, app: &App) {
    let text = app.lang.text();
    let keys = text.help_keys;
    let area = popup(frame, app, text.help, (64, keys.len() as u16 + 2));
    let key_width = keys.iter().map(|(key, _)| key.width()).max().unwrap_or(0) + 2;
    for (row, (key, action)) in keys.iter().enumerate() {
        if row >= usize::from(area.height) {
            break;
        }
        let line = Line::from(vec![
            Span::styled(
                format!(" {key:<key_width$}"),
                Style::new().fg(app.palette().accent).bold(),
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
    use ratatui::style::Color;

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
        terminal
            .draw(|frame| {
                render(frame, app, at);
            })
            .unwrap();
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

    fn with_cities(queries: &[&str]) -> App {
        let mut app = app();
        for query in queries {
            app.open_city(cities::db().search(query, 1)[0]);
        }
        app
    }

    fn with_city(query: &str) -> App {
        let mut app = app();
        app.open_city(cities::db().search(query, 1)[0]);
        app
    }

    #[test]
    fn never_panics_at_any_size_or_mode() {
        let mut apps = vec![app(), with_city("Tokyo")];
        let mut ringing = with_city("Reykjavik");
        ringing.add_alarm(When::In(Duration::seconds(1)), "Stretch".into(), now());
        ringing.tick(now() + Duration::seconds(1));
        apps.push(ringing);
        let many = [
            "Tokyo", "London", "Lima", "Cairo", "Sydney", "Moscow", "Nairobi", "Denver", "Mumbai",
            "Seoul", "Oslo", "Bogotá",
        ];
        apps.push(with_cities(&many));
        let mut overview = with_cities(&many);
        overview.active = 0;
        overview.show_map = true;
        apps.push(overview);
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

    #[test]
    fn shows_a_tab_per_clock_with_its_time() {
        let app = with_cities(&["Tokyo", "London"]);
        let text = screen(&draw(&app, 100, 30, now()));
        let bar = text.lines().next().unwrap();
        assert_eq!(
            bar.trim_end(),
            " Tokyo 01:04 │ London 17:04",
            "local time steps aside"
        );
        assert!(text.contains("c city  ←→ tabs  x close"), "{text}");
    }

    #[test]
    fn hides_the_tab_bar_with_only_the_local_time() {
        let text = screen(&draw(&app(), 80, 24, now()));
        assert!(!text.contains("Local 13:04"), "{text}");
        assert!(!text.contains("←→") && !text.contains("x close"), "{text}");
    }

    #[test]
    fn scrolls_the_tabs_to_keep_the_one_on_screen_visible() {
        let cities = [
            "Tokyo", "London", "Lima", "Cairo", "Sydney", "Moscow", "Nairobi",
        ];
        let text = screen(&draw(&with_cities(&cities), 40, 20, now()));
        let bar = text.lines().next().unwrap();
        assert_eq!(bar.trim_end(), "‹ Cairo │ Sydney │ Moscow │ Nairobi");
    }

    #[test]
    fn the_local_map_marks_your_city() {
        let mut app = app();
        app.home = Some(cities::db().search("Belo Horizonte", 1)[0]);
        app.show_map = true;
        let text = screen(&draw(&app, 120, 40, now()));
        assert_eq!(text.matches('●').count(), 1, "{text}");
        assert!(text.contains(" Belo Horizonte "), "{text}");
        assert!(text.contains("Local time"), "{text}");
    }

    #[test]
    fn a_city_tab_marks_only_its_city() {
        let text = screen(&draw(&with_cities(&["Tokyo", "London"]), 120, 40, now()));
        let below_tabs = text.lines().skip(1).collect::<Vec<_>>().join("\n");
        assert_eq!(below_tabs.matches('●').count(), 1, "{text}");
    }

    #[test]
    fn says_when_the_map_does_not_fit() {
        let app = with_city("Tokyo");
        let small = screen(&draw(&app, 50, 10, now()));
        assert!(small.contains("The map needs a bigger window"), "{small}");
        let large = screen(&draw(&app, 120, 40, now()));
        assert!(!large.contains("bigger window"), "{large}");
        let mut without_map = with_city("Tokyo");
        without_map.show_map = false;
        let hidden = screen(&draw(&without_map, 50, 10, now()));
        assert!(!hidden.contains("bigger window"), "{hidden}");
    }

    fn cell_colors(buf: &Buffer, symbol: &str) -> (Color, Color) {
        let cell = buf
            .content()
            .iter()
            .find(|cell| cell.symbol() == symbol)
            .unwrap();
        (cell.fg, cell.bg)
    }

    #[test]
    fn paints_the_screen_with_the_theme() {
        let mut app = app();
        app.theme = crate::theme::find("Tokyo Night").unwrap();
        let buf = draw(&app, 80, 24, now());
        let background = Color::Rgb(0x1a, 0x1b, 0x26);
        assert!(buf.content().iter().all(|cell| cell.bg == background));
        assert_eq!(
            cell_colors(&buf, "█").0,
            Color::Rgb(0x7a, 0xa2, 0xf7),
            "digits"
        );
    }

    #[test]
    fn the_command_line_color_wins_over_the_theme_digits() {
        let mut app = app();
        app.theme = crate::theme::find("Dracula").unwrap();
        app.digits_color = Some(Color::Green);
        assert_eq!(cell_colors(&draw(&app, 80, 24, now()), "█").0, Color::Green);
    }

    #[test]
    fn lists_the_themes_with_their_colors() {
        let mut app = app();
        app.on_key(KeyEvent::from(KeyCode::Char('t')), now());
        let text = screen(&draw(&app, 60, 30, now()));
        assert!(
            text.contains(" Terminal ") && text.contains(" Tokyo Night "),
            "{text}"
        );
        assert!(text.contains("↑↓ try  Enter use  Esc cancel"), "{text}");
        for ch in "rose".chars() {
            app.on_key(KeyEvent::from(KeyCode::Char(ch)), now());
        }
        let text = screen(&draw(&app, 60, 30, now()));
        assert!(
            text.contains("Rosé Pine Moon") && !text.contains("Dracula"),
            "{text}"
        );
    }

    #[test]
    fn blinks_in_the_theme_colors() {
        let mut app = app();
        app.theme = crate::theme::find("Nord").unwrap();
        app.add_alarm(When::In(Duration::seconds(1)), String::new(), now());
        let ring = now() + Duration::seconds(1);
        app.tick(ring);
        let buf = draw(&app, 80, 24, ring);
        assert_eq!(buf[(0, 0)].bg, Color::Rgb(0xbf, 0x61, 0x6a));
    }

    #[test]
    fn says_when_the_state_could_not_be_saved() {
        let mut app = app();
        app.unsaved = true;
        let text = screen(&draw(&app, 80, 24, now()));
        assert!(text.contains("Could not save the state"), "{text}");
    }

    fn with_weather(query: &str) -> App {
        let mut app = with_city(query);
        let city = app.city().unwrap();
        let forecast = weather::parse(weather::tests::SAMPLE).unwrap();
        app.weather.insert(
            weather::key(&city),
            crate::app::WeatherSlot::ready(forecast),
        );
        app
    }

    #[test]
    fn shows_the_city_weather_and_the_next_hours() {
        let text = screen(&draw(&with_weather("Tokyo"), 100, 30, now()));
        assert!(text.contains("Drizzle 21°C · rain 97% at 05h"), "{text}");
        assert!(
            text.contains("02h 21° 20%  03h 21° 64%  05h 20° 97%  06h 20° 0%"),
            "{text}"
        );
    }

    #[test]
    fn fits_the_weather_to_narrow_panes() {
        let text = screen(&draw(&with_weather("Tokyo"), 28, 18, now()));
        assert!(text.contains("21°C · rain 97% at 05h"), "{text}");
        assert!(!text.contains("Drizzle"), "{text}");
        assert!(text.contains("02h 21° 20%  03h 21° 64%"), "{text}");
        assert!(!text.contains("05h 20°"), "{text}");
    }

    #[test]
    fn the_weather_key_cycles_through_info_still_live_and_off() {
        use crate::weather_fx::Fx;
        let mut app = with_weather("Tokyo");
        app.show_map = false;
        // Info -> info with a still scene: the text stays.
        app.on_key(KeyEvent::from(KeyCode::Char('w')), now());
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(text.contains("Drizzle"), "{text}");
        assert_eq!((app.show_weather, app.weather_fx), (true, Fx::Still));
        // Still -> live scene on its own: the text goes away.
        app.on_key(KeyEvent::from(KeyCode::Char('w')), now());
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(!text.contains("Drizzle"), "{text}");
        assert_eq!((app.show_weather, app.weather_fx), (false, Fx::Live));
        assert!(
            text.contains(['╵', '╷']),
            "the live scene drizzles:\n{text}"
        );
        // Live -> off, then back to info.
        app.on_key(KeyEvent::from(KeyCode::Char('w')), now());
        assert_eq!((app.show_weather, app.weather_fx), (false, Fx::Off));
        app.on_key(KeyEvent::from(KeyCode::Char('w')), now());
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(text.contains("Drizzle"), "{text}");
    }

    /// What `render_clock` puts where: `d` digits, `t` a line, `s` the scene.
    fn laid_out(app: &App, area: Rect) -> Vec<(Rect, char)> {
        let local = app.zone().local_time(now());
        let full = local.format("%H:%M:%S").to_string();
        let short = local.format("%H:%M").to_string();
        let texts = [full.as_str(), short.as_str()];
        let lines = |width, weather| infos(app, now(), local, width, false, weather);
        let scene = app.fx_scene(now()).is_some();
        let plan = plan(&texts, &lines, area, scene, app.show_weather);
        let mut laid: Vec<(Rect, char)> = plan
            .rows
            .iter()
            .zip(plan.rects)
            .filter(|(_, rect)| !rect.is_empty())
            .map(|(row, rect)| {
                (
                    rect,
                    if matches!(row, Row::Digits(..)) {
                        'd'
                    } else {
                        't'
                    },
                )
            })
            .collect();
        laid.extend(plan.scene.map(|rect| (rect, 's')));
        laid
    }

    #[test]
    fn the_weather_scene_keeps_to_the_top_right_corner() {
        use crate::weather_fx::{self, Fx};
        let mut app = with_weather("Tokyo");
        app.weather_fx = Fx::Still;
        let mut shown = 0;
        for width in (0..=260).step_by(7) {
            for height in (0..=80).step_by(3) {
                let area = Rect::new(3, 2, width, height);
                let laid = laid_out(&app, area);
                for (i, (rect, kind)) in laid.iter().enumerate() {
                    assert!(
                        area.union(*rect) == area,
                        "{kind} {rect:?} outside {area:?}"
                    );
                    for (other, other_kind) in &laid[i + 1..] {
                        assert!(
                            !rect.intersects(*other),
                            "{kind} {rect:?} over {other_kind} {other:?} in {area:?}"
                        );
                    }
                }
                if let Some((scene, _)) = laid.iter().find(|(_, kind)| *kind == 's') {
                    shown += 1;
                    assert_eq!((scene.right(), scene.y), (area.right() - 1, area.y + 1));
                    assert!(
                        (weather_fx::MIN_ROWS..=weather_fx::MAX_ROWS).contains(&scene.height),
                        "{scene:?}"
                    );
                    assert_eq!(scene.width, weather_fx::width(scene.height));
                }
            }
        }
        assert!(
            shown > 300,
            "the scene shows wherever there is room: {shown}"
        );
    }

    #[test]
    fn roomy_panes_keep_the_clock_and_small_ones_share_it() {
        use crate::weather_fx::Fx;
        let mut app = with_weather("Tokyo");
        let find = |laid: &[(Rect, char)], wanted| {
            laid.iter()
                .find(|(_, kind)| *kind == wanted)
                .map(|(rect, _)| *rect)
        };
        // With room to spare the clock keeps its size, at most stepping down.
        for (width, height) in [(120, 40), (86, 26), (60, 20), (200, 50)] {
            let area = Rect::new(0, 0, width, height);
            app.weather_fx = Fx::Off;
            let alone = laid_out(&app, area);
            app.weather_fx = Fx::Still;
            let laid = laid_out(&app, area);
            let clock: Vec<_> = laid.iter().filter(|(_, kind)| *kind != 's').collect();
            assert_eq!(clock.len(), alone.len(), "{width}x{height}");
            let drop = clock[0].0.y - alone[0].0.y;
            for ((rect, _), (before, _)) in clock.iter().zip(&alone) {
                let (x, y, w, h) = (before.x, before.y + drop, before.width, before.height);
                assert_eq!(*rect, Rect::new(x, y, w, h), "{width}x{height}");
            }
            assert!(find(&laid, 's').is_some(), "{width}x{height}");
        }
        let laid = laid_out(&app, Rect::new(0, 0, 120, 40));
        assert_eq!(find(&laid, 's'), Some(Rect::new(101, 1, 18, 8)));
        // A small pane gives the scene a column at the right, and the clock
        // centers in the rest, with big digits where they fit.
        for (width, height) in [(86, 13), (43, 12), (30, 12)] {
            let laid = laid_out(&app, Rect::new(0, 0, width, height));
            let scene = find(&laid, 's').expect("a scene");
            assert_eq!((scene.right(), scene.y), (width - 1, 1));
            assert!(find(&laid, 'd').is_some(), "{width}x{height}");
            for (rect, kind) in &laid {
                assert!(
                    *kind == 's' || rect.right() + 2 <= scene.x,
                    "{kind} {rect:?}"
                );
            }
        }
        // A pane too narrow for big digits keeps the scene over its clock.
        let laid = laid_out(&app, Rect::new(0, 0, 15, 11));
        assert!(find(&laid, 's').is_some() && find(&laid, 'd').is_none());
    }

    #[test]
    fn lines_beside_the_scene_are_never_cut() {
        use crate::weather_fx::Fx;
        let mut app = with_weather("Tokyo");
        app.lang = Lang::Pt;
        app.show_map = false;
        app.weather_fx = Fx::Live;
        app.show_weather = false;
        app.calendar.address = Some("https://example.com/basic.ics".into());
        app.calendar.events = vec![crate::calendar::Event {
            uid: "1".into(),
            title: "Fórum - Otimização de Operação".into(),
            start: now() + Duration::minutes(56),
        }];
        let local = app.zone().local_time(now());
        let full = local.format("%H:%M:%S").to_string();
        let short = local.format("%H:%M").to_string();
        let texts = [full.as_str(), short.as_str()];
        let lines = |width, weather| infos(&app, now(), local, width, false, weather);
        for width in 24..=140 {
            for height in 6..=24 {
                let area = Rect::new(0, 0, width, height);
                let plan = plan(&texts, &lines, area, true, false);
                for (row, rect) in plan.rows.iter().zip(&plan.rects) {
                    if let (Row::Text(line), false) = (row, rect.is_empty()) {
                        assert!(
                            line.width() <= usize::from(rect.width),
                            "{width}x{height}: {line} in {rect:?}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_larger_pane_never_lacks_the_scene_a_smaller_one_shows() {
        use crate::weather_fx::{self, Fx};
        let mut app = with_weather("Tokyo");
        app.weather_fx = Fx::Live;
        app.show_weather = false;
        for width in 0..=90 {
            for height in 0..=30 {
                let area = Rect::new(0, 0, width, height);
                let shows = laid_out(&app, area).iter().any(|(_, kind)| *kind == 's');
                let room = width >= weather_fx::width(weather_fx::MIN_ROWS) + 2
                    && height >= weather_fx::MIN_ROWS + 2;
                assert_eq!(shows, room, "{width}x{height}");
            }
        }
    }

    #[test]
    fn without_room_the_live_scene_shows_as_text() {
        use crate::weather_fx::Fx;
        let mut app = with_weather("Tokyo");
        app.show_map = false;
        let as_text = screen(&draw(&app, 16, 5, now()));
        assert!(as_text.contains("21°C"), "{as_text}");
        app.cycle_weather();
        app.cycle_weather();
        assert_eq!((app.show_weather, app.weather_fx), (false, Fx::Live));
        assert_eq!(screen(&draw(&app, 16, 5, now())), as_text);
        let roomy = screen(&draw(&app, 100, 30, now()));
        assert!(!roomy.contains("21°C"), "the scene stands for it:\n{roomy}");
    }

    #[test]
    fn a_live_scene_on_screen_sets_the_pace() {
        use crate::weather_fx::Fx;
        let pace = |fx, width, height| {
            let mut app = with_weather("Tokyo");
            app.show_map = false;
            app.weather_fx = fx;
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            let mut animation = None;
            terminal
                .draw(|frame| animation = render(frame, &app, now()))
                .unwrap();
            animation
        };
        assert_eq!(pace(Fx::Live, 100, 30), Some(125), "drizzle");
        assert_eq!(pace(Fx::Still, 100, 30), None);
        assert_eq!(pace(Fx::Off, 100, 30), None);
        assert_eq!(pace(Fx::Live, 40, 5), None, "not on screen");
    }

    #[test]
    fn says_when_the_weather_could_not_be_downloaded() {
        let (net, jobs, answers) = crate::net::Net::manual();
        let mut app = with_city("Tokyo");
        app.net = Some(net);
        app.tick(now());
        let (key, _) = jobs.try_recv().unwrap();
        answers
            .send((key, crate::net::Answer::Text(Err("503".into()))))
            .unwrap();
        app.tick(now());
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(text.contains("Weather unavailable: 503"), "{text}");
        let narrow = screen(&draw(&app, 40, 16, now()));
        assert!(narrow.contains("Weather unavailable"), "{narrow}");
    }

    fn with_event(title: &str, start: DateTime<Utc>) -> App {
        let mut app = app();
        app.calendar.address =
            Some("https://calendar.google.com/calendar/ical/x/private-secret/basic.ics".into());
        app.calendar.events = vec![crate::calendar::Event {
            uid: "1".into(),
            title: title.into(),
            start,
        }];
        app
    }

    #[test]
    fn shows_the_next_event() {
        let app = with_event("Planning", now() + Duration::minutes(35));
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(
            text.contains("Next event 13:39 Planning · in 35m 00s"),
            "{text}"
        );
        let tomorrow = with_event("Retro", now() + Duration::days(1));
        let text = screen(&draw(&tomorrow, 100, 30, now()));
        assert!(text.contains("Next event Sat 13:04 Retro"), "{text}");
    }

    #[test]
    fn keeps_the_event_time_in_view_when_the_title_is_too_long() {
        let title =
            "[Training] Tax knowledge & the tax reform | Learn+ - Flows & Processes edition";
        let app = with_event(title, now() + Duration::minutes(35));
        let text = screen(&draw(&app, 60, 30, now()));
        assert!(text.contains("13:39 [Training] Tax knowledge"), "{text}");
        assert!(text.contains('…') && !text.contains("edition"), "{text}");
    }

    #[test]
    fn the_calendar_panel_hides_the_secret_address() {
        let mut app = with_event("Planning", now() + Duration::minutes(35));
        app.calendar.updated = Some(now());
        app.on_key(KeyEvent::from(KeyCode::Char('g')), now());
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(
            text.contains("Source: calendar.google.com/…/basic.ics"),
            "{text}"
        );
        assert!(!text.contains("private-secret"), "{text}");
        assert!(text.contains("Reminder: 5 min before"), "{text}");
        assert!(text.contains("Fri 13:39 Planning"), "{text}");
        assert!(text.contains("e edit address"), "{text}");
    }

    #[test]
    fn explains_where_to_find_the_address() {
        let mut app = app();
        app.on_key(KeyEvent::from(KeyCode::Char('g')), now());
        app.on_key(KeyEvent::from(KeyCode::Down), now());
        app.on_key(KeyEvent::from(KeyCode::Enter), now());
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(text.contains("Secret address in iCal format"), "{text}");
        assert!(text.contains("iCal address:"), "{text}");
    }

    #[test]
    fn offers_the_calendar_sources() {
        let mut app = app();
        app.on_key(KeyEvent::from(KeyCode::Char('g')), now());
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(text.contains("iCal address"), "{text}");
        if crate::maccal::SUPPORTED {
            assert!(text.contains("› Mac Calendar"), "{text}");
            assert!(text.contains("Google included"), "{text}");
        }
        assert!(text.contains("Enter connect"), "{text}");
    }

    #[test]
    fn says_when_the_mac_calendar_lost_its_access() {
        let mut app = app();
        app.calendar.mac = true;
        let text = screen(&draw(&app, 80, 24, now()));
        assert!(
            !text.contains("Calendar"),
            "nothing before macOS answers:\n{text}"
        );
        app.calendar.access = Some(Access::NotAsked);
        app.calendar.error = Some("no access to the calendars".into());
        let text = screen(&draw(&app, 80, 24, now()));
        assert!(text.contains("Calendar without access: press g"), "{text}");
    }

    #[test]
    fn explains_a_denied_mac_calendar() {
        let mut app = app();
        app.calendar.mac = true;
        app.calendar.access = Some(Access::Denied);
        app.on_key(KeyEvent::from(KeyCode::Char('g')), now());
        let text = screen(&draw(&app, 110, 30, now()));
        assert!(text.contains("Source: Mac Calendar"), "{text}");
        assert!(
            text.contains("macOS denied access to the calendars"),
            "{text}"
        );
        assert!(text.contains("o privacy"), "{text}");
    }

    #[test]
    fn suggests_adding_the_account_when_the_mac_calendar_is_empty() {
        let mut app = app();
        app.calendar.mac = true;
        app.calendar.access = Some(Access::Granted);
        app.calendar.updated = Some(now());
        app.on_key(KeyEvent::from(KeyCode::Char('g')), now());
        let text = screen(&draw(&app, 110, 30, now()));
        assert!(text.contains("No events in the next 7 days"), "{text}");
        assert!(
            text.contains("Is your Google account in the Calendar app?"),
            "{text}"
        );
    }

    #[test]
    fn shows_the_weather_of_your_city_in_the_local_tab() {
        let mut app = app();
        let home = cities::db().search("Belo Horizonte", 1)[0];
        app.home = Some(home);
        let forecast = weather::parse(weather::tests::SAMPLE).unwrap();
        app.weather.insert(
            weather::key(&home),
            crate::app::WeatherSlot::ready(forecast),
        );
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(text.contains("Belo Horizonte: Drizzle 21°C"), "{text}");
    }

    #[test]
    fn says_the_forecast_is_loading() {
        let (net, _jobs, _answers) = crate::net::Net::manual();
        let mut app = with_city("Tokyo");
        app.net = Some(net);
        app.tick(now());
        let text = screen(&draw(&app, 100, 30, now()));
        assert!(text.contains("Loading the forecast…"), "{text}");
    }

    #[test]
    fn shows_why_macos_did_not_ask() {
        let mut app = app();
        app.calendar.mac = true;
        app.calendar.access = Some(Access::NotAsked);
        app.calendar.error = Some("The operation couldn't be completed.".into());
        app.on_key(KeyEvent::from(KeyCode::Char('g')), now());
        let text = screen(&draw(&app, 120, 30, now()));
        assert!(
            text.contains("macOS did not show the request: The operation couldn't be completed."),
            "{text}"
        );
        assert!(text.contains("Enter asks macOS"), "{text}");
    }
}
