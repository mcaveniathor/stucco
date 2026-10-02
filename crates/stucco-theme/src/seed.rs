//! Deterministic theme generation from a seed.

use crate::personality::{
    ButtonShape, ControlStyle, Elevation, HeaderStyle, HeadingWeight, TableStyle,
};
use crate::{Color, Density, Fonts, Radius, Theme, TypeScale};

/// SplitMix64: small, fast and stable, so a seed means the same theme on
/// every platform.
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `[lo, hi)`, rounded to `places` decimals so generated CSS
    /// stays short.
    fn range(&mut self, lo: f64, hi: f64, places: i32) -> f64 {
        let f = 10f64.powi(places);
        ((lo + self.unit() * (hi - lo)) * f).round() / f
    }

    /// One of `items`, chosen by integer `weights`.
    fn weighted<T: Copy>(&mut self, items: &[(T, u32)]) -> T {
        let total: u32 = items.iter().map(|(_, w)| w).sum();
        let mut pick = (self.next() % u64::from(total)) as u32;
        for (item, w) in items {
            if pick < *w {
                return *item;
            }
            pick -= w;
        }
        items[items.len() - 1].0
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

/// FNV-1a, for turning names into seeds.
fn fnv1a(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    })
}

impl Theme {
    /// A complete theme derived from `seed`: accent and neutral hues, tint,
    /// fonts, type scale, spacing, radius, density and a style personality
    /// (elevation, table, control, header, heading and button styles).
    ///
    /// The result always passes [`Theme::build`]'s contrast checks, and the
    /// same seed gives the same theme on every platform. The mapping from
    /// seed to theme stays fixed across patch releases; a minor release may
    /// change it. Adjust the result with the other builder methods; changing
    /// colours can make it fail validation again.
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
        let mut rng = Rng(seed);
        for _ in 0..32 {
            let theme = candidate(&mut rng);
            if theme.clone().build().is_ok() {
                return theme;
            }
        }
        // Not reached for any seed the tests cover; a safe fallback anyway.
        Theme::from_seed(250.0)
    }

    /// [`Theme::seeded`] with a seed hashed from `name`, so an application
    /// can derive its theme from its own name.
    ///
    /// ```
    /// use stucco_theme::Theme;
    /// assert_eq!(Theme::seeded_str("acme"), Theme::seeded_str("acme"));
    /// assert_ne!(Theme::seeded_str("acme"), Theme::seeded_str("globex"));
    /// ```
    pub fn seeded_str(name: &str) -> Theme {
        Theme::seeded(fnv1a(name))
    }
}

fn candidate(rng: &mut Rng) -> Theme {
    let accent_hue = rng.range(0.0, 360.0, 1);
    let accent_chroma = rng.range(0.07, 0.19, 3);
    let neutral_hue = match rng.weighted(&[(0u8, 5), (1, 3), (2, 2)]) {
        0 => accent_hue,
        1 => (accent_hue + rng.range(-40.0, 40.0, 1)).rem_euclid(360.0),
        _ => rng.range(0.0, 360.0, 1),
    };
    let neutral_tint = rng.range(0.0, 0.03, 3);
    let radius = rng.weighted(&[(Radius::Sharp, 1), (Radius::Soft, 2), (Radius::Round, 1)]);
    let density = rng.weighted(&[(Density::Comfortable, 3), (Density::Compact, 1)]);
    let ratio = rng.weighted(&[(1.125, 2), (1.2, 3), (1.25, 2), (1.333, 1)]);
    let base_px = rng.weighted(&[(15.0, 1), (16.0, 3), (17.0, 1)]);
    let space = rng.weighted(&[(3.5, 1), (4.0, 3), (4.5, 1)]);
    let sans = rng.weighted(SANS_STACKS);
    let ink = rng.weighted(&[(false, 11), (true, 1)]);
    let buttons = if radius == Radius::Sharp {
        ButtonShape::Rounded
    } else {
        rng.weighted(&[(ButtonShape::Rounded, 3), (ButtonShape::Pill, 1)])
    };
    let mut theme = Theme::from_seed(accent_hue)
        .accent(Color::oklch(0.6, accent_chroma, accent_hue))
        .neutral_hue(neutral_hue)
        .neutral_tint(neutral_tint)
        .radius(radius)
        .density(density)
        .type_scale(TypeScale::new(base_px, ratio))
        .space(space)
        .fonts(Fonts::system().sans_stack(sans))
        .elevation(rng.weighted(&[
            (Elevation::Flat, 1),
            (Elevation::Outlined, 2),
            (Elevation::Raised, 1),
        ]))
        .table_style(rng.weighted(&[
            (TableStyle::Lined, 2),
            (TableStyle::Striped, 1),
            (TableStyle::Open, 1),
        ]))
        .control_style(rng.weighted(&[
            (ControlStyle::Outlined, 3),
            (ControlStyle::Filled, 1),
            (ControlStyle::Underlined, 1),
        ]))
        .header_style(rng.weighted(&[
            (HeaderStyle::Bar, 2),
            (HeaderStyle::Plain, 1),
            (HeaderStyle::Tinted, 1),
        ]))
        .heading_weight(rng.weighted(&[
            (HeadingWeight::Regular, 1),
            (HeadingWeight::Bold, 2),
            (HeadingWeight::Heavy, 1),
        ]))
        .button_shape(buttons);
    if ink {
        theme = theme.accent_from_neutral();
    }
    theme
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Scheme, Scope};

    #[test]
    fn every_seed_in_a_wide_range_builds_in_both_schemes() {
        for seed in 0..2000 {
            let theme = Theme::seeded(seed);
            assert!(
                theme.clone().build().is_ok(),
                "seed {seed} fell back or fails"
            );
            assert_ne!(theme, Theme::from_seed(250.0), "seed {seed} fell back");
        }
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
    fn the_seed_to_theme_mapping_is_pinned() {
        // Changing this output changes every app's seeded theme: only do it
        // in a minor release, and update the expected values deliberately.
        let built = Theme::seeded(1).build().unwrap();
        assert_eq!(
            built.role(Scheme::Light, "accent").unwrap().css(),
            "oklch(60.00% 0.1024 204.00)"
        );
        let css = built.css(Scope::Root);
        for line in [
            r#"--st-font-sans: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;"#,
            "--st-radius-sm: 4px; --st-radius-md: 8px; --st-radius-lg: 12px; --st-radius-full: 9999px;",
            "--st-table-rule: transparent;",
            "--st-control-bg: var(--st-surface);",
            "--st-heading-weight: 760;",
        ] {
            assert!(css.contains(line), "seed 1 changed; missing {line}\n{css}");
        }
    }

    #[test]
    fn names_hash_to_stable_seeds() {
        assert_eq!(fnv1a(""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a("a"), 0xaf63_dc4c_8601_ec8c);
    }
}
