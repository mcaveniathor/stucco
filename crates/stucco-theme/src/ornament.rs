//! Parametric SVG ornaments: a lit plaster relief on the page, a pattern
//! behind the application header and the shape of the header's bottom edge.
//! Each is drawn from a few numbers, some of them taken from the theme's
//! motif seed, so seeded themes get their own.

use std::f64::consts::TAU;
use std::fmt::Write;

use crate::SeedRng;

/// A raised plaster relief on the page background, lit from above.
///
/// The relief is a faint grey shading. [`Theme::build`] checks text on the
/// page against its darkest and lightest points, and pages drop it for
/// visitors who ask for more contrast. The motif seed (see
/// [`Theme::motif_seed`]) sets the direction of the light and the layout of
/// the plaster.
///
/// [`Theme::build`]: crate::Theme::build
/// [`Theme::motif_seed`]: crate::Theme::motif_seed
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Relief {
    /// No relief (default).
    #[default]
    Flat,
    /// Broad, smooth swirls, like polished Venetian plaster.
    Venetian,
    /// Long, shallow strokes, like the sweep of a trowel.
    Trowel,
    /// Small raised islands, like a skip-trowel coat.
    Skip,
}

/// The light's elevation in degrees, and the brightness it gives a flat
/// surface (its sine).
const ELEVATION: f64 = 50.0;

impl Relief {
    /// The relief's opacity, and the darkest and lightest greys (sRGB) it
    /// draws, or `None` when there is no relief.
    pub(crate) fn extremes(self) -> Option<(f64, f64, f64)> {
        let alpha = match self {
            Relief::Flat => return None,
            Relief::Venetian => 0.05,
            Relief::Trowel => 0.045,
            Relief::Skip => 0.045,
        };
        Some((alpha, 0.1, 0.9))
    }

    /// A tiling SVG relief as a CSS `url()`, or `none`.
    pub(crate) fn image(self, motif: u32, scale: f64) -> String {
        // (turbulence type, base frequency, octaves, surface scale, tile size)
        let (kind, frequency, octaves, height, size) = match self {
            Relief::Flat => return "none".into(),
            Relief::Venetian => ("fractalNoise", "0.008", 4, 5, 400),
            Relief::Trowel => ("fractalNoise", "0.003 0.02", 3, 6, 400),
            Relief::Skip => ("turbulence", "0.035", 2, 3, 300),
        };
        let mut rng = SeedRng::new(u64::from(motif)).fork("relief");
        // Light from anywhere between the left and the top.
        let azimuth = rng.range(195.0, 285.0).round();
        let noise_seed = rng.next_u64() % 1000;
        let (alpha, lo, hi) = self.extremes().expect("not flat");
        let alpha = trim(alpha * scale, 4);
        // Shade grey around mid-grey: a flat surface maps to 0.5, the most
        // brightly lit slope to `hi`, and darker slopes down to `lo`.
        let flat = ELEVATION.to_radians().sin();
        let slope = (hi - 0.5) / (1.0 - flat);
        let table: Vec<String> = (0..=10)
            .map(|i| {
                let v = (0.5 + slope * (f64::from(i) / 10.0 - flat)).clamp(lo, hi);
                trim(v, 3)
            })
            .collect();
        let table = table.join(" ");
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{size}' height='{size}'>\
             <filter id='r' x='0' y='0' width='100%' height='100%' color-interpolation-filters='sRGB'>\
             <feTurbulence type='{kind}' baseFrequency='{frequency}' numOctaves='{octaves}' \
             seed='{noise_seed}' stitchTiles='stitch'/>\
             <feDiffuseLighting surfaceScale='{height}' diffuseConstant='1' lighting-color='white'>\
             <feDistantLight azimuth='{azimuth}' elevation='{ELEVATION}'/></feDiffuseLighting>\
             <feComponentTransfer><feFuncR type='table' tableValues='{table}'/>\
             <feFuncG type='table' tableValues='{table}'/><feFuncB type='table' tableValues='{table}'/>\
             <feFuncA type='linear' slope='0' intercept='{alpha}'/></feComponentTransfer>\
             </filter><rect width='100%' height='100%' filter='url(#r)'/></svg>"
        );
        svg_url(&svg)
    }
}

/// A faint grey pattern behind the application header.
///
/// [`Theme::build`] checks the header's text against the pattern's lines,
/// and it goes for visitors who ask for more contrast. The motif seed (see
/// [`Theme::motif_seed`]) sets its spacing and layout; every seed draws
/// different contour lines.
///
/// [`Theme::build`]: crate::Theme::build
/// [`Theme::motif_seed`]: crate::Theme::motif_seed
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Pattern {
    /// No pattern (default).
    #[default]
    Plain,
    /// A grid of dots, square or staggered.
    Dots,
    /// Fine grid lines.
    Grid,
    /// Topographic contour lines.
    Contours,
    /// Rows of gentle waves.
    Waves,
}

/// The grey (sRGB) that patterns draw with.
pub(crate) const PATTERN_GREY: f64 = 128.0 / 255.0;

impl Pattern {
    /// The opacity of the pattern's lines, or 0 when there is none.
    pub(crate) fn alpha(self) -> f64 {
        match self {
            Pattern::Plain => 0.0,
            Pattern::Dots => 0.12,
            Pattern::Grid => 0.08,
            Pattern::Contours => 0.1,
            Pattern::Waves => 0.1,
        }
    }

    /// A tiling SVG pattern as a CSS `url()`, or `none`.
    pub(crate) fn image(self, motif: u32, scale: f64) -> String {
        let mut rng = SeedRng::new(u64::from(motif)).fork("pattern");
        let alpha = trim(self.alpha() * scale, 4);
        let (w, h, body) = match self {
            Pattern::Plain => return "none".into(),
            Pattern::Dots => {
                let s = rng.weighted(&[(14.0, 1), (18.0, 2), (22.0, 1)]);
                let r = 1.25;
                if rng.unit() < 0.5 {
                    // Square: a dot at each corner, which tiles to one per cell.
                    let dots = [(0.0, 0.0), (s, 0.0), (0.0, s), (s, s)];
                    (s, s, circles(&dots, r))
                } else {
                    // Staggered: corners plus one in the middle.
                    let t = (s * 1.732).round();
                    let dots = [(0.0, 0.0), (s, 0.0), (0.0, t), (s, t), (s / 2.0, t / 2.0)];
                    (s, t, circles(&dots, r))
                }
            }
            Pattern::Grid => {
                let s = rng.weighted(&[(16.0, 1), (20.0, 2), (24.0, 1)]);
                let path = format!("M0 0.5H{s}M0.5 0V{s}");
                (s, s, stroke(&path))
            }
            Pattern::Contours => {
                let size = 240.0;
                (size, size, stroke(&contours(&mut rng, size)))
            }
            Pattern::Waves => {
                let w = rng.weighted(&[(64.0, 1), (80.0, 2), (96.0, 1)]);
                let h = rng.weighted(&[(12.0, 1), (16.0, 2)]);
                let a = h * 0.3;
                let m = h / 2.0;
                let path = format!(
                    "M0 {m}Q{} {} {} {m}T{w} {m}",
                    trim(w / 4.0, 1),
                    trim(m - 2.0 * a, 1),
                    trim(w / 2.0, 1)
                );
                (w, h, stroke(&path))
            }
        };
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}' \
             fill='#808080' stroke='#808080' fill-opacity='{alpha}' stroke-opacity='{alpha}'>{body}</svg>"
        );
        svg_url(&svg)
    }
}

/// The shape of the application header's bottom edge.
///
/// Shaped edges replace the header's bottom rule, so they show on headers
/// with a fill (the Bar and Tinted header styles). The motif seed (see
/// [`Theme::motif_seed`]) sets their rhythm, and the tear of a torn edge.
///
/// [`Theme::motif_seed`]: crate::Theme::motif_seed
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HeaderEdge {
    /// A straight edge with the header's rule (default).
    #[default]
    Straight,
    /// A gentle wave.
    Wave,
    /// Sawtooth points.
    Zigzag,
    /// An irregular torn edge, like a broken plaster coat.
    Torn,
}

impl HeaderEdge {
    /// `(mask, extra bottom padding, rule width)` for the header.
    pub(crate) fn tokens(self, motif: u32) -> (String, &'static str, &'static str) {
        let mut rng = SeedRng::new(u64::from(motif)).fork("edge");
        let (w, h, path) = match self {
            HeaderEdge::Straight => return ("none".into(), "0px", "var(--st-line)"),
            HeaderEdge::Wave => {
                let w = rng.weighted(&[(48.0, 1), (64.0, 2), (80.0, 1)]);
                let h = 10.0;
                let m = h / 2.0;
                let path = format!(
                    "M0 0H{w}V{m}Q{} {h} {} {m}T0 {m}Z",
                    trim(w * 0.75, 1),
                    trim(w / 2.0, 1)
                );
                (w, h, path)
            }
            HeaderEdge::Zigzag => {
                let w = rng.weighted(&[(14.0, 1), (18.0, 2), (24.0, 1)]);
                let h = 8.0;
                let path = format!("M0 0H{w}V1L{} {h}L0 1Z", trim(w / 2.0, 1));
                (w, h, path)
            }
            HeaderEdge::Torn => {
                let (w, h) = (180.0, 9.0);
                let start = trim(h * 0.5, 1);
                // Back from the right edge to the left, at random depths.
                let mut path = format!("M0 0H{w}V{start}");
                let mut x = w;
                loop {
                    x -= rng.range(3.0, 9.0);
                    if x <= 3.0 {
                        break;
                    }
                    let y = rng.range(h * 0.15, h);
                    let _ = write!(path, "L{} {}", trim(x, 1), trim(y, 1));
                }
                let _ = write!(path, "L0 {start}Z");
                (w, h, path)
            }
        };
        let svg = format!(
            "<svg xmlns='http://www.w3.org/2000/svg' width='{w}' height='{h}'><path d='{path}'/></svg>"
        );
        let mask = format!(
            "linear-gradient(black, black) top / 100% calc(100% - {h}px) no-repeat, \
             {} bottom left / {w}px {h}px repeat-x",
            svg_url(&svg)
        );
        let pad = match self {
            HeaderEdge::Zigzag => "8px",
            HeaderEdge::Torn => "9px",
            _ => "10px",
        };
        (mask, pad, "0px")
    }
}

/// Contour lines of a random field that tiles a `size` square: a sum of a
/// few waves with whole-number frequencies, traced by marching squares.
fn contours(rng: &mut SeedRng, size: f64) -> String {
    const N: usize = 40;
    const LEVELS: usize = 7;
    let waves: Vec<(f64, f64, f64, f64)> = (0..5)
        .map(|_| {
            let kx = rng.weighted(&[(-2.0, 1), (-1.0, 2), (0.0, 1), (1.0, 2), (2.0, 1)]);
            let mut ky = rng.weighted(&[(0.0, 1), (1.0, 2), (2.0, 1)]);
            if kx == 0.0 && ky == 0.0 {
                ky = 1.0;
            }
            let amp = rng.range(0.5, 1.0) / f64::hypot(kx, ky);
            (kx, ky, amp, rng.range(0.0, TAU))
        })
        .collect();
    let step = size / N as f64;
    let field = |i: usize, j: usize| -> f64 {
        let (x, y) = (i as f64 / N as f64, j as f64 / N as f64);
        waves
            .iter()
            .map(|(kx, ky, a, p)| a * (TAU * (kx * x + ky * y) + p).sin())
            .sum()
    };
    let v: Vec<Vec<f64>> = (0..=N)
        .map(|i| (0..=N).map(|j| field(i, j)).collect())
        .collect();
    let (min, max) = v
        .iter()
        .flatten()
        .fold((f64::MAX, f64::MIN), |(lo, hi), &x| (lo.min(x), hi.max(x)));
    let mut path = String::new();
    for l in 0..LEVELS {
        let level = min + (max - min) * (l as f64 + 0.5) / LEVELS as f64;
        // Each crossing lies on one grid edge: (vertical?, i, j).
        type Edge = (bool, usize, usize);
        let point = |e: Edge| -> (f64, f64) {
            let (vert, i, j) = e;
            let (a, b, (x0, y0), (x1, y1)) = if vert {
                (v[i][j], v[i][j + 1], (i, j), (i, j + 1))
            } else {
                (v[i][j], v[i + 1][j], (i, j), (i + 1, j))
            };
            let t = (level - a) / (b - a);
            let lerp = |p: usize, q: usize| (p as f64 + t * (q as f64 - p as f64)) * step;
            (lerp(x0, x1), lerp(y0, y1))
        };
        let mut segments: Vec<(Edge, Edge)> = Vec::new();
        for i in 0..N {
            for j in 0..N {
                // Corners: top-left, top-right, bottom-right, bottom-left.
                let c = [v[i][j], v[i + 1][j], v[i + 1][j + 1], v[i][j + 1]];
                let case = c
                    .iter()
                    .enumerate()
                    .fold(0, |acc, (k, &x)| acc | (usize::from(x > level) << k));
                let (top, right, bottom, left) = (
                    (false, i, j),
                    (true, i + 1, j),
                    (false, i, j + 1),
                    (true, i, j),
                );
                let centre_high = c.iter().sum::<f64>() / 4.0 > level;
                let pairs: &[(Edge, Edge)] = &match case {
                    0 | 15 => vec![],
                    1 | 14 => vec![(left, top)],
                    2 | 13 => vec![(top, right)],
                    3 | 12 => vec![(left, right)],
                    4 | 11 => vec![(right, bottom)],
                    6 | 9 => vec![(top, bottom)],
                    7 | 8 => vec![(left, bottom)],
                    5 if centre_high => vec![(left, bottom), (top, right)],
                    5 => vec![(left, top), (right, bottom)],
                    10 if centre_high => vec![(left, top), (right, bottom)],
                    _ => vec![(left, bottom), (top, right)],
                };
                segments.extend_from_slice(pairs);
            }
        }
        // Chain segments that share a crossing into polylines.
        let mut by_edge: std::collections::HashMap<Edge, Vec<usize>> = Default::default();
        for (n, (a, b)) in segments.iter().enumerate() {
            by_edge.entry(*a).or_default().push(n);
            by_edge.entry(*b).or_default().push(n);
        }
        let mut used = vec![false; segments.len()];
        let next = |e: Edge, used: &[bool]| -> Option<usize> {
            by_edge[&e].iter().copied().find(|&n| !used[n])
        };
        for start in 0..segments.len() {
            if used[start] {
                continue;
            }
            // Walk back to an end of the line, then forward along it.
            let (mut tail, mut head) = segments[start];
            used[start] = true;
            let mut line = std::collections::VecDeque::from([tail, head]);
            while let Some(n) = next(head, &used) {
                used[n] = true;
                let (a, b) = segments[n];
                head = if a == head { b } else { a };
                line.push_back(head);
            }
            while let Some(n) = next(tail, &used) {
                used[n] = true;
                let (a, b) = segments[n];
                tail = if a == tail { b } else { a };
                line.push_front(tail);
            }
            for (k, e) in line.into_iter().enumerate() {
                let (x, y) = point(e);
                let _ = write!(
                    path,
                    "{}{} {}",
                    if k == 0 { 'M' } else { 'L' },
                    trim(x, 1),
                    trim(y, 1)
                );
            }
        }
    }
    path
}

fn circles(dots: &[(f64, f64)], r: f64) -> String {
    dots.iter()
        .map(|(x, y)| {
            format!(
                "<circle cx='{}' cy='{}' r='{r}' stroke='none'/>",
                trim(*x, 1),
                trim(*y, 1)
            )
        })
        .collect()
}

fn stroke(path: &str) -> String {
    format!("<path d='{path}' fill='none' stroke-width='1'/>")
}

/// `x` with at most `places` decimals and no trailing zeros.
fn trim(x: f64, places: usize) -> String {
    let s = format!("{x:.places$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_owned()
    } else {
        s
    }
}

/// `svg` as a CSS `url()` data URI.
fn svg_url(svg: &str) -> String {
    let mut out = String::from("url(\"data:image/svg+xml,");
    for c in svg.chars() {
        match c {
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '#' => out.push_str("%23"),
            '%' => out.push_str("%25"),
            '"' => out.push_str("%22"),
            c => out.push(c),
        }
    }
    out.push_str("\")");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ornaments_are_svg_urls_that_vary_with_the_motif() {
        for relief in [Relief::Venetian, Relief::Trowel, Relief::Skip] {
            let a = relief.image(1, 1.0);
            assert!(a.starts_with("url(\"data:image/svg+xml,%3Csvg"));
            assert!(!a.contains(['<', '>', '#']));
            assert_ne!(a, relief.image(2, 1.0));
        }
        for pattern in [
            Pattern::Dots,
            Pattern::Grid,
            Pattern::Contours,
            Pattern::Waves,
        ] {
            let a = pattern.image(7, 1.0);
            assert!(a.ends_with("%3C/svg%3E\")"));
            assert!(!a.contains(['<', '>', '#']));
        }
        assert_ne!(
            Pattern::Contours.image(1, 1.0),
            Pattern::Contours.image(2, 1.0)
        );
        assert_ne!(HeaderEdge::Torn.tokens(1).0, HeaderEdge::Torn.tokens(2).0);
        assert_eq!(Relief::Flat.image(3, 1.0), "none");
        assert_eq!(HeaderEdge::Straight.tokens(3).0, "none");
    }

    #[test]
    fn contours_stay_small() {
        for motif in 0..50 {
            let len = Pattern::Contours.image(motif, 1.0).len();
            assert!(len < 24_000, "motif {motif}: {len} bytes");
        }
    }

    #[test]
    fn the_relief_shades_a_flat_surface_mid_grey() {
        let flat = ELEVATION.to_radians().sin();
        let slope = 0.5 / (1.0 - flat);
        assert!((0.5 + slope * (1.0 - flat) - 1.0).abs() < 1e-9);
    }
}
