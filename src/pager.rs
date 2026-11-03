//! The pager: `meridian page "Lunch is ready"`, from any terminal or script,
//! puts a message in the state file; the meridian running picks it up
//! within a second, beeps, and scrolls it across a band at the bottom of the
//! screen, like a pager, until it is dismissed. An emoticon on its own, such
//! as :) or 👍, passes by as ASCII art instead, as large as the pane allows.
//!
//! Where the message is at any moment follows from when it was sent and the
//! time, as the weather scene's frames do: nothing is kept between frames,
//! and copies running side by side scroll in step.

use chrono::{DateTime, Utc};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

use crate::i18n::Lang;
use crate::theme::Palette;
use crate::zone::Zone;

/// A message sent with `meridian page`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub text: String,
    pub sent: DateTime<Utc>,
    /// When it goes away by itself; absent, it stays until dismissed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub until: Option<DateTime<Utc>>,
}

/// The longest message, in characters: a pager's worth.
pub const MAX_CHARS: usize = 500;

/// The band moves one column every this many milliseconds.
pub const STEP_MS: u64 = 100;

/// How long the band blinks as a page comes in, and how fast.
const BEEP_MS: i64 = 2_000;
const BLINK_MS: i64 = 250;

/// Rows and columns of the large drawings and of the small ones.
const LARGE: u16 = 9;
const LARGE_WIDTH: u16 = 19;
const SMALL: u16 = 5;
const SMALL_WIDTH: u16 = 13;

/// Columns between the drawings of one message.
const APART: u16 = 3;

/// The fewest columns between one pass of a line and the next.
const LINE_GAP: u16 = 8;

/// The message on one line: each run of spaces, line breaks and other
/// control characters becomes one space, and a message longer than
/// `MAX_CHARS` characters ends in "…" there.
pub fn clean(text: &str) -> String {
    let line = text
        .split(|ch: char| ch.is_whitespace() || ch.is_control())
        .filter(|word| !word.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if line.chars().count() <= MAX_CHARS {
        return line;
    }
    let kept: String = line.chars().take(MAX_CHARS - 1).collect();
    format!("{}…", kept.trim_end())
}

/// Where the emoticons get their color: faces and hands are yellow like
/// their emoji, hearts take the alarm's color and people the digits'.
#[derive(Clone, Copy)]
enum Tint {
    Sun,
    Marker,
    Digits,
}

impl Tint {
    fn color(self, palette: &Palette) -> Color {
        match self {
            Self::Sun => palette.sun,
            Self::Marker => palette.marker,
            Self::Digits => palette.digits,
        }
    }
}

/// An emoticon: the ways to type it, the emoji that mean it, how it reads
/// in one line of ASCII, and its drawings, large and small.
pub struct Emoticon {
    typed: &'static [&'static str],
    emoji: &'static [char],
    /// What shows where no drawing fits.
    pub text: &'static str,
    large: [&'static str; LARGE as usize],
    small: [&'static str; SMALL as usize],
    tint: Tint,
}

static EMOTICONS: [Emoticon; 20] = [
    Emoticon {
        typed: &[":)", ":-)", "=)", ":]", ":-]", "(:", "(-:"],
        emoji: &['🙂', '☺'],
        text: ":)",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   _       _   \ ",
            r"|   (o)     (o)   |",
            r"|                 |",
            r"|   \         /   |",
            r" \   '._____.'   / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  o   o  \ ",
            r"|           |",
            r" \  \___/  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &["^_^", "^^", "^.^", "^-^", "(^_^)"],
        emoji: &['😊', '😌'],
        text: "^_^",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /               \ ",
            r"|   .-.     .-.   |",
            r"|                 |",
            r"|   \         /   |",
            r" \   '._____.'   / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  ^   ^  \ ",
            r"|           |",
            r" \  \___/  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[":D", ":-D", "=D"],
        emoji: &['😀', '😃', '😁', '😄'],
        text: ":D",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   _       _   \ ",
            r"|   (o)     (o)   |",
            r"|                 |",
            r"|   \_________/   |",
            r" \   \       /   / ",
            r"  '.  '-----'  .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  o   o  \ ",
            r"|  \_____/  |",
            r" \  \___/  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &["xD", "XD", "x-D", "X-D", ":'D"],
        emoji: &['😆', '😂', '🤣'],
        text: "xD",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   '.     .'   \ ",
            r"|    .'     '.    |",
            r"|                 |",
            r"|   \_________/   |",
            r" \   \       /   / ",
            r"  '.  '-----'  .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  >   <  \ ",
            r"|  \_____/  |",
            r" \  \___/  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[";)", ";-)", ";]", ";D", ";-D"],
        emoji: &['😉'],
        text: ";)",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   _           \ ",
            r"|   (o)     ---   |",
            r"|                 |",
            r"|   \         /   |",
            r" \   '._____.'   / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  o   -  \ ",
            r"|           |",
            r" \  \___/  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[":(", ":-(", "=(", ":[", ":-[", "):", ")-:"],
        emoji: &['🙁', '☹', '😞', '😟', '😔'],
        text: ":(",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   _       _   \ ",
            r"|   (o)     (o)   |",
            r"|                 |",
            r"|     .-----.     |",
            r" \   /       \   / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  o   o  \ ",
            r"|    ___    |",
            r" \  /   \  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[":'(", ":'-(", ";(", ";-(", ";_;", "T_T", "T.T"],
        emoji: &['😢', '😭'],
        text: ":'(",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /  ___     ___  \ ",
            r"|    |       |    |",
            r"|    |       |    |",
            r"|     .-----.     |",
            r" \   /       \   / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  T   T  \ ",
            r"|    ___    |",
            r" \  /   \  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[
            ":O", ":o", ":-O", ":-o", "=O", "O_O", "o_O", "O_o", "o.O", "O.o",
        ],
        emoji: &['😮', '😯', '😲', '😱', '😳'],
        text: ":O",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   _       _   \ ",
            r"|   (O)     (O)   |",
            r"|                 |",
            r"|       .-.       |",
            r" \     (   )     / ",
            r"  '.    '-'    .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  O   O  \ ",
            r"|    .-.    |",
            r" \   '-'   / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[":P", ":p", ":-P", ":-p", "=P", "=p", ";P", ";p", "xP", "XP"],
        emoji: &['😛', '😜', '😝', '😋'],
        text: ":P",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   _       _   \ ",
            r"|   (o)     (o)   |",
            r"|                 |",
            r"|   \_________/   |",
            r" \        | |    / ",
            r"  '.      '-'  .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  o   o  \ ",
            r"|   \___/   |",
            r" \    |_|  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &["B)", "B-)", "8)", "8-)"],
        emoji: &['😎'],
        text: "B)",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /  _____ _____  \ ",
            r"|---\___/-\___/---|",
            r"|                 |",
            r"|   \         /   |",
            r" \   '._____.'   / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" / [#]-[#] \ ",
            r"|           |",
            r" \  \___/  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[">:(", ">:-(", ">:[", ">_<"],
        emoji: &['😠', '😡', '🤬', '😤'],
        text: ">:(",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /  \__     __/  \ ",
            r"|   (o)     (o)   |",
            r"|                 |",
            r"|     .-----.     |",
            r" \   /       \   / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" / \o   o/ \ ",
            r"|    ___    |",
            r" \  /   \  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[":*", ":-*", "=*", ";*", ";-*"],
        emoji: &['😘', '😗', '😙', '😚', '💋'],
        text: ":*",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   _           \ ",
            r"|   (o)     ---   |",
            r"|                 |",
            r"|       .-.  <3   |",
            r" \      '-'      / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  o   -  \ ",
            r"|     o  <3 |",
            r" \         / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[":|", ":-|", "=|", "-_-", "-.-"],
        emoji: &['😐', '😑', '😶'],
        text: ":|",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   _       _   \ ",
            r"|   (o)     (o)   |",
            r"|                 |",
            r"|                 |",
            r" \    -------    / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  o   o  \ ",
            r"|           |",
            r" \  -----  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[":/", ":-/", r":\", r":-\", "=/", r"=\"],
        emoji: &['😕', '🫤', '🤨', '🤔'],
        text: ":/",
        large: [
            r"    .-'''''''-.    ",
            r"  .'           '.  ",
            r" /   _       _   \ ",
            r"|   (o)     (o)   |",
            r"|                 |",
            r"|         _.-'    |",
            r" \    _.-'       / ",
            r"  '.           .'  ",
            r"    '-._____.-'    ",
        ],
        small: [
            r"  .-'''''-.  ",
            r" /  o   o  \ ",
            r"|           |",
            r" \   _.-'  / ",
            r"  '-.___.-'  ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &["<3", "<33"],
        emoji: &[
            '❤', '♥', '❣', '💙', '💚', '💛', '💜', '🧡', '🖤', '🤍', '🤎', '🩷', '🩵', '🩶', '💗',
            '💖', '💓', '💕', '💘', '💝', '💞', '😍', '🥰',
        ],
        text: "<3",
        large: [
            r"  .-'''-. .-'''-.  ",
            r" /       V       \ ",
            r"|                 |",
            r" \               / ",
            r"  \             /  ",
            r"   '.         .'   ",
            r"     '.     .'     ",
            r"       '. .'       ",
            r"         V         ",
        ],
        small: [
            r" .-'-. .-'-. ",
            r"(     V     )",
            r" \         / ",
            r"  '.     .'  ",
            r"    '. .'    ",
        ],
        tint: Tint::Marker,
    },
    Emoticon {
        typed: &["</3", r"<\3"],
        emoji: &['💔'],
        text: "</3",
        large: [
            r"  .-'''-. .-'''-.  ",
            r" /       V       \ ",
            r"|        \        |",
            r" \       /       / ",
            r"  \      \      /  ",
            r"   '.    /    .'   ",
            r"     '.  \  .'     ",
            r"       './.'       ",
            r"         V         ",
        ],
        small: [
            r" .-'-. .-'-. ",
            r"(     V     )",
            r" \    /    / ",
            r"  '.  \  .'  ",
            r"    './.'    ",
        ],
        tint: Tint::Marker,
    },
    Emoticon {
        typed: &["(y)", "(Y)", "+1"],
        emoji: &['👍'],
        text: "(y)",
        large: [
            r"      .--.         ",
            r"      |  |         ",
            r"      |  |         ",
            r"  .---'  '------.  ",
            r" (____          |  ",
            r" (____          |  ",
            r" (____          |  ",
            r" (____          |  ",
            r"  '-------------'  ",
        ],
        small: [
            r"    .-.      ",
            r"  .-' '----. ",
            r" (___      | ",
            r" (___      | ",
            r"  '--------' ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &["(n)", "(N)", "-1"],
        emoji: &['👎'],
        text: "(n)",
        large: [
            r"  .-------------.  ",
            r" (____          |  ",
            r" (____          |  ",
            r" (____          |  ",
            r" (____          |  ",
            r"  '---.  .------'  ",
            r"      |  |         ",
            r"      |  |         ",
            r"      '--'         ",
        ],
        small: [
            r"  .--------. ",
            r" (___      | ",
            r" (___      | ",
            r"  '-. .----' ",
            r"    '-'      ",
        ],
        tint: Tint::Sun,
    },
    Emoticon {
        typed: &[r"\o/", r"\O/", r"\0/"],
        emoji: &['🎉', '🎊', '🙌', '🥳'],
        text: r"\o/",
        large: [
            r" *   \       /   . ",
            r"      \  _  /      ",
            r"   '   \(_)/   *   ",
            r"        \|/        ",
            r" .       |       ' ",
            r"         |         ",
            r"   *    / \    .   ",
            r"       /   \       ",
            r" '   _/     \_   * ",
        ],
        small: [
            r" * \  _  / . ",
            r"    \(_)/    ",
            r"  .  \|/  *  ",
            r"      |      ",
            r" '   / \   * ",
        ],
        tint: Tint::Digits,
    },
    Emoticon {
        typed: &["o/", r"\o", "O/", r"\O"],
        emoji: &['👋'],
        text: "o/",
        large: [
            r"             /  )  ",
            r"         _  /  )   ",
            r"        (_)/       ",
            r"        /|/        ",
            r"       / |         ",
            r"         |         ",
            r"        / \        ",
            r"       /   \       ",
            r"     _/     \_     ",
        ],
        small: [
            r"      _   / )",
            r"     (_) /   ",
            r"     /|-'    ",
            r"      |      ",
            r"     / \     ",
        ],
        tint: Tint::Digits,
    },
];

/// The emoticons a message is made of, when it is nothing else: one or
/// more, typed (":)", "<3") or as emoji ("🙂", "👍🏽"), apart or together.
pub fn emoticons(message: &str) -> Option<Vec<&'static Emoticon>> {
    let mut found = Vec::new();
    for word in message.split_whitespace() {
        if let Some(emoticon) = EMOTICONS.iter().find(|e| e.typed.contains(&word)) {
            found.push(emoticon);
            continue;
        }
        let mut chars = word.chars().peekable();
        while let Some(ch) = chars.next() {
            found.push(EMOTICONS.iter().find(|e| e.emoji.contains(&ch))?);
            // What follows an emoji belongs to it: a variation selector, a
            // skin tone, or a joiner and the character it joins.
            while let Some(&next) = chars.peek() {
                match next {
                    '\u{FE0E}' | '\u{FE0F}' | '\u{1F3FB}'..='\u{1F3FF}' => {
                        chars.next();
                    }
                    '\u{200D}' => {
                        chars.next();
                        chars.next();
                    }
                    _ => break,
                }
            }
        }
    }
    (!found.is_empty()).then_some(found)
}

/// The rows the page's band can take, tallest first, for the layout to pick
/// the first that fits: a drawing with a blank row above it or without,
/// large or small, else one line.
pub fn heights(page: &Page) -> &'static [u16] {
    if emoticons(&clean(&page.text)).is_some() {
        &[LARGE + 1, LARGE, SMALL + 1, SMALL, 1]
    } else {
        &[1]
    }
}

/// When the page came, in the computer's time: "14:02", or "Mon 14:02" for
/// one from another day.
pub fn label(page: &Page, zone: Zone, lang: Lang, now: DateTime<Utc>) -> String {
    let sent = zone.local_time(page.sent);
    let time = sent.format("%H:%M");
    if sent.date_naive() == zone.local_time(now).date_naive() {
        time.to_string()
    } else {
        format!("{} {time}", lang.short_weekday(sent.date_naive()))
    }
}

/// A character of what scrolls by, `x` columns from its start.
struct Glyph<'a> {
    x: u16,
    symbol: &'a str,
    width: u16,
    style: Style,
}

/// What scrolls by: rows of glyphs, `width` columns wide.
struct Strip<'a> {
    rows: Vec<Vec<Glyph<'a>>>,
    width: u16,
}

/// Draws the page in its band, `area`, where it has scrolled to at `now`:
/// the drawings of its emoticons when it is only that and they fit, else
/// one line, after `label`. The band blinks as the page comes in. Returns
/// how often it moves, in milliseconds.
pub fn draw(
    page: &Page,
    label: &str,
    area: Rect,
    palette: &Palette,
    now: DateTime<Utc>,
    buf: &mut Buffer,
) -> Option<u64> {
    if area.is_empty() {
        return None;
    }
    let moved = moved(page, now);
    let message = clean(&page.text);
    let found = emoticons(&message);
    if let Some(found) = &found
        && area.height >= SMALL
    {
        let large = area.height >= LARGE;
        let strip = picture(found, large, palette);
        let each = if large { LARGE_WIDTH } else { SMALL_WIDTH };
        roll(&strip, area, moved, (area.width / 3).max(each), buf);
    } else {
        let (text, style) = match &found {
            // Where no drawing fits, the emoticons still show in ASCII.
            Some(found) => (
                found.iter().map(|e| e.text).collect::<Vec<_>>().join(" "),
                Style::new().fg(found[0].tint.color(palette)).bold(),
            ),
            None => (message, Style::new().bold()),
        };
        let spans = [
            Span::styled(label, palette.muted()),
            Span::raw(" "),
            Span::styled(text.as_str(), style),
        ];
        roll(
            &line(&spans),
            area,
            moved,
            (area.width / 3).max(LINE_GAP),
            buf,
        );
    }
    if lit(page, now) {
        buf.set_style(area, palette.flash());
    }
    Some(STEP_MS)
}

/// Columns the page has moved since it was sent: one every `STEP_MS`, on
/// the clock's beat, so each wake-up of the screen finds it one further.
fn moved(page: &Page, now: DateTime<Utc>) -> i64 {
    let step = STEP_MS as i64;
    let steps = |at: DateTime<Utc>| at.timestamp_millis().div_euclid(step);
    (steps(now) - steps(page.sent)).max(0)
}

/// Whether the band is lit: it blinks for a moment as the page comes in,
/// like a pager's screen.
fn lit(page: &Page, now: DateTime<Utc>) -> bool {
    let ms = (now - page.sent).num_milliseconds();
    (0..BEEP_MS).contains(&ms) && ms % (2 * BLINK_MS) < BLINK_MS
}

/// One line of text, whatever its characters' widths.
fn line<'a>(spans: &'a [Span<'a>]) -> Strip<'a> {
    let mut glyphs = Vec::new();
    let mut x: u16 = 0;
    let graphemes = spans
        .iter()
        .flat_map(|span| span.styled_graphemes(Style::new()));
    for grapheme in graphemes {
        let width = grapheme.symbol.width() as u16;
        if width == 0 {
            continue;
        }
        glyphs.push(Glyph {
            x,
            symbol: grapheme.symbol,
            width,
            style: grapheme.style,
        });
        x = x.saturating_add(width);
    }
    Strip {
        rows: vec![glyphs],
        width: x,
    }
}

/// The drawings of `found` side by side, large or small.
fn picture(found: &[&'static Emoticon], large: bool, palette: &Palette) -> Strip<'static> {
    let (height, width) = match large {
        true => (LARGE, LARGE_WIDTH),
        false => (SMALL, SMALL_WIDTH),
    };
    let mut rows: Vec<Vec<Glyph>> = (0..height).map(|_| Vec::new()).collect();
    for (index, emoticon) in found.iter().enumerate() {
        let left = (index as u16).saturating_mul(width + APART);
        let style = Style::new().fg(emoticon.tint.color(palette));
        let drawing: &[&'static str] = match large {
            true => &emoticon.large,
            false => &emoticon.small,
        };
        for (row, &text) in rows.iter_mut().zip(drawing) {
            for (x, byte) in text.bytes().enumerate() {
                if byte != b' ' {
                    row.push(Glyph {
                        x: left.saturating_add(x as u16),
                        symbol: &text[x..=x],
                        width: 1,
                        style,
                    });
                }
            }
        }
    }
    let count = found.len() as u16;
    Strip {
        rows,
        width: count.saturating_mul(width + APART).saturating_sub(APART),
    }
}

/// Draws copies of `strip` one after another, `gap` columns apart, at the
/// bottom of `area`: the first one entered at its right edge `moved`
/// columns ago, and the rest follow.
fn roll(strip: &Strip, area: Rect, moved: i64, gap: u16, buf: &mut Buffer) {
    let period = i64::from(strip.width) + i64::from(gap.max(1));
    let (left, right) = (i64::from(area.left()), i64::from(area.right()));
    // Copy `n` starts at `right - moved + n * period`: draw the ones whose
    // columns reach into the area.
    let first = ((left - right + moved - i64::from(strip.width)).div_euclid(period) + 1).max(0);
    let last = (moved - 1).div_euclid(period);
    let top = area
        .bottom()
        .saturating_sub(strip.rows.len() as u16)
        .max(area.top());
    for copy in first..=last {
        let start = right - moved + copy * period;
        for (y, glyphs) in (top..area.bottom()).zip(&strip.rows) {
            for glyph in glyphs {
                let x = start + i64::from(glyph.x);
                if x >= left && x + i64::from(glyph.width) <= right {
                    let width = usize::from(glyph.width);
                    buf.set_stringn(x as u16, y, glyph.symbol, width, glyph.style);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Paint, THEMES};
    use chrono::{Duration, NaiveDate};
    use std::collections::HashSet;

    fn sent() -> DateTime<Utc> {
        NaiveDate::from_ymd_opt(2026, 9, 25)
            .unwrap()
            .and_hms_opt(16, 4, 5)
            .unwrap()
            .and_utc()
    }

    fn after(ms: i64) -> DateTime<Utc> {
        sent() + Duration::milliseconds(ms)
    }

    fn page(text: &str) -> Page {
        Page {
            text: text.into(),
            sent: sent(),
            until: None,
        }
    }

    fn palette() -> Palette {
        THEMES[0].palette(true)
    }

    fn band(page: &Page, width: u16, height: u16, at: DateTime<Utc>) -> Buffer {
        let area = Rect::new(0, 0, width, height);
        let mut buf = Buffer::empty(area);
        assert_eq!(
            draw(page, "16:04", area, &palette(), at, &mut buf),
            Some(STEP_MS)
        );
        buf
    }

    fn rows(buf: &Buffer) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect()
    }

    fn texts(message: &str) -> Option<Vec<&'static str>> {
        emoticons(message).map(|found| found.iter().map(|emoticon| emoticon.text).collect())
    }

    #[test]
    fn puts_the_message_on_one_line() {
        assert_eq!(
            clean("  Deploy\n\tdone\u{1b}[31m  ok \r\n"),
            "Deploy done [31m ok"
        );
        assert_eq!(clean(" \n "), "");
        let cut = clean(&"a".repeat(MAX_CHARS + 10));
        assert_eq!(cut.chars().count(), MAX_CHARS);
        assert!(cut.ends_with("a…"));
        let words = clean(&"word ".repeat(MAX_CHARS));
        assert!(
            words.ends_with("word…") || words.ends_with("wor…"),
            "{words}"
        );
    }

    #[test]
    fn tells_an_emoticon_alone_from_a_message() {
        assert_eq!(texts(":)"), Some(vec![":)"]));
        assert_eq!(texts("  :-)  "), Some(vec![":)"]));
        assert_eq!(texts("🙂"), Some(vec![":)"]));
        assert_eq!(texts("-_-"), Some(vec![":|"]));
        assert_eq!(texts("👍🏽"), Some(vec!["(y)"]), "with a skin tone");
        assert_eq!(texts("❤️"), Some(vec!["<3"]), "with a variation selector");
        assert_eq!(texts("❤️‍🔥"), Some(vec!["<3"]), "joined to another emoji");
        assert_eq!(texts("😂😂"), Some(vec!["xD", "xD"]));
        assert_eq!(texts(":D <3 👍"), Some(vec![":D", "<3", "(y)"]));
        for message in ["Bom dia :)", ":)!", "🚀", "👍 done", ":", "", "  "] {
            assert_eq!(texts(message), None, "{message:?}");
        }
    }

    #[test]
    fn every_emoticon_is_its_own_and_drawn_in_ascii() {
        let mut typed = HashSet::new();
        let mut emoji = HashSet::new();
        for emoticon in &EMOTICONS {
            assert_eq!(texts(emoticon.text), Some(vec![emoticon.text]));
            for form in emoticon.typed {
                assert!(typed.insert(form), "{form} typed twice");
                assert_eq!(texts(form), Some(vec![emoticon.text]), "{form}");
            }
            for ch in emoticon.emoji {
                assert!(emoji.insert(ch), "{ch} twice");
                assert_eq!(texts(&ch.to_string()), Some(vec![emoticon.text]), "{ch}");
            }
            let drawings = [
                (&emoticon.large[..], LARGE_WIDTH),
                (&emoticon.small[..], SMALL_WIDTH),
            ];
            for (drawing, width) in drawings {
                for row in drawing {
                    assert!(row.bytes().all(|byte| (b' '..=b'~').contains(&byte)));
                    assert_eq!(row.len(), usize::from(width), "{}: {row:?}", emoticon.text);
                }
                assert!(
                    !drawing[0].trim().is_empty() && !drawing[drawing.len() - 1].trim().is_empty(),
                    "{}: no blank rows at the edges",
                    emoticon.text
                );
            }
        }
    }

    #[test]
    fn an_emoticon_asks_for_room_for_its_drawing() {
        assert_eq!(heights(&page("Lunch is ready")), [1]);
        assert_eq!(heights(&page(":)")), [10, 9, 6, 5, 1]);
        assert_eq!(heights(&page("👍 🎉")), [10, 9, 6, 5, 1]);
    }

    #[test]
    fn enters_from_the_right_and_scrolls_left() {
        let hi = page("Hi");
        assert_eq!(rows(&band(&hi, 20, 1, sent())), [" ".repeat(20)]);
        assert_eq!(
            rows(&band(&hi, 20, 1, after(399))),
            ["                 16:"]
        );
        assert_eq!(
            rows(&band(&hi, 20, 1, after(1_250))),
            ["        16:04 Hi    "]
        );
        // The next pass follows a third of the band behind, eight columns at
        // the least.
        assert_eq!(
            rows(&band(&hi, 20, 1, after(2_000))),
            ["16:04 Hi        16:0"]
        );
        assert_eq!(
            rows(&band(&hi, 20, 1, after(2_800))),
            ["        16:04 Hi    "]
        );
    }

    #[test]
    fn keeps_scrolling_however_long_it_stays() {
        let page = page("Deploy finished");
        let period = 15 + 6 + (60 / 3);
        let day = 86_400_000;
        let early = rows(&band(&page, 60, 1, after(day)));
        assert_eq!(early, rows(&band(&page, 60, 1, after(day + period * 100))));
        assert!(early[0].contains("finished"), "{early:?}");
        assert_ne!(early, rows(&band(&page, 60, 1, after(day + 100))));
    }

    #[test]
    fn blinks_as_it_comes_in() {
        let page = page("Hi");
        let flash = palette().flash().bg;
        let lit = |ms| band(&page, 10, 1, after(ms))[(0, 0)].bg;
        assert_eq!(lit(100), flash.unwrap());
        assert_ne!(lit(300), flash.unwrap());
        assert_eq!(lit(1_600), flash.unwrap());
        assert_ne!(lit(2_100), flash.unwrap());
        assert_ne!(lit(-100), flash.unwrap(), "not before it is sent");
    }

    #[test]
    fn an_emoticon_passes_by_in_ascii_art() {
        let smile = page(":)");
        let large = rows(&band(&smile, 40, 10, after(3_000)));
        assert!(large[0].trim().is_empty(), "a blank row above it");
        assert!(large[4].contains("|   (o)     (o)   |"), "{large:#?}");
        assert!(large[7].contains(r"\   '._____.'   /"), "{large:#?}");

        let small = rows(&band(&smile, 40, 5, after(3_000)));
        assert!(small[1].contains(r"/  o   o  \"), "{small:#?}");
        assert!(small.iter().all(|row| row.is_ascii()));

        let thumbs = rows(&band(&page("👍"), 40, 1, after(3_000)));
        assert!(
            thumbs[0].contains("16:04 (y)"),
            "too short for a drawing: {thumbs:?}"
        );
    }

    #[test]
    fn several_emoticons_pass_by_side_by_side() {
        let found = emoticons("👍 <3").unwrap();
        let strip = picture(&found, false, &palette());
        assert_eq!(strip.width, 2 * SMALL_WIDTH + APART);
        let area = Rect::new(0, 0, strip.width, SMALL);
        let mut buf = Buffer::empty(area);
        roll(&strip, area, i64::from(strip.width), 50, &mut buf);
        assert_eq!(rows(&buf)[1], r"  .-' '----.    (     V     )");
    }

    #[test]
    fn says_when_the_page_came() {
        let zone = Zone::City(chrono_tz::UTC);
        let page = page("Hi");
        assert_eq!(label(&page, zone, Lang::En, after(60_000)), "16:04");
        let tomorrow = after(86_400_000);
        assert_eq!(label(&page, zone, Lang::En, tomorrow), "Fri 16:04");
        assert_eq!(label(&page, zone, Lang::Pt, tomorrow), "sex 16:04");
    }

    #[test]
    fn never_panics_at_any_size_or_time() {
        let (long, smiles) = ("long ".repeat(200), "🙂".repeat(300));
        let messages = [
            "",
            "Hi",
            ":)",
            "👍 🎉 💔 \\o/",
            "Reunião às 15h na sala 3 · 会議室 🚀",
            long.as_str(),
            smiles.as_str(),
        ];
        for message in messages {
            let page = page(message);
            for (width, height) in [(1, 1), (2, 1), (3, 5), (7, 9), (19, 10), (80, 6), (250, 30)] {
                for ms in [-5_000, 0, 1, 99, 100, 777, 60_000, 86_400_000] {
                    band(&page, width, height, after(ms));
                }
            }
            let mut buf = Buffer::empty(Rect::new(0, 0, 10, 3));
            let empty = Rect::new(4, 1, 0, 0);
            assert_eq!(
                draw(&page, "16:04", empty, &palette(), sent(), &mut buf),
                None
            );
        }
    }
}
