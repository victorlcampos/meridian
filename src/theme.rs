//! Color themes after well-known editor themes, plus the terminal's own
//! colors: the 50 that vitals has too, kept in cerne.

use ratatui::style::{Color, Modifier, Style};

use cerne::themes::{Colors, color as to_color};
pub use cerne::themes::{THEMES, Theme, find, truecolor_from_env};

/// A theme's colors, ready to draw with.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// `Color::Reset` for the terminal's own background.
    pub bg: Color,
    pub fg: Color,
    muted: Option<Color>,
    pub digits: Color,
    /// Tabs, keys and borders.
    pub accent: Color,
    /// Land in daylight, at dusk and at night on the world map.
    pub day: Color,
    pub dusk: Color,
    pub night: Color,
    /// City markers, the alarm message and the blinking screen.
    pub marker: Color,
    pub sun: Color,
    /// Chance of rain in the forecast.
    pub rain: Color,
    /// The weather scene's colors.
    pub sky: SkyColors,
}

/// The colors the weather scene shades and blends, as `0xRRGGBB`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkyColors {
    /// The screen's background, which soft edges and glows fade into;
    /// `None` for the terminal's own, whose color is unknown.
    pub bg: Option<u32>,
    /// The lighter and the darker of the text and the background: every
    /// gray of the scene lies between them.
    pub light: u32,
    pub dark: u32,
    pub sun: u32,
    /// The sun's warm edge.
    pub orange: u32,
    pub rain: u32,
    truecolor: bool,
}

impl SkyColors {
    /// `rgb` as the terminal shows it.
    pub fn color(&self, rgb: u32) -> Color {
        to_color(rgb, self.truecolor)
    }

    /// Whether the scene can blend its edges into the background: it has to
    /// know its color and show it exactly, in 24-bit color.
    pub fn soft(&self) -> bool {
        self.truecolor && self.bg.is_some()
    }
}

impl Palette {
    /// Secondary text: the theme's comment color, or dimmed text in the terminal's colors.
    pub fn muted(&self) -> Style {
        match self.muted {
            Some(color) => Style::new().fg(color),
            None => Style::new().add_modifier(Modifier::DIM),
        }
    }

    /// Colors for the whole screen.
    pub fn base(&self) -> Style {
        Style::new().fg(self.fg).bg(self.bg)
    }

    /// The screen while an alarm rings.
    pub fn flash(&self) -> Style {
        if self.bg == Color::Reset {
            Style::new().fg(Color::White).bg(Color::Red)
        } else {
            Style::new().fg(self.bg).bg(self.marker)
        }
    }
}

/// A theme's colors as meridian uses them.
pub trait Paint {
    /// With `truecolor` off, colors come from the 256-color palette.
    fn palette(&self, truecolor: bool) -> Palette;
}

impl Paint for Theme {
    fn palette(&self, truecolor: bool) -> Palette {
        let Some(colors) = self.colors else {
            return Palette {
                bg: Color::Reset,
                fg: Color::Reset,
                muted: None,
                digits: Color::Cyan,
                accent: Color::Cyan,
                day: Color::Green,
                dusk: Color::Yellow,
                night: Color::Blue,
                marker: Color::Red,
                sun: Color::Yellow,
                rain: Color::Blue,
                // Mid tones that show on dark and light backgrounds alike.
                sky: SkyColors {
                    bg: None,
                    light: 0xe8ebf0,
                    dark: 0x3c414c,
                    sun: 0xffc53d,
                    orange: 0xff8a3d,
                    rain: 0x5aa8ff,
                    truecolor,
                },
            };
        };
        let Colors {
            bg,
            fg,
            comment,
            red,
            orange,
            yellow,
            green,
            blue,
            ..
        } = colors;
        let color = |rgb: u32| to_color(rgb, truecolor);
        let (light, dark) = if luminance(fg) >= luminance(bg) {
            (fg, bg)
        } else {
            (bg, fg)
        };
        Palette {
            bg: color(bg),
            fg: color(fg),
            muted: Some(color(comment)),
            digits: color(colors.slot(self.primary)),
            accent: color(colors.slot(self.accent)),
            day: color(green),
            dusk: color(orange),
            // Night is the theme's blue sunk halfway into the background.
            night: color(mix(blue, bg)),
            marker: color(red),
            sun: color(yellow),
            rain: color(blue),
            sky: SkyColors {
                bg: Some(bg),
                light,
                dark,
                sun: yellow,
                orange,
                rain: blue,
                truecolor,
            },
        }
    }
}

/// How bright a color looks, from 0 to 255.
fn luminance(rgb: u32) -> f64 {
    let [_, r, g, b] = rgb.to_be_bytes().map(f64::from);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// The average of two colors, channel by channel.
fn mix(a: u32, b: u32) -> u32 {
    let (a, b) = (a.to_be_bytes(), b.to_be_bytes());
    u32::from_be_bytes(std::array::from_fn(|i| {
        ((u16::from(a[i]) + u16::from(b[i])) / 2) as u8
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_the_theme_colors() {
        let tokyo = THEMES[find("Tokyo Night").unwrap()].palette(true);
        assert_eq!(tokyo.bg, Color::Rgb(0x1a, 0x1b, 0x26));
        assert_eq!(tokyo.digits, Color::Rgb(0x7a, 0xa2, 0xf7));
        assert_eq!(tokyo.accent, Color::Rgb(0xbb, 0x9a, 0xf7));
        assert_eq!(tokyo.muted(), Style::new().fg(Color::Rgb(0x56, 0x5f, 0x89)));
        // Night land: blue #7aa2f7 halfway into the background #1a1b26.
        assert_eq!(tokyo.night, Color::Rgb(0x4a, 0x5e, 0x8e));
        assert_eq!(tokyo.flash(), Style::new().fg(tokyo.bg).bg(tokyo.marker));
    }

    #[test]
    fn keeps_the_terminal_colors_by_default() {
        let terminal = THEMES[0].palette(true);
        assert_eq!((terminal.bg, terminal.digits), (Color::Reset, Color::Cyan));
        assert_eq!(terminal.muted(), Style::new().add_modifier(Modifier::DIM));
        assert_eq!(
            terminal.flash(),
            Style::new().fg(Color::White).bg(Color::Red)
        );
    }

    #[test]
    fn falls_back_to_the_256_color_palette() {
        let tokyo = THEMES[find("Tokyo Night").unwrap()].palette(false);
        assert_eq!(tokyo.bg, Color::Indexed(234));
        assert_eq!(tokyo.digits, Color::Indexed(111));
    }
}
