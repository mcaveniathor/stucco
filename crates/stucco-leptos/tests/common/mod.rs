//! Shared helpers for comparing Leptos SSR output with Stucco's static renderer.

#![allow(dead_code)]

/// Reduces HTML to a form where equivalent markup compares equal: hydration
/// markers (`<!>`) and comments are dropped, attribute names are lowercased and sorted, and
/// whitespace between tags is removed. Self-closing tags are written as open tags,
/// and SVG shapes have no closing tag. Text content is left untouched.
pub fn normalize(html: &str) -> String {
    let mut out = String::new();
    let mut rest = html;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.find("-->").map_or("", |end| &after[end + 3..]);
        } else if let Some(after) = rest.strip_prefix("<!>") {
            rest = after;
        } else if rest.starts_with('<') {
            let end = tag_end(rest);
            out.push_str(&normalize_tag(&rest[..end]));
            rest = &rest[end..];
        } else {
            let end = rest.find('<').unwrap_or(rest.len());
            let text = &rest[..end];
            if !out.ends_with('>') || !text.trim().is_empty() {
                out.push_str(text);
            }
            rest = &rest[end..];
        }
    }
    out
}

/// Panics, showing both normalized strings, unless the markup is equivalent.
pub fn assert_parity(leptos: &str, stucco: &str) {
    let (leptos, stucco) = (normalize(leptos), normalize(stucco));
    assert!(
        leptos == stucco,
        "markup differs\n  leptos: {leptos}\n  stucco: {stucco}"
    );
}

/// Byte length of the tag at the start of `s`, honouring quoted attribute values.
fn tag_end(s: &str) -> usize {
    let mut quote = None;
    for (index, c) in s.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '>') => return index + 1,
            _ => {}
        }
    }
    s.len()
}

const VOID_SHAPES: [&str; 5] = ["path", "circle", "polyline", "line", "rect"];

fn normalize_tag(tag: &str) -> String {
    let inner = tag.trim_start_matches('<').trim_end_matches('>');
    let (closing, inner) = match inner.strip_prefix('/') {
        Some(inner) => (true, inner),
        None => (false, inner),
    };
    if closing {
        // Shapes are void in practice: `<path/>` and `<path></path>` are the same.
        let name = inner.trim();
        return if VOID_SHAPES.contains(&name) {
            String::new()
        } else {
            format!("</{name}>")
        };
    }
    let inner = inner.strip_suffix('/').unwrap_or(inner);
    let name_end = inner.find(char::is_whitespace).unwrap_or(inner.len());
    let (name, attrs) = inner.split_at(name_end);
    let mut attrs: Vec<String> = split_attributes(attrs)
        .into_iter()
        .map(|attr| match attr.split_once('=') {
            Some((name, value)) => format!("{}={value}", name.to_ascii_lowercase()),
            None => attr.to_ascii_lowercase(),
        })
        .collect();
    attrs.sort();
    let mut out = format!("<{name}");
    for attr in attrs {
        out.push(' ');
        out.push_str(&attr);
    }
    out.push('>');
    out
}

fn split_attributes(attrs: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    for c in attrs.chars() {
        match (quote, c) {
            (None, '"' | '\'') => {
                quote = Some(c);
                current.push(c);
            }
            (Some(q), c) if c == q => {
                quote = None;
                current.push(c);
            }
            (None, c) if c.is_whitespace() => {
                if !current.is_empty() {
                    result.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(c),
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    result
}

/// Renames the ids in `leptos` to the ids `stucco` generated, matching them in
/// order of appearance, so markup with generated ids can be compared. References
/// such as `for` and `aria-describedby` are renamed with them.
pub fn align_ids(leptos: &str, stucco: &str) -> String {
    let ids = |html: &str| -> Vec<String> {
        html.match_indices(" id=\"")
            .map(|(index, found)| {
                let start = index + found.len();
                html[start..].split('"').next().unwrap_or("").to_owned()
            })
            .collect()
    };
    let mut aligned = leptos.to_owned();
    for (from, to) in ids(leptos).into_iter().zip(ids(stucco)) {
        aligned = aligned.replace(&format!("\"{from}\""), &format!("\"{to}\""));
    }
    aligned
}
