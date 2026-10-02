//! Token CSS generation.

use std::fmt::Write;

use crate::Scheme;
use crate::roles::Source;
use crate::theme::{BuiltTheme, Density, Radius, Scope};

/// Type steps: name and exponent of the ratio.
const TYPE_STEPS: &[(&str, i32)] = &[
    ("xs", -2),
    ("sm", -1),
    ("base", 0),
    ("lg", 1),
    ("xl", 2),
    ("2xl", 3),
    ("3xl", 4),
];

/// Spacing multipliers for `--st-space-1..12`.
const SPACE: [f64; 12] = [1., 2., 3., 4., 5., 6., 8., 10., 12., 16., 20., 24.];

pub(crate) fn token_css(built: &BuiltTheme, scope: Scope<'_>) -> String {
    let t = &built.theme;
    let mut css = String::from("@layer stucco.tokens {\n");
    // Only the root sets `color-scheme`: a named theme inherits the scheme, so
    // a forced `data-theme` above (or on) it still applies.
    match scope {
        Scope::Root => css.push_str("  :root { color-scheme: light dark;\n"),
        Scope::Named(name) => {
            let _ = writeln!(css, "  [data-st-theme=\"{name}\"] {{");
        }
    }
    for (name, scale) in &built.scales {
        for n in 1..=12 {
            let _ = writeln!(
                css,
                "    --st-{name}-{n}: light-dark({}, {});",
                scale.step(Scheme::Light, n).css(),
                scale.step(Scheme::Dark, n).css()
            );
        }
    }
    for (role, source) in built.roles() {
        match source {
            Source::Step(scale, n) => {
                let _ = writeln!(css, "    --st-{role}: var(--st-{scale}-{n});");
            }
            Source::AccentHover => {
                let step = |scheme| {
                    let on_black = built.role(scheme, "on-accent").expect("role exists").l < 0.5;
                    crate::theme::hover_step(scheme, on_black)
                };
                let _ = writeln!(
                    css,
                    "    --st-{role}: light-dark(var(--st-accent-{}), var(--st-accent-{}));",
                    step(Scheme::Light),
                    step(Scheme::Dark)
                );
            }
            Source::OnAccent => {
                let (l, d) = (
                    built.role(Scheme::Light, role).expect("role exists"),
                    built.role(Scheme::Dark, role).expect("role exists"),
                );
                let _ = writeln!(
                    css,
                    "    --st-{role}: light-dark({}, {});",
                    l.css(),
                    d.css()
                );
            }
        }
    }
    let _ = writeln!(css, "    --st-font-sans: {};", t.fonts.sans);
    let _ = writeln!(css, "    --st-font-mono: {};", t.fonts.mono);
    for (name, k) in TYPE_STEPS {
        let _ = writeln!(
            css,
            "    --st-text-{name}: {};",
            fluid(t.type_scale.base_px * t.type_scale.ratio.powi(*k))
        );
    }
    for (i, m) in SPACE.iter().enumerate() {
        let _ = writeln!(css, "    --st-space-{}: {}px;", i + 1, m * t.space);
    }
    let (sm, md, lg) = match t.radius {
        Radius::Sharp => ("0", "0", "0"),
        Radius::Soft => ("4px", "8px", "12px"),
        Radius::Round => ("8px", "14px", "20px"),
    };
    let _ = writeln!(
        css,
        "    --st-radius-sm: {sm}; --st-radius-md: {md}; --st-radius-lg: {lg}; --st-radius-full: 9999px;"
    );
    let (h, pad) = match t.density {
        Density::Compact => ("32px", "0.75"),
        Density::Comfortable => ("40px", "1"),
    };
    let _ = writeln!(css, "    --st-control-h: {h}; --st-pad-scale: {pad};");
    css.push_str(
        "    --st-duration-fast: 120ms; --st-duration: 200ms; --st-duration-slow: 320ms; \
         --st-ease: cubic-bezier(.2,0,0,1);\n",
    );
    css.push_str(
        "    --st-z-dropdown: 100; --st-z-sticky: 200; --st-z-overlay: 300; --st-z-toast: 400;\n",
    );
    for (i, pct) in [8, 12, 18].iter().enumerate() {
        let _ = writeln!(
            css,
            "    --st-shadow-{}: 0 {}px {}px color-mix(in oklch, var(--st-neutral-12) {pct}%, transparent);",
            i + 1,
            (i + 1) * 2,
            (i + 1) * 8
        );
    }
    for (name, value) in t.personality.tokens() {
        let _ = writeln!(css, "    --st-{name}: {value};");
    }
    css.push_str("  }\n");
    if scope == Scope::Root {
        css.push_str("  [data-theme=\"light\"] { color-scheme: light; }\n");
        css.push_str("  [data-theme=\"dark\"] { color-scheme: dark; }\n");
    }
    css.push_str("}\n");
    css
}

/// `clamp(min, a + b·vw, max)` growing from 90% of `px` at a 360px viewport to
/// `px` at 1280px.
fn fluid(px: f64) -> String {
    let (min, max) = (px * 0.9, px);
    let slope = (max - min) / (1280.0 - 360.0);
    let intercept = min - slope * 360.0;
    format!(
        "clamp({:.4}rem, {:.4}rem + {:.4}vw, {:.4}rem)",
        min / 16.0,
        intercept / 16.0,
        slope * 100.0,
        max / 16.0
    )
}
