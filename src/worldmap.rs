//! World map drawn with text: a land mask sampled to any size, day/night
//! shading and a marker for the selected city.

use std::sync::OnceLock;

use clap::ValueEnum;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::solar::{self, Light};

/// Equirectangular land mask (1 bit per pixel, 1 = land) from Natural Earth.
const LAND_PBM: &[u8] = include_bytes!("../assets/land.pbm");

/// Visible latitudes: most of Antarctica and the high Arctic are cropped.
pub const NORTH: f64 = 84.0;
pub const SOUTH: f64 = -58.0;

/// Columns per row of an undistorted map (terminal cells are about twice as tall as wide).
const IDEAL_RATIO: f64 = 2.0 * 360.0 / (NORTH - SOUTH);
/// How much the map may stretch away from `IDEAL_RATIO` to fill its area.
const MAX_STRETCH: f64 = 1.25;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum MapStyle {
    /// Plain ASCII characters shaded by how much land each cell covers.
    #[default]
    Ascii,
    /// Braille dots: 2×4 sub-pixels per cell.
    Braille,
    /// Half blocks: 1×2 sub-pixels per cell.
    Blocks,
}

impl MapStyle {
    pub fn next(self) -> Self {
        match self {
            Self::Ascii => Self::Braille,
            Self::Braille => Self::Blocks,
            Self::Blocks => Self::Ascii,
        }
    }

    fn symbol(self, mask: &LandMask, map: &Projection, col: u16, row: u16) -> Option<char> {
        let land = |dx: f64, dy: f64| {
            let (lat, lon) = map.geo(f64::from(col) + dx, f64::from(row) + dy);
            mask.is_land(lat, lon)
        };
        match self {
            Self::Ascii => {
                const SAMPLES: u32 = 4;
                let step = 1.0 / f64::from(SAMPLES);
                let covered = (0..SAMPLES * SAMPLES)
                    .filter(|i| {
                        let (sx, sy) = (f64::from(i % SAMPLES), f64::from(i / SAMPLES));
                        land((sx + 0.5) * step, (sy + 0.5) * step)
                    })
                    .count();
                match covered {
                    0 => None,
                    1..=3 => Some('.'),
                    4..=7 => Some(':'),
                    8..=11 => Some('+'),
                    _ => Some('#'),
                }
            }
            Self::Braille => {
                // Dot bits of U+2800 + n, indexed by [row][column].
                const DOTS: [[u32; 2]; 4] =
                    [[0x01, 0x08], [0x02, 0x10], [0x04, 0x20], [0x40, 0x80]];
                let mut bits = 0;
                for (dy, dots) in DOTS.iter().enumerate() {
                    for (dx, dot) in dots.iter().enumerate() {
                        if land((dx as f64 + 0.5) / 2.0, (dy as f64 + 0.5) / 4.0) {
                            bits |= dot;
                        }
                    }
                }
                (bits != 0).then(|| char::from_u32(0x2800 + bits).unwrap_or('#'))
            }
            Self::Blocks => match (land(0.5, 0.25), land(0.5, 0.75)) {
                (true, true) => Some('█'),
                (true, false) => Some('▀'),
                (false, true) => Some('▄'),
                (false, false) => None,
            },
        }
    }
}

struct LandMask {
    width: usize,
    height: usize,
    row_bytes: usize,
    bits: &'static [u8],
}

impl LandMask {
    fn get() -> &'static Self {
        static MASK: OnceLock<LandMask> = OnceLock::new();
        MASK.get_or_init(|| Self::parse(LAND_PBM).expect("assets/land.pbm is a P4 bitmap"))
    }

    /// Binary PBM: "P4", width and height separated by whitespace, one
    /// whitespace byte, then rows of bits packed MSB first.
    fn parse(data: &'static [u8]) -> Option<Self> {
        let mut fields = Vec::with_capacity(3);
        let mut pos = 0;
        while fields.len() < 3 {
            while data.get(pos)?.is_ascii_whitespace() {
                pos += 1;
            }
            let start = pos;
            while !data.get(pos)?.is_ascii_whitespace() {
                pos += 1;
            }
            fields.push(std::str::from_utf8(&data[start..pos]).ok()?);
            pos += 1;
        }
        if fields[0] != "P4" {
            return None;
        }
        let width: usize = fields[1].parse().ok()?;
        let height: usize = fields[2].parse().ok()?;
        let row_bytes = width.div_ceil(8);
        let bits = data.get(pos..pos + row_bytes * height)?;
        Some(Self {
            width,
            height,
            row_bytes,
            bits,
        })
    }

    fn is_land(&self, lat: f64, lon: f64) -> bool {
        let x = ((lon + 180.0) / 360.0 * self.width as f64).floor() as isize;
        let y = ((90.0 - lat) / 180.0 * self.height as f64).floor() as isize;
        let x = x.rem_euclid(self.width as isize) as usize;
        let y = y.clamp(0, self.height as isize - 1) as usize;
        self.bits[y * self.row_bytes + x / 8] & (0x80 >> (x % 8)) != 0
    }
}

/// The part of `area` the map fills: as large as possible while its aspect
/// ratio stays within `MAX_STRETCH` of an undistorted map.
pub fn fit(area: Rect) -> Rect {
    let (width, height) = (f64::from(area.width), f64::from(area.height));
    if area.is_empty() {
        return Rect::new(area.x, area.y, 0, 0);
    }
    let ratio = width / height;
    let (width, height) = if ratio > IDEAL_RATIO * MAX_STRETCH {
        (
            (height * IDEAL_RATIO * MAX_STRETCH).round() as u16,
            area.height,
        )
    } else if ratio < IDEAL_RATIO / MAX_STRETCH {
        (
            area.width,
            (width / IDEAL_RATIO * MAX_STRETCH).round() as u16,
        )
    } else {
        (area.width, area.height)
    };
    let (width, height) = (width.min(area.width), height.min(area.height));
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

/// Equirectangular projection onto the cells of `area`.
struct Projection {
    area: Rect,
}

impl Projection {
    /// Latitude and longitude at a point measured in cells from the top-left corner.
    fn geo(&self, x: f64, y: f64) -> (f64, f64) {
        let lon = -180.0 + 360.0 * x / f64::from(self.area.width);
        let lat = NORTH - (NORTH - SOUTH) * y / f64::from(self.area.height);
        (lat, lon)
    }

    /// Screen cell showing a latitude and longitude, if it is inside the map.
    fn cell(&self, lat: f64, lon: f64) -> Option<(u16, u16)> {
        if !(SOUTH..=NORTH).contains(&lat) {
            return None;
        }
        let lon = (lon + 180.0).rem_euclid(360.0);
        let x = (lon / 360.0 * f64::from(self.area.width)) as u16;
        let y = ((NORTH - lat) / (NORTH - SOUTH) * f64::from(self.area.height)) as u16;
        Some((
            self.area.x + x.min(self.area.width - 1),
            self.area.y + y.min(self.area.height - 1),
        ))
    }
}

pub struct Marker<'a> {
    pub lat: f64,
    pub lon: f64,
    pub label: &'a str,
}

pub struct WorldMap<'a> {
    pub style: MapStyle,
    /// Where the Sun is overhead; when present, the night side is shaded.
    pub sun: Option<(f64, f64)>,
    pub marker: Option<Marker<'a>>,
}

impl Widget for WorldMap<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let area = fit(area.intersection(buf.area));
        if area.is_empty() {
            return;
        }
        let mask = LandMask::get();
        let map = Projection { area };
        for row in 0..area.height {
            for col in 0..area.width {
                let Some(symbol) = self.style.symbol(mask, &map, col, row) else {
                    continue;
                };
                let (lat, lon) = map.geo(f64::from(col) + 0.5, f64::from(row) + 0.5);
                let light = self
                    .sun
                    .map_or(Light::Day, |sun| solar::light(sun, lat, lon));
                buf[(area.x + col, area.y + row)]
                    .set_char(symbol)
                    .set_fg(land_color(light));
            }
        }
        if let Some((x, y)) = self.sun.and_then(|(lat, lon)| map.cell(lat, lon)) {
            buf[(x, y)]
                .set_char('☼')
                .set_style(Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD));
        }
        if let Some(marker) = self.marker {
            draw_marker(buf, &map, &marker);
        }
    }
}

fn land_color(light: Light) -> Color {
    match light {
        Light::Day => Color::Green,
        Light::Twilight => Color::Yellow,
        Light::Night => Color::Blue,
    }
}

/// A red dot on the city and its name on whichever side has room.
fn draw_marker(buf: &mut Buffer, map: &Projection, marker: &Marker) {
    let Some((x, y)) = map.cell(marker.lat, marker.lon) else {
        return;
    };
    buf[(x, y)]
        .set_char('●')
        .set_style(Style::new().fg(Color::Red).add_modifier(Modifier::BOLD));

    let tag = format!(" {} ", marker.label);
    let width = tag.width() as u16;
    let room_right = map.area.right().saturating_sub(x + 2);
    let room_left = x.saturating_sub(map.area.left() + 1);
    let style = Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD);
    if width <= room_right || room_right >= room_left {
        buf.set_stringn(x + 2, y, &tag, usize::from(room_right), style);
    } else {
        let width = width.min(room_left);
        buf.set_stringn(x - 1 - width, y, &tag, usize::from(width), style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn render(style: MapStyle, width: u16, height: u16, marker: Option<Marker>) -> Buffer {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| {
                let map = WorldMap {
                    style,
                    sun: Some((0.0, 0.0)),
                    marker,
                };
                frame.render_widget(map, frame.area());
            })
            .unwrap();
        terminal.backend().buffer().clone()
    }

    #[test]
    fn knows_land_from_water() {
        let mask = LandMask::get();
        assert_eq!((mask.width, mask.height), (1440, 720));
        assert!(mask.is_land(-15.8, -47.9), "Brasília");
        assert!(mask.is_land(23.0, 10.0), "Sahara");
        assert!(mask.is_land(64.0, 100.0), "Siberia");
        assert!(!mask.is_land(0.0, -30.0), "Atlantic");
        assert!(!mask.is_land(0.0, -150.0), "Pacific");
        assert!(!mask.is_land(42.0, 50.5), "Caspian Sea");
    }

    #[test]
    fn keeps_the_map_proportions() {
        assert_eq!(fit(Rect::new(0, 0, 100, 20)), Rect::new(0, 0, 100, 20));
        // Too wide: the map is centered horizontally.
        let wide = fit(Rect::new(0, 0, 300, 20));
        assert_eq!((wide.width, wide.height), (127, 20));
        assert_eq!(wide.x, (300 - 127) / 2);
        // Too tall: centered vertically.
        let tall = fit(Rect::new(0, 0, 50, 40));
        assert_eq!((tall.width, tall.height), (50, 12));
        assert!(fit(Rect::new(3, 3, 0, 10)).is_empty());
    }

    #[test]
    fn projects_cities_onto_cells() {
        let map = Projection {
            area: Rect::new(10, 5, 360, 142),
        };
        assert_eq!(map.cell(0.0, 0.0), Some((190, 89)));
        assert_eq!(map.cell(NORTH, -180.0), Some((10, 5)));
        assert_eq!(map.cell(-89.0, 0.0), None);
        let (lat, lon) = map.geo(180.5, 84.5);
        assert!((lat - -0.5).abs() < 1e-9 && (lon - 0.5).abs() < 1e-9);
    }

    #[test]
    fn draws_land_in_every_style_and_size() {
        for style in [MapStyle::Ascii, MapStyle::Braille, MapStyle::Blocks] {
            for (width, height) in [(1, 1), (2, 1), (7, 3), (40, 8), (120, 24), (300, 90)] {
                let buf = render(style, width, height, None);
                let land = buf.content().iter().filter(|c| c.symbol() != " ").count();
                if width >= 40 {
                    assert!(land > usize::from(width), "{style:?} {width}x{height}");
                }
            }
        }
    }

    #[test]
    fn marks_the_city_with_its_name() {
        let marker = Marker {
            lat: -23.5,
            lon: -46.6,
            label: "São Paulo",
        };
        let buf = render(MapStyle::Ascii, 100, 20, Some(marker));
        let text: String = buf.content().iter().map(|c| c.symbol()).collect();
        let dot = text.chars().position(|c| c == '●').expect("marker drawn");
        let (x, y) = (dot % 100, dot / 100);
        assert_eq!((x, y), (37, 15));
        assert!(text.contains(" São Paulo "));
    }

    #[test]
    fn puts_the_label_on_the_left_near_the_right_edge() {
        let marker = Marker {
            lat: -36.85,
            lon: 174.76,
            label: "Auckland",
        };
        let buf = render(MapStyle::Braille, 100, 20, Some(marker));
        let row: Vec<String> = (0..100).map(|x| buf[(x, 17)].symbol().to_owned()).collect();
        let dot = row.iter().position(|cell| cell == "●").unwrap();
        assert_eq!(dot, 98);
        assert_eq!(row[87..97].concat(), " Auckland ");
    }
}
