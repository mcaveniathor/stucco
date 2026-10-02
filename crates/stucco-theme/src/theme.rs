//! The theme builder and its options.

use std::fmt;

use crate::color::{luminance_ratio, srgb_luminance};
use crate::personality::{
    ButtonDepth, ButtonShape, ControlStyle, CornerStyle, Elevation, Finish, FocusStyle,
    HeaderStyle, HeadingFont, HeadingWeight, IconWeight, LabelStyle, Leading, LineWeight,
    LinkStyle, Motion, NavStyle, PanelStyle, Personality, RuleStyle, ShellLayout, TableStyle,
    TagStyle,
};
use crate::roles::{INK_ACCENT, Kind, PAIRS, ROLES, STATUS, Source};
use crate::seed::Palette;
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
        self.sans = format!("\"{}\", {}", css_string(family), self.sans);
        self
    }

    /// Prepends `family` (quoted) to the code stack.
    pub fn mono(mut self, family: &str) -> Fonts {
        self.mono = format!("\"{}\", {}", css_string(family), self.mono);
        self
    }

    /// Replaces the body stack verbatim (used by presets).
    pub(crate) fn sans_stack(mut self, stack: &str) -> Fonts {
        self.sans = stack.to_owned();
        self
    }
}

/// Escapes `s` for use inside a double-quoted CSS string, including `<` so the
/// value can never close an inline `<style>`.
fn css_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str(r"\\"),
            '"' => out.push_str(r#"\""#),
            '<' => out.push_str(r"\3c "),
            '\n' => out.push_str(r"\a "),
            '\r' => out.push_str(r"\d "),
            c => out.push(c),
        }
    }
    out
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

/// Lightness of steps 9, 10 and 11 (light, dark) under `high_contrast`.
const HIGH_CONTRAST_L: [(f64, f64); 3] = [(0.45, 0.78), (0.40, 0.84), (0.36, 0.88)];

/// A theme definition. Validation happens only in [`Theme::build`].
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub(crate) accent_hue: f64,
    pub(crate) accent_chroma: f64,
    pub(crate) neutral_hue: Option<f64>,
    pub(crate) neutral_tint: f64,
    pub(crate) accent_from_neutral: bool,
    pub(crate) text_lightness: Option<(f64, f64)>,
    pub(crate) high_contrast: bool,
    pub(crate) fonts: Fonts,
    pub(crate) type_scale: TypeScale,
    pub(crate) space: f64,
    pub(crate) radius: Radius,
    pub(crate) density: Density,
    pub(crate) min_contrast: f64,
    pub(crate) personality: Personality,
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
            high_contrast: false,
            fonts: Fonts::system(),
            type_scale: TypeScale::new(16.0, 1.2),
            space: 4.0,
            radius: Radius::Soft,
            density: Density::Comfortable,
            min_contrast: 4.5,
            personality: Personality::default(),
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

    /// Applies every colour choice in `palette`: accent, neutral hue and
    /// tint, and the ink accent.
    pub fn palette(mut self, palette: Palette) -> Theme {
        self = self.accent(palette.accent);
        self.neutral_hue = palette.neutral_hue;
        self.neutral_tint = palette.neutral_tint;
        self.accent_from_neutral = palette.ink;
        self
    }

    /// Uses the text colour as the accent ("ink" accent, for monochrome
    /// themes).
    pub fn accent_from_neutral(mut self) -> Theme {
        self.accent_from_neutral = true;
        self
    }

    /// Steeper lightness for steps 9–11 of every scale, so text and accent
    /// pairs reach WCAG AAA (7:1).
    pub fn high_contrast(mut self) -> Theme {
        self.high_contrast = true;
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

    /// How surfaces separate from the page.
    pub fn elevation(mut self, elevation: Elevation) -> Theme {
        self.personality.elevation = elevation;
        self
    }

    /// How table rows are separated.
    pub fn table_style(mut self, style: TableStyle) -> Theme {
        self.personality.table = style;
        self
    }

    /// How text inputs and selects are drawn.
    pub fn control_style(mut self, style: ControlStyle) -> Theme {
        self.personality.controls = style;
        self
    }

    /// How the application header sits against the page.
    pub fn header_style(mut self, style: HeaderStyle) -> Theme {
        self.personality.header = style;
        self
    }

    /// Heading weight and tracking.
    pub fn heading_weight(mut self, weight: HeadingWeight) -> Theme {
        self.personality.headings = weight;
        self
    }

    /// Button corner shape.
    pub fn button_shape(mut self, shape: ButtonShape) -> Theme {
        self.personality.buttons = shape;
        self
    }

    /// How links in running text are drawn.
    pub fn link_style(mut self, style: LinkStyle) -> Theme {
        self.personality.links = style;
        self
    }

    /// How navigation marks the current page.
    pub fn nav_style(mut self, style: NavStyle) -> Theme {
        self.personality.nav = style;
        self
    }

    /// The keyboard focus ring.
    pub fn focus_style(mut self, style: FocusStyle) -> Theme {
        self.personality.focus = style;
        self
    }

    /// The texture of the page background.
    pub fn finish(mut self, finish: Finish) -> Theme {
        self.personality.finish = finish;
        self
    }

    /// How the application shell arranges its navigation and content.
    pub fn shell_layout(mut self, layout: ShellLayout) -> Theme {
        self.personality.shell = layout;
        self
    }

    /// How panels frame their content.
    pub fn panel_style(mut self, style: PanelStyle) -> Theme {
        self.personality.panels = style;
        self
    }

    /// The shape of the corners of cards, panels and other large surfaces.
    pub fn corner_style(mut self, style: CornerStyle) -> Theme {
        self.personality.corners = style;
        self
    }

    /// The weight of rules and borders.
    pub fn line_weight(mut self, weight: LineWeight) -> Theme {
        self.personality.lines = weight;
        self
    }

    /// How buttons stand off the page.
    pub fn button_depth(mut self, depth: ButtonDepth) -> Theme {
        self.personality.depth = depth;
        self
    }

    /// How small labels such as table column headings look.
    pub fn label_style(mut self, style: LabelStyle) -> Theme {
        self.personality.labels = style;
        self
    }

    /// The stroke width of icons.
    pub fn icon_weight(mut self, weight: IconWeight) -> Theme {
        self.personality.icons = weight;
        self
    }

    /// How quickly, and with what feel, things move.
    pub fn motion(mut self, motion: Motion) -> Theme {
        self.personality.motion = motion;
        self
    }

    /// The typeface of headings.
    pub fn heading_font(mut self, font: HeadingFont) -> Theme {
        self.personality.heading_font = font;
        self
    }

    /// The line height of running text.
    pub fn leading(mut self, leading: Leading) -> Theme {
        self.personality.leading = leading;
        self
    }

    /// How tags are drawn.
    pub fn tag_style(mut self, style: TagStyle) -> Theme {
        self.personality.tags = style;
        self
    }

    /// The pattern of dividing rules.
    pub fn rule_style(mut self, style: RuleStyle) -> Theme {
        self.personality.rules = style;
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
        if self.high_contrast {
            for (_, scale) in &mut scales {
                for (i, (light, dark)) in HIGH_CONTRAST_L.iter().enumerate() {
                    scale.light[8 + i].l = *light;
                    scale.dark[8 + i].l = *dark;
                }
            }
        }
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
        // A textured finish pulls the page background towards mid-grey by
        // up to its strongest opacity (browsers blend in gamma-encoded
        // sRGB), so text on the page must also clear that point.
        let alpha = built.theme.personality.finish.max_alpha();
        if alpha > 0.0 {
            for scheme in Scheme::BOTH {
                for (fg, _, kind) in PAIRS.iter().filter(|(_, bg, _)| *bg == "bg") {
                    let required = match kind {
                        Kind::Text => built.theme.min_contrast,
                        Kind::Ui => 3.0,
                    };
                    let fg_c = built.role(scheme, fg).expect("pair roles exist");
                    let bg_c = built.role(scheme, "bg").expect("bg exists");
                    let grain = bg_c.to_srgb().map(|v| v * (1.0 - alpha) + 0.5 * alpha);
                    let ratio = luminance_ratio(fg_c.luminance(), srgb_luminance(grain));
                    if ratio < required {
                        failures.push(ContrastFailure {
                            fg,
                            bg: "bg with finish",
                            scheme,
                            ratio,
                            required,
                        });
                    }
                }
            }
        }
        let t = &built.theme;
        let invalid: Vec<&'static str> = [
            ("min_contrast", t.min_contrast),
            ("space", t.space),
            ("type_scale.base_px", t.type_scale.base_px),
            ("type_scale.ratio", t.type_scale.ratio),
        ]
        .into_iter()
        .filter(|(_, v)| !(v.is_finite() && *v > 0.0))
        .map(|(name, _)| name)
        .collect();
        if failures.is_empty() && invalid.is_empty() {
            Ok(built)
        } else {
            Err(ContrastReport { failures, invalid })
        }
    }
}

/// The accent step used for hover: lighter than step 9 under black text,
/// darker under white text. Light scales get darker with the step number,
/// dark scales lighter.
pub(crate) fn hover_step(scheme: Scheme, on_black: bool) -> usize {
    match (scheme, on_black) {
        (Scheme::Light, true) | (Scheme::Dark, false) => 8,
        (Scheme::Light, false) | (Scheme::Dark, true) => 10,
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
            Source::AccentHover => {
                let on_black = self.role(scheme, "on-accent")?.l < 0.5;
                self.scale("accent")
                    .step(scheme, hover_step(scheme, on_black))
            }
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
    /// Options that are not finite and positive (contrast cannot be checked
    /// against them).
    pub invalid: Vec<&'static str>,
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
        for name in &self.invalid {
            writeln!(f, "invalid {name}: must be finite and positive")?;
        }
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
    fn textured_finishes_check_text_against_the_grain() {
        // Require exactly the weakest flat ratio on the page background, so
        // the flat checks pass and only the grain can fail.
        let flat = Theme::preset(crate::Preset::Slate).build().unwrap();
        let mut weakest = f64::INFINITY;
        for scheme in Scheme::BOTH {
            let bg = flat.role(scheme, "bg").unwrap();
            for (fg, b, kind) in PAIRS {
                if *b == "bg" && *kind == Kind::Text {
                    weakest = weakest.min(contrast(flat.role(scheme, fg).unwrap(), bg));
                }
            }
        }
        let strict = Theme::preset(crate::Preset::Slate).min_contrast(weakest - 1e-9);
        let grain = |report: Result<BuiltTheme, ContrastReport>| {
            report
                .err()
                .is_some_and(|r| r.failures.iter().any(|f| f.bg == "bg with finish"))
        };
        assert!(!grain(strict.clone().build()));
        assert!(grain(strict.finish(Finish::Sand).build()));
    }

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
        assert!(
            !css.contains("color-scheme"),
            "a named theme must not reset a forced scheme"
        );
    }

    #[test]
    fn non_finite_or_non_positive_options_fail_validation() {
        for theme in [
            Theme::from_seed(250.0).min_contrast(f64::NAN),
            Theme::from_seed(250.0).min_contrast(0.0),
            Theme::from_seed(250.0).space(f64::NAN),
            Theme::from_seed(250.0).type_scale(TypeScale::new(f64::INFINITY, 1.2)),
            Theme::from_seed(250.0).type_scale(TypeScale::new(16.0, -1.0)),
        ] {
            let err = theme.build().unwrap_err();
            assert!(!err.invalid.is_empty(), "{err}");
        }
    }

    #[test]
    fn high_contrast_reaches_aaa_for_every_text_pair() {
        Theme::from_seed(250.0)
            .high_contrast()
            .min_contrast(7.0)
            .build()
            .unwrap();
    }

    #[test]
    fn font_families_cannot_break_out_of_the_token_block() {
        let css = Theme::from_seed(250.0)
            .fonts(Fonts::system().sans("x\"; } </style><script>alert(1)</script>"))
            .build()
            .unwrap()
            .css(Scope::Root);
        assert!(!css.contains("</"), "{css}");
        assert!(css.contains(r#"--st-font-sans: "x\"; } \3c /style>\3c script>"#));
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
