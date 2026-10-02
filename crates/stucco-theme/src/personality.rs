//! Style personality: presentation choices that change how components look
//! without changing their markup. Each choice becomes token CSS that the
//! component stylesheets read.

/// How surfaces separate from the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Elevation {
    /// Borders only; nothing casts a shadow.
    Flat,
    /// Borders everywhere; cards add a light shadow (default).
    #[default]
    Outlined,
    /// Panels and table cards cast a light shadow, cards a stronger one.
    Raised,
}

/// How table rows are separated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TableStyle {
    /// A rule under every row and a filled header row (default).
    #[default]
    Lined,
    /// Alternate rows filled instead of ruled.
    Striped,
    /// No rules or fills apart from the line under the header.
    Open,
}

/// How text inputs and selects are drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ControlStyle {
    /// A border on every side over the surface colour (default).
    #[default]
    Outlined,
    /// A border on every side over a raised fill.
    Filled,
    /// A raised fill with a rule along the bottom edge only.
    Underlined,
}

/// How the application header sits against the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HeaderStyle {
    /// A surface-coloured bar with a bottom border (default).
    #[default]
    Bar,
    /// The page background with a bottom border.
    Plain,
    /// A soft accent fill without a border.
    Tinted,
}

/// Heading weight and tracking.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum HeadingWeight {
    /// Semibold with near-normal tracking.
    Regular,
    /// Between semibold and bold, slightly tightened (default).
    #[default]
    Bold,
    /// Heavy and tight, for display-led themes.
    Heavy,
}

/// Button corner shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ButtonShape {
    /// The theme's medium radius (default).
    #[default]
    Rounded,
    /// Fully rounded ends.
    Pill,
}

/// Every personality choice; the default reproduces stucco's base look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(crate) struct Personality {
    pub(crate) elevation: Elevation,
    pub(crate) table: TableStyle,
    pub(crate) controls: ControlStyle,
    pub(crate) header: HeaderStyle,
    pub(crate) headings: HeadingWeight,
    pub(crate) buttons: ButtonShape,
}

impl Personality {
    /// `(token, value)` pairs, without the `--st-` prefix.
    pub(crate) fn tokens(&self) -> Vec<(&'static str, &'static str)> {
        let (surface_shadow, card_shadow) = match self.elevation {
            Elevation::Flat => ("none", "none"),
            Elevation::Outlined => ("none", "var(--st-shadow-1)"),
            Elevation::Raised => ("var(--st-shadow-1)", "var(--st-shadow-2)"),
        };
        let (rule, stripe, head) = match self.table {
            TableStyle::Lined => (
                "var(--st-border)",
                "transparent",
                "var(--st-surface-raised)",
            ),
            TableStyle::Striped => (
                "transparent",
                "var(--st-surface-raised)",
                "var(--st-surface-raised)",
            ),
            TableStyle::Open => ("transparent", "transparent", "transparent"),
        };
        let (control_bg, borders, control_radius, invalid) = match self.controls {
            ControlStyle::Outlined => (
                "var(--st-surface)",
                "1px",
                "var(--st-radius-md)",
                "0 0 0 1px var(--st-danger)",
            ),
            ControlStyle::Filled => (
                "var(--st-surface-raised)",
                "1px",
                "var(--st-radius-md)",
                "0 0 0 1px var(--st-danger)",
            ),
            ControlStyle::Underlined => (
                "var(--st-surface-raised)",
                "0 0 1px",
                "var(--st-radius-sm) var(--st-radius-sm) 0 0",
                "inset 0 -1px 0 var(--st-danger)",
            ),
        };
        let (header_bg, header_border) = match self.header {
            HeaderStyle::Bar => ("var(--st-surface)", "var(--st-border)"),
            HeaderStyle::Plain => ("transparent", "var(--st-border)"),
            HeaderStyle::Tinted => ("var(--st-accent-soft)", "transparent"),
        };
        let (weight, tracking) = match self.headings {
            HeadingWeight::Regular => ("600", "-0.005em"),
            HeadingWeight::Bold => ("650", "-0.01em"),
            HeadingWeight::Heavy => ("760", "-0.02em"),
        };
        let button_radius = match self.buttons {
            ButtonShape::Rounded => "var(--st-radius-md)",
            ButtonShape::Pill => "var(--st-radius-full)",
        };
        vec![
            ("surface-shadow", surface_shadow),
            ("card-shadow", card_shadow),
            ("table-rule", rule),
            ("table-stripe", stripe),
            ("table-head-bg", head),
            ("control-bg", control_bg),
            ("control-border-width", borders),
            ("control-radius", control_radius),
            ("control-invalid-ring", invalid),
            ("header-bg", header_bg),
            ("header-border", header_border),
            ("heading-weight", weight),
            ("heading-tracking", tracking),
            ("button-radius", button_radius),
        ]
    }
}
