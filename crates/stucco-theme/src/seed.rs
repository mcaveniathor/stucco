//! Deterministic generation from seeds: the [`Seeded`] trait, its random
//! stream [`SeedRng`], and implementations for every theme option.

use crate::personality::{
    ButtonShape, ControlStyle, Elevation, HeaderStyle, HeadingWeight, TableStyle,
};
use crate::{Color, Density, Fonts, Radius, Theme, TypeScale};

/// A deterministic random stream (SplitMix64), the same on every platform.
///
/// Implement [`Seeded`] for your own types by drawing from it; use
/// [`SeedRng::fork`] to give independent parts their own streams.
///
/// ```
/// use stucco_theme::SeedRng;
/// let mut a = SeedRng::new(7);
/// let mut b = SeedRng::new(7);
/// assert_eq!(a.next_u64(), b.next_u64());
/// assert!((0.0..1.0).contains(&a.unit()));
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeedRng {
    state: u64,
}

impl SeedRng {
    /// A stream starting from `seed`.
    pub fn new(seed: u64) -> SeedRng {
        SeedRng { state: seed }
    }

    /// A stream whose seed is hashed from `name` (FNV-1a), so names make
    /// stable seeds.
    pub fn from_name(name: &str) -> SeedRng {
        SeedRng::new(fnv1a(name))
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        mix(self.state)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[lo, hi)`.
    pub fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + self.unit() * (hi - lo)
    }

    /// One of `items`, each chosen in proportion to its weight. Panics if
    /// `items` is empty or every weight is zero.
    pub fn weighted<T: Copy>(&mut self, items: &[(T, u32)]) -> T {
        let total: u64 = items.iter().map(|(_, w)| u64::from(*w)).sum();
        assert!(total > 0, "weighted needs at least one non-zero weight");
        let mut pick = self.next_u64() % total;
        for (item, w) in items {
            if pick < u64::from(*w) {
                return *item;
            }
            pick -= u64::from(*w);
        }
        unreachable!("pick is below the total weight")
    }

    /// An independent stream for the part named `label`. Forking depends
    /// only on this stream's current position and the label, not on what
    /// other forks draw, so adding a part never changes the others.
    pub fn fork(&self, label: &str) -> SeedRng {
        SeedRng::new(mix(self.state ^ fnv1a(label)))
    }

    /// [`SeedRng::range`] rounded to `places` decimals, so generated CSS
    /// stays short.
    fn rounded(&mut self, lo: f64, hi: f64, places: i32) -> f64 {
        let f = 10f64.powi(places);
        (self.range(lo, hi) * f).round() / f
    }
}

/// The SplitMix64 output function.
fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// FNV-1a, for turning names into seeds.
pub(crate) fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A value that can be derived deterministically from a seed.
///
/// Every theme option implements it, and so does [`Theme`]. Each option
/// draws from its own named stream ([`Seeded::STREAM`]), and
/// `Theme::seeded(s)` forks the same streams, so seeding one option gives
/// the value a fully seeded theme would choose for it. Mix and match freely:
///
/// ```
/// use stucco_theme::{Elevation, Fonts, Preset, Radius, Seeded, Theme};
///
/// // Slate's colours with seed 42's corners, fonts and elevation.
/// let theme = Theme::preset(Preset::Slate)
///     .radius(Radius::seeded(42))
///     .fonts(Fonts::seeded(42))
///     .elevation(Elevation::seeded_str("acme"));
/// assert!(theme.build().is_ok());
/// ```
///
/// The mapping from seed to value stays fixed across patch releases; a
/// minor release may change it.
pub trait Seeded: Sized {
    /// The name of this option's stream within a seeded theme.
    const STREAM: &'static str;

    /// Draws a value from `rng`.
    fn from_rng(rng: &mut SeedRng) -> Self;

    /// The value for `seed`, from this option's own stream.
    fn seeded(seed: u64) -> Self {
        Self::from_rng(&mut SeedRng::new(seed).fork(Self::STREAM))
    }

    /// [`Seeded::seeded`] with a seed hashed from `name`.
    fn seeded_str(name: &str) -> Self {
        Self::from_rng(&mut SeedRng::from_name(name).fork(Self::STREAM))
    }
}

/// A [`Seeded`] value from a fresh random seed. Implemented for every
/// `Seeded` type, including [`Theme`].
///
/// The seed comes from the standard library's per-process random hasher
/// keys mixed with the clock: fine for variety, not for cryptography. Keep
/// the seed from [`Random::random_with_seed`] to recreate a result you like
/// with [`Seeded::seeded`].
///
/// ```
/// use stucco_theme::{Random, Seeded, Theme};
///
/// let (theme, seed) = Theme::random_with_seed();
/// assert_eq!(theme, Theme::seeded(seed));
/// assert!(theme.build().is_ok());
/// ```
pub trait Random: Seeded {
    /// A value from a fresh random seed.
    fn random() -> Self {
        Self::random_with_seed().0
    }

    /// A value from a fresh random seed, and that seed.
    fn random_with_seed() -> (Self, u64) {
        let seed = random_seed();
        (Self::seeded(seed), seed)
    }
}

impl<T: Seeded> Random for T {}

/// A fresh, non-cryptographic random seed.
///
/// ```
/// assert_ne!(stucco_theme::random_seed(), stucco_theme::random_seed());
/// ```
pub fn random_seed() -> u64 {
    use std::hash::BuildHasher;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    // A counter keeps calls distinct even when the clock does not move.
    static CALLS: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let calls = CALLS.fetch_add(1, Ordering::Relaxed);
    std::collections::hash_map::RandomState::new().hash_one((nanos, calls))
}

/// A theme's colour choices: the accent, the neutral scale's hue and tint,
/// and whether the accent is the text colour ("ink").
///
/// ```
/// use stucco_theme::{Color, Palette, Seeded, Theme};
/// let theme = Theme::from_seed(0.0)
///     .palette(Palette::new(Color::oklch(0.6, 0.15, 160.0)).neutral(150.0, 0.012));
/// assert!(theme.build().is_ok());
/// assert!(Theme::from_seed(0.0).palette(Palette::seeded(9)).build().is_ok());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub(crate) accent: Color,
    pub(crate) neutral_hue: Option<f64>,
    pub(crate) neutral_tint: f64,
    pub(crate) ink: bool,
}

impl Palette {
    /// A palette with `accent` (its lightness is ignored) and neutrals
    /// tinted slightly towards the accent hue.
    pub fn new(accent: Color) -> Palette {
        Palette {
            accent,
            neutral_hue: None,
            neutral_tint: 0.01,
            ink: false,
        }
    }

    /// The neutral scale's hue and chroma (0 is pure grey).
    pub fn neutral(mut self, hue: f64, tint: f64) -> Palette {
        self.neutral_hue = Some(hue);
        self.neutral_tint = tint;
        self
    }

    /// Uses the text colour as the accent, for monochrome themes.
    pub fn ink(mut self) -> Palette {
        self.ink = true;
        self
    }
}

impl Seeded for Palette {
    const STREAM: &'static str = "palette";

    /// A palette that passes every contrast check of a default-contrast
    /// theme, whatever its other options: candidates that fail are skipped.
    fn from_rng(rng: &mut SeedRng) -> Palette {
        for _ in 0..32 {
            let hue = rng.rounded(0.0, 360.0, 1);
            let chroma = rng.rounded(0.07, 0.19, 3);
            let neutral_hue = match rng.weighted(&[(0u8, 5), (1, 3), (2, 2)]) {
                0 => hue,
                1 => (hue + rng.rounded(-40.0, 40.0, 1)).rem_euclid(360.0),
                _ => rng.rounded(0.0, 360.0, 1),
            };
            let tint = rng.rounded(0.0, 0.03, 3);
            let mut palette =
                Palette::new(Color::oklch(0.6, chroma, hue)).neutral(neutral_hue, tint);
            if rng.weighted(&[(false, 11), (true, 1)]) {
                palette = palette.ink();
            }
            if Theme::from_seed(0.0).palette(palette).build().is_ok() {
                return palette;
            }
        }
        // Not reached for any seed the tests cover; a safe fallback anyway.
        Palette::new(Color::oklch(0.6, 0.15, 250.0))
    }
}

/// System-only body stacks (no web fonts are downloaded).
const SANS_STACKS: &[(&str, u32)] = &[
    (
        r#"system-ui, -apple-system, "Segoe UI", Roboto, sans-serif"#,
        5,
    ),
    (
        r#"Seravek, "Gill Sans Nova", Ubuntu, Calibri, "DejaVu Sans", source-sans-pro, sans-serif"#,
        3,
    ),
    (
        r#"Avenir, Montserrat, Corbel, "URW Gothic", source-sans-pro, sans-serif"#,
        2,
    ),
    (
        r#""Helvetica Neue", "Arial Nova", "Nimbus Sans", Arial, sans-serif"#,
        2,
    ),
    (
        r#"ui-rounded, "Hiragino Maru Gothic ProN", Quicksand, Comfortaa, Manjari, "Arial Rounded MT", Calibri, source-sans-pro, sans-serif"#,
        1,
    ),
    (
        r#"Charter, "Bitstream Charter", "Sitka Text", Cambria, serif"#,
        2,
    ),
    (
        r#""Iowan Old Style", "Palatino Linotype", "URW Palladio L", P052, serif"#,
        1,
    ),
    (
        "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
        1,
    ),
];

impl Seeded for Fonts {
    const STREAM: &'static str = "fonts";

    /// A system body stack (sans, serif, rounded or monospace); code keeps
    /// the system monospace stack.
    fn from_rng(rng: &mut SeedRng) -> Fonts {
        Fonts::system().sans_stack(rng.weighted(SANS_STACKS))
    }
}

impl Seeded for TypeScale {
    const STREAM: &'static str = "type-scale";

    fn from_rng(rng: &mut SeedRng) -> TypeScale {
        let base_px = rng.weighted(&[(15.0, 1), (16.0, 3), (17.0, 1)]);
        let ratio = rng.weighted(&[(1.125, 2), (1.2, 3), (1.25, 2), (1.333, 1)]);
        TypeScale::new(base_px, ratio)
    }
}

/// Implements [`Seeded`] for an option enum by weighted choice.
macro_rules! seeded_enum {
    ($ty:ty, $stream:literal, [$(($variant:expr, $weight:literal)),+ $(,)?]) => {
        impl Seeded for $ty {
            const STREAM: &'static str = $stream;

            fn from_rng(rng: &mut SeedRng) -> $ty {
                rng.weighted(&[$(($variant, $weight)),+])
            }
        }
    };
}

seeded_enum!(
    Radius,
    "radius",
    [(Radius::Sharp, 1), (Radius::Soft, 2), (Radius::Round, 1)]
);
seeded_enum!(
    Density,
    "density",
    [(Density::Comfortable, 3), (Density::Compact, 1)]
);
seeded_enum!(
    Elevation,
    "elevation",
    [
        (Elevation::Flat, 1),
        (Elevation::Outlined, 2),
        (Elevation::Raised, 1)
    ]
);
seeded_enum!(
    TableStyle,
    "table-style",
    [
        (TableStyle::Lined, 2),
        (TableStyle::Striped, 1),
        (TableStyle::Open, 1)
    ]
);
seeded_enum!(
    ControlStyle,
    "control-style",
    [
        (ControlStyle::Outlined, 3),
        (ControlStyle::Filled, 1),
        (ControlStyle::Underlined, 1),
    ]
);
seeded_enum!(
    HeaderStyle,
    "header-style",
    [
        (HeaderStyle::Bar, 2),
        (HeaderStyle::Plain, 1),
        (HeaderStyle::Tinted, 1)
    ]
);
seeded_enum!(
    HeadingWeight,
    "heading-weight",
    [
        (HeadingWeight::Regular, 1),
        (HeadingWeight::Bold, 2),
        (HeadingWeight::Heavy, 1),
    ]
);
seeded_enum!(
    ButtonShape,
    "button-shape",
    [(ButtonShape::Rounded, 3), (ButtonShape::Pill, 1)]
);

impl Seeded for Theme {
    const STREAM: &'static str = "theme";

    /// Every option from its own fork of `rng`: palette, fonts, type scale,
    /// spacing, radius, density and the six personality options. Pill
    /// buttons become rounded on sharp themes. The result always passes
    /// [`Theme::build`].
    fn from_rng(rng: &mut SeedRng) -> Theme {
        let radius = Radius::from_rng(&mut rng.fork(Radius::STREAM));
        let buttons = match (
            radius,
            ButtonShape::from_rng(&mut rng.fork(ButtonShape::STREAM)),
        ) {
            (Radius::Sharp, _) => ButtonShape::Rounded,
            (_, shape) => shape,
        };
        let space = rng.fork("space").weighted(&[(3.5, 1), (4.0, 3), (4.5, 1)]);
        Theme::from_seed(0.0)
            .palette(Palette::from_rng(&mut rng.fork(Palette::STREAM)))
            .fonts(Fonts::from_rng(&mut rng.fork(Fonts::STREAM)))
            .type_scale(TypeScale::from_rng(&mut rng.fork(TypeScale::STREAM)))
            .space(space)
            .radius(radius)
            .density(Density::from_rng(&mut rng.fork(Density::STREAM)))
            .elevation(Elevation::from_rng(&mut rng.fork(Elevation::STREAM)))
            .table_style(TableStyle::from_rng(&mut rng.fork(TableStyle::STREAM)))
            .control_style(ControlStyle::from_rng(&mut rng.fork(ControlStyle::STREAM)))
            .header_style(HeaderStyle::from_rng(&mut rng.fork(HeaderStyle::STREAM)))
            .heading_weight(HeadingWeight::from_rng(
                &mut rng.fork(HeadingWeight::STREAM),
            ))
            .button_shape(buttons)
    }

    /// The theme for `seed`; its options match each option's own
    /// `seeded(seed)`, except that pill buttons become rounded on sharp
    /// themes.
    fn seeded(seed: u64) -> Theme {
        Theme::from_rng(&mut SeedRng::new(seed))
    }

    fn seeded_str(name: &str) -> Theme {
        Theme::from_rng(&mut SeedRng::from_name(name))
    }
}

impl Theme {
    /// A complete theme derived from `seed` (see [`Seeded`]): colours,
    /// fonts, type scale, spacing, radius, density and personality. It always
    /// passes [`Theme::build`]'s contrast checks, and the same seed gives the
    /// same theme on every platform. Callable without importing [`Seeded`].
    ///
    /// Unlike [`Theme::from_seed`], which takes only an accent hue, this
    /// varies every option.
    ///
    /// ```
    /// use stucco_theme::Theme;
    /// let theme = Theme::seeded(42);
    /// assert_eq!(theme, Theme::seeded(42));
    /// assert!(theme.build().is_ok());
    /// ```
    pub fn seeded(seed: u64) -> Theme {
        <Theme as Seeded>::seeded(seed)
    }

    /// [`Theme::seeded`] with a seed hashed from `name`.
    ///
    /// ```
    /// use stucco_theme::Theme;
    /// assert_eq!(Theme::seeded_str("acme"), Theme::seeded_str("acme"));
    /// assert_ne!(Theme::seeded_str("acme"), Theme::seeded_str("globex"));
    /// ```
    pub fn seeded_str(name: &str) -> Theme {
        <Theme as Seeded>::seeded_str(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scheme, Scope};

    #[test]
    fn every_seed_in_a_wide_range_builds_in_both_schemes() {
        for seed in 0..2000 {
            if let Err(report) = Theme::seeded(seed).build() {
                panic!("seed {seed} fails:\n{report}");
            }
        }
    }

    #[test]
    fn seeded_palettes_never_fall_back() {
        let fallback = Palette::new(Color::oklch(0.6, 0.15, 250.0));
        for seed in 0..2000 {
            assert_ne!(Palette::seeded(seed), fallback, "seed {seed} fell back");
        }
    }

    #[test]
    fn options_seeded_alone_match_the_seeded_theme() {
        for seed in 0..200 {
            let theme = Theme::seeded(seed);
            let p = theme.personality;
            assert_eq!(theme.radius, Radius::seeded(seed));
            assert_eq!(theme.density, Density::seeded(seed));
            assert_eq!(theme.fonts, Fonts::seeded(seed));
            assert_eq!(theme.type_scale, TypeScale::seeded(seed));
            assert_eq!(p.elevation, Elevation::seeded(seed));
            assert_eq!(p.table, TableStyle::seeded(seed));
            assert_eq!(p.controls, ControlStyle::seeded(seed));
            assert_eq!(p.header, HeaderStyle::seeded(seed));
            assert_eq!(p.headings, HeadingWeight::seeded(seed));
            if theme.radius != Radius::Sharp {
                assert_eq!(p.buttons, ButtonShape::seeded(seed));
            }
            let palette = Palette::seeded(seed);
            assert_eq!(theme.accent_hue, palette.accent.h);
            assert_eq!(theme.accent_from_neutral, palette.ink);
        }
        assert_eq!(Theme::seeded_str("acme").radius, Radius::seeded_str("acme"));
    }

    #[test]
    fn seeds_are_deterministic_and_varied() {
        assert_eq!(Theme::seeded(7), Theme::seeded(7));
        let distinct: std::collections::HashSet<String> = (0..50)
            .map(|s| Theme::seeded(s).build().unwrap().css(Scope::Root))
            .collect();
        assert_eq!(distinct.len(), 50);
        let personalities: std::collections::HashSet<_> =
            (0..200).map(|s| Theme::seeded(s).personality).collect();
        assert!(personalities.len() > 40, "{}", personalities.len());
    }

    #[test]
    fn forks_are_independent_of_each_other_and_of_order() {
        let root = SeedRng::new(3);
        let mut a = root.fork("radius");
        let _ = root.fork("fonts").next_u64();
        assert_eq!(a.next_u64(), root.fork("radius").next_u64());
        assert_ne!(root.fork("radius"), root.fork("fonts"));
    }

    #[test]
    fn the_seed_to_theme_mapping_is_pinned() {
        // Changing this output changes every app's seeded theme: only do it
        // in a minor release, and update the expected values deliberately.
        let built = Theme::seeded(1).build().unwrap();
        assert_eq!(
            built.role(Scheme::Light, "accent").unwrap().css(),
            "oklch(60.00% 0.1310 147.40)"
        );
        let css = built.css(Scope::Root);
        for line in [
            r#"--st-font-sans: Seravek, "Gill Sans Nova", Ubuntu, Calibri, "DejaVu Sans", source-sans-pro, sans-serif;"#,
            "--st-space-1: 4px;",
            "--st-radius-sm: 0; --st-radius-md: 0; --st-radius-lg: 0; --st-radius-full: 9999px;",
            "--st-card-shadow: var(--st-shadow-2);",
            "--st-table-rule: var(--st-border);",
            "--st-control-bg: var(--st-surface);",
            "--st-heading-weight: 650;",
            "--st-button-radius: var(--st-radius-md);",
        ] {
            assert!(css.contains(line), "seed 1 changed; missing {line}\n{css}");
        }
    }

    #[test]
    fn random_values_are_reproducible_from_their_seed() {
        let (theme, seed) = Theme::random_with_seed();
        assert_eq!(theme, Theme::seeded(seed));
        let (radius, seed) = Radius::random_with_seed();
        assert_eq!(radius, Radius::seeded(seed));
        let seeds: std::collections::HashSet<u64> = (0..100).map(|_| random_seed()).collect();
        assert_eq!(seeds.len(), 100);
        assert!(Theme::random().build().is_ok());
    }

    #[test]
    fn names_hash_to_stable_seeds() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    #[should_panic(expected = "non-zero weight")]
    fn weighted_choice_needs_a_weight() {
        let _ = SeedRng::new(0).weighted::<u8>(&[(1, 0)]);
    }
}
