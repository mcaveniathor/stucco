//! HTML escaping for text and attribute values.

/// Appends `s` to `out`, escaping `&`, `<` and `>` for use as HTML text.
pub fn escape_text(s: &str, out: &mut String) {
    escape(s, out, |b| match b {
        b'&' => Some("&amp;"),
        b'<' => Some("&lt;"),
        b'>' => Some("&gt;"),
        _ => None,
    });
}

/// Appends `s` to `out`, escaping `&`, `<`, `>`, `"` and `'` for use inside a
/// quoted attribute value.
pub fn escape_attr(s: &str, out: &mut String) {
    escape(s, out, |b| match b {
        b'&' => Some("&amp;"),
        b'<' => Some("&lt;"),
        b'>' => Some("&gt;"),
        b'"' => Some("&quot;"),
        b'\'' => Some("&#39;"),
        _ => None,
    });
}

/// Copies unescaped runs as slices; every replaced byte is ASCII, so slice
/// boundaries always fall on character boundaries.
fn escape(s: &str, out: &mut String, replace: impl Fn(u8) -> Option<&'static str>) {
    let mut start = 0;
    for (i, b) in s.bytes().enumerate() {
        if let Some(rep) = replace(b) {
            out.push_str(&s[start..i]);
            out.push_str(rep);
            start = i + 1;
        }
    }
    out.push_str(&s[start..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_escapes_markup_characters() {
        let mut s = String::new();
        escape_text("<a href=\"x\">&'</a>", &mut s);
        assert_eq!(s, "&lt;a href=\"x\"&gt;&amp;'&lt;/a&gt;");
    }

    #[test]
    fn attributes_also_escape_quotes() {
        let mut s = String::new();
        escape_attr("a\"b'c<d>&", &mut s);
        assert_eq!(s, "a&quot;b&#39;c&lt;d&gt;&amp;");
    }
}
