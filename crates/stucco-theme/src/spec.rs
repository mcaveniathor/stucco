//! Theme specifications: a starting theme and option overrides, written as a
//! short query string such as `seed=42&radius=round`.
//!
//! The `stucco` command-line tool and the documentation site's playground
//! read and write this format, so a playground link, a CLI argument and a
//! line in your configuration all describe the same theme.
//!
//! ```
//! use stucco_theme::spec::ThemeSpec;
//! use stucco_theme::{Radius, Theme};
//!
//! let spec: ThemeSpec = "seed=42&radius=round".parse()?;
//! assert_eq!(spec.theme(), Theme::seeded(42).radius(Radius::Round));
//! assert_eq!(spec.to_query(), "seed=42&radius=round");
//! # Ok::<(), stucco_theme::spec::SpecError>(())
//! ```

use std::fmt::{self, Write};
use std::str::FromStr;

use crate::{
    Backdrop, BuiltTheme, ButtonDepth, ButtonShape, ControlStyle, CornerStyle, Density, Elevation,
    Finish, FocusStyle, HeaderStyle, HeadingFont, HeadingWeight, IconWeight, LabelStyle, Leading,
    LineWeight, LinkStyle, Material, Motion, NavStyle, PanelStyle, Preset, Radius, RuleStyle,
    Scheme, ShadowStyle, ShellLayout, TableStyle, TagStyle, Theme,
};

/// One choice for an option.
#[derive(Debug, PartialEq, Eq)]
pub struct Choice {
    /// The value in a query string, such as `round`.
    pub value: &'static str,
    /// A label for people, such as `Round`.
    pub label: &'static str,
    /// The Rust enum variant, such as `Round`.
    pub variant: &'static str,
}

/// An option a [`ThemeSpec`] can override.
#[derive(Debug, PartialEq, Eq)]
pub struct OptionInfo {
    /// The key in a query string, such as `radius`.
    pub key: &'static str,
    /// A label for people, such as `Corners`.
    pub label: &'static str,
    /// The option's Rust type, such as `Radius`.
    pub ty: &'static str,
    /// The [`Theme`] builder method that sets it, such as `radius`.
    pub method: &'static str,
    /// Every choice, in display order.
    pub choices: &'static [Choice],
}

impl OptionInfo {
    /// The option with query key `key`.
    pub fn find(key: &str) -> Option<&'static OptionInfo> {
        OPTIONS.iter().find(|o| o.key == key)
    }

    /// The choice with query value `value`.
    pub fn choice(&self, value: &str) -> Option<&'static Choice> {
        self.choices.iter().find(|c| c.value == value)
    }
}

const fn choice(value: &'static str, label: &'static str) -> Choice {
    Choice {
        value,
        label,
        variant: label,
    }
}

/// Every option a [`ThemeSpec`] can override, in display order.
pub const OPTIONS: &[OptionInfo] = &[
    OptionInfo {
        key: "radius",
        label: "Corners",
        ty: "Radius",
        method: "radius",
        choices: &[
            choice("sharp", "Sharp"),
            choice("soft", "Soft"),
            choice("round", "Round"),
        ],
    },
    OptionInfo {
        key: "density",
        label: "Density",
        ty: "Density",
        method: "density",
        choices: &[
            choice("comfortable", "Comfortable"),
            choice("compact", "Compact"),
        ],
    },
    OptionInfo {
        key: "elevation",
        label: "Elevation",
        ty: "Elevation",
        method: "elevation",
        choices: &[
            choice("flat", "Flat"),
            choice("outlined", "Outlined"),
            choice("raised", "Raised"),
        ],
    },
    OptionInfo {
        key: "table",
        label: "Tables",
        ty: "TableStyle",
        method: "table_style",
        choices: &[
            choice("lined", "Lined"),
            choice("striped", "Striped"),
            choice("open", "Open"),
        ],
    },
    OptionInfo {
        key: "controls",
        label: "Inputs",
        ty: "ControlStyle",
        method: "control_style",
        choices: &[
            choice("outlined", "Outlined"),
            choice("filled", "Filled"),
            choice("underlined", "Underlined"),
        ],
    },
    OptionInfo {
        key: "header",
        label: "Header",
        ty: "HeaderStyle",
        method: "header_style",
        choices: &[
            choice("bar", "Bar"),
            choice("plain", "Plain"),
            choice("tinted", "Tinted"),
        ],
    },
    OptionInfo {
        key: "headings",
        label: "Headings",
        ty: "HeadingWeight",
        method: "heading_weight",
        choices: &[
            choice("regular", "Regular"),
            choice("bold", "Bold"),
            choice("heavy", "Heavy"),
        ],
    },
    OptionInfo {
        key: "buttons",
        label: "Buttons",
        ty: "ButtonShape",
        method: "button_shape",
        choices: &[choice("rounded", "Rounded"), choice("pill", "Pill")],
    },
    OptionInfo {
        key: "links",
        label: "Links",
        ty: "LinkStyle",
        method: "link_style",
        choices: &[
            choice("underlined", "Underlined"),
            choice("subtle", "Subtle"),
            choice("bold", "Bold"),
            choice("highlight", "Highlight"),
        ],
    },
    OptionInfo {
        key: "nav",
        label: "Current page",
        ty: "NavStyle",
        method: "nav_style",
        choices: &[
            choice("soft", "Soft"),
            choice("solid", "Solid"),
            choice("bar", "Bar"),
        ],
    },
    OptionInfo {
        key: "focus",
        label: "Focus ring",
        ty: "FocusStyle",
        method: "focus_style",
        choices: &[
            choice("ring", "Ring"),
            choice("thick", "Thick"),
            choice("snug", "Snug"),
        ],
    },
    OptionInfo {
        key: "finish",
        label: "Finish",
        ty: "Finish",
        method: "finish",
        choices: &[
            choice("smooth", "Smooth"),
            choice("sand", "Sand"),
            choice("float", "Float"),
            choice("knockdown", "Knockdown"),
        ],
    },
    OptionInfo {
        key: "shell",
        label: "App layout",
        ty: "ShellLayout",
        method: "shell_layout",
        choices: &[
            choice("sidebar", "Sidebar"),
            choice("rail", "Rail"),
            choice("topbar", "Topbar"),
        ],
    },
    OptionInfo {
        key: "panels",
        label: "Panels",
        ty: "PanelStyle",
        method: "panel_style",
        choices: &[
            choice("boxed", "Boxed"),
            choice("ruled", "Ruled"),
            choice("headed", "Headed"),
        ],
    },
    OptionInfo {
        key: "corners",
        label: "Corner shape",
        ty: "CornerStyle",
        method: "corner_style",
        choices: &[
            choice("even", "Even"),
            choice("squircle", "Squircle"),
            choice("bevel", "Bevel"),
            choice("hand", "Hand"),
        ],
    },
    OptionInfo {
        key: "lines",
        label: "Line weight",
        ty: "LineWeight",
        method: "line_weight",
        choices: &[choice("fine", "Fine"), choice("heavy", "Heavy")],
    },
    OptionInfo {
        key: "depth",
        label: "Button depth",
        ty: "ButtonDepth",
        method: "button_depth",
        choices: &[
            choice("flat", "Flat"),
            choice("raised", "Raised"),
            choice("offset", "Offset"),
        ],
    },
    OptionInfo {
        key: "labels",
        label: "Labels",
        ty: "LabelStyle",
        method: "label_style",
        choices: &[
            choice("plain", "Plain"),
            choice("caps", "Caps"),
            choice("strong", "Strong"),
        ],
    },
    OptionInfo {
        key: "icons",
        label: "Icon weight",
        ty: "IconWeight",
        method: "icon_weight",
        choices: &[
            choice("light", "Light"),
            choice("regular", "Regular"),
            choice("bold", "Bold"),
        ],
    },
    OptionInfo {
        key: "motion",
        label: "Motion",
        ty: "Motion",
        method: "motion",
        choices: &[
            choice("smooth", "Smooth"),
            choice("snappy", "Snappy"),
            choice("gentle", "Gentle"),
            choice("springy", "Springy"),
        ],
    },
    OptionInfo {
        key: "heading-font",
        label: "Heading font",
        ty: "HeadingFont",
        method: "heading_font",
        choices: &[
            choice("body", "Body"),
            choice("serif", "Serif"),
            choice("rounded", "Rounded"),
            choice("mono", "Mono"),
        ],
    },
    OptionInfo {
        key: "leading",
        label: "Leading",
        ty: "Leading",
        method: "leading",
        choices: &[
            choice("tight", "Tight"),
            choice("normal", "Normal"),
            choice("airy", "Airy"),
        ],
    },
    OptionInfo {
        key: "tags",
        label: "Tags",
        ty: "TagStyle",
        method: "tag_style",
        choices: &[
            choice("pill", "Pill"),
            choice("square", "Square"),
            choice("outline", "Outline"),
        ],
    },
    OptionInfo {
        key: "rules",
        label: "Rules",
        ty: "RuleStyle",
        method: "rule_style",
        choices: &[
            choice("solid", "Solid"),
            choice("dashed", "Dashed"),
            choice("dotted", "Dotted"),
        ],
    },
    OptionInfo {
        key: "material",
        label: "Material",
        ty: "Material",
        method: "material",
        choices: &[
            choice("solid", "Solid"),
            choice("glass", "Glass"),
            choice("frost", "Frost"),
        ],
    },
    OptionInfo {
        key: "backdrop",
        label: "Backdrop",
        ty: "Backdrop",
        method: "backdrop",
        choices: &[
            choice("plain", "Plain"),
            choice("glow", "Glow"),
            choice("wash", "Wash"),
        ],
    },
    OptionInfo {
        key: "shadows",
        label: "Shadows",
        ty: "ShadowStyle",
        method: "shadow_style",
        choices: &[
            choice("soft", "Soft"),
            choice("layered", "Layered"),
            choice("tinted", "Tinted"),
            choice("hard", "Hard"),
        ],
    },
];

/// The semantic colour roles, in the order [`tokens_json`] lists them.
pub const ROLES: &[&str] = &[
    "bg",
    "surface",
    "surface-raised",
    "hover",
    "text",
    "text-muted",
    "border",
    "border-strong",
    "accent",
    "accent-hover",
    "accent-text",
    "accent-soft",
    "on-accent",
    "focus",
    "success",
    "success-text",
    "success-soft",
    "warning",
    "warning-text",
    "warning-soft",
    "danger",
    "danger-text",
    "danger-soft",
    "info",
    "info-text",
    "info-soft",
];

/// Where a [`ThemeSpec`] starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Base {
    /// A built-in preset: `preset=pine`.
    Preset(Preset),
    /// [`Theme::seeded`]: `seed=42`.
    Seed(u64),
    /// [`Theme::seeded_str`]: `name=acme`.
    Name(String),
}

impl fmt::Display for Base {
    /// `Slate`, `Seed 42` or `Name "acme"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Base::Preset(p) => f.write_str(&title_case(p.name())),
            Base::Seed(s) => write!(f, "Seed {s}"),
            Base::Name(n) => write!(f, "Name {n:?}"),
        }
    }
}

impl Base {
    fn theme(&self) -> Theme {
        match self {
            Base::Preset(p) => Theme::preset(*p),
            Base::Seed(s) => Theme::seeded(*s),
            Base::Name(n) => Theme::seeded_str(n),
        }
    }
}

/// Why a query string isn't a valid [`ThemeSpec`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SpecError {
    /// A key that isn't `preset`, `seed`, `name` or an option.
    UnknownKey(String),
    /// A preset name that doesn't exist.
    UnknownPreset(String),
    /// A seed that isn't a whole number from 0 to 2^64 − 1.
    InvalidSeed(String),
    /// A value that isn't one of the option's choices.
    UnknownValue {
        /// The option.
        option: &'static OptionInfo,
        /// The rejected value.
        value: String,
    },
    /// More than one of `preset`, `seed` and `name`.
    ConflictingBase,
}

impl fmt::Display for SpecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SpecError::UnknownKey(key) => {
                let keys: Vec<_> = OPTIONS.iter().map(|o| o.key).collect();
                write!(
                    f,
                    "unknown key `{key}`; expected preset, seed, name, {}",
                    keys.join(", ")
                )
            }
            SpecError::UnknownPreset(name) => {
                let names: Vec<_> = Preset::ALL.iter().map(|p| p.name()).collect();
                write!(
                    f,
                    "unknown preset `{name}`; expected one of {}",
                    names.join(", ")
                )
            }
            SpecError::InvalidSeed(seed) => write!(
                f,
                "invalid seed `{seed}`; expected a whole number from 0 to {}",
                u64::MAX
            ),
            SpecError::UnknownValue { option, value } => {
                let values: Vec<_> = option.choices.iter().map(|c| c.value).collect();
                write!(
                    f,
                    "unknown {} `{value}`; expected one of {}",
                    option.key,
                    values.join(", ")
                )
            }
            SpecError::ConflictingBase => f.write_str("give only one of preset, seed and name"),
        }
    }
}

impl std::error::Error for SpecError {}

/// A theme as a starting point ([`Base`]) plus option overrides.
///
/// [`ThemeSpec::to_query`] and [`str::parse`] convert to and from the query
/// form; [`ThemeSpec::theme`] builds the [`Theme`], and [`ThemeSpec::rust`]
/// writes the Rust code that builds it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemeSpec {
    base: Base,
    overrides: Vec<(&'static str, &'static str)>,
}

impl Default for ThemeSpec {
    /// The Slate preset with no overrides.
    fn default() -> Self {
        ThemeSpec::new(Base::Preset(Preset::Slate))
    }
}

impl FromStr for ThemeSpec {
    type Err = SpecError;

    /// Parses a query string strictly: unknown keys or values, a bad seed,
    /// or more than one base is an error. A leading `?` is ignored, and an
    /// empty string is the default spec.
    fn from_str(query: &str) -> Result<Self, SpecError> {
        let mut spec = ThemeSpec::default();
        let mut has_base = false;
        for (key, value) in pairs(query) {
            let base = match key.as_str() {
                "preset" => Base::Preset(
                    Preset::ALL
                        .into_iter()
                        .find(|p| p.name() == value)
                        .ok_or(SpecError::UnknownPreset(value))?,
                ),
                "seed" => Base::Seed(value.parse().map_err(|_| SpecError::InvalidSeed(value))?),
                "name" => Base::Name(value),
                _ => {
                    spec.set(&key, &value)?;
                    continue;
                }
            };
            if has_base {
                return Err(SpecError::ConflictingBase);
            }
            has_base = true;
            spec.base = base;
        }
        Ok(spec)
    }
}

impl ThemeSpec {
    /// `base` with no overrides.
    pub fn new(base: Base) -> ThemeSpec {
        ThemeSpec {
            base,
            overrides: Vec::new(),
        }
    }

    /// Parses a query string leniently, for input from a URL: unknown keys
    /// and values are skipped, so any input gives a usable spec. A seed
    /// wins over a name, and a name over a preset.
    pub fn parse_lossy(query: &str) -> ThemeSpec {
        let mut spec = ThemeSpec::default();
        let (mut seed, mut name) = (None, None);
        for (key, value) in pairs(query) {
            match key.as_str() {
                "preset" => {
                    if let Some(p) = Preset::ALL.into_iter().find(|p| p.name() == value) {
                        spec.base = Base::Preset(p);
                    }
                }
                "seed" => seed = value.parse::<u64>().ok().or(seed),
                "name" => name = Some(value),
                _ => {
                    let _ = spec.set(&key, &value);
                }
            }
        }
        if let Some(name) = name {
            spec.base = Base::Name(name);
        }
        if let Some(seed) = seed {
            spec.base = Base::Seed(seed);
        }
        spec
    }

    /// The starting theme.
    pub fn base(&self) -> &Base {
        &self.base
    }

    /// Replaces the starting theme, keeping the overrides.
    pub fn set_base(&mut self, base: Base) {
        self.base = base;
    }

    /// `(key, value)` for each override, in [`OPTIONS`] order.
    pub fn overrides(&self) -> &[(&'static str, &'static str)] {
        &self.overrides
    }

    /// Overrides option `key` with choice `value`, replacing any earlier
    /// override of it.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), SpecError> {
        let option = OptionInfo::find(key).ok_or_else(|| SpecError::UnknownKey(key.to_owned()))?;
        let choice = option
            .choice(value)
            .ok_or_else(|| SpecError::UnknownValue {
                option,
                value: value.to_owned(),
            })?;
        self.overrides.retain(|(k, _)| *k != option.key);
        self.overrides.push((option.key, choice.value));
        let order = |k: &str| OPTIONS.iter().position(|o| o.key == k);
        self.overrides.sort_by_key(|(k, _)| order(k));
        Ok(())
    }

    /// [`ThemeSpec::set`], by value.
    pub fn with(mut self, key: &str, value: &str) -> Result<ThemeSpec, SpecError> {
        self.set(key, value)?;
        Ok(self)
    }

    /// The canonical query string: the base, then overrides in [`OPTIONS`]
    /// order.
    pub fn to_query(&self) -> String {
        let mut out = match &self.base {
            Base::Preset(p) => format!("preset={}", p.name()),
            Base::Seed(s) => format!("seed={s}"),
            Base::Name(n) => format!("name={}", encode(n)),
        };
        for (k, v) in &self.overrides {
            let _ = write!(out, "&{k}={v}");
        }
        out
    }

    /// The theme this spec describes.
    pub fn theme(&self) -> Theme {
        self.overrides
            .iter()
            .fold(self.base.theme(), |theme, (key, value)| {
                apply(theme, key, value)
            })
    }

    /// Each option's effective choice, `(key, value)` in [`OPTIONS`] order:
    /// the override if there is one, otherwise the base's own choice.
    pub fn summary(&self) -> Vec<(&'static str, &'static str)> {
        let theme = self.theme();
        OPTIONS
            .iter()
            .map(|o| (o.key, current(&theme, o.key)))
            .collect()
    }

    /// Rust code that builds this theme and a `stucco::Bundle` from it.
    pub fn rust(&self) -> String {
        let mut types: Vec<&str> = vec!["Theme"];
        let mut chain = match &self.base {
            Base::Preset(p) => {
                types.push("Preset");
                format!("Theme::preset(Preset::{})", title_case(p.name()))
            }
            Base::Seed(s) => format!("Theme::seeded({s})"),
            Base::Name(n) => format!("Theme::seeded_str({n:?})"),
        };
        for (key, value) in &self.overrides {
            let option = OptionInfo::find(key).expect("overrides use known keys");
            let choice = option.choice(value).expect("overrides use known values");
            types.push(option.ty);
            let _ = write!(
                chain,
                "\n    .{}({}::{})",
                option.method, option.ty, choice.variant
            );
        }
        types.sort_unstable();
        types.dedup();
        let imports = match types.as_slice() {
            [one] => (*one).to_owned(),
            many => format!("{{{}}}", many.join(", ")),
        };
        format!(
            "use stucco::Bundle;\nuse stucco::theme::{imports};\n\n\
             // {}\nlet theme = {chain};\n\
             let bundle = Bundle::new(theme.build().expect(\"the theme passes its contrast checks\"));\n",
            self.to_query()
        )
    }
}

impl fmt::Display for ThemeSpec {
    /// The canonical query string.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_query())
    }
}

/// The light and dark value of every role in [`ROLES`], as pretty-printed
/// JSON: `{"light": {"bg": "oklch(…)", …}, "dark": {…}}`.
pub fn tokens_json(built: &BuiltTheme) -> String {
    let mut out = String::from("{\n");
    for (i, (name, scheme)) in [("light", Scheme::Light), ("dark", Scheme::Dark)]
        .into_iter()
        .enumerate()
    {
        let _ = writeln!(out, "  \"{name}\": {{");
        for (j, role) in ROLES.iter().enumerate() {
            let value = built
                .role(scheme, role)
                .map(|c| c.css())
                .unwrap_or_default();
            let comma = if j + 1 < ROLES.len() { "," } else { "" };
            let _ = writeln!(out, "    \"{role}\": \"{value}\"{comma}");
        }
        out.push_str(if i == 0 { "  },\n" } else { "  }\n" });
    }
    out.push('}');
    out
}

/// The `key=value` pairs of a query string, percent-decoded.
fn pairs(query: &str) -> impl Iterator<Item = (String, String)> + '_ {
    query
        .trim_start_matches('?')
        .split('&')
        .filter(|pair| !pair.is_empty())
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (decode(key), decode(value))
        })
}

/// Percent-decodes `s`, reading `+` as a space. Invalid escapes are kept.
fn decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 1 < bytes.len() => {
                match (hex(bytes[i + 1]), bytes.get(i + 2).copied().and_then(hex)) {
                    (Some(hi), Some(lo)) => {
                        out.push((hi * 16 + lo) as u8);
                        i += 2;
                    }
                    _ => out.push(b'%'),
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Percent-encodes everything but unreserved characters.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Applies one override to `theme`.
fn apply(theme: Theme, key: &str, value: &str) -> Theme {
    match (key, value) {
        ("radius", "sharp") => theme.radius(Radius::Sharp),
        ("radius", "soft") => theme.radius(Radius::Soft),
        ("radius", "round") => theme.radius(Radius::Round),
        ("density", "comfortable") => theme.density(Density::Comfortable),
        ("density", "compact") => theme.density(Density::Compact),
        ("elevation", "flat") => theme.elevation(Elevation::Flat),
        ("elevation", "outlined") => theme.elevation(Elevation::Outlined),
        ("elevation", "raised") => theme.elevation(Elevation::Raised),
        ("table", "lined") => theme.table_style(TableStyle::Lined),
        ("table", "striped") => theme.table_style(TableStyle::Striped),
        ("table", "open") => theme.table_style(TableStyle::Open),
        ("controls", "outlined") => theme.control_style(ControlStyle::Outlined),
        ("controls", "filled") => theme.control_style(ControlStyle::Filled),
        ("controls", "underlined") => theme.control_style(ControlStyle::Underlined),
        ("header", "bar") => theme.header_style(HeaderStyle::Bar),
        ("header", "plain") => theme.header_style(HeaderStyle::Plain),
        ("header", "tinted") => theme.header_style(HeaderStyle::Tinted),
        ("headings", "regular") => theme.heading_weight(HeadingWeight::Regular),
        ("headings", "bold") => theme.heading_weight(HeadingWeight::Bold),
        ("headings", "heavy") => theme.heading_weight(HeadingWeight::Heavy),
        ("buttons", "rounded") => theme.button_shape(ButtonShape::Rounded),
        ("buttons", "pill") => theme.button_shape(ButtonShape::Pill),
        ("links", "underlined") => theme.link_style(LinkStyle::Underlined),
        ("links", "subtle") => theme.link_style(LinkStyle::Subtle),
        ("links", "bold") => theme.link_style(LinkStyle::Bold),
        ("links", "highlight") => theme.link_style(LinkStyle::Highlight),
        ("nav", "soft") => theme.nav_style(NavStyle::Soft),
        ("nav", "solid") => theme.nav_style(NavStyle::Solid),
        ("nav", "bar") => theme.nav_style(NavStyle::Bar),
        ("focus", "ring") => theme.focus_style(FocusStyle::Ring),
        ("focus", "thick") => theme.focus_style(FocusStyle::Thick),
        ("focus", "snug") => theme.focus_style(FocusStyle::Snug),
        ("finish", "smooth") => theme.finish(Finish::Smooth),
        ("finish", "sand") => theme.finish(Finish::Sand),
        ("finish", "float") => theme.finish(Finish::Float),
        ("finish", "knockdown") => theme.finish(Finish::Knockdown),
        ("shell", "sidebar") => theme.shell_layout(ShellLayout::Sidebar),
        ("shell", "rail") => theme.shell_layout(ShellLayout::Rail),
        ("shell", "topbar") => theme.shell_layout(ShellLayout::Topbar),
        ("panels", "boxed") => theme.panel_style(PanelStyle::Boxed),
        ("panels", "ruled") => theme.panel_style(PanelStyle::Ruled),
        ("panels", "headed") => theme.panel_style(PanelStyle::Headed),
        ("corners", "even") => theme.corner_style(CornerStyle::Even),
        ("corners", "squircle") => theme.corner_style(CornerStyle::Squircle),
        ("corners", "bevel") => theme.corner_style(CornerStyle::Bevel),
        ("corners", "hand") => theme.corner_style(CornerStyle::Hand),
        ("lines", "fine") => theme.line_weight(LineWeight::Fine),
        ("lines", "heavy") => theme.line_weight(LineWeight::Heavy),
        ("depth", "flat") => theme.button_depth(ButtonDepth::Flat),
        ("depth", "raised") => theme.button_depth(ButtonDepth::Raised),
        ("depth", "offset") => theme.button_depth(ButtonDepth::Offset),
        ("labels", "plain") => theme.label_style(LabelStyle::Plain),
        ("labels", "caps") => theme.label_style(LabelStyle::Caps),
        ("labels", "strong") => theme.label_style(LabelStyle::Strong),
        ("icons", "light") => theme.icon_weight(IconWeight::Light),
        ("icons", "regular") => theme.icon_weight(IconWeight::Regular),
        ("icons", "bold") => theme.icon_weight(IconWeight::Bold),
        ("motion", "smooth") => theme.motion(Motion::Smooth),
        ("motion", "snappy") => theme.motion(Motion::Snappy),
        ("motion", "gentle") => theme.motion(Motion::Gentle),
        ("motion", "springy") => theme.motion(Motion::Springy),
        ("heading-font", "body") => theme.heading_font(HeadingFont::Body),
        ("heading-font", "serif") => theme.heading_font(HeadingFont::Serif),
        ("heading-font", "rounded") => theme.heading_font(HeadingFont::Rounded),
        ("heading-font", "mono") => theme.heading_font(HeadingFont::Mono),
        ("leading", "tight") => theme.leading(Leading::Tight),
        ("leading", "normal") => theme.leading(Leading::Normal),
        ("leading", "airy") => theme.leading(Leading::Airy),
        ("tags", "pill") => theme.tag_style(TagStyle::Pill),
        ("tags", "square") => theme.tag_style(TagStyle::Square),
        ("tags", "outline") => theme.tag_style(TagStyle::Outline),
        ("rules", "solid") => theme.rule_style(RuleStyle::Solid),
        ("rules", "dashed") => theme.rule_style(RuleStyle::Dashed),
        ("rules", "dotted") => theme.rule_style(RuleStyle::Dotted),
        ("material", "solid") => theme.material(Material::Solid),
        ("material", "glass") => theme.material(Material::Glass),
        ("material", "frost") => theme.material(Material::Frost),
        ("backdrop", "plain") => theme.backdrop(Backdrop::Plain),
        ("backdrop", "glow") => theme.backdrop(Backdrop::Glow),
        ("backdrop", "wash") => theme.backdrop(Backdrop::Wash),
        ("shadows", "soft") => theme.shadow_style(ShadowStyle::Soft),
        ("shadows", "layered") => theme.shadow_style(ShadowStyle::Layered),
        ("shadows", "tinted") => theme.shadow_style(ShadowStyle::Tinted),
        ("shadows", "hard") => theme.shadow_style(ShadowStyle::Hard),
        _ => theme,
    }
}

/// `theme`'s choice for option `key`.
fn current(theme: &Theme, key: &str) -> &'static str {
    let p = &theme.personality;
    match key {
        "radius" => match theme.radius {
            Radius::Sharp => "sharp",
            Radius::Soft => "soft",
            Radius::Round => "round",
        },
        "density" => match theme.density {
            Density::Comfortable => "comfortable",
            Density::Compact => "compact",
        },
        "elevation" => match p.elevation {
            Elevation::Flat => "flat",
            Elevation::Outlined => "outlined",
            Elevation::Raised => "raised",
        },
        "table" => match p.table {
            TableStyle::Lined => "lined",
            TableStyle::Striped => "striped",
            TableStyle::Open => "open",
        },
        "controls" => match p.controls {
            ControlStyle::Outlined => "outlined",
            ControlStyle::Filled => "filled",
            ControlStyle::Underlined => "underlined",
        },
        "header" => match p.header {
            HeaderStyle::Bar => "bar",
            HeaderStyle::Plain => "plain",
            HeaderStyle::Tinted => "tinted",
        },
        "headings" => match p.headings {
            HeadingWeight::Regular => "regular",
            HeadingWeight::Bold => "bold",
            HeadingWeight::Heavy => "heavy",
        },
        "buttons" => match p.buttons {
            ButtonShape::Rounded => "rounded",
            ButtonShape::Pill => "pill",
        },
        "links" => match p.links {
            LinkStyle::Underlined => "underlined",
            LinkStyle::Subtle => "subtle",
            LinkStyle::Bold => "bold",
            LinkStyle::Highlight => "highlight",
        },
        "nav" => match p.nav {
            NavStyle::Soft => "soft",
            NavStyle::Solid => "solid",
            NavStyle::Bar => "bar",
        },
        "focus" => match p.focus {
            FocusStyle::Ring => "ring",
            FocusStyle::Thick => "thick",
            FocusStyle::Snug => "snug",
        },
        "finish" => match p.finish {
            Finish::Smooth => "smooth",
            Finish::Sand => "sand",
            Finish::Float => "float",
            Finish::Knockdown => "knockdown",
        },
        "shell" => match p.shell {
            ShellLayout::Sidebar => "sidebar",
            ShellLayout::Rail => "rail",
            ShellLayout::Topbar => "topbar",
        },
        "panels" => match p.panels {
            PanelStyle::Boxed => "boxed",
            PanelStyle::Ruled => "ruled",
            PanelStyle::Headed => "headed",
        },
        "corners" => match p.corners {
            CornerStyle::Even => "even",
            CornerStyle::Squircle => "squircle",
            CornerStyle::Bevel => "bevel",
            CornerStyle::Hand => "hand",
        },
        "lines" => match p.lines {
            LineWeight::Fine => "fine",
            LineWeight::Heavy => "heavy",
        },
        "depth" => match p.depth {
            ButtonDepth::Flat => "flat",
            ButtonDepth::Raised => "raised",
            ButtonDepth::Offset => "offset",
        },
        "labels" => match p.labels {
            LabelStyle::Plain => "plain",
            LabelStyle::Caps => "caps",
            LabelStyle::Strong => "strong",
        },
        "icons" => match p.icons {
            IconWeight::Light => "light",
            IconWeight::Regular => "regular",
            IconWeight::Bold => "bold",
        },
        "motion" => match p.motion {
            Motion::Smooth => "smooth",
            Motion::Snappy => "snappy",
            Motion::Gentle => "gentle",
            Motion::Springy => "springy",
        },
        "heading-font" => match p.heading_font {
            HeadingFont::Body => "body",
            HeadingFont::Serif => "serif",
            HeadingFont::Rounded => "rounded",
            HeadingFont::Mono => "mono",
        },
        "leading" => match p.leading {
            Leading::Tight => "tight",
            Leading::Normal => "normal",
            Leading::Airy => "airy",
        },
        "tags" => match p.tags {
            TagStyle::Pill => "pill",
            TagStyle::Square => "square",
            TagStyle::Outline => "outline",
        },
        "rules" => match p.rules {
            RuleStyle::Solid => "solid",
            RuleStyle::Dashed => "dashed",
            RuleStyle::Dotted => "dotted",
        },
        "material" => match p.material {
            Material::Solid => "solid",
            Material::Glass => "glass",
            Material::Frost => "frost",
        },
        "backdrop" => match p.backdrop {
            Backdrop::Plain => "plain",
            Backdrop::Glow => "glow",
            Backdrop::Wash => "wash",
        },
        "shadows" => match p.shadows {
            ShadowStyle::Soft => "soft",
            ShadowStyle::Layered => "layered",
            ShadowStyle::Tinted => "tinted",
            ShadowStyle::Hard => "hard",
        },
        _ => unreachable!("only known keys are looked up"),
    }
}

/// `slate` → `Slate`.
fn title_case(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scope;
    use crate::seed::fnv1a;

    #[test]
    fn lossy_queries_parse_and_normalise() {
        let s = ThemeSpec::parse_lossy("?radius=round&seed=42&bogus=1&table=nope&elevation=raised");
        assert_eq!(s.base(), &Base::Seed(42));
        assert_eq!(
            s.overrides(),
            [("radius", "round"), ("elevation", "raised")]
        );
        assert_eq!(s.to_query(), "seed=42&radius=round&elevation=raised");
        assert_eq!(s.to_query().parse::<ThemeSpec>(), Ok(s));
        assert_eq!(
            ThemeSpec::parse_lossy("preset=pine").base(),
            &Base::Preset(Preset::Pine)
        );
        assert_eq!(ThemeSpec::parse_lossy("preset=nope"), ThemeSpec::default());
        assert_eq!(ThemeSpec::parse_lossy(""), ThemeSpec::default());
        assert_eq!(
            ThemeSpec::parse_lossy("name=acme&preset=pine").base(),
            &Base::Name("acme".into())
        );
    }

    #[test]
    fn strict_parsing_reports_each_mistake() {
        let err = |q: &str| q.parse::<ThemeSpec>().unwrap_err();
        assert_eq!(err("bogus=1"), SpecError::UnknownKey("bogus".into()));
        assert_eq!(err("preset=nope"), SpecError::UnknownPreset("nope".into()));
        assert_eq!(err("seed=-1"), SpecError::InvalidSeed("-1".into()));
        assert_eq!(err("seed=1&preset=sol"), SpecError::ConflictingBase);
        assert_eq!(
            err("table=nope").to_string(),
            "unknown table `nope`; expected one of lined, striped, open"
        );
        assert_eq!("".parse(), Ok(ThemeSpec::default()));
    }

    #[test]
    fn names_round_trip_through_percent_encoding() {
        let spec = ThemeSpec::new(Base::Name("Acme & Co.".into()));
        assert_eq!(spec.to_query(), "name=Acme%20%26%20Co.");
        assert_eq!(spec.to_query().parse(), Ok(spec.clone()));
        assert_eq!(ThemeSpec::parse_lossy("name=Acme+%26+Co."), spec);
        assert_eq!(spec.theme(), Theme::seeded_str("Acme & Co."));
        assert_eq!(decode("100%"), "100%");
        assert_eq!(decode("%zz%41"), "%zzA");
        assert_eq!(Base::Name("acme".into()).to_string(), "Name \"acme\"");
        assert_eq!(Base::Preset(Preset::Sol).to_string(), "Sol");
    }

    #[test]
    fn a_base_without_overrides_is_its_theme() {
        assert_eq!(ThemeSpec::parse_lossy("seed=7").theme(), Theme::seeded(7));
        assert_eq!(
            ThemeSpec::parse_lossy("preset=ember").theme(),
            Theme::preset(Preset::Ember)
        );
        assert_eq!(
            ThemeSpec::parse_lossy("name=acme").theme(),
            Theme::seeded(fnv1a("acme"))
        );
    }

    #[test]
    fn overrides_reach_the_css() {
        let css = ThemeSpec::parse_lossy("preset=slate&table=striped&buttons=pill")
            .theme()
            .build()
            .unwrap()
            .css(Scope::Root);
        assert!(css.contains("--st-table-stripe: var(--st-surface-raised);"));
        assert!(css.contains("--st-button-radius: var(--st-radius-full);"));
    }

    #[test]
    fn every_option_value_applies_and_round_trips() {
        for option in OPTIONS {
            let mut css = std::collections::HashSet::new();
            for choice in option.choices {
                let query = format!("preset=slate&{}={}", option.key, choice.value);
                let spec: ThemeSpec = query.parse().unwrap();
                assert_eq!(spec.to_query(), query);
                assert_eq!(current(&spec.theme(), option.key), choice.value);
                css.insert(spec.theme().build().unwrap().css(Scope::Root));
            }
            assert_eq!(
                css.len(),
                option.choices.len(),
                "{} choices differ",
                option.key
            );
        }
    }

    #[test]
    fn summaries_report_the_effective_choices() {
        for seed in 0..100 {
            let spec = ThemeSpec::parse_lossy(&format!("seed={seed}&density=compact"));
            let theme = spec.theme();
            for (key, value) in spec.summary() {
                assert_eq!(apply(theme.clone(), key, value), theme, "seed {seed} {key}");
            }
            assert!(spec.summary().contains(&("density", "compact")));
        }
        let slate = ThemeSpec::default().summary();
        assert!(slate.contains(&("elevation", "outlined")));
        assert_eq!(slate.len(), OPTIONS.len());
    }

    #[test]
    fn rust_snippets_name_every_type_they_use() {
        let rust = ThemeSpec::parse_lossy("seed=9&radius=round&table=open").rust();
        assert!(rust.contains("use stucco::theme::{Radius, TableStyle, Theme};"));
        assert!(rust.contains(
            "let theme = Theme::seeded(9)\n    .radius(Radius::Round)\n    .table_style(TableStyle::Open);"
        ));
        let rust = ThemeSpec::parse_lossy("preset=sol").rust();
        assert!(rust.contains("use stucco::theme::{Preset, Theme};"));
        assert!(rust.contains("Theme::preset(Preset::Sol);"));
        let rust = ThemeSpec::new(Base::Name("a\"b".into())).rust();
        assert!(rust.contains("use stucco::theme::Theme;\n"));
        assert!(rust.contains(r#"Theme::seeded_str("a\"b");"#));
    }

    #[test]
    fn every_role_resolves() {
        let built = Theme::preset(Preset::Slate).build().unwrap();
        for role in ROLES {
            assert!(built.role(Scheme::Light, role).is_some(), "{role}");
            assert!(built.role(Scheme::Dark, role).is_some(), "{role}");
        }
        let json = tokens_json(&built);
        assert!(json.starts_with("{\n  \"light\": {\n    \"bg\": \"oklch("));
        assert!(json.ends_with("  }\n}"));
    }
}
