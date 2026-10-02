//! The documentation site's theme engine: turns a playground configuration
//! (a preset or seed plus option overrides) into token CSS, a Rust snippet
//! and a JSON token export. Compiled to WebAssembly so the playground uses
//! exactly the code applications run.

use std::fmt::Write;

use stucco_theme::{
    BuiltTheme, ButtonShape, ControlStyle, Density, Elevation, HeaderStyle, HeadingWeight, Preset,
    Radius, Scheme, Scope, Seeded, TableStyle, Theme,
};

#[cfg(target_arch = "wasm32")]
mod abi;

/// One theme option the playground can override.
#[derive(Debug)]
pub struct OptionSpec {
    /// Query key.
    pub key: &'static str,
    /// Form label.
    pub label: &'static str,
    /// The option's Rust type.
    pub ty: &'static str,
    /// The `Theme` builder method.
    pub method: &'static str,
    /// `(query value, label, Rust variant)` for each choice.
    pub choices: &'static [(&'static str, &'static str, &'static str)],
}

/// Every overridable option, in display order.
pub const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        key: "radius",
        label: "Corners",
        ty: "Radius",
        method: "radius",
        choices: &[
            ("sharp", "Sharp", "Sharp"),
            ("soft", "Soft", "Soft"),
            ("round", "Round", "Round"),
        ],
    },
    OptionSpec {
        key: "density",
        label: "Density",
        ty: "Density",
        method: "density",
        choices: &[
            ("comfortable", "Comfortable", "Comfortable"),
            ("compact", "Compact", "Compact"),
        ],
    },
    OptionSpec {
        key: "elevation",
        label: "Elevation",
        ty: "Elevation",
        method: "elevation",
        choices: &[
            ("flat", "Flat", "Flat"),
            ("outlined", "Outlined", "Outlined"),
            ("raised", "Raised", "Raised"),
        ],
    },
    OptionSpec {
        key: "table",
        label: "Tables",
        ty: "TableStyle",
        method: "table_style",
        choices: &[
            ("lined", "Lined", "Lined"),
            ("striped", "Striped", "Striped"),
            ("open", "Open", "Open"),
        ],
    },
    OptionSpec {
        key: "controls",
        label: "Inputs",
        ty: "ControlStyle",
        method: "control_style",
        choices: &[
            ("outlined", "Outlined", "Outlined"),
            ("filled", "Filled", "Filled"),
            ("underlined", "Underlined", "Underlined"),
        ],
    },
    OptionSpec {
        key: "header",
        label: "Header",
        ty: "HeaderStyle",
        method: "header_style",
        choices: &[
            ("bar", "Bar", "Bar"),
            ("plain", "Plain", "Plain"),
            ("tinted", "Tinted", "Tinted"),
        ],
    },
    OptionSpec {
        key: "headings",
        label: "Headings",
        ty: "HeadingWeight",
        method: "heading_weight",
        choices: &[
            ("regular", "Regular", "Regular"),
            ("bold", "Bold", "Bold"),
            ("heavy", "Heavy", "Heavy"),
        ],
    },
    OptionSpec {
        key: "buttons",
        label: "Buttons",
        ty: "ButtonShape",
        method: "button_shape",
        choices: &[("rounded", "Rounded", "Rounded"), ("pill", "Pill", "Pill")],
    },
];

/// Semantic colour roles, for the JSON export.
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

/// Where a theme starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Base {
    /// A built-in preset.
    Preset(Preset),
    /// `Theme::seeded(seed)`.
    Seed(u64),
}

/// A playground configuration: a base and per-option overrides.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Config {
    /// The starting theme.
    pub base: Base,
    /// `(option key, choice value)`, in [`OPTIONS`] order.
    pub overrides: Vec<(&'static str, &'static str)>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            base: Base::Preset(Preset::Slate),
            overrides: Vec::new(),
        }
    }
}

impl Config {
    /// Parses `preset=slate` or `seed=42`, plus option keys such as
    /// `radius=round`. Unknown keys and values are ignored, so any input
    /// gives a usable theme; a seed wins over a preset.
    pub fn parse(query: &str) -> Config {
        let mut config = Config::default();
        let mut seed = None;
        for pair in query.trim_start_matches(['?', '#']).split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key {
                "preset" => {
                    if let Some(p) = Preset::ALL.into_iter().find(|p| p.name() == value) {
                        config.base = Base::Preset(p);
                    }
                }
                "seed" => seed = value.parse::<u64>().ok().or(seed),
                _ => {
                    if let Some(spec) = OPTIONS.iter().find(|o| o.key == key) {
                        if let Some((v, _, _)) = spec.choices.iter().find(|(v, _, _)| *v == value) {
                            config.overrides.retain(|(k, _)| *k != spec.key);
                            config.overrides.push((spec.key, v));
                        }
                    }
                }
            }
        }
        if let Some(seed) = seed {
            config.base = Base::Seed(seed);
        }
        let order = |k: &str| OPTIONS.iter().position(|o| o.key == k);
        config.overrides.sort_by_key(|(k, _)| order(k));
        config
    }

    /// The canonical query string for this configuration.
    pub fn to_query(&self) -> String {
        let mut out = match self.base {
            Base::Preset(p) => format!("preset={}", p.name()),
            Base::Seed(s) => format!("seed={s}"),
        };
        for (k, v) in &self.overrides {
            let _ = write!(out, "&{k}={v}");
        }
        out
    }

    /// The theme this configuration describes.
    pub fn theme(&self) -> Theme {
        let base = match self.base {
            Base::Preset(p) => Theme::preset(p),
            Base::Seed(s) => Theme::seeded(s),
        };
        self.overrides
            .iter()
            .fold(base, |theme, (key, value)| apply(theme, key, value))
    }

    /// Each option's effective choice: the override, the seed's choice, or
    /// `None` when a preset's built-in default applies.
    pub fn summary(&self) -> Vec<(&'static str, Option<&'static str>)> {
        OPTIONS
            .iter()
            .map(|o| {
                let chosen = self
                    .overrides
                    .iter()
                    .find(|(k, _)| *k == o.key)
                    .map(|(_, v)| *v);
                let seeded = match self.base {
                    Base::Seed(s) => Some(seeded_choice(o.key, s)),
                    Base::Preset(_) => None,
                };
                (o.key, chosen.or(seeded))
            })
            .collect()
    }

    /// Rust code that builds this theme and a bundle from it.
    pub fn rust(&self) -> String {
        let mut types: Vec<&str> = vec!["Theme"];
        let mut chain = match self.base {
            Base::Preset(p) => {
                types.push("Preset");
                format!("Theme::preset(Preset::{})", variant_name(p.name()))
            }
            Base::Seed(s) => format!("Theme::seeded({s})"),
        };
        for (key, value) in &self.overrides {
            let spec = OPTIONS.iter().find(|o| o.key == *key).expect("known key");
            let (_, _, variant) = spec
                .choices
                .iter()
                .find(|(v, _, _)| v == value)
                .expect("known value");
            types.push(spec.ty);
            let _ = write!(chain, "\n    .{}({}::{variant})", spec.method, spec.ty);
        }
        types.sort_unstable();
        types.dedup();
        format!(
            "use stucco::Bundle;\nuse stucco::theme::{{{}}};\n\n\
             // {}\nlet theme = {chain};\n\
             let bundle = Bundle::new(theme.build().expect(\"the theme passes its contrast checks\"));\n",
            types.join(", "),
            self.to_query()
        )
    }
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
        _ => theme,
    }
}

/// What `Theme::seeded(seed)` chooses for option `key`.
fn seeded_choice(key: &str, seed: u64) -> &'static str {
    match key {
        "radius" => match Radius::seeded(seed) {
            Radius::Sharp => "sharp",
            Radius::Soft => "soft",
            Radius::Round => "round",
        },
        "density" => match Density::seeded(seed) {
            Density::Comfortable => "comfortable",
            Density::Compact => "compact",
        },
        "elevation" => match Elevation::seeded(seed) {
            Elevation::Flat => "flat",
            Elevation::Outlined => "outlined",
            Elevation::Raised => "raised",
        },
        "table" => match TableStyle::seeded(seed) {
            TableStyle::Lined => "lined",
            TableStyle::Striped => "striped",
            TableStyle::Open => "open",
        },
        "controls" => match ControlStyle::seeded(seed) {
            ControlStyle::Outlined => "outlined",
            ControlStyle::Filled => "filled",
            ControlStyle::Underlined => "underlined",
        },
        "header" => match HeaderStyle::seeded(seed) {
            HeaderStyle::Bar => "bar",
            HeaderStyle::Plain => "plain",
            HeaderStyle::Tinted => "tinted",
        },
        "headings" => match HeadingWeight::seeded(seed) {
            HeadingWeight::Regular => "regular",
            HeadingWeight::Bold => "bold",
            HeadingWeight::Heavy => "heavy",
        },
        "buttons" => {
            // Seeded themes keep rounded buttons when their corners are sharp.
            if Radius::seeded(seed) == Radius::Sharp {
                "rounded"
            } else {
                match ButtonShape::seeded(seed) {
                    ButtonShape::Rounded => "rounded",
                    ButtonShape::Pill => "pill",
                }
            }
        }
        _ => "",
    }
}

/// `slate` → `Slate`.
fn variant_name(name: &str) -> String {
    let mut chars = name.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}

/// A display name for the base: `Slate` or `Seed 42`.
pub fn base_label(base: Base) -> String {
    match base {
        Base::Preset(p) => variant_name(p.name()),
        Base::Seed(s) => format!("Seed {s}"),
    }
}

/// Everything the playground shows for `query`, as JSON:
/// `{query, label, rust, css, scoped, tokens, summary}`. `scope` names the
/// `data-st-theme` block in `scoped` (letters, digits and `-`; anything else
/// becomes `playground`).
pub fn generate_json(query: &str, scope: &str) -> String {
    let config = Config::parse(query);
    let scope = if !scope.is_empty()
        && scope
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        scope
    } else {
        "playground"
    };
    let built = config
        .theme()
        .build()
        .unwrap_or_else(|_| Theme::preset(Preset::Slate).build().expect("Slate builds"));
    let tokens = tokens_json(&built);
    let summary: Vec<String> = config
        .summary()
        .into_iter()
        .map(|(k, v)| match v {
            Some(v) => format!("\"{k}\":\"{v}\""),
            None => format!("\"{k}\":null"),
        })
        .collect();
    format!(
        "{{\"query\":{},\"label\":{},\"rust\":{},\"css\":{},\"scoped\":{},\"tokens\":{},\"summary\":{{{}}}}}",
        json_string(&config.to_query()),
        json_string(&base_label(config.base)),
        json_string(&config.rust()),
        json_string(&built.css(Scope::Root)),
        json_string(&built.css(Scope::Named(scope))),
        json_string(&tokens),
        summary.join(",")
    )
}

/// The light and dark value of every role in [`ROLES`], as pretty JSON.
pub fn tokens_json(built: &BuiltTheme) -> String {
    let mut tokens = String::from("{\n");
    for (i, scheme) in [Scheme::Light, Scheme::Dark].into_iter().enumerate() {
        let name = if i == 0 { "light" } else { "dark" };
        let _ = writeln!(tokens, "  \"{name}\": {{");
        for (j, role) in ROLES.iter().enumerate() {
            let value = built
                .role(scheme, role)
                .map(|c| c.css())
                .unwrap_or_default();
            let comma = if j + 1 < ROLES.len() { "," } else { "" };
            let _ = writeln!(tokens, "    \"{role}\": \"{value}\"{comma}");
        }
        tokens.push_str(if i == 0 { "  },\n" } else { "  }\n" });
    }
    tokens.push('}');
    tokens
}

/// `s` as a JSON string literal.
fn json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queries_parse_and_normalise() {
        let c = Config::parse("?radius=round&seed=42&bogus=1&table=nope&elevation=raised");
        assert_eq!(c.base, Base::Seed(42));
        assert_eq!(c.overrides, [("radius", "round"), ("elevation", "raised")]);
        assert_eq!(c.to_query(), "seed=42&radius=round&elevation=raised");
        assert_eq!(Config::parse(&c.to_query()), c);
        assert_eq!(
            Config::parse("preset=pine").base,
            Base::Preset(Preset::Pine)
        );
        assert_eq!(Config::parse("preset=nope"), Config::default());
        assert_eq!(Config::parse(""), Config::default());
    }

    #[test]
    fn a_seed_without_overrides_is_the_seeded_theme() {
        assert_eq!(Config::parse("seed=7").theme(), Theme::seeded(7));
        assert_eq!(
            Config::parse("preset=ember").theme(),
            Theme::preset(Preset::Ember)
        );
    }

    #[test]
    fn overrides_reach_the_css() {
        let css = Config::parse("preset=slate&table=striped&buttons=pill")
            .theme()
            .build()
            .unwrap()
            .css(Scope::Root);
        assert!(css.contains("--st-table-stripe: var(--st-surface-raised);"));
        assert!(css.contains("--st-button-radius: var(--st-radius-full);"));
    }

    #[test]
    fn every_option_value_applies_and_round_trips() {
        for spec in OPTIONS {
            let mut css = std::collections::HashSet::new();
            for (value, _, _) in spec.choices {
                let query = format!("preset=slate&{}={value}", spec.key);
                let config = Config::parse(&query);
                assert_eq!(config.to_query(), query);
                css.insert(config.theme().build().unwrap().css(Scope::Root));
            }
            assert_eq!(css.len(), spec.choices.len(), "{} choices differ", spec.key);
        }
    }

    #[test]
    fn summaries_report_seeded_choices_and_overrides() {
        for seed in 0..100 {
            let config = Config::parse(&format!("seed={seed}&density=compact"));
            let theme = config.theme();
            for (key, value) in config.summary() {
                let value = value.expect("seeded options are known");
                let expected = apply(Theme::seeded(seed), key, value);
                let expected = if key == "density" {
                    expected
                } else {
                    expected.density(Density::Compact)
                };
                assert_eq!(theme, expected, "seed {seed} {key}={value}");
            }
        }
        assert!(
            Config::parse("preset=slate")
                .summary()
                .iter()
                .all(|(_, v)| v.is_none())
        );
    }

    #[test]
    fn rust_snippets_name_every_type_they_use() {
        let rust = Config::parse("seed=9&radius=round&table=open").rust();
        assert!(rust.contains("use stucco::theme::{Radius, TableStyle, Theme};"));
        assert!(rust.contains(
            "let theme = Theme::seeded(9)\n    .radius(Radius::Round)\n    .table_style(TableStyle::Open);"
        ));
        let rust = Config::parse("preset=sol").rust();
        assert!(rust.contains("use stucco::theme::{Preset, Theme};"));
        assert!(rust.contains("Theme::preset(Preset::Sol);"));
    }

    #[test]
    fn every_role_resolves_and_json_is_escaped() {
        let built = Theme::preset(Preset::Slate).build().unwrap();
        for role in ROLES {
            assert!(built.role(Scheme::Light, role).is_some(), "{role}");
        }
        let json = generate_json("seed=3", "x\"y");
        assert!(json.starts_with("{\"query\":\"seed=3\",\"label\":\"Seed 3\""));
        assert!(json.contains("[data-st-theme=\\\"playground\\\"]"));
        assert!(!json.contains('\n'));
        assert_eq!(json_string("a\"b\\c\n\u{1}"), "\"a\\\"b\\\\c\\n\\u0001\"");
    }
}
