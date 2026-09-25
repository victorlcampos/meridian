//! Big digits drawn with half blocks, scaled to the space available.
//!
//! Each glyph is a 3×5 pixel bitmap (the colon is 1×5). At scale `s` a pixel
//! takes `s` columns and `s` half rows, so pixels stay roughly square.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

const GLYPH_HEIGHT: u16 = 5;

const FONT: [(char, [&str; 5]); 11] = [
    ('0', ["###", "#.#", "#.#", "#.#", "###"]),
    ('1', [".#.", "##.", ".#.", ".#.", "###"]),
    ('2', ["###", "..#", "###", "#..", "###"]),
    ('3', ["###", "..#", "###", "..#", "###"]),
    ('4', ["#.#", "#.#", "###", "..#", "..#"]),
    ('5', ["###", "#..", "###", "..#", "###"]),
    ('6', ["###", "#..", "###", "#.#", "###"]),
    ('7', ["###", "..#", "..#", "..#", "..#"]),
    ('8', ["###", "#.#", "###", "#.#", "###"]),
    ('9', ["###", "#.#", "###", "..#", "###"]),
    (':', [".", "#", ".", "#", "."]),
];

fn glyph(ch: char) -> Option<&'static [&'static str; 5]> {
    FONT.iter().find(|(c, _)| *c == ch).map(|(_, rows)| rows)
}

/// Pixel columns of `text`, glyphs separated by one blank column.
fn columns(text: &str) -> Vec<[bool; GLYPH_HEIGHT as usize]> {
    let mut columns = Vec::new();
    for (i, rows) in text.chars().filter_map(glyph).enumerate() {
        if i > 0 {
            columns.push([false; GLYPH_HEIGHT as usize]);
        }
        for x in 0..rows[0].len() {
            columns.push(std::array::from_fn(|y| rows[y].as_bytes()[x] == b'#'));
        }
    }
    columns
}

/// Size in cells of `text` drawn at `scale`.
pub fn size(text: &str, scale: u16) -> (u16, u16) {
    let width = columns(text).len() as u16;
    (width * scale, (GLYPH_HEIGHT * scale).div_ceil(2))
}

/// Largest scale at which `text` fits in `width` × `height` cells; 0 when it does not fit at all.
pub fn fit_scale(text: &str, width: u16, height: u16) -> u16 {
    let pixels = columns(text).len() as u16;
    if pixels == 0 {
        return 0;
    }
    (width / pixels).min(height.saturating_mul(2) / GLYPH_HEIGHT)
}

/// Draws `text` at `scale` with its top-left corner at `at`, clipped to the buffer.
pub fn draw(text: &str, scale: u16, at: (u16, u16), style: Style, buf: &mut Buffer) {
    if scale == 0 {
        return;
    }
    let columns = columns(text);
    let (width, height) = size(text, scale);
    let pixel = |x: u16, half_row: u16| {
        let y = usize::from(half_row / scale);
        y < GLYPH_HEIGHT as usize && columns[usize::from(x / scale)][y]
    };
    let area = Rect::new(at.0, at.1, width, height).intersection(buf.area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let (dx, dy) = (x - at.0, y - at.1);
            let symbol = match (pixel(dx, 2 * dy), pixel(dx, 2 * dy + 1)) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => continue,
            };
            buf[(x, y)].set_char(symbol).set_style(style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(buf: &Buffer) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| {
                (0..buf.area.width)
                    .map(|x| buf[(x, y)].symbol().to_owned())
                    .collect()
            })
            .collect()
    }

    #[test]
    fn measures_text() {
        assert_eq!(size("12:34:56", 1), (27, 3));
        assert_eq!(size("12:34", 2), (34, 5));
        assert_eq!(size("12:34:56", 4), (108, 10));
    }

    #[test]
    fn picks_the_largest_scale_that_fits() {
        assert_eq!(fit_scale("12:34:56", 26, 50), 0);
        assert_eq!(fit_scale("12:34:56", 27, 3), 1);
        assert_eq!(fit_scale("12:34:56", 200, 12), 4);
        assert_eq!(fit_scale("12:34:56", 90, 100), 3);
        assert_eq!(fit_scale("12:34", 17, 3), 1);
        assert_eq!(fit_scale("", 100, 100), 0);
    }

    #[test]
    fn draws_half_block_pixels() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 7, 3));
        draw("1:", 1, (0, 0), Style::new(), &mut buf);
        assert_eq!(lines(&buf), ["▄█  ▄  ", " █  ▄  ", "▀▀▀    "]);
    }

    #[test]
    fn scales_up() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 6, 5));
        draw("7", 2, (0, 0), Style::new(), &mut buf);
        assert_eq!(
            lines(&buf),
            ["██████", "    ██", "    ██", "    ██", "    ██"]
        );
    }

    #[test]
    fn clips_to_the_buffer() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 5, 2));
        draw("88:88", 3, (2, 1), Style::new(), &mut buf);
        assert_eq!(lines(&buf)[0], "     ");
        assert_eq!(lines(&buf)[1], "  ███");
    }
}
