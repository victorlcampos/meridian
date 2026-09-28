//! The weather scene: a small pixel-art sky in the clock's corner — the sun, or
//! the moon in its phase at night, clouds, drizzle, rain, sleet, hail, snow,
//! fog and lightning — from the WMO weather code of the city on screen.
//!
//! Shapes are painted on a canvas of half-block pixels, two stacked in each
//! cell, in a few shades per material taken from the theme, with soft edges
//! and glows wherever the background color is known. Rain, snow and stars go
//! on top as thin glyphs, in the cells the shapes leave empty. A frame is a
//! pure function of the size and the time: nothing is kept between frames,
//! and two copies showing the same city stay in step.

use std::f64::consts::TAU;
use std::ops::Range;

use chrono::{DateTime, Utc};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::theme::{Palette, SkyColors};

/// How the weather scene shows: off, one still frame, or animated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Fx {
    #[default]
    Off,
    Still,
    Live,
}

/// What to paint: the WMO code of the weather now, whether the sun is down
/// at the city on screen, whether the city lies south of the equator (where
/// the moon shows upside down), and whether the scene moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scene {
    code: u8,
    night: bool,
    south: bool,
    live: bool,
}

impl Scene {
    /// The scene for a WMO code; `None` for a code without one.
    pub fn new(code: u8, night: bool, south: bool, live: bool) -> Option<Self> {
        sky(code).map(|_| Self {
            code,
            night,
            south,
            live,
        })
    }

    /// How often the scene changes while it moves, in milliseconds: rain
    /// and lightning need ten frames a second, drifting clouds four.
    pub fn frame_ms(&self) -> Option<u64> {
        let sky = sky(self.code)?;
        self.live.then_some(match sky.fall {
            Some((Fall::Drizzle | Fall::Snow, _)) => 125,
            Some(_) => 100,
            None => 250,
        })
    }
}

/// The smallest and the largest scene, in rows: below the one it is too
/// coarse to read, above the other it would crowd the clock.
pub const MIN_ROWS: u16 = 5;
pub const MAX_ROWS: u16 = 8;

/// Columns of a scene `rows` tall: two a row keep its pixels (two to a
/// cell, stacked) about square, and two more give the clouds room.
pub fn width(rows: u16) -> u16 {
    2 * rows + 2
}

/// How much of the sun, or of the moon at night, shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Body {
    Full,
    /// Half hidden behind a cloud.
    Behind,
    /// Pale behind fog.
    Veiled,
    Hidden,
}

/// No clouds, a small one, a large one in front of the sun, or two covering
/// the sky.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cover {
    Clear,
    Few,
    Broken,
    Overcast,
}

/// What falls from the clouds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Fall {
    Drizzle,
    Rain,
    /// Freezing drizzle or rain: short drops and ice pellets.
    Sleet,
    Snow,
    /// Rain with hailstones.
    Hail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Sky {
    body: Body,
    cover: Cover,
    /// What falls, and how hard: 1 to 3.
    fall: Option<(Fall, u8)>,
    fog: bool,
    storm: bool,
}

/// The sky for a WMO weather code.
fn sky(code: u8) -> Option<Sky> {
    let clear = Sky {
        body: Body::Full,
        cover: Cover::Clear,
        fall: None,
        fog: false,
        storm: false,
    };
    let partly = Sky {
        body: Body::Behind,
        cover: Cover::Broken,
        ..clear
    };
    let overcast = Sky {
        body: Body::Hidden,
        cover: Cover::Overcast,
        ..clear
    };
    let storm = Sky {
        storm: true,
        ..overcast
    };
    let falling = |sky: Sky, fall: Fall, amount: u8| Sky {
        fall: Some((fall, amount)),
        ..sky
    };
    Some(match code {
        0 => clear,
        1 => Sky {
            cover: Cover::Few,
            ..clear
        },
        2 => partly,
        3 => overcast,
        45 | 48 => Sky {
            body: Body::Veiled,
            fog: true,
            ..clear
        },
        51 | 53 | 55 => falling(overcast, Fall::Drizzle, (code - 49) / 2),
        56 | 57 => falling(overcast, Fall::Sleet, code - 55),
        61 | 63 | 65 => falling(overcast, Fall::Rain, (code - 59) / 2),
        66 | 67 => falling(overcast, Fall::Sleet, code - 64),
        71 | 73 | 75 => falling(overcast, Fall::Snow, (code - 69) / 2),
        77 => falling(overcast, Fall::Snow, 1),
        80..=82 => falling(partly, Fall::Rain, code - 79),
        85 => falling(partly, Fall::Snow, 1),
        86 => falling(partly, Fall::Snow, 3),
        95 => falling(storm, Fall::Rain, 3),
        96 => falling(storm, Fall::Hail, 2),
        99 => falling(storm, Fall::Hail, 3),
        _ => return None,
    })
}

/// A still scene shows this moment of the animation: rays half out, rain
/// mid-fall and the lightning lit.
const STILL_MS: u64 = 50;

/// Paints the scene over the blank cells of `area`, which it fills best
/// when [`width`] columns wide for its rows.
pub fn draw(buf: &mut Buffer, area: Rect, palette: &Palette, scene: Scene, now: DateTime<Utc>) {
    let area = area.intersection(buf.area);
    let Some(sky) = sky(scene.code) else {
        return;
    };
    if area.is_empty() {
        return;
    }
    let ms = if scene.live {
        now.timestamp_millis().max(0) as u64
    } else {
        STILL_MS
    };
    let t = ms as f64 / 1000.0;
    let colors = &palette.sky;
    // Clouds darken at night, with heavier rain or snow, and in a storm.
    let gloom = f64::from(sky.fall.map_or(0, |(_, amount)| amount)) * 0.05
        + if sky.storm { 0.16 } else { 0.0 }
        + if scene.night { 0.14 } else { 0.0 };
    let tones = Tones::new(colors, gloom);
    let mut canvas = Canvas::new(area.width, area.height, colors);
    let (w, h, u) = (canvas.width as f64, canvas.height as f64, canvas.unit);

    if let Some((x, y, r)) = body(sky, scene.night, w, h, u) {
        // Fog pales the sun and the moon, and hides the sun's rays.
        let veiled = sky.body == Body::Veiled;
        let tones = if veiled { tones.veiled() } else { tones };
        if scene.night {
            moon(&mut canvas, &tones, (x, y), r, moon_phase(now), scene.south);
        } else {
            sun(&mut canvas, &tones, (x, y), r, !veiled, t);
        }
    }
    // Clouds drift to and fro, each at its own pace.
    let drift = |reach: f64, period: f64| {
        if scene.live {
            reach * u * (t * TAU / period).sin()
        } else {
            0.0
        }
    };
    let [back, front] = clouds(sky, w, u, drift);
    let flash = sky.storm && flash(ms);
    for (cloud, shades) in [(back, tones.back), (front, tones.front)] {
        if let Some(cloud) = cloud {
            let shades = if flash {
                lit(shades, tones.bolt)
            } else {
                shades
            };
            cloud.paint(&mut canvas, &shades);
        }
    }
    if sky.fog {
        mist(&mut canvas, tones.mist, if scene.live { t } else { 0.0 });
    }
    if let (true, Some(cloud)) = (flash, front) {
        bolt(&mut canvas, tones.bolt, &cloud, ms / STRIKE_MS);
    }
    canvas.show(buf, area, colors);

    let live = scene.live.then_some(t);
    let mut overlay = Overlay {
        buf,
        area,
        canvas: &canvas,
        palette,
        tones: &tones,
    };
    if scene.night && sky.cover != Cover::Overcast && !sky.fog {
        overlay.stars(live);
    }
    if let (Some((fall, amount)), Some(front)) = (sky.fall, front) {
        let zone = Zone::under(back, front, &canvas);
        overlay.precipitation(&zone, fall, amount, ms, live);
    }
}

/// Where the sun or the moon goes, and its radius, in pixels.
fn body(sky: Sky, night: bool, w: f64, h: f64, u: f64) -> Option<(f64, f64, f64)> {
    let (x, y, sun, moon) = match sky.body {
        Body::Hidden => return None,
        Body::Veiled => (0.5, 0.36, 3.4, 3.8),
        Body::Full if sky.cover == Cover::Few => (0.42, 0.42, 3.0, 3.9),
        Body::Full => (0.5, 0.5, 3.4, 4.6),
        Body::Behind if sky.fall.is_some() => (0.29, 0.3, 2.5, 3.0),
        Body::Behind => (0.37, 0.38, 2.9, 3.6),
    };
    Some((x * w, y * h, if night { moon } else { sun } * u))
}

/// The cloud behind and the cloud in front, if any; `drift(reach, period)`
/// moves each to and fro.
fn clouds(sky: Sky, w: f64, u: f64, drift: impl Fn(f64, f64) -> f64) -> [Option<Cloud>; 2] {
    let falling = sky.fall.is_some();
    let cloud = |x: f64, bottom: f64, size: f64, (reach, period): (f64, f64)| {
        // Rain clouds end on a row boundary: rain starts in the next row.
        let bottom = if falling {
            (bottom * u / 2.0).round() * 2.0
        } else {
            bottom * u
        };
        Some(Cloud {
            x: x * w + drift(reach, period),
            bottom,
            size: size * u,
        })
    };
    match (sky.cover, falling) {
        (Cover::Clear, _) => [None, None],
        // The little cloud keeps six pixels across, to read at any size.
        (Cover::Few, _) => [None, cloud(0.66, 14.8, 0.6_f64.max(0.55 / u), (0.6, 9.0))],
        (Cover::Broken, false) => [None, cloud(0.57, 14.6, 0.95, (0.7, 12.0))],
        (Cover::Broken, true) => [None, cloud(0.63, 8.6, 0.88, (0.5, 12.0))],
        (Cover::Overcast, false) => [
            cloud(0.64, 8.8, 0.8, (-0.9, 15.0)),
            cloud(0.41, 14.4, 1.0, (0.6, 10.0)),
        ],
        (Cover::Overcast, true) => [
            cloud(0.66, 5.8, 0.72, (-0.8, 15.0)),
            cloud(0.44, 8.6, 0.96, (0.5, 10.0)),
        ],
    }
}

/// Age of the moon, 0 (new) to 1, from the synodic month of 29.53 days past
/// the new moon of January 6th, 2000, 18:14 UTC.
fn moon_phase(now: DateTime<Utc>) -> f64 {
    const NEW_MOON: i64 = 947_182_440;
    let age_days = (now.timestamp() - NEW_MOON) as f64 / 86_400.0;
    (age_days / 29.530_588_853).rem_euclid(1.0)
}

/// Lightning strikes twice in quick succession, once every six seconds.
const STRIKE_MS: u64 = 6_000;

fn flash(ms: u64) -> bool {
    matches!(ms % STRIKE_MS, 0..100 | 200..300)
}

type Rgb = u32;

const WHITE: Rgb = 0xff_ffff;

/// `a` moved `t` of the way (0 to 1) to `b`.
fn lerp(a: Rgb, b: Rgb, t: f64) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    let (a, b) = (a.to_be_bytes(), b.to_be_bytes());
    u32::from_be_bytes(std::array::from_fn(|i| {
        (f64::from(a[i]) + (f64::from(b[i]) - f64::from(a[i])) * t).round() as u8
    }))
}

/// 0 below `from`, 1 above `to`, and a smooth ramp between.
fn smoothstep(from: f64, to: f64, x: f64) -> f64 {
    let t = ((x - from) / (to - from)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A small deterministic hash, to scatter things without keeping any state.
fn hash(value: u64) -> u64 {
    let mut x = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

/// The shades a scene paints with.
#[derive(Clone, Copy)]
struct Tones {
    sun: Rgb,
    sun_core: Rgb,
    sun_edge: Rgb,
    moon: Rgb,
    crater: Rgb,
    /// The dark side of the moon, faintly lit by the Earth.
    earthshine: Rgb,
    /// Lit rim, body, shade and base of the cloud in front, and of the
    /// darker one behind it.
    front: [Rgb; 4],
    back: [Rgb; 4],
    mist: Rgb,
    bolt: Rgb,
    rain: Rgb,
    drizzle: Rgb,
    ice: Rgb,
}

impl Tones {
    /// The shades for the theme's colors, the clouds darker by `gloom`
    /// (0 to about ½).
    fn new(colors: &SkyColors, gloom: f64) -> Self {
        // On a light background the clouds take a darker shade to show at
        // all.
        let on_light = colors.bg == Some(colors.light);
        let shade = gloom + if on_light { 0.14 } else { 0.0 };
        let gray = |t: f64| lerp(colors.light, colors.dark, (t + shade).min(0.8));
        let moon = if on_light {
            lerp(gray(0.3), colors.sun, 0.25)
        } else {
            lerp(colors.light, colors.sun, 0.12)
        };
        let far_side = if on_light { colors.dark } else { colors.light };
        Tones {
            sun: colors.sun,
            sun_core: lerp(colors.sun, WHITE, 0.6),
            sun_edge: lerp(colors.sun, colors.orange, 0.7),
            moon,
            crater: lerp(moon, colors.dark, 0.15),
            earthshine: colors.bg.map_or(colors.dark, |bg| lerp(bg, far_side, 0.08)),
            front: [0.0, 0.14, 0.3, 0.46].map(gray),
            back: [0.22, 0.32, 0.44, 0.56].map(gray),
            mist: gray(0.32),
            bolt: lerp(colors.sun, WHITE, 0.45),
            rain: colors.rain,
            drizzle: lerp(colors.rain, colors.light, 0.35),
            ice: lerp(colors.rain, colors.light, 0.5),
        }
    }

    /// The sun and the moon paled by fog.
    fn veiled(&self) -> Self {
        let pale = |color: Rgb, t: f64| lerp(color, self.mist, t);
        Self {
            sun_core: pale(self.sun_core, 0.3),
            sun: pale(self.sun, 0.45),
            sun_edge: pale(self.sun_edge, 0.55),
            moon: pale(self.moon, 0.35),
            crater: pale(self.crater, 0.35),
            ..*self
        }
    }
}

/// Cloud shades lit from below by a lightning flash.
fn lit([rim, body, shade, base]: [Rgb; 4], flash: Rgb) -> [Rgb; 4] {
    [rim, body, lerp(shade, flash, 0.2), lerp(base, flash, 0.35)]
}

/// The pixels that `lo` to `hi` touches, within the first `limit`.
fn span(lo: f64, hi: f64, limit: usize) -> Range<usize> {
    let start = lo.floor().max(0.0) as usize;
    let end = (hi.ceil().max(0.0) as usize).min(limit);
    start..end.max(start)
}

/// A disc centered on a pixel with a radius of whole pixels and a half, so
/// it comes out round and symmetric even with crisp edges.
fn on_grid((x, y): (f64, f64), r: f64) -> ((f64, f64), f64) {
    (
        (x.floor() + 0.5, y.floor() + 0.5),
        (r - 0.5).round().max(1.0) + 0.5,
    )
}

/// How far `p` lies from the segment from `a` to `b`.
fn to_segment(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let along = ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / (dx * dx + dy * dy).max(1e-9);
    let t = along.clamp(0.0, 1.0);
    (p.0 - a.0 - t * dx).hypot(p.1 - a.1 - t * dy)
}

/// A picture in pixels two to a cell, one above the other.
struct Canvas {
    width: usize,
    height: usize,
    /// Pixels per unit of the design, which is 16 units tall.
    unit: f64,
    pixels: Vec<Option<Rgb>>,
    /// The background that edges and glows fade into; without it (the
    /// terminal's own, or no 24-bit color) edges stay crisp.
    bg: Option<Rgb>,
}

impl Canvas {
    fn new(width: u16, rows: u16, colors: &SkyColors) -> Self {
        let (width, height) = (usize::from(width), 2 * usize::from(rows));
        Self {
            width,
            height,
            unit: height as f64 / 16.0,
            pixels: vec![None; width * height],
            bg: colors.bg.filter(|_| colors.soft()),
        }
    }

    fn pixel(&self, x: usize, y: usize) -> Option<Rgb> {
        self.pixels[y * self.width + x]
    }

    /// Whether the cell at `(col, row)` holds no pixel.
    fn clear(&self, col: usize, row: usize) -> bool {
        self.pixel(col, 2 * row).is_none() && self.pixel(col, 2 * row + 1).is_none()
    }

    /// Lays `color` over a pixel, covering `alpha` of it: blended over what
    /// is known to be under it, else all or nothing.
    fn blend(&mut self, x: usize, y: usize, color: Rgb, alpha: f64) {
        let pixel = &mut self.pixels[y * self.width + x];
        match pixel.or(self.bg) {
            Some(under) if alpha > 0.03 => *pixel = Some(lerp(under, color, alpha)),
            None if alpha >= 0.5 => *pixel = Some(color),
            _ => {}
        }
    }

    /// Paints what `ink` gives at each point of the box `(x0, y0, x1, y1)`,
    /// `opacity` times its coverage: each pixel is sampled at 4×4 points and
    /// the colors found there averaged, which smooths edges and gradients.
    fn paint(
        &mut self,
        (x0, y0, x1, y1): (f64, f64, f64, f64),
        opacity: f64,
        ink: impl Fn(f64, f64) -> Option<Rgb>,
    ) {
        const N: u32 = 4;
        for y in span(y0, y1, self.height) {
            for x in span(x0, x1, self.width) {
                let (mut sum, mut hits) = ([0; 3], 0);
                for sample in 0..N * N {
                    let px = x as f64 + (f64::from(sample % N) + 0.5) / f64::from(N);
                    let py = y as f64 + (f64::from(sample / N) + 0.5) / f64::from(N);
                    if let Some(rgb) = ink(px, py) {
                        let [_, r, g, b] = rgb.to_be_bytes();
                        for (total, channel) in sum.iter_mut().zip([r, g, b]) {
                            *total += u32::from(channel);
                        }
                        hits += 1;
                    }
                }
                if hits > 0 {
                    let [r, g, b] = sum.map(|total| ((total + hits / 2) / hits) as u8);
                    let alpha = opacity * f64::from(hits) / f64::from(N * N);
                    self.blend(x, y, u32::from_be_bytes([0, r, g, b]), alpha);
                }
            }
        }
    }

    /// A soft light around `center`, fading out from radius `inner` to
    /// `outer`; only where the canvas can blend.
    fn glow(&mut self, center: (f64, f64), inner: f64, outer: f64, color: Rgb, strength: f64) {
        if self.bg.is_none() {
            return;
        }
        let (cx, cy) = center;
        for y in span(cy - outer, cy + outer, self.height) {
            for x in span(cx - outer, cx + outer, self.width) {
                let d = (x as f64 + 0.5 - cx).hypot(y as f64 + 0.5 - cy);
                let fade = ((outer - d) / (outer - inner)).clamp(0.0, 1.0);
                self.blend(x, y, color, strength * fade * fade);
            }
        }
    }

    /// A line through `points`, `half` a width to either side.
    fn stroke(&mut self, points: &[(f64, f64)], half: f64, color: Rgb) {
        let (xs, ys) = (points.iter().map(|p| p.0), points.iter().map(|p| p.1));
        let bounds = (
            xs.clone().fold(f64::MAX, f64::min) - half,
            ys.clone().fold(f64::MAX, f64::min) - half,
            xs.fold(f64::MIN, f64::max) + half,
            ys.fold(f64::MIN, f64::max) + half,
        );
        self.paint(bounds, 1.0, |x, y| {
            points
                .windows(2)
                .any(|pair| to_segment((x, y), pair[0], pair[1]) < half)
                .then_some(color)
        });
    }

    /// Puts the picture on screen in half blocks, over blank cells only.
    fn show(&self, buf: &mut Buffer, area: Rect, colors: &SkyColors) {
        for row in 0..self.height / 2 {
            for col in 0..self.width {
                let color = |y| self.pixel(col, y).map(|rgb| colors.color(rgb));
                let (symbol, fg, bg) = match (color(2 * row), color(2 * row + 1)) {
                    (None, None) => continue,
                    (Some(top), None) => ("▀", top, None),
                    (None, Some(bottom)) => ("▄", bottom, None),
                    (Some(top), Some(bottom)) if top == bottom => ("█", top, None),
                    (Some(top), Some(bottom)) => ("▀", top, Some(bottom)),
                };
                let cell = &mut buf[(area.x + col as u16, area.y + row as u16)];
                if cell.symbol() == " " {
                    cell.set_symbol(symbol).set_fg(fg);
                    if let Some(bg) = bg {
                        cell.set_bg(bg);
                    }
                }
            }
        }
    }
}

/// The sun: a disc glowing from a white-hot middle to a warm rim, and eight
/// rays that take turns reaching out.
fn sun(canvas: &mut Canvas, tones: &Tones, center: (f64, f64), r: f64, rays: bool, t: f64) {
    let ((cx, cy), r) = on_grid(center, r);
    canvas.glow(
        (cx, cy),
        r,
        1.9 * r,
        tones.sun,
        if rays { 0.3 } else { 0.2 },
    );
    if rays {
        // A pixel of sky between the disc and the rays, which reach out a
        // pixel or two, the straight ones and the slanted ones in turn.
        let pulse = (t * TAU / 3.0).sin();
        let start = r + 1.0;
        let long = (0.55 * r).round().max(1.0);
        for k in 0..8 {
            let turn = if k % 2 == 0 { pulse } else { -pulse };
            let reach = (long + 0.5 * turn).max(1.0);
            let (dy, dx) = (f64::from(k) * TAU / 8.0).sin_cos();
            let ray = [
                (cx + dx * start, cy + dy * start),
                (cx + dx * (start + reach), cy + dy * (start + reach)),
            ];
            canvas.stroke(&ray, 0.5, tones.sun);
        }
    }
    canvas.paint((cx - r, cy - r, cx + r, cy + r), 1.0, |x, y| {
        let d = (x - cx).hypot(y - cy) / r;
        (d < 1.0).then(|| {
            let hot = lerp(tones.sun_core, tones.sun, smoothstep(0.0, 0.7, d));
            lerp(hot, tones.sun_edge, smoothstep(0.6, 1.0, d))
        })
    });
}

/// The moon in its `phase` (0 new, ½ full): the lit part with a few
/// craters, the rest faintly showing. Waxing moons are lit on the right;
/// south of the equator the moon turns over.
fn moon(canvas: &mut Canvas, tones: &Tones, center: (f64, f64), r: f64, phase: f64, south: bool) {
    let ((cx, cy), r) = on_grid(center, r);
    const CRATERS: [(f64, f64, f64); 4] = [
        (-0.38, -0.2, 0.26),
        (0.28, 0.34, 0.2),
        (0.12, -0.52, 0.14),
        (-0.1, 0.55, 0.12),
    ];
    let lit_share = (1.0 - (TAU * phase).cos()) / 2.0;
    canvas.glow((cx, cy), r, r * 1.9, tones.moon, 0.2 * lit_share);
    // Past the first sliver the crescent keeps a pixel's width; a new moon
    // shows only its dark side.
    let thinnest = 1.0 - 1.2 / r;
    let edge = if lit_share < 0.015 {
        1.0
    } else {
        (TAU * phase).cos().min(thinnest)
    };
    let flip = if south { -1.0 } else { 1.0 };
    canvas.paint((cx - r, cy - r, cx + r, cy + r), 1.0, |x, y| {
        let (nx, ny) = (flip * (x - cx) / r, flip * (y - cy) / r);
        if nx * nx + ny * ny >= 1.0 {
            return None;
        }
        // The terminator is a half ellipse, from one pole to the other.
        let terminator = edge * (1.0 - ny * ny).sqrt();
        let lit = if phase < 0.5 {
            nx > terminator
        } else {
            nx < -terminator
        };
        if !lit {
            return Some(tones.earthshine);
        }
        let crater = CRATERS
            .iter()
            .any(|&(ax, ay, ar)| (nx - ax).powi(2) + (ny - ay).powi(2) < ar * ar);
        Some(if crater { tones.crater } else { tones.moon })
    });
}

/// A cumulus: three puffs on a flat base, 11.2 units wide and 6.7 tall.
#[derive(Clone, Copy, Debug)]
struct Cloud {
    /// The middle of its base, in pixels.
    x: f64,
    bottom: f64,
    /// Pixels per unit.
    size: f64,
}

impl Cloud {
    /// Puffs, as center and radius in units from the middle of the base.
    const PUFFS: [(f64, f64, f64); 3] = [(-2.8, -2.3, 2.2), (0.6, -3.5, 3.2), (3.6, -1.9, 1.9)];
    const HALF_WIDTH: f64 = 5.6;
    const HEIGHT: f64 = 6.7;

    fn contains(&self, x: f64, y: f64) -> bool {
        let (x, y) = ((x - self.x) / self.size, (y - self.bottom) / self.size);
        // The base: a bar with round ends.
        let (dx, dy) = ((x.abs() - 4.3).max(0.0), y + 1.3);
        dx * dx + dy * dy < 1.3 * 1.3
            || Self::PUFFS
                .iter()
                .any(|&(px, py, r)| (x - px).powi(2) + (y - py).powi(2) < r * r)
    }

    fn left(&self) -> f64 {
        self.x - Self::HALF_WIDTH * self.size
    }

    fn right(&self) -> f64 {
        self.x + Self::HALF_WIDTH * self.size
    }

    /// Paints the cloud lit from above: a bright rim wherever it faces up,
    /// then darker toward the base.
    fn paint(&self, canvas: &mut Canvas, [rim, body, shade, base]: &[Rgb; 4]) {
        // Crisp clouds move a whole pixel at a time, or their outline would
        // flicker from one frame to the next.
        let crisp = canvas.bg.is_none();
        let cloud = Cloud {
            x: if crisp { self.x.round() } else { self.x },
            ..*self
        };
        let u = canvas.unit;
        let height = Self::HEIGHT * cloud.size;
        let top = cloud.bottom - height;
        canvas.paint(
            (cloud.left(), top, cloud.right(), cloud.bottom),
            1.0,
            |x, y| {
                if !cloud.contains(x, y) {
                    return None;
                }
                let depth = (y - top) / height;
                Some(*if !cloud.contains(x - 0.4 * u, y - 1.1 * u) {
                    rim
                } else if depth > 0.84 {
                    base
                } else if depth > 0.62 {
                    shade
                } else {
                    body
                })
            },
        );
    }
}

/// Wisps of mist drifting slowly across the lower half, `t` seconds in,
/// every other one the opposite way.
fn mist(canvas: &mut Canvas, color: Rgb, t: f64) {
    let (w, h, u) = (canvas.width as f64, canvas.height as f64, canvas.unit);
    let half = (0.5 * u).max(0.5);
    for (i, row) in [0.52, 0.66, 0.8, 0.94].into_iter().enumerate() {
        let i = i as f64;
        // Through the middle of a pixel row, so crisp wisps never vanish
        // between two.
        let y = (row * h).floor() + 0.5;
        let period = 0.85 * w + 2.0 * i;
        let length = (0.74 + 0.08 * (i % 2.0)) * period;
        let speed = u * (0.35 + 0.12 * i) * if i % 2.0 == 0.0 { 1.0 } else { -1.0 };
        let shift = t * speed + 5.3 * i;
        let tone = lerp(color, WHITE, 0.08 * i);
        // A faint full length, then a denser middle, so the ends fade out
        // where the canvas blends (and stay crisp where it cannot).
        for (reach, opacity) in [(1.0, 0.4), (0.75, 0.5)] {
            let length = reach * length;
            canvas.paint(
                (0.0, y - 2.0 * half, w, y + 2.0 * half),
                opacity,
                |x, py| {
                    // A bar with round ends, repeating.
                    let along = (x - shift).rem_euclid(period) - length / 2.0;
                    let d = (along.abs() - (length / 2.0 - half)).max(0.0).hypot(py - y);
                    (d < half).then_some(tone)
                },
            );
        }
    }
}

/// A lightning bolt, from inside the cloud down to the bottom, in a spot
/// that changes with each `strike`.
fn bolt(canvas: &mut Canvas, color: Rgb, cloud: &Cloud, strike: u64) {
    /// The bolt's outline, one unit tall: a top bar, a jog to the right and
    /// a long taper to the tip.
    const BOLT: [(f64, f64); 7] = [
        (0.30, 0.0),
        (0.62, 0.0),
        (0.42, 0.4),
        (0.62, 0.4),
        (0.12, 1.0),
        (0.28, 0.54),
        (0.08, 0.54),
    ];
    let top = cloud.bottom - 0.35 * Cloud::HEIGHT * cloud.size;
    let height = canvas.height as f64 - 0.3 * canvas.unit - top;
    if height < 4.0 {
        return;
    }
    let offset = (hash(strike) % 101) as f64 / 100.0 - 0.5;
    let left = cloud.x + offset * 4.0 * cloud.size - 0.35 * height;
    canvas.glow(
        (left + 0.35 * height, top + 0.55 * height),
        0.0,
        0.5 * height,
        color,
        0.15,
    );
    canvas.paint(
        (left, top, left + 0.62 * height, top + height),
        1.0,
        |x, y| {
            let (x, y) = ((x - left) / height, (y - top) / height);
            // Even-odd rule: a ray to the right crosses the outline an odd
            // number of times from inside.
            let crossings = (0..BOLT.len())
                .filter(|&i| {
                    let ((ax, ay), (bx, by)) = (BOLT[i], BOLT[(i + 1) % BOLT.len()]);
                    (ay > y) != (by > y) && x < ax + (y - ay) / (by - ay) * (bx - ax)
                })
                .count();
            (crossings % 2 == 1).then_some(color)
        },
    );
}

/// Where rain or snow falls: the columns under the clouds, from the row of
/// the front cloud's base down.
struct Zone {
    cols: Range<usize>,
    top: usize,
    rows: usize,
}

impl Zone {
    fn under(back: Option<Cloud>, front: Cloud, canvas: &Canvas) -> Self {
        let (left, right) = back.map_or((front.left(), front.right()), |back| {
            (
                back.left().min(front.left()),
                back.right().max(front.right()),
            )
        });
        let inset = 0.8 * front.size;
        let top = (front.bottom / 2.0) as usize;
        Self {
            cols: span(left + inset, right - inset, canvas.width),
            top,
            rows: (canvas.height / 2).saturating_sub(top),
        }
    }
}

/// How rain falls: streaks `len` half rows long, moving `speed` half rows a
/// frame, in `share` sixths of the columns.
struct Drops {
    len: u64,
    speed: u64,
    share: u64,
}

/// Snow, ice or hail: `count` glyphs, each moving down a row every `pace`
/// frames or so, and swaying from side to side when `sway` gives the time.
struct Flurry<'a> {
    count: usize,
    pace: u64,
    sway: Option<f64>,
    looks: &'a [(&'a str, Style)],
}

/// Glyphs over the picture, in the cells it leaves empty.
struct Overlay<'a> {
    buf: &'a mut Buffer,
    area: Rect,
    canvas: &'a Canvas,
    palette: &'a Palette,
    tones: &'a Tones,
}

impl Overlay<'_> {
    /// Puts `symbol` in a cell, if the picture and the screen left it empty.
    fn put(&mut self, col: usize, row: usize, symbol: &str, style: Style) {
        if col >= self.canvas.width || 2 * row >= self.canvas.height || !self.canvas.clear(col, row)
        {
            return;
        }
        let cell = &mut self.buf[(self.area.x + col as u16, self.area.y + row as u16)];
        if cell.symbol() == " " {
            cell.set_symbol(symbol).set_style(style);
        }
    }

    /// Stars twinkling in the empty sky: a dim dot most of the time, a
    /// bright cross at their peak; `live` gives the time.
    fn stars(&mut self, live: Option<f64>) {
        let (cols, rows) = (self.canvas.width as u64, self.canvas.height as u64 / 2);
        let styles = [
            self.palette.muted(),
            Style::new().fg(self.palette.fg),
            Style::new()
                .fg(self.palette.fg)
                .add_modifier(Modifier::BOLD),
        ];
        for i in 0..(cols * rows / 16).max(3) {
            let h = hash(i ^ 0x57a2);
            let glow = match live {
                Some(t) => (t * TAU / (2.0 + (h >> 40) as f64 % 3.0) + (h >> 48) as f64).sin(),
                None => (h >> 24) as f64 % 2.0 - 1.0,
            };
            let (symbol, style) = match glow {
                0.7.. => ("+", styles[2]),
                -0.3.. => ("·", styles[1]),
                _ => ("·", styles[0]),
            };
            self.put(
                (h % cols) as usize,
                ((h >> 16) % rows) as usize,
                symbol,
                style,
            );
        }
    }

    /// Whatever falls from the clouds, `ms` milliseconds in.
    fn precipitation(&mut self, zone: &Zone, fall: Fall, amount: u8, ms: u64, live: Option<f64>) {
        let color = |rgb| Style::new().fg(self.palette.sky.color(rgb));
        let (rain, drizzle, ice) = (
            color(self.tones.rain),
            color(self.tones.drizzle),
            color(self.tones.ice),
        );
        let snow = Style::new().fg(self.palette.fg);
        let small = self.palette.muted();
        let amount = u64::from(amount);
        let count = |per: usize| (zone.cols.len() * zone.rows * amount as usize / per).max(2);
        let short = Drops {
            len: 1,
            speed: 1,
            share: 2 + amount,
        };
        match fall {
            Fall::Drizzle => self.drops(zone, ms / 125, short, drizzle),
            Fall::Rain => {
                let heavy = amount / 3;
                let drops = Drops {
                    len: 2 + heavy,
                    speed: 1 + heavy,
                    share: (2 + 2 * amount).min(6),
                };
                self.drops(zone, ms / 100, drops, rain);
            }
            Fall::Sleet => {
                self.drops(zone, ms / 100, short, rain);
                let flurry = Flurry {
                    count: count(10),
                    pace: 1,
                    sway: None,
                    looks: &[("•", ice), ("·", ice)],
                };
                self.flurry(zone, ms / 100, flurry);
            }
            Fall::Snow => {
                let flurry = Flurry {
                    count: count(5),
                    pace: 2,
                    sway: live,
                    looks: &[("*", snow), ("•", snow), ("·", small)],
                };
                self.flurry(zone, ms / 125, flurry);
            }
            Fall::Hail => {
                self.drops(zone, ms / 100, short, rain);
                let flurry = Flurry {
                    count: count(12),
                    pace: 1,
                    sway: None,
                    looks: &[("•", snow), ("∙", ice)],
                };
                self.flurry(zone, ms / 100, flurry);
            }
        }
    }

    /// Rain in thin streaks: `╷` `│` `╵` put a drop on either half of a
    /// cell, so it falls half a row at a time.
    fn drops(&mut self, zone: &Zone, frame: u64, drops: Drops, style: Style) {
        let span = 2 * zone.rows as u64;
        for col in zone.cols.clone() {
            let h = hash(col as u64 ^ 0xd409);
            if h % 6 >= drops.share {
                continue;
            }
            let cycle = span + drops.len + 1 + (h >> 8) % 3;
            let head = (frame * drops.speed + (h >> 16)) % cycle;
            let wet = |half: u64| half <= head && head < half + drops.len;
            for row in 0..zone.rows {
                let symbol = match (wet(2 * row as u64), wet(2 * row as u64 + 1)) {
                    (true, true) => "│",
                    (true, false) => "╵",
                    (false, true) => "╷",
                    (false, false) => continue,
                };
                self.put(col, zone.top + row, symbol, style);
            }
        }
    }

    /// Snow, ice or hail, one glyph each, falling a row at a time.
    fn flurry(&mut self, zone: &Zone, frame: u64, flurry: Flurry) {
        let (width, rows) = (zone.cols.len() as u64, zone.rows as u64);
        if width == 0 || rows == 0 {
            return;
        }
        for i in 0..flurry.count as u64 {
            let h = hash(i.wrapping_mul(0x2545_f491) ^ 0x5a0e);
            let pace = flurry.pace + (h >> 60) % 2;
            let cycle = rows + 1 + (h >> 20) % 3;
            let row = (frame / pace + (h >> 8)) % cycle;
            let sway = flurry.sway.map_or(0.0, |t| {
                1.2 * (t * TAU / (2.0 + (h >> 40) as f64 % 3.0) + (h >> 44) as f64).sin()
            });
            let col = (zone.cols.start as u64 + h % width) as f64 + sway.round();
            if row >= rows || col < zone.cols.start as f64 || col >= zone.cols.end as f64 {
                continue;
            }
            let (symbol, style) = flurry.looks[(h >> 32) as usize % flurry.looks.len()];
            self.put(col as usize, zone.top + row as usize, symbol, style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::THEMES;

    /// Tokyo Night, which blends, and the terminal's colors, which do not.
    fn palettes() -> [Palette; 2] {
        [THEMES[1].palette(true), THEMES[0].palette(true)]
    }

    fn at(ms: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_millis(1_789_000_000_000 + ms).unwrap()
    }

    fn shot(palette: &Palette, scene: Scene, rows: u16, now: DateTime<Utc>) -> Buffer {
        let mut buf = Buffer::empty(Rect::new(0, 0, width(rows) + 2, rows + 2));
        draw(
            &mut buf,
            Rect::new(1, 1, width(rows), rows),
            palette,
            scene,
            now,
        );
        buf
    }

    fn scene(code: u8, night: bool, live: bool) -> Scene {
        Scene::new(code, night, false, live).unwrap()
    }

    fn painted(buf: &Buffer) -> usize {
        buf.content()
            .iter()
            .filter(|cell| cell.symbol() != " ")
            .count()
    }

    #[test]
    fn maps_wmo_codes_to_skies() {
        let fall = |code| sky(code).unwrap().fall;
        assert_eq!(sky(0).unwrap().cover, Cover::Clear);
        assert_eq!(sky(2).unwrap().body, Body::Behind);
        assert_eq!(sky(3).unwrap().body, Body::Hidden);
        assert!(sky(45).unwrap().fog);
        assert_eq!(fall(51), Some((Fall::Drizzle, 1)));
        assert_eq!(fall(55), Some((Fall::Drizzle, 3)));
        assert_eq!(fall(63), Some((Fall::Rain, 2)));
        assert_eq!(fall(67), Some((Fall::Sleet, 3)));
        assert_eq!(fall(75), Some((Fall::Snow, 3)));
        assert_eq!(fall(82), Some((Fall::Rain, 3)));
        assert_eq!(
            sky(82).unwrap().body,
            Body::Behind,
            "showers let the sun through"
        );
        assert!(sky(95).unwrap().storm);
        assert_eq!(fall(99), Some((Fall::Hail, 3)));
        assert_eq!(sky(4), None);
        assert_eq!(Scene::new(200, false, false, true), None);
    }

    #[test]
    fn follows_the_moon_phases() {
        use chrono::NaiveDate;
        let at = |y, mo, d, h, min| {
            NaiveDate::from_ymd_opt(y, mo, d)
                .unwrap()
                .and_hms_opt(h, min, 0)
                .unwrap()
                .and_utc()
        };
        assert!(moon_phase(at(2000, 1, 6, 18, 14)) < 0.02, "new moon");
        let full = moon_phase(at(2000, 1, 21, 5, 0));
        assert!((full - 0.5).abs() < 0.03, "full moon");
        let quarter = moon_phase(at(2000, 1, 14, 0, 0));
        assert!((quarter - 0.25).abs() < 0.03, "first quarter");
    }

    #[test]
    fn every_sky_paints_inside_its_area_at_every_size() {
        for code in (0..=99).filter(|&code| sky(code).is_some()) {
            for palette in palettes() {
                for night in [false, true] {
                    for rows in MIN_ROWS..=MAX_ROWS {
                        let buf = shot(&palette, scene(code, night, true), rows, at(0));
                        let inside = Rect::new(1, 1, width(rows), rows);
                        for (i, cell) in buf.content().iter().enumerate() {
                            let (x, y) = buf.pos_of(i);
                            if !inside.contains((x, y).into()) {
                                assert_eq!(cell.symbol(), " ", "code {code} at ({x}, {y})");
                            }
                        }
                        assert!(painted(&buf) > 0, "code {code}, {rows} rows");
                    }
                }
            }
        }
    }

    #[test]
    fn never_panics_on_odd_areas() {
        for (width, height) in [(0, 0), (1, 1), (3, 2), (60, 3), (5, 30)] {
            let mut buf = Buffer::empty(Rect::new(0, 0, width, height));
            let area = Rect::new(0, 0, width + 4, height + 4);
            for code in [0, 2, 3, 45, 65, 75, 99] {
                draw(
                    &mut buf,
                    area,
                    &palettes()[0],
                    scene(code, true, true),
                    at(0),
                );
            }
        }
    }

    #[test]
    fn live_moves_while_still_holds() {
        let palette = &palettes()[0];
        for code in [0, 2, 3, 45, 55, 65, 75, 95] {
            let frame =
                |live, ms| format!("{:?}", shot(palette, scene(code, false, live), 8, at(ms)));
            assert_ne!(frame(true, 0), frame(true, 3_300), "code {code} moves");
            assert_eq!(
                frame(false, 0),
                frame(false, 3_300),
                "code {code} holds still"
            );
            assert_eq!(
                frame(true, 700),
                frame(true, 700),
                "code {code}: time alone decides"
            );
        }
    }

    #[test]
    fn rain_falls_as_thin_streaks_under_the_cloud() {
        let buf = shot(&palettes()[0], scene(63, false, false), 8, at(0));
        let rows: Vec<u16> = buf
            .content()
            .iter()
            .enumerate()
            .filter(|(_, cell)| ["│", "╵", "╷"].contains(&cell.symbol()))
            .map(|(i, _)| buf.pos_of(i).1)
            .collect();
        assert!(!rows.is_empty(), "{buf:?}");
        assert!(rows.iter().all(|&y| y >= 5), "below the cloud: {rows:?}");
    }

    #[test]
    fn lightning_strikes_twice_every_six_seconds() {
        assert!(flash(0) && flash(250) && flash(6_050));
        assert!(!flash(150) && !flash(300) && !flash(3_000));
        let palette = &palettes()[0];
        let lit = shot(
            palette,
            scene(95, false, true),
            8,
            at(-(1_789_000_000_000 % 6_000) + 50),
        );
        let dark = shot(
            palette,
            scene(95, false, true),
            8,
            at(-(1_789_000_000_000 % 6_000) + 1_000),
        );
        let bright = |buf: &Buffer| {
            let bolt = palette.sky.color(Tones::new(&palette.sky, 0.3).bolt);
            buf.content()
                .iter()
                .any(|cell| cell.fg == bolt || cell.bg == bolt)
        };
        assert!(bright(&lit) && !bright(&dark));
        assert!(
            bright(&shot(palette, scene(95, false, false), 8, at(1_000))),
            "still storms show it"
        );
    }

    #[test]
    fn the_moon_waxes_on_the_right_and_turns_over_down_south() {
        let colors = &palettes()[0].sky;
        let tones = Tones::new(colors, 0.14);
        let lit_side = |south| {
            let mut canvas = Canvas::new(18, 8, colors);
            moon(&mut canvas, &tones, (9.0, 8.0), 6.0, 0.25, south);
            let lit = |x: usize| {
                (0..16)
                    .filter(|&y| canvas.pixel(x, y) == Some(tones.moon))
                    .count()
            };
            (
                (3..9).map(lit).sum::<usize>(),
                (9..15).map(lit).sum::<usize>(),
            )
        };
        let (left, right) = lit_side(false);
        assert!(
            right > 4 * left.max(1),
            "first quarter, north: {left} {right}"
        );
        let (left, right) = lit_side(true);
        assert!(
            left > 4 * right.max(1),
            "first quarter, south: {left} {right}"
        );
    }

    #[test]
    fn keeps_text_readable() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 20, 8));
        buf.set_string(3, 2, "12:00", Style::new());
        let area = buf.area;
        draw(
            &mut buf,
            area,
            &palettes()[0],
            scene(65, false, false),
            at(0),
        );
        assert_eq!(buf[(3, 2)].symbol(), "1");
        assert!(painted(&buf) > 20);
    }

    #[test]
    fn soft_edges_only_where_the_background_is_known() {
        let [themed, terminal] = palettes();
        let canvas = |palette: &Palette| {
            let mut canvas = Canvas::new(18, 8, &palette.sky);
            let tones = Tones::new(&palette.sky, 0.0);
            sun(&mut canvas, &tones, (9.0, 8.0), 3.4, true, 0.0);
            canvas.pixels.iter().flatten().count()
        };
        assert!(
            canvas(&themed) > canvas(&terminal),
            "the glow fades into the theme"
        );
    }

    #[test]
    fn frame_rates_follow_the_weather() {
        let rate = |code, live| Scene::new(code, false, false, live).unwrap().frame_ms();
        assert_eq!(rate(0, true), Some(250));
        assert_eq!(rate(75, true), Some(125));
        assert_eq!(rate(95, true), Some(100));
        assert_eq!(rate(95, false), None);
    }
}
