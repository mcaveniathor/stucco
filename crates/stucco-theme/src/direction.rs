//! Coordinated recipes and bounded, versioned variations.
use crate::*;

/// A complete visual direction. Existing presets and seed mappings are unchanged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Warm plaster, serif display typography and terracotta.
    Atelier,
    /// Compact, cool and sharply aligned.
    Precision,
    /// Luminous accents, selective frost and layered depth.
    Prism,
    /// Heavy outlines, bold type and offset shadows.
    Play,
}

impl Direction {
    /// Every signature direction, in display order.
    pub const ALL: [Self; 4] = [Self::Atelier, Self::Precision, Self::Prism, Self::Play];

    /// Stable identifier for URLs and named scopes.
    pub fn name(self) -> &'static str {
        match self {
            Self::Atelier => "atelier",
            Self::Precision => "precision",
            Self::Prism => "prism",
            Self::Play => "play",
        }
    }

    /// The composed signature, ready for ordinary builder overrides.
    pub fn theme(self) -> Theme {
        match self {
            Self::Atelier => Theme::preset(Preset::Sand)
                .fonts(Fonts::system())
                .heading_font(HeadingFont::Serif)
                .heading_weight(HeadingWeight::Regular)
                .finish(Finish::Sand)
                .header_style(HeaderStyle::Plain)
                .shell_layout(ShellLayout::Topbar)
                .nav_style(NavStyle::Bar)
                .panel_style(PanelStyle::Ruled)
                .radius(Radius::Soft)
                .button_depth(ButtonDepth::Raised)
                .motion(Motion::Gentle),
            Self::Precision => Theme::preset(Preset::Slate)
                .density(Density::Compact)
                .radius(Radius::Sharp)
                .heading_font(HeadingFont::Body)
                .heading_weight(HeadingWeight::Bold)
                .label_style(LabelStyle::Caps)
                .nav_style(NavStyle::Bar)
                .panel_style(PanelStyle::Ruled)
                .table_style(TableStyle::Lined)
                .button_depth(ButtonDepth::Flat)
                .motion(Motion::Snappy),
            Self::Prism => Theme::preset(Preset::Iris)
                .radius(Radius::Round)
                .elevation(Elevation::Raised)
                .heading_weight(HeadingWeight::Bold)
                .header_style(HeaderStyle::Tinted)
                .material(Material::Frost)
                .backdrop(Backdrop::Glow)
                .shadow_style(ShadowStyle::Layered)
                .button_depth(ButtonDepth::Raised)
                .nav_style(NavStyle::Soft)
                .motion(Motion::Smooth),
            Self::Play => Theme::preset(Preset::Sol)
                .heading_font(HeadingFont::Rounded)
                .heading_weight(HeadingWeight::Heavy)
                .line_weight(LineWeight::Heavy)
                .button_depth(ButtonDepth::Offset)
                .shadow_style(ShadowStyle::Hard)
                .radius(Radius::Soft)
                .tag_style(TagStyle::Square)
                .nav_style(NavStyle::Solid)
                .focus_style(FocusStyle::Thick)
                .motion(Motion::Springy),
        }
    }

    /// Version 1 guided generation. Seed zero is the exact signature.
    ///
    /// Only compatible palettes, heading treatments and surface shapes vary.
    /// The mapping is frozen independently of [`Theme::seeded`]. Persist the
    /// direction, version, seed and any builder overrides to reproduce it.
    pub fn guided_v1(self, seed: u64) -> Theme {
        let theme = self.theme();
        if seed == 0 {
            return theme;
        }
        let rng = SeedRng::new(seed);
        let palettes = match self {
            Self::Atelier => [Preset::Sand, Preset::Sepia, Preset::Stone],
            Self::Precision => [Preset::Slate, Preset::Fjord, Preset::Graphite],
            Self::Prism => [Preset::Iris, Preset::Ocean, Preset::Rose],
            Self::Play => [Preset::Sol, Preset::Rose, Preset::Pine],
        };
        let palette = palettes[(rng.fork("palette").next_u64() % 3) as usize];
        let alternate_type = rng.fork("type").next_u64() % 2 == 1;
        let alternate_shape = rng.fork("shape").next_u64() % 2 == 1;
        let theme = theme.palette_preset(palette);
        let theme = if alternate_type {
            match self {
                Self::Atelier => theme
                    .heading_weight(HeadingWeight::Bold)
                    .leading(Leading::Airy),
                Self::Precision => theme.heading_font(HeadingFont::Mono),
                Self::Prism => theme.heading_weight(HeadingWeight::Heavy),
                Self::Play => theme.heading_font(HeadingFont::Body),
            }
        } else {
            theme
        };
        if alternate_shape {
            match self {
                Self::Atelier => theme.radius(Radius::Sharp),
                Self::Precision => theme.radius(Radius::Soft),
                Self::Prism => theme.radius(Radius::Soft).shadow_style(ShadowStyle::Tinted),
                Self::Play => theme.radius(Radius::Round).button_shape(ButtonShape::Pill),
            }
        } else {
            theme
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{Base, ThemeSpec};

    #[test]
    fn signatures_and_bounded_variations_build_and_round_trip() {
        for direction in Direction::ALL {
            assert_eq!(direction.guided_v1(0), direction.theme());
            for seed in (0..128).chain([u64::MAX]) {
                let spec = ThemeSpec::new(Base::GuidedV1(direction, seed));
                let parsed: ThemeSpec = spec.to_query().parse().unwrap();
                assert_eq!(parsed, spec);
                let built = parsed.theme().build().unwrap();
                assert_eq!(
                    built.css(Scope::Root),
                    direction.guided_v1(seed).build().unwrap().css(Scope::Root)
                );
                assert!(spec.export_json(&built).contains(&spec.to_query()));
            }
        }
    }

    #[test]
    fn v1_choices_are_stable_and_locks_override_the_generated_base() {
        let original: ThemeSpec = "guided=atelier:1:42".parse().unwrap();
        let summary = original.summary();
        assert!(summary.contains(&("palette", "stone")));
        assert!(summary.contains(&("heading-font", "serif")));
        assert!(summary.contains(&("radius", "soft")));
        let locked = ThemeSpec::new(Base::GuidedV1(Direction::Atelier, 9))
            .with("palette", "stone")
            .unwrap()
            .with("heading-font", "serif")
            .unwrap();
        assert!(locked.summary().contains(&("palette", "stone")));
        assert!(locked.summary().contains(&("heading-font", "serif")));
        assert!(locked.rust().contains("Direction::Atelier.guided_v1(9)"));
        assert!(locked.rust().contains(".palette_preset(Preset::Stone)"));
        for invalid in [
            "guided=atelier:2:42",
            "guided=atelier:1:-1",
            "guided=nope:1:0",
            "direction=nope",
            "direction=play&seed=1",
        ] {
            assert!(invalid.parse::<ThemeSpec>().is_err(), "{invalid}");
        }
    }
}
