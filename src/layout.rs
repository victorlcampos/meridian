//! Splits the window between the tab bar, the clock, the world map and the key
//! hints, so the clock stays readable from a full screen down to a small
//! tiling pane.

use ratatui::layout::Rect;

use crate::{clockface, worldmap};

#[derive(Debug, PartialEq, Eq)]
pub struct Screen {
    pub tabs: Option<Rect>,
    pub clock: Rect,
    pub map: Option<Rect>,
    pub footer: Option<Rect>,
}

/// Smallest map worth drawing, in cells.
const MIN_MAP: (u16, u16) = (24, 5);
/// Rows the clock uses besides its digits: place, date, alarm and a gap.
const INFO_ROWS: u16 = 4;

/// The tab bar and the key hints each take a row when the window is tall enough.
pub fn split(area: Rect, with_map: bool, with_tabs: bool) -> Screen {
    let tabs = (with_tabs && area.height >= 8).then_some(Rect { height: 1, ..area });
    let footer = (area.height >= 10 && area.width >= 20).then(|| Rect {
        y: area.bottom() - 1,
        height: 1,
        ..area
    });
    let top = u16::from(tabs.is_some());
    let body = Rect {
        y: area.y + top,
        height: area.height - top - u16::from(footer.is_some()),
        ..area
    };
    match with_map.then(|| best_split(body)).flatten() {
        Some((clock, map)) => Screen {
            tabs,
            clock,
            map: Some(map),
            footer,
        },
        None => Screen {
            tabs,
            clock: body,
            map: None,
            footer,
        },
    }
}

/// Tries every clock size with the map below it and beside it, and keeps the
/// most balanced arrangement: the largest product of digits area and map
/// area. The map gets only the rows or columns it can fill; the clock gets
/// the rest.
fn best_split(body: Rect) -> Option<(Rect, Rect)> {
    let mut best: Option<(u64, Rect, Rect)> = None;
    for scale in 1..=64 {
        let (digits_width, digits_height) = clockface::size("00:00:00", scale);
        let (clock_width, clock_height) = (digits_width + 2, digits_height + INFO_ROWS);
        if clock_width > body.width && clock_height > body.height {
            break;
        }
        let digits = u64::from(digits_width) * u64::from(digits_height);

        if clock_width <= body.width && clock_height < body.height {
            let rest = Rect {
                y: body.y + clock_height,
                height: body.height - clock_height,
                ..body
            };
            let fitted = worldmap::fit(rest);
            let map = Rect {
                y: body.bottom() - fitted.height,
                height: fitted.height,
                ..body
            };
            let clock = Rect {
                height: body.height - fitted.height,
                ..body
            };
            consider(&mut best, digits, fitted, clock, map);
        }
        if clock_height <= body.height && clock_width < body.width {
            let rest = Rect {
                x: body.x + clock_width,
                width: body.width - clock_width,
                ..body
            };
            let fitted = worldmap::fit(rest);
            let map = Rect {
                x: body.right() - fitted.width,
                width: fitted.width,
                ..body
            };
            let clock = Rect {
                width: body.width - fitted.width,
                ..body
            };
            consider(&mut best, digits, fitted, clock, map);
        }
    }
    best.map(|(_, clock, map)| (clock, map))
}

fn consider(
    best: &mut Option<(u64, Rect, Rect)>,
    digits: u64,
    fitted: Rect,
    clock: Rect,
    map: Rect,
) {
    if fitted.width < MIN_MAP.0 || fitted.height < MIN_MAP.1 {
        return;
    }
    let score = digits * u64::from(fitted.area());
    if best
        .as_ref()
        .is_none_or(|(best_score, ..)| score > *best_score)
    {
        *best = Some((score, clock, map));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inside(inner: Rect, outer: Rect) -> bool {
        inner.is_empty() || outer.union(inner) == outer
    }

    #[test]
    fn every_window_size_gets_a_consistent_layout() {
        for width in (0..=320).step_by(7) {
            for height in (0..=100).step_by(3) {
                let area = Rect::new(2, 1, width, height);
                for (with_map, with_tabs) in
                    [(false, false), (true, false), (false, true), (true, true)]
                {
                    let screen = split(area, with_map, with_tabs);
                    let parts: Vec<Rect> =
                        [screen.tabs, Some(screen.clock), screen.map, screen.footer]
                            .into_iter()
                            .flatten()
                            .collect();
                    for (i, part) in parts.iter().enumerate() {
                        assert!(inside(*part, area), "{part:?} outside {area:?}");
                        for other in &parts[i + 1..] {
                            assert!(!part.intersects(*other), "{part:?} overlaps {other:?}");
                        }
                    }
                    if let Some(map) = screen.map {
                        let fitted = worldmap::fit(map);
                        assert!(fitted.width >= MIN_MAP.0 && fitted.height >= MIN_MAP.1);
                        assert!(screen.clock.width >= 29 && screen.clock.height >= 7);
                    }
                }
            }
        }
    }

    #[test]
    fn hides_the_footer_in_tiny_windows() {
        assert!(split(Rect::new(0, 0, 80, 9), false, false).footer.is_none());
        assert!(
            split(Rect::new(0, 0, 19, 30), false, false)
                .footer
                .is_none()
        );
        assert_eq!(
            split(Rect::new(0, 0, 80, 24), false, false).footer,
            Some(Rect::new(0, 23, 80, 1))
        );
    }

    #[test]
    fn puts_the_tab_bar_on_top_when_there_is_room() {
        let screen = split(Rect::new(0, 0, 80, 24), true, true);
        assert_eq!(screen.tabs, Some(Rect::new(0, 0, 80, 1)));
        assert_eq!(screen.clock.y, 1);
        assert!(split(Rect::new(0, 0, 80, 7), false, true).tabs.is_none());
        assert!(split(Rect::new(0, 0, 80, 24), false, false).tabs.is_none());
    }

    #[test]
    fn stacks_the_map_under_the_clock_in_tall_panes() {
        let screen = split(Rect::new(0, 0, 95, 50), true, false);
        let map = screen.map.unwrap();
        assert_eq!((screen.clock.x, map.x, map.width), (0, 0, 95));
        assert!(map.y >= screen.clock.bottom());
    }

    #[test]
    fn puts_the_map_beside_the_clock_in_wide_panes() {
        let screen = split(Rect::new(0, 0, 200, 24), true, false);
        let map = screen.map.unwrap();
        assert_eq!((screen.clock.y, map.y), (0, 0));
        assert!(map.x >= screen.clock.right());
    }

    #[test]
    fn drops_the_map_when_there_is_no_room() {
        assert!(split(Rect::new(0, 0, 40, 10), true, false).map.is_none());
        assert!(split(Rect::new(0, 0, 28, 40), true, false).map.is_none());
    }

    #[test]
    fn balances_clock_and_map_in_a_short_wide_pane() {
        // Neither a giant clock beside a stamp-sized map nor the reverse.
        let screen = split(Rect::new(0, 0, 200, 24), true, false);
        let map = worldmap::fit(screen.map.unwrap());
        assert_eq!((screen.clock.width, map.width, map.height), (110, 90, 22));
    }
}
