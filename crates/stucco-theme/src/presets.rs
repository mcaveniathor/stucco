//! The 14 built-in presets, each validated in CI in both schemes.

use crate::{BuiltTheme, Color, Fonts, Radius, Theme};

/// A built-in theme. Unmodified presets convert to [`BuiltTheme`]
/// infallibly; modified ones go through [`Theme::build`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    /// Cool grey with a blue accent (default).
    Slate,
    /// True grey with an ink accent.
    Graphite,
    /// Warm grey with an amber accent.
    Stone,
    /// Paper beige with a terracotta accent, serif text.
    Sand,
    /// Parchment with a brown accent, serif text.
    Sepia,
    /// Green-tinted with a mint accent.
    Pine,
    /// Arctic blue-grey with a frost-blue accent.
    Fjord,
    /// Blue-teal with a cyan accent.
    Ocean,
    /// Violet-tinted with a violet accent.
    Iris,
    /// Rose-tinted with a pink accent.
    Rose,
    /// Charcoal with a red-orange accent.
    Ember,
    /// Warm cream with a gold accent.
    Sol,
    /// Near-black with a phosphor-green accent, monospace text.
    Terminal,
    /// Black and white with a strong blue accent; AAA text contrast.
    Contrast,
}

impl Preset {
    /// Every preset.
    pub const ALL: [Preset; 14] = [
        Preset::Slate,
        Preset::Graphite,
        Preset::Stone,
        Preset::Sand,
        Preset::Sepia,
        Preset::Pine,
        Preset::Fjord,
        Preset::Ocean,
        Preset::Iris,
        Preset::Rose,
        Preset::Ember,
        Preset::Sol,
        Preset::Terminal,
        Preset::Contrast,
    ];

    /// Lowercase name, used for named themes.
    pub fn name(self) -> &'static str {
        match self {
            Preset::Slate => "slate",
            Preset::Graphite => "graphite",
            Preset::Stone => "stone",
            Preset::Sand => "sand",
            Preset::Sepia => "sepia",
            Preset::Pine => "pine",
            Preset::Fjord => "fjord",
            Preset::Ocean => "ocean",
            Preset::Iris => "iris",
            Preset::Rose => "rose",
            Preset::Ember => "ember",
            Preset::Sol => "sol",
            Preset::Terminal => "terminal",
            Preset::Contrast => "contrast",
        }
    }
}

impl Theme {
    /// The definition of `preset`, ready to customise.
    pub fn preset(preset: Preset) -> Theme {
        // (accent hue, accent chroma, neutral hue, neutral tint)
        let base = |ah: f64, ac: f64, nh: f64, nt: f64| {
            Theme::from_seed(ah)
                .accent(Color::oklch(0.6, ac, ah))
                .neutral_hue(nh)
                .neutral_tint(nt)
        };
        let serif = || Fonts::system().sans_stack(r#""Iowan Old Style", Georgia, serif"#);
        match preset {
            Preset::Slate => base(250.0, 0.15, 250.0, 0.012),
            Preset::Graphite => base(0.0, 0.0, 0.0, 0.0).accent_from_neutral(),
            Preset::Stone => base(70.0, 0.14, 60.0, 0.010).radius(Radius::Round),
            Preset::Sand => base(40.0, 0.12, 75.0, 0.020).fonts(serif()),
            Preset::Sepia => base(55.0, 0.09, 70.0, 0.030).fonts(serif()),
            Preset::Pine => base(160.0, 0.11, 165.0, 0.015),
            Preset::Fjord => base(230.0, 0.08, 225.0, 0.018),
            Preset::Ocean => base(205.0, 0.13, 215.0, 0.012).radius(Radius::Sharp),
            Preset::Iris => base(290.0, 0.17, 290.0, 0.014),
            Preset::Rose => base(355.0, 0.14, 350.0, 0.012).radius(Radius::Round),
            Preset::Ember => base(35.0, 0.19, 40.0, 0.008).radius(Radius::Sharp),
            Preset::Sol => base(90.0, 0.15, 85.0, 0.020),
            Preset::Terminal => {
                let mono = Fonts::system();
                let stack = mono.mono.clone();
                base(145.0, 0.18, 145.0, 0.010)
                    .fonts(mono.sans_stack(&stack))
                    .radius(Radius::Sharp)
            }
            Preset::Contrast => base(255.0, 0.20, 0.0, 0.0)
                .high_contrast()
                .min_contrast(7.0)
                .text_lightness(0.10, 1.0),
        }
    }
}

impl From<Preset> for BuiltTheme {
    fn from(preset: Preset) -> BuiltTheme {
        Theme::preset(preset)
            .build()
            .expect("presets are validated in CI")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Radius, Scheme, Scope, contrast};

    #[test]
    fn every_preset_builds_in_both_schemes() {
        for p in Preset::ALL {
            if let Err(report) = Theme::preset(p).build() {
                panic!("{} fails:\n{report}", p.name());
            }
        }
    }

    #[test]
    fn accent_labels_stay_readable_on_hover() {
        for p in Preset::ALL {
            let built = Theme::preset(p).build().unwrap();
            for scheme in Scheme::BOTH {
                let r = contrast(
                    built.role(scheme, "on-accent").unwrap(),
                    built.role(scheme, "accent-hover").unwrap(),
                );
                assert!(r >= 4.5, "{} {scheme:?}: {r:.2}", p.name());
            }
        }
    }

    #[test]
    fn contrast_preset_meets_aaa_for_text() {
        let built = Theme::preset(Preset::Contrast).build().unwrap();
        for scheme in Scheme::BOTH {
            for (fg, bg) in [("text", "bg"), ("text-muted", "bg"), ("accent-text", "bg")] {
                let r = contrast(
                    built.role(scheme, fg).unwrap(),
                    built.role(scheme, bg).unwrap(),
                );
                assert!(r >= 7.0, "{fg} on {bg} {scheme:?}: {r}");
            }
        }
    }

    #[test]
    fn modified_presets_must_be_built() {
        let built: BuiltTheme = Theme::preset(Preset::Sand)
            .radius(Radius::Sharp)
            .build()
            .unwrap();
        assert!(built.css(Scope::Root).contains("--st-radius-md: 0;"));
    }

    #[test]
    fn names_are_lowercase_and_unique() {
        let names: std::collections::HashSet<_> = Preset::ALL.iter().map(|p| p.name()).collect();
        assert_eq!(names.len(), 14);
        assert_eq!(Preset::Terminal.name(), "terminal");
    }
}
