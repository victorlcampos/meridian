//! Color themes after well-known editor themes, plus the terminal's own colors.

use ratatui::style::{Color, Modifier, Style};

use crate::cities::fold;

/// Which color of a theme's palette a part of the clock takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Slot {
    Red,
    Orange,
    Yellow,
    Green,
    Cyan,
    Blue,
    Purple,
}

pub struct Theme {
    pub name: &'static str,
    /// Background, foreground, comments, red, orange, yellow, green, cyan, blue
    /// and purple, as the theme defines them. `None` keeps the terminal's colors.
    colors: Option<[u32; 10]>,
    digits: Slot,
    accent: Slot,
}

const fn theme(name: &'static str, colors: [u32; 10], digits: Slot, accent: Slot) -> Theme {
    Theme {
        name,
        colors: Some(colors),
        digits,
        accent,
    }
}

use Slot::{Blue, Cyan, Green, Orange, Purple, Red, Yellow};

#[rustfmt::skip]
pub const THEMES: [Theme; 51] = [
    Theme {
        name: "Terminal",
        colors: None,
        digits: Cyan,
        accent: Cyan,
    },
    theme("Tokyo Night", [0x1a1b26, 0xc0caf5, 0x565f89, 0xf7768e, 0xff9e64, 0xe0af68, 0x9ece6a, 0x7dcfff, 0x7aa2f7, 0xbb9af7], Blue, Purple),
    theme("Tokyo Night Storm", [0x24283b, 0xc0caf5, 0x565f89, 0xf7768e, 0xff9e64, 0xe0af68, 0x9ece6a, 0x7dcfff, 0x7aa2f7, 0xbb9af7], Cyan, Blue),
    theme("Tokyo Night Moon", [0x222436, 0xc8d3f5, 0x636da6, 0xff757f, 0xff966c, 0xffc777, 0xc3e88d, 0x86e1fc, 0x82aaff, 0xc099ff], Purple, Blue),
    theme("Tokyo Night Day", [0xe1e2e7, 0x3760bf, 0x848cb5, 0xf52a65, 0xb15c00, 0x8c6c3e, 0x587539, 0x007197, 0x2e7de9, 0x9854f1], Blue, Purple),
    theme("Dracula", [0x282a36, 0xf8f8f2, 0x6272a4, 0xff5555, 0xffb86c, 0xf1fa8c, 0x50fa7b, 0x8be9fd, 0xbd93f9, 0xff79c6], Blue, Purple),
    theme("One Dark", [0x282c34, 0xabb2bf, 0x5c6370, 0xe06c75, 0xd19a66, 0xe5c07b, 0x98c379, 0x56b6c2, 0x61afef, 0xc678dd], Blue, Purple),
    theme("One Light", [0xfafafa, 0x383a42, 0xa0a1a7, 0xe45649, 0x986801, 0xc18401, 0x50a14f, 0x0184bc, 0x4078f2, 0xa626a4], Blue, Purple),
    theme("Monokai", [0x272822, 0xf8f8f2, 0x75715e, 0xf92672, 0xfd971f, 0xe6db74, 0xa6e22e, 0x66d9ef, 0x66d9ef, 0xae81ff], Red, Green),
    theme("Monokai Pro", [0x2d2a2e, 0xfcfcfa, 0x727072, 0xff6188, 0xfc9867, 0xffd866, 0xa9dc76, 0x78dce8, 0x78dce8, 0xab9df2], Yellow, Red),
    theme("Nord", [0x2e3440, 0xd8dee9, 0x616e88, 0xbf616a, 0xd08770, 0xebcb8b, 0xa3be8c, 0x88c0d0, 0x81a1c1, 0xb48ead], Cyan, Blue),
    theme("Gruvbox Dark", [0x282828, 0xebdbb2, 0x928374, 0xfb4934, 0xfe8019, 0xfabd2f, 0xb8bb26, 0x8ec07c, 0x83a598, 0xd3869b], Yellow, Orange),
    theme("Gruvbox Light", [0xfbf1c7, 0x3c3836, 0x928374, 0x9d0006, 0xaf3a03, 0xb57614, 0x79740e, 0x427b58, 0x076678, 0x8f3f71], Orange, Blue),
    theme("Solarized Dark", [0x002b36, 0x839496, 0x586e75, 0xdc322f, 0xcb4b16, 0xb58900, 0x859900, 0x2aa198, 0x268bd2, 0xd33682], Blue, Yellow),
    theme("Solarized Light", [0xfdf6e3, 0x657b83, 0x93a1a1, 0xdc322f, 0xcb4b16, 0xb58900, 0x859900, 0x2aa198, 0x268bd2, 0xd33682], Blue, Yellow),
    theme("Catppuccin Mocha", [0x1e1e2e, 0xcdd6f4, 0x6c7086, 0xf38ba8, 0xfab387, 0xf9e2af, 0xa6e3a1, 0x94e2d5, 0x89b4fa, 0xcba6f7], Purple, Blue),
    theme("Catppuccin Macchiato", [0x24273a, 0xcad3f5, 0x6e738d, 0xed8796, 0xf5a97f, 0xeed49f, 0xa6da95, 0x8bd5ca, 0x8aadf4, 0xc6a0f6], Purple, Blue),
    theme("Catppuccin Frappé", [0x303446, 0xc6d0f5, 0x737994, 0xe78284, 0xef9f76, 0xe5c890, 0xa6d189, 0x81c8be, 0x8caaee, 0xca9ee6], Purple, Blue),
    theme("Catppuccin Latte", [0xeff1f5, 0x4c4f69, 0x9ca0b0, 0xd20f39, 0xfe640b, 0xdf8e1d, 0x40a02b, 0x179299, 0x1e66f5, 0x8839ef], Purple, Blue),
    theme("Rosé Pine", [0x191724, 0xe0def4, 0x6e6a86, 0xeb6f92, 0xebbcba, 0xf6c177, 0x9ccfd8, 0x9ccfd8, 0x31748f, 0xc4a7e7], Orange, Purple),
    theme("Rosé Pine Moon", [0x232136, 0xe0def4, 0x6e6a86, 0xeb6f92, 0xea9a97, 0xf6c177, 0x9ccfd8, 0x9ccfd8, 0x3e8fb0, 0xc4a7e7], Orange, Purple),
    theme("Rosé Pine Dawn", [0xfaf4ed, 0x575279, 0x9893a5, 0xb4637a, 0xd7827e, 0xea9d34, 0x56949f, 0x56949f, 0x286983, 0x907aa9], Orange, Purple),
    theme("Kanagawa", [0x1f1f28, 0xdcd7ba, 0x727169, 0xff5d62, 0xffa066, 0xe6c384, 0x98bb6c, 0x7aa89f, 0x7e9cd8, 0x957fb8], Blue, Orange),
    theme("Kanagawa Dragon", [0x181616, 0xc5c9c5, 0x737c73, 0xc4746e, 0xb6927b, 0xc4b28a, 0x87a987, 0x8ea4a2, 0x8ba4b0, 0x8992a7], Red, Yellow),
    theme("Everforest Dark", [0x2d353b, 0xd3c6aa, 0x859289, 0xe67e80, 0xe69875, 0xdbbc7f, 0xa7c080, 0x83c092, 0x7fbbb3, 0xd699b6], Green, Yellow),
    theme("Everforest Light", [0xfdf6e3, 0x5c6a72, 0x939f91, 0xf85552, 0xf57d26, 0xdfa000, 0x8da101, 0x35a77c, 0x3a94c5, 0xdf69ba], Green, Orange),
    theme("Ayu Dark", [0x0b0e14, 0xbfbdb6, 0x565b66, 0xf07178, 0xff8f40, 0xe6b450, 0xaad94c, 0x95e6cb, 0x59c2ff, 0xd2a6ff], Orange, Blue),
    theme("Ayu Mirage", [0x1f2430, 0xcccac2, 0x707a8c, 0xf28779, 0xffad66, 0xffcc66, 0xd5ff80, 0x95e6cb, 0x73d0ff, 0xdfbfff], Yellow, Blue),
    theme("Ayu Light", [0xfcfcfc, 0x5c6166, 0xadaeb1, 0xf07171, 0xfa8d3e, 0xf2ae49, 0x86b300, 0x4cbf99, 0x399ee6, 0xa37acc], Orange, Blue),
    theme("GitHub Dark", [0x0d1117, 0xc9d1d9, 0x8b949e, 0xff7b72, 0xffa657, 0xd29922, 0x7ee787, 0x56d4dd, 0x58a6ff, 0xd2a8ff], Blue, Purple),
    theme("GitHub Light", [0xffffff, 0x24292f, 0x6e7781, 0xcf222e, 0x953800, 0x9a6700, 0x1a7f37, 0x1b7c83, 0x0969da, 0x8250df], Blue, Purple),
    theme("VS Code Dark+", [0x1e1e1e, 0xd4d4d4, 0x858585, 0xf44747, 0xce9178, 0xdcdcaa, 0x6a9955, 0x4ec9b0, 0x569cd6, 0xc586c0], Blue, Cyan),
    theme("VS Code Light+", [0xffffff, 0x000000, 0x6e6e6e, 0xa31515, 0x795e26, 0xbf8803, 0x008000, 0x267f99, 0x0000ff, 0xaf00db], Blue, Purple),
    theme("Darcula", [0x2b2b2b, 0xa9b7c6, 0x808080, 0xbc3f3c, 0xcc7832, 0xffc66d, 0x6a8759, 0x299999, 0x6897bb, 0x9876aa], Orange, Yellow),
    theme("Material", [0x263238, 0xeeffff, 0x546e7a, 0xf07178, 0xf78c6c, 0xffcb6b, 0xc3e88d, 0x89ddff, 0x82aaff, 0xc792ea], Cyan, Purple),
    theme("Palenight", [0x292d3e, 0xa6accd, 0x676e95, 0xff5370, 0xf78c6c, 0xffcb6b, 0xc3e88d, 0x89ddff, 0x82aaff, 0xc792ea], Purple, Blue),
    theme("Night Owl", [0x011627, 0xd6deeb, 0x637777, 0xef5350, 0xf78c6c, 0xecc48d, 0xaddb67, 0x7fdbca, 0x82aaff, 0xc792ea], Blue, Cyan),
    theme("Cobalt2", [0x193549, 0xffffff, 0x0088ff, 0xff628c, 0xff9d00, 0xffc600, 0x3ad900, 0x9effff, 0x0088ff, 0xfb94ff], Yellow, Orange),
    theme("Shades of Purple", [0x2d2b55, 0xffffff, 0xa599e9, 0xff628c, 0xff9d00, 0xfad000, 0xa5ff90, 0x9effff, 0x6943ff, 0xb362ff], Yellow, Purple),
    theme("SynthWave '84", [0x262335, 0xffffff, 0x848bbd, 0xfe4450, 0xf97e72, 0xfede5d, 0x72f1b8, 0x36f9f6, 0x03edf9, 0xff7edb], Purple, Cyan),
    theme("Oceanic Next", [0x1b2b34, 0xcdd3de, 0x65737e, 0xec5f67, 0xf99157, 0xfac863, 0x99c794, 0x5fb3b3, 0x6699cc, 0xc594c5], Cyan, Orange),
    theme("Panda", [0x292a2b, 0xe6e6e6, 0x676b79, 0xff2c6d, 0xffb86c, 0xffcc95, 0x19f9d8, 0x19f9d8, 0x6fc1ff, 0xff75b5], Cyan, Purple),
    theme("Horizon", [0x1c1e26, 0xd5d8da, 0x6c6f93, 0xe95678, 0xfab795, 0xfac29a, 0x29d398, 0x59e1e3, 0x26bbd9, 0xee64ac], Red, Orange),
    theme("Zenburn", [0x3f3f3f, 0xdcdccc, 0x7f9f7f, 0xcc9393, 0xdfaf8f, 0xf0dfaf, 0x9fc59f, 0x93e0e3, 0x8cd0d3, 0xdc8cc3], Yellow, Blue),
    theme("Tomorrow Night", [0x1d1f21, 0xc5c8c6, 0x969896, 0xcc6666, 0xde935f, 0xf0c674, 0xb5bd68, 0x8abeb7, 0x81a2be, 0xb294bb], Blue, Yellow),
    theme("Tomorrow Night Blue", [0x002451, 0xffffff, 0x7285b7, 0xff9da4, 0xffc58f, 0xffeead, 0xd1f1a9, 0x99ffff, 0xbbdaff, 0xebbbff], Cyan, Purple),
    theme("Nightfox", [0x192330, 0xcdcecf, 0x738091, 0xc94f6d, 0xf4a261, 0xdbc074, 0x81b29a, 0x63cdcf, 0x719cd6, 0x9d79d6], Blue, Cyan),
    theme("Oxocarbon", [0x161616, 0xf2f4f8, 0x525252, 0xee5396, 0xff832b, 0xf1c21b, 0x42be65, 0x3ddbd9, 0x78a9ff, 0xbe95ff], Cyan, Red),
    theme("Vitesse Dark", [0x121212, 0xdbd7ca, 0x758575, 0xcb7676, 0xd4976c, 0xe6cc77, 0x4d9375, 0x5eaab5, 0x6394bf, 0xd9739f], Green, Yellow),
    theme("Xcode Dark", [0x292a30, 0xdfdfe0, 0x7f8c98, 0xff8170, 0xffa14f, 0xd9c97c, 0x67b7a4, 0x6bdfff, 0x4eb0cc, 0xff7ab2], Purple, Cyan),
    theme("Mariana", [0x303841, 0xd8dee9, 0xa6acb9, 0xec5f66, 0xf9ae58, 0xfac761, 0x99c794, 0x5fb4b4, 0x6699cc, 0xc695c6], Orange, Blue),
];

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

impl Theme {
    /// With `truecolor` off, colors come from the 256-color palette.
    pub fn palette(&self, truecolor: bool) -> Palette {
        let Some(
            [
                bg,
                fg,
                comment,
                red,
                orange,
                yellow,
                green,
                cyan,
                blue,
                purple,
            ],
        ) = self.colors
        else {
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
            };
        };
        let color = |rgb: u32| {
            if truecolor {
                let [_, r, g, b] = rgb.to_be_bytes();
                Color::Rgb(r, g, b)
            } else {
                Color::Indexed(ansi256(rgb))
            }
        };
        let slot = |slot: Slot| match slot {
            Red => red,
            Orange => orange,
            Yellow => yellow,
            Green => green,
            Cyan => cyan,
            Blue => blue,
            Purple => purple,
        };
        Palette {
            bg: color(bg),
            fg: color(fg),
            muted: Some(color(comment)),
            digits: color(slot(self.digits)),
            accent: color(slot(self.accent)),
            day: color(green),
            dusk: color(orange),
            // Night is the theme's blue sunk halfway into the background.
            night: color(mix(blue, bg)),
            marker: color(red),
            sun: color(yellow),
            rain: color(blue),
        }
    }
}

/// The theme called `name`, ignoring case, accents, spaces and punctuation
/// ("tokyo-night", "rose pine"), or else the first whose name starts that way
/// ("gruvbox").
pub fn find(name: &str) -> Option<usize> {
    let key = |text: &str| fold(text).replace(' ', "");
    let wanted = key(name);
    if wanted.is_empty() {
        return None;
    }
    THEMES
        .iter()
        .position(|theme| key(theme.name) == wanted)
        .or_else(|| {
            THEMES
                .iter()
                .position(|theme| key(theme.name).starts_with(&wanted))
        })
}

/// Whether the terminal says it shows 24-bit color.
pub fn truecolor_from_env() -> bool {
    std::env::var("COLORTERM")
        .is_ok_and(|value| value.contains("truecolor") || value.contains("24bit"))
}

/// The average of two colors, channel by channel.
fn mix(a: u32, b: u32) -> u32 {
    let (a, b) = (a.to_be_bytes(), b.to_be_bytes());
    u32::from_be_bytes(std::array::from_fn(|i| {
        ((u16::from(a[i]) + u16::from(b[i])) / 2) as u8
    }))
}

/// Nearest color of the xterm 256-color palette: its 6×6×6 cube or its gray ramp.
fn ansi256(rgb: u32) -> u8 {
    const LEVELS: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let [_, r, g, b] = rgb.to_be_bytes().map(i32::from);
    let level = |c: i32| match c {
        ..48 => 0,
        48..115 => 1,
        _ => ((c - 35) / 40) as usize,
    };
    let (ri, gi, bi) = (level(r), level(g), level(b));
    let gray_step = match (r + g + b) / 3 {
        239.. => 23,
        average => (average - 3).max(0) / 10,
    };
    let gray = 8 + 10 * gray_step;
    let distance = |(x, y, z): (i32, i32, i32)| (x - r).pow(2) + (y - g).pow(2) + (z - b).pow(2);
    if distance((gray, gray, gray)) < distance((LEVELS[ri], LEVELS[gi], LEVELS[bi])) {
        232 + gray_step as u8
    } else {
        (16 + 36 * ri + 6 * gi + bi) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_fifty_editor_themes_after_the_terminal_colors() {
        assert_eq!(THEMES[0].name, "Terminal");
        assert!(THEMES[0].colors.is_none());
        assert_eq!(
            THEMES[1..]
                .iter()
                .filter(|theme| theme.colors.is_some())
                .count(),
            50
        );
    }

    #[test]
    fn theme_names_are_distinct() {
        let mut keys: Vec<String> = THEMES
            .iter()
            .map(|theme| fold(theme.name).replace(' ', ""))
            .collect();
        keys.sort();
        keys.dedup();
        assert_eq!(keys.len(), THEMES.len());
    }

    #[test]
    fn finds_themes_by_loose_names() {
        let name = |query| find(query).map(|index| THEMES[index].name);
        assert_eq!(name("tokyo-night"), Some("Tokyo Night"));
        assert_eq!(name("TOKYO NIGHT STORM"), Some("Tokyo Night Storm"));
        assert_eq!(name("rose pine dawn"), Some("Rosé Pine Dawn"));
        assert_eq!(name("synthwave84"), Some("SynthWave '84"));
        assert_eq!(name("gruvbox"), Some("Gruvbox Dark"));
        assert_eq!(name("catppuccin frappe"), Some("Catppuccin Frappé"));
        assert_eq!(name("terminal"), Some("Terminal"));
        assert_eq!(name("no such theme"), None);
        assert_eq!(name(""), None);
    }

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
        assert_eq!(ansi256(0x000000), 16);
        assert_eq!(ansi256(0xffffff), 231);
        assert_eq!(ansi256(0xff0000), 196);
        assert_eq!(ansi256(0x808080), 244);
        assert_eq!(ansi256(0x1a1b26), 234);
        let tokyo = THEMES[find("Tokyo Night").unwrap()].palette(false);
        assert_eq!(tokyo.bg, Color::Indexed(234));
        assert_eq!(tokyo.digits, Color::Indexed(111));
    }
}
