//! Semantic roles: which scale step each role uses, and which role pairs
//! must meet a contrast requirement.

/// Where a role's colour comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Source {
    /// Step `n` of the named scale.
    Step(&'static str, usize),
    /// Black or white, whichever contrasts more with `accent`.
    OnAccent,
    /// The accent step next to 9 that moves away from `on-accent` (lighter
    /// under black text, darker under white), so hover keeps the label
    /// readable.
    AccentHover,
}

use Source::{AccentHover, OnAccent, Step};

/// Every role, in CSS output order.
pub(crate) const ROLES: &[(&str, Source)] = &[
    ("bg", Step("neutral", 1)),
    ("surface", Step("neutral", 2)),
    ("surface-raised", Step("neutral", 3)),
    ("hover", Step("neutral", 4)),
    ("text", Step("neutral", 12)),
    ("text-muted", Step("neutral", 11)),
    ("border", Step("neutral", 6)),
    ("border-strong", Step("neutral", 9)),
    ("accent", Step("accent", 9)),
    ("accent-hover", AccentHover),
    ("accent-text", Step("accent", 11)),
    ("accent-soft", Step("accent", 3)),
    ("on-accent", OnAccent),
    ("focus", Step("accent", 9)),
    ("success", Step("success", 9)),
    ("success-text", Step("success", 11)),
    ("success-soft", Step("success", 3)),
    ("warning", Step("warning", 9)),
    ("warning-text", Step("warning", 11)),
    ("warning-soft", Step("warning", 3)),
    ("danger", Step("danger", 9)),
    ("danger-text", Step("danger", 11)),
    ("danger-soft", Step("danger", 3)),
    ("info", Step("info", 9)),
    ("info-text", Step("info", 11)),
    ("info-soft", Step("info", 3)),
];

/// Overrides for the "ink" accent (`Theme::accent_from_neutral`).
pub(crate) const INK_ACCENT: &[(&str, Source)] = &[
    ("accent", Step("neutral", 12)),
    ("accent-hover", Step("neutral", 11)),
    ("accent-text", Step("neutral", 12)),
    ("accent-soft", Step("neutral", 3)),
    ("focus", Step("neutral", 12)),
];

/// Status scales: name, hue, chroma.
pub(crate) const STATUS: &[(&str, f64, f64)] = &[
    ("success", 150.0, 0.14),
    ("warning", 85.0, 0.14),
    ("danger", 25.0, 0.14),
    ("info", 240.0, 0.14),
];

/// Whether a pair is text (theme minimum) or a UI boundary (3:1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Text,
    Ui,
}

/// `(foreground, background, kind)` pairs checked by `Theme::build`.
pub(crate) const PAIRS: &[(&str, &str, Kind)] = &[
    ("text", "bg", Kind::Text),
    ("text", "surface", Kind::Text),
    ("text", "surface-raised", Kind::Text),
    ("text-muted", "bg", Kind::Text),
    ("text-muted", "surface", Kind::Text),
    ("accent-text", "bg", Kind::Text),
    ("accent-text", "surface", Kind::Text),
    ("on-accent", "accent", Kind::Text),
    ("on-accent", "accent-hover", Kind::Text),
    ("accent-text", "accent-soft", Kind::Text),
    // The tinted header and the current sidebar link sit on accent-soft.
    ("text", "accent-soft", Kind::Text),
    ("text-muted", "accent-soft", Kind::Text),
    ("text-muted", "surface-raised", Kind::Text),
    ("success-text", "bg", Kind::Text),
    ("success-text", "success-soft", Kind::Text),
    ("warning-text", "bg", Kind::Text),
    ("warning-text", "warning-soft", Kind::Text),
    ("danger-text", "bg", Kind::Text),
    ("danger-text", "danger-soft", Kind::Text),
    ("info-text", "bg", Kind::Text),
    ("info-text", "info-soft", Kind::Text),
    ("border-strong", "bg", Kind::Ui),
    ("focus", "bg", Kind::Ui),
];
