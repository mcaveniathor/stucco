//! The theme builder and its options.

use std::fmt;

use crate::roles::{INK_ACCENT, Kind, PAIRS, ROLES, STATUS, Source};
use crate::{Color, Scale, Scheme, contrast};

/// Font stacks for body text and code.
#[derive(Clone, Debug, PartialEq)]
pub struct Fonts {
    pub(crate) sans: String,
    pub(crate) mono: String,
}

const SYSTEM_SANS: &str = r#"system-ui, -apple-system, "Segoe UI", Roboto, sans-serif"#;
const SYSTEM_MONO: &str = "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace";

impl Fonts {
    /// The platform's own fonts.
    pub fn system() -> Fonts {
        Fonts {
            sans: SYSTEM_SANS.to_owned(),
            mono: SYSTEM_MONO.to_owned(),
        }
    }

    /// Prepends `family` (quoted) to the body stack.
    pub fn sans(mut self, family: &str) -> Fonts {
        self.sans = format!("\"{family}\", {}", self.sans);
        self
    }

    /// Prepends `family` (quoted) to the code stack.
    pub fn mono(mut self, family: &str) -> Fonts {
        self.mono = format!("\"{family}\", {}", self.mono);
        self
    }

    /// Replaces the body stack verbatim (used by presets).
    #[allow(dead_code)] // used by presets (Task 9)
    pub(crate) fn sans_stack(mut self, stack: &str) -> Fonts {
        self.sans = stack.to_owned();
        self
    }
}

/// A modular type scale: `base_px × ratio^step`, fluid between 360px and
/// 1280px viewports.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeScale {
    pub(crate) base_px: f64,
    pub(crate) ratio: f64,
}

impl TypeScale {
    /// A scale with body size `base_px` and step `ratio`.
    pub fn new(base_px: f64, ratio: f64) -> TypeScale {
        TypeScale { base_px, ratio }
    }
}

/// Corner rounding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Radius {
    /// Square corners.
    Sharp,
    /// Slightly rounded (default).
    Soft,
    /// Strongly rounded.
    Round,
}

/// Control size and padding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Density {
    /// Smaller controls for data-dense screens.
    Compact,
    /// Default control size.
    Comfortable,
}

/// A theme definition. Validation happens only in [`Theme::build`].
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub(crate) accent_hue: f64,
    pub(crate) accent_chroma: f64,
    pub(crate) neutral_hue: Option<f64>,
    pub(crate) neutral_tint: f64,
    pub(crate) accent_from_neutral: bool,
    pub(crate) text_lightness: Option<(f64, f64)>,
    pub(crate) fonts: Fonts,
    pub(crate) type_scale: TypeScale,
    pub(crate) space: f64,
    pub(crate) radius: Radius,
    pub(crate) density: Density,
    pub(crate) min_contrast: f64,
}

impl Theme {
    /// A theme whose accent has OKLCH hue `hue`.
    pub fn from_seed(hue: f64) -> Theme {
        Theme {
            accent_hue: hue,
            accent_chroma: 0.15,
            neutral_hue: None,
            neutral_tint: 0.01,
            accent_from_neutral: false,
            text_lightness: None,
            fonts: Fonts::system(),
            type_scale: TypeScale::new(16.0, 1.2),
            space: 4.0,
            radius: Radius::Soft,
            density: Density::Comfortable,
            min_contrast: 4.5,
        }
    }

    /// Chroma of the neutral scale (0 = pure grey).
    pub fn neutral_tint(mut self, chroma: f64) -> Theme {
        self.neutral_tint = chroma;
        self
    }

    /// Hue of the neutral scale (defaults to the accent hue).
    pub fn neutral_hue(mut self, hue: f64) -> Theme {
        self.neutral_hue = Some(hue);
        self
    }

    /// Sets the accent hue and chroma from `color` (its lightness is ignored;
    /// scales set lightness per step).
    pub fn accent(mut self, color: Color) -> Theme {
        self.accent_hue = color.h;
        self.accent_chroma = color.c;
        self
    }

    /// Uses the text colour as the accent ("ink" accent, for monochrome
    /// themes).
    pub fn accent_from_neutral(mut self) -> Theme {
        self.accent_from_neutral = true;
        self
    }

    /// Overrides the lightness of neutral step 12 (body text) per scheme.
    pub fn text_lightness(mut self, light: f64, dark: f64) -> Theme {
        self.text_lightness = Some((light, dark));
        self
    }

    /// Font stacks.
    pub fn fonts(mut self, fonts: Fonts) -> Theme {
        self.fonts = fonts;
        self
    }

    /// Type scale.
    pub fn type_scale(mut self, scale: TypeScale) -> Theme {
        self.type_scale = scale;
        self
    }

    /// Spacing unit in pixels.
    pub fn space(mut self, px: f64) -> Theme {
        self.space = px;
        self
    }

    /// Corner rounding.
    pub fn radius(mut self, radius: Radius) -> Theme {
        self.radius = radius;
        self
    }

    /// Control density.
    pub fn density(mut self, density: Density) -> Theme {
        self.density = density;
        self
    }

    /// Minimum contrast for text roles (default 4.5, WCAG AA).
    pub fn min_contrast(mut self, ratio: f64) -> Theme {
        self.min_contrast = ratio;
        self
    }

    /// Generates the scales and roles and checks contrast in both schemes.
    pub fn build(self) -> Result<BuiltTheme, ContrastReport> {
        let mut neutral = Scale::new(
            self.neutral_hue.unwrap_or(self.accent_hue),
            self.neutral_tint,
        );
        if let Some((light, dark)) = self.text_lightness {
            neutral.light[11].l = light.clamp(0.0, 1.0);
            neutral.dark[11].l = dark.clamp(0.0, 1.0);
        }
        let mut scales = vec![
            ("neutral", neutral),
            ("accent", Scale::new(self.accent_hue, self.accent_chroma)),
        ];
        scales.extend(
            STATUS
                .iter()
                .map(|(name, h, c)| (*name, Scale::new(*h, *c))),
        );
        let built = BuiltTheme {
            theme: self,
            scales,
        };
        let mut failures = Vec::new();
        for scheme in Scheme::BOTH {
            for (fg, bg, kind) in PAIRS {
                let required = match kind {
                    Kind::Text => built.theme.min_contrast,
                    Kind::Ui => 3.0,
                };
                let fg_c = built.role(scheme, fg).expect("pair roles exist");
                let bg_c = built.role(scheme, bg).expect("pair roles exist");
                let ratio = contrast(fg_c, bg_c);
                if ratio < required {
                    failures.push(ContrastFailure {
                        fg,
                        bg,
                        scheme,
                        ratio,
                        required,
                    });
                }
            }
        }
        if failures.is_empty() {
            Ok(built)
        } else {
            Err(ContrastReport { failures })
        }
    }
}

/// Which elements a theme's tokens apply to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope<'a> {
    /// The document (`:root`), with `data-theme` scheme switches.
    Root,
    /// Subtrees marked `data-st-theme="<name>"`.
    Named(&'a str),
}

/// A validated theme, ready to emit CSS.
#[derive(Clone, Debug, PartialEq)]
pub struct BuiltTheme {
    pub(crate) theme: Theme,
    pub(crate) scales: Vec<(&'static str, Scale)>,
}

impl BuiltTheme {
    /// Token CSS for `scope`, inside `@layer stucco.tokens`.
    pub fn css(&self, scope: Scope<'_>) -> String {
        crate::css::token_css(self, scope)
    }

    /// The colour of role `name` (without `--st-`) in `scheme`.
    pub fn role(&self, scheme: Scheme, name: &str) -> Option<Color> {
        let (_, source) = self.roles().find(|(n, _)| *n == name)?;
        Some(match source {
            Source::Step(scale, n) => self.scale(scale).step(scheme, n),
            Source::OnAccent => {
                let accent = self.role(scheme, "accent")?;
                let (black, white) = (Color::oklch(0.0, 0.0, 0.0), Color::oklch(1.0, 0.0, 0.0));
                if contrast(black, accent) >= contrast(white, accent) {
                    black
                } else {
                    white
                }
            }
        })
    }

    /// Effective roles, with the ink-accent overrides applied.
    pub(crate) fn roles(&self) -> impl Iterator<Item = (&'static str, Source)> + '_ {
        ROLES.iter().map(move |(name, source)| {
            let ink = self
                .theme
                .accent_from_neutral
                .then(|| INK_ACCENT.iter().find(|(n, _)| n == name))
                .flatten();
            (*name, ink.map_or(*source, |(_, s)| *s))
        })
    }

    fn scale(&self, name: &str) -> &Scale {
        &self
            .scales
            .iter()
            .find(|(n, _)| *n == name)
            .expect("roles reference known scales")
            .1
    }
}

/// Role pairs that failed their contrast requirement.
#[derive(Clone, Debug, PartialEq)]
pub struct ContrastReport {
    /// Every failing pair.
    pub failures: Vec<ContrastFailure>,
}

/// One role pair below its required contrast.
#[derive(Clone, Debug, PartialEq)]
pub struct ContrastFailure {
    /// Foreground role.
    pub fg: &'static str,
    /// Background role.
    pub bg: &'static str,
    /// Scheme in which it failed.
    pub scheme: Scheme,
    /// Measured ratio.
    pub ratio: f64,
    /// Required ratio.
    pub required: f64,
}

impl fmt::Display for ContrastReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for x in &self.failures {
            writeln!(
                f,
                "{} on {} ({:?}): {:.2} < {}",
                x.fg, x.bg, x.scheme, x.ratio, x.required
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for ContrastReport {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contrast;

    #[test]
    fn default_seed_meets_aa_in_both_schemes() {
        let built = Theme::from_seed(250.0).build().expect("valid");
        for scheme in Scheme::BOTH {
            assert!(
                contrast(
                    built.role(scheme, "text").unwrap(),
                    built.role(scheme, "bg").unwrap()
                ) >= 4.5
            );
        }
    }

    #[test]
    fn an_impossible_minimum_reports_failing_pairs() {
        let err = Theme::from_seed(250.0)
            .min_contrast(30.0)
            .build()
            .unwrap_err();
        assert!(
            err.failures
                .iter()
                .any(|f| f.fg == "text" && f.bg == "bg" && f.scheme == Scheme::Dark)
        );
        assert!(err.to_string().contains("text on bg"));
    }

    #[test]
    fn root_css_has_layer_scales_roles_and_scheme_switches() {
        let css = Theme::from_seed(250.0).build().unwrap().css(Scope::Root);
        assert!(css.starts_with("@layer stucco.tokens {"));
        assert!(css.contains("--st-neutral-1: light-dark(oklch("));
        assert!(css.contains("--st-bg: var(--st-neutral-1);"));
        assert!(css.contains("[data-theme=\"dark\"] { color-scheme: dark; }"));
        assert!(css.contains("--st-space-3: 12px;"));
    }

    #[test]
    fn named_scope_has_no_scheme_switches() {
        let css = Theme::from_seed(300.0)
            .build()
            .unwrap()
            .css(Scope::Named("brand"));
        assert!(css.contains("[data-st-theme=\"brand\"] {") && !css.contains("[data-theme="));
    }

    #[test]
    fn ink_accent_uses_the_text_colour() {
        let built = Theme::from_seed(0.0).accent_from_neutral().build().unwrap();
        assert_eq!(
            built.role(Scheme::Light, "accent"),
            built.role(Scheme::Light, "text")
        );
    }
}
