//! The documentation site's theme engine: turns a playground query (a preset
//! or seed plus option overrides, parsed by [`stucco_theme::spec`]) into
//! token CSS, a Rust snippet and a JSON token export. Compiled to
//! WebAssembly so the playground uses exactly the code applications run.

use std::fmt::Write;

use stucco_theme::spec::{ThemeSpec, tokens_json};
use stucco_theme::{Preset, Scope, Theme};

#[cfg(target_arch = "wasm32")]
mod abi;

/// Everything the playground shows for `query`, as JSON:
/// `{query, label, rust, css, scoped, tokens, summary}`. `scope` names the
/// `data-st-theme` block in `scoped` (letters, digits and `-`; anything else
/// becomes `playground`).
pub fn generate_json(query: &str, scope: &str) -> String {
    let spec = ThemeSpec::parse_lossy(query);
    let scope = if !scope.is_empty()
        && scope
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        scope
    } else {
        "playground"
    };
    let built = spec
        .theme()
        .build()
        .unwrap_or_else(|_| Theme::preset(Preset::Slate).build().expect("Slate builds"));
    let summary: Vec<String> = spec
        .summary()
        .into_iter()
        .map(|(k, v)| format!("\"{k}\":\"{v}\""))
        .collect();
    format!(
        "{{\"query\":{},\"label\":{},\"rust\":{},\"css\":{},\"scoped\":{},\"tokens\":{},\"summary\":{{{}}}}}",
        json_string(&spec.to_query()),
        json_string(&spec.base().to_string()),
        json_string(&spec.rust()),
        json_string(&built.css(Scope::Root)),
        json_string(&built.css(Scope::Named(scope))),
        json_string(&tokens_json(&built)),
        summary.join(",")
    )
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
    fn json_is_one_escaped_line() {
        let json = generate_json("seed=3", "x\"y");
        assert!(json.starts_with("{\"query\":\"seed=3\",\"label\":\"Seed 3\""));
        assert!(json.contains("[data-st-theme=\\\"playground\\\"]"));
        assert!(json.contains("\"summary\":{\"radius\":\""));
        assert!(!json.contains('\n'));
        assert_eq!(json_string("a\"b\\c\n\u{1}"), "\"a\\\"b\\\\c\\n\\u0001\"");
    }
}
