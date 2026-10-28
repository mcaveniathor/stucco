//! URL safety shared by link-bearing components.

/// Returns `url` when it is relative or uses `http`, `https`, `mailto` or `tel`;
/// any other scheme, such as `javascript:`, becomes `"#"`.
///
/// This applies the same rule as `stucco_core::Href`, which the WASM build does
/// not link: leading control characters are ignored, tabs and line breaks are
/// removed before the scheme is read, and scheme names are case-insensitive.
pub(crate) fn safe_href(url: &str) -> String {
    let normalised: String = url
        .trim_start_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    let allowed = match normalised.find([':', '/', '?', '#']) {
        Some(i) if normalised.as_bytes()[i] == b':' => {
            let scheme = &normalised[..i];
            ["http", "https", "mailto", "tel"]
                .iter()
                .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
        }
        _ => true,
    };
    if allowed {
        url.to_owned()
    } else {
        "#".to_owned()
    }
}
