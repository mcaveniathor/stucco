//! The base stylesheet layers and the rules every component stylesheet obeys
//! (spec §5.4).

/// The cascade layer order statement.
pub const LAYERS_CSS: &str = include_str!("../css/layers.css");
/// The `stucco.reset` layer.
pub const RESET_CSS: &str = include_str!("../css/reset.css");
/// The `stucco.base` layer.
pub const BASE_CSS: &str = include_str!("../css/base.css");

const LAYERS: &[&str] = &[
    "stucco.reset",
    "stucco.tokens",
    "stucco.base",
    "stucco.layout",
    "stucco.components",
    "stucco.utilities",
];
const PALETTES: &[&str] = &["neutral", "accent", "success", "warning", "danger", "info"];
const COLOUR_FUNCTIONS: &[&str] = &["oklch(", "rgb(", "rgba(", "hsl(", "hsla("];

/// Checks a stylesheet against the component CSS rules: no colour literals,
/// only semantic colour tokens (never palette steps), and everything inside
/// a `stucco.*` cascade layer.
pub fn check_component_css(name: &str, css: &str) -> Result<(), String> {
    let css = strip_comments(css);
    for f in COLOUR_FUNCTIONS {
        if css.contains(f) {
            return Err(format!("{name}: colour literal `{f}`"));
        }
    }
    if let Some(hex) = hex_colour(&css) {
        return Err(format!("{name}: colour literal `#{hex}`"));
    }
    for (i, _) in css.match_indices("var(--st-") {
        let token: String = css[i + 9..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
            .collect();
        if let Some((palette, step)) = token.rsplit_once('-') {
            if PALETTES.contains(&palette) && step.parse::<u8>().is_ok() {
                return Err(format!("{name}: palette step `--st-{token}`; use a role"));
            }
        }
    }
    check_layers(name, &css)
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = rest[start + 2..]
            .find("*/")
            .map_or("", |end| &rest[start + 2 + end + 2..]);
    }
    out.push_str(rest);
    out
}

/// `#` followed by 3, 4, 6 or 8 hex digits and then a non-identifier character.
fn hex_colour(css: &str) -> Option<String> {
    for (i, _) in css.match_indices('#') {
        let hex: String = css[i + 1..]
            .chars()
            .take_while(|c| c.is_ascii_hexdigit())
            .collect();
        let next = css[i + 1 + hex.len()..].chars().next().unwrap_or(' ');
        if matches!(hex.len(), 3 | 4 | 6 | 8)
            && !(next.is_ascii_alphanumeric() || next == '-' || next == '_')
        {
            return Some(hex);
        }
    }
    None
}

/// Every top-level item must be an `@layer stucco.<layer> { … }` block.
fn check_layers(name: &str, css: &str) -> Result<(), String> {
    let mut rest = css.trim_start();
    while !rest.is_empty() {
        let Some(after) = rest.strip_prefix("@layer") else {
            return Err(format!("{name}: content outside a stucco.* layer"));
        };
        let open = after
            .find('{')
            .ok_or_else(|| format!("{name}: @layer without a block"))?;
        let layer = after[..open].trim();
        if !LAYERS.contains(&layer) {
            return Err(format!("{name}: unknown layer `{layer}`"));
        }
        let mut depth = 0usize;
        let mut end = None;
        for (i, c) in after[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(open + i + 1);
                        break;
                    }
                }
                _ => {}
            }
        }
        let end = end.ok_or_else(|| format!("{name}: unbalanced braces"))?;
        rest = after[end..].trim_start();
    }
    Ok(())
}
