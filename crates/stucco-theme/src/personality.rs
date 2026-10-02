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

/// How links in running text are drawn. Every style keeps an underline, so
/// links never rely on colour alone.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum LinkStyle {
    /// A full-strength underline that thickens on hover (default).
    #[default]
    Underlined,
    /// A thin, faint underline that turns full strength on hover.
    Subtle,
    /// A thick underline and medium weight.
    Bold,
    /// An underline over a soft accent highlight along the baseline.
    Highlight,
}

/// How navigation marks the current page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NavStyle {
    /// A soft accent fill with accent text (default).
    #[default]
    Soft,
    /// A solid accent fill with contrasting text.
    Solid,
    /// An accent bar along the leading edge, without a fill.
    Bar,
}

/// The keyboard focus ring.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FocusStyle {
    /// A 2px ring with a 2px gap (default).
    #[default]
    Ring,
    /// A 3px ring with a 2px gap.
    Thick,
    /// A 2px ring with a 1px gap, close to the control.
    Snug,
}

/// The texture of the page background, like the finish of a plastered
/// wall. Surfaces such as cards and inputs stay smooth.
///
/// Textured finishes are a faint grey grain. [`Theme::build`] checks text
/// on the page background against the grain's darkest and lightest points,
/// and the grain is removed for visitors who ask for more contrast.
///
/// [`Theme::build`]: crate::Theme::build
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Finish {
    /// No texture (default).
    #[default]
    Smooth,
    /// A fine, even grain.
    Sand,
    /// A softer, medium grain, like a floated coat.
    Float,
    /// Broad, mottled patches, like a knockdown coat.
    Knockdown,
}

impl Finish {
    /// The grain's strongest opacity: how far it can pull the background
    /// towards mid-grey.
    pub(crate) fn max_alpha(self) -> f64 {
        match self {
            Finish::Smooth => 0.0,
            Finish::Sand => 0.06,
            Finish::Float => 0.05,
            Finish::Knockdown => 0.05,
        }
    }

    /// A tiling SVG noise image as a CSS `url()`, or `none`.
    fn image(self) -> &'static str {
        // Grey (0.5) grain whose opacity follows the noise, so the same
        // image darkens light backgrounds and lightens dark ones.
        match self {
            Finish::Smooth => "none",
            Finish::Sand => concat!(
                "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='180' height='180'%3E",
                "%3Cfilter id='f'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.85' numOctaves='3' stitchTiles='stitch'/%3E",
                "%3CfeColorMatrix values='0 0 0 0 0.5 0 0 0 0 0.5 0 0 0 0 0.5 0.06 0 0 0 0'/%3E%3C/filter%3E",
                "%3Crect width='100%25' height='100%25' filter='url(%23f)'/%3E%3C/svg%3E\")"
            ),
            Finish::Float => concat!(
                "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='240' height='240'%3E",
                "%3Cfilter id='f'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.4' numOctaves='4' stitchTiles='stitch'/%3E",
                "%3CfeColorMatrix values='0 0 0 0 0.5 0 0 0 0 0.5 0 0 0 0 0.5 0.05 0 0 0 0'/%3E%3C/filter%3E",
                "%3Crect width='100%25' height='100%25' filter='url(%23f)'/%3E%3C/svg%3E\")"
            ),
            Finish::Knockdown => concat!(
                "url(\"data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='320' height='320'%3E",
                "%3Cfilter id='f'%3E%3CfeTurbulence type='turbulence' baseFrequency='0.03' numOctaves='2' stitchTiles='stitch'/%3E",
                "%3CfeColorMatrix values='0 0 0 0 0.5 0 0 0 0 0.5 0 0 0 0 0.5 0.05 0 0 0 0'/%3E%3C/filter%3E",
                "%3Crect width='100%25' height='100%25' filter='url(%23f)'/%3E%3C/svg%3E\")"
            ),
        }
    }
}

/// How the application shell arranges its navigation and content.
///
/// The shell's primary links (`AppShell::link`) follow the layout; its
/// sidebar is always a column beside the content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ShellLayout {
    /// A filled side column with the primary links above the sidebar
    /// (default).
    #[default]
    Sidebar,
    /// A narrower side column on the page background, separated by a rule.
    Rail,
    /// The primary links in a row under the header; a sidebar, if any,
    /// stays a column beside the content.
    Topbar,
}

/// How panels frame their content.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PanelStyle {
    /// A bordered box on the surface colour (default).
    #[default]
    Boxed,
    /// No box: a strong rule above the title, for an editorial look.
    Ruled,
    /// A bordered box whose title sits in a raised band.
    Headed,
}

/// The shape of the corners of cards, panels and other large surfaces.
///
/// Squircle and Bevel use the CSS `corner-shape` property; browsers
/// without it show ordinary rounded corners. All three keep the theme's
/// radius, so sharp themes stay sharp.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CornerStyle {
    /// Evenly rounded corners (default).
    #[default]
    Even,
    /// Continuous, squircle-like curves.
    Squircle,
    /// Corners cut at an angle.
    Bevel,
    /// Slightly uneven corners, as if shaped by hand.
    Hand,
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
    pub(crate) links: LinkStyle,
    pub(crate) nav: NavStyle,
    pub(crate) focus: FocusStyle,
    pub(crate) finish: Finish,
    pub(crate) shell: ShellLayout,
    pub(crate) panels: PanelStyle,
    pub(crate) corners: CornerStyle,
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
        // (underline colour, thickness, hover thickness, background, weight)
        let (link_line, link_thickness, link_hover, link_bg, link_weight) = match self.links {
            LinkStyle::Underlined => ("currentColor", "auto", "2px", "none", "inherit"),
            LinkStyle::Subtle => (
                "color-mix(in oklch, currentColor 35%, transparent)",
                "1px",
                "1px",
                "none",
                "inherit",
            ),
            LinkStyle::Bold => ("currentColor", "2px", "3px", "none", "550"),
            LinkStyle::Highlight => (
                "currentColor",
                "auto",
                "2px",
                "linear-gradient(transparent 62%, var(--st-accent-soft) 0)",
                "inherit",
            ),
        };
        let (nav_bg, nav_color, nav_bar) = match self.nav {
            NavStyle::Soft => (
                "var(--st-accent-soft)",
                "var(--st-accent-text)",
                "transparent",
            ),
            NavStyle::Solid => ("var(--st-accent)", "var(--st-on-accent)", "transparent"),
            NavStyle::Bar => ("transparent", "var(--st-text)", "var(--st-accent)"),
        };
        let (focus_width, focus_offset) = match self.focus {
            FocusStyle::Ring => ("2px", "2px"),
            FocusStyle::Thick => ("3px", "2px"),
            FocusStyle::Snug => ("2px", "1px"),
        };
        // The shell body is a grid: the primary links ("nav"), the sidebar
        // ("side") and main. `solo` areas apply when there is no sidebar.
        // (columns, areas, solo columns, solo areas)
        let (columns, areas, solo_columns, solo_areas) = match self.shell {
            ShellLayout::Sidebar => (
                "15rem minmax(0, 1fr)",
                "\"nav main\" \"side main\"",
                "15rem minmax(0, 1fr)",
                "\"nav main\" \"nav main\"",
            ),
            ShellLayout::Rail => (
                "11rem minmax(0, 1fr)",
                "\"nav main\" \"side main\"",
                "11rem minmax(0, 1fr)",
                "\"nav main\" \"nav main\"",
            ),
            ShellLayout::Topbar => (
                "15rem minmax(0, 1fr)",
                "\"nav nav\" \"side main\"",
                "minmax(0, 1fr)",
                "\"nav\" \"main\"",
            ),
        };
        // The primary links: a column at the top of the side, or a row.
        // (direction, block padding, inline rule, block rule, side fill)
        let (nav_dir, nav_pad, nav_rule_inline, nav_rule_block, side_bg) = match self.shell {
            ShellLayout::Sidebar => (
                "column",
                "var(--st-space-4) var(--st-space-2)",
                "1px",
                "0",
                "var(--st-surface)",
            ),
            ShellLayout::Rail => (
                "column",
                "var(--st-space-4) var(--st-space-2)",
                "1px",
                "0",
                "transparent",
            ),
            ShellLayout::Topbar => ("row", "var(--st-space-2)", "0", "1px", "var(--st-surface)"),
        };
        // Where the Bar nav style marks the current primary link: the
        // leading edge in a column, under the link in a row.
        let (bar_block, bar_inline, bar_w, bar_h) = match self.shell {
            ShellLayout::Topbar => ("auto 0", "var(--st-space-3)", "auto", "3px"),
            _ => ("var(--st-space-1)", "0 auto", "3px", "auto"),
        };
        let surface_radius = match self.corners {
            CornerStyle::Hand => concat!(
                "calc(var(--st-radius-lg) * 0.75) calc(var(--st-radius-lg) * 1.25) ",
                "calc(var(--st-radius-lg) * 0.9) calc(var(--st-radius-lg) * 1.15) / ",
                "calc(var(--st-radius-lg) * 1.1) calc(var(--st-radius-lg) * 0.8) ",
                "calc(var(--st-radius-lg) * 1.2) calc(var(--st-radius-lg) * 0.85)"
            ),
            _ => "var(--st-radius-lg)",
        };
        let corner_shape = match self.corners {
            CornerStyle::Even | CornerStyle::Hand => "round",
            CornerStyle::Squircle => "squircle",
            CornerStyle::Bevel => "bevel",
        };
        // (border, top rule, fill, radius, inline padding, shadow,
        //  header fill, header padding, header margin, header rule)
        let panel = match self.panels {
            PanelStyle::Boxed => [
                "1px solid var(--st-border)",
                "1px solid var(--st-border)",
                "var(--st-surface)",
                "var(--st-surface-radius)",
                "var(--st-space-6)",
                "var(--st-surface-shadow)",
                "transparent",
                "0",
                "0 0 var(--st-space-4)",
                "0 solid transparent",
            ],
            PanelStyle::Ruled => [
                "0 solid transparent",
                "2px solid var(--st-border-strong)",
                "transparent",
                "0",
                "0",
                "none",
                "transparent",
                "0",
                "0 0 var(--st-space-4)",
                "0 solid transparent",
            ],
            PanelStyle::Headed => [
                "1px solid var(--st-border)",
                "1px solid var(--st-border)",
                "var(--st-surface)",
                "var(--st-surface-radius)",
                "var(--st-space-6)",
                "var(--st-surface-shadow)",
                "var(--st-surface-raised)",
                "var(--st-space-4) var(--st-space-6)",
                "calc(-1 * var(--st-space-6)) calc(-1 * var(--st-space-6)) var(--st-space-5)",
                "1px solid var(--st-border)",
            ],
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
            ("link-line", link_line),
            ("link-thickness", link_thickness),
            ("link-hover-thickness", link_hover),
            ("link-bg", link_bg),
            ("link-weight", link_weight),
            ("nav-current-bg", nav_bg),
            ("nav-current-text", nav_color),
            ("nav-current-bar", nav_bar),
            ("focus-width", focus_width),
            ("focus-offset", focus_offset),
            ("finish", self.finish.image()),
            ("shell-columns", columns),
            ("shell-areas", areas),
            ("shell-columns-solo", solo_columns),
            ("shell-areas-solo", solo_areas),
            ("shell-nav-direction", nav_dir),
            ("shell-nav-pad-block", nav_pad),
            ("shell-nav-rule-inline", nav_rule_inline),
            ("shell-nav-rule-block", nav_rule_block),
            ("shell-side-bg", side_bg),
            ("shell-bar-inset-block", bar_block),
            ("shell-bar-inset-inline", bar_inline),
            ("shell-bar-inline-size", bar_w),
            ("shell-bar-block-size", bar_h),
            ("surface-radius", surface_radius),
            ("corner-shape", corner_shape),
            ("panel-border", panel[0]),
            ("panel-rule", panel[1]),
            ("panel-bg", panel[2]),
            ("panel-radius", panel[3]),
            ("panel-pad-inline", panel[4]),
            ("panel-shadow", panel[5]),
            ("panel-head-bg", panel[6]),
            ("panel-head-pad", panel[7]),
            ("panel-head-margin", panel[8]),
            ("panel-head-rule", panel[9]),
        ]
    }
}
