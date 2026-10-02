//! URLs checked against an allow-list of schemes (spec §4.4).

/// A URL safe to place in `href`, `src` and similar attributes.
///
/// [`Href::new`] accepts relative URLs and the schemes `http`, `https`,
/// `mailto` and `tel`, after browser-style normalisation; anything else
/// (including `javascript:` and `data:`) becomes invalid and renders as `#`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Href(Option<String>);

impl Href {
    /// Checks `url` and keeps it if safe.
    pub fn new(url: impl Into<String>) -> Href {
        let url = url.into();
        if is_safe(&url) {
            Href(Some(url))
        } else {
            Href(None)
        }
    }

    /// Keeps `url` without checking. This is a trust decision.
    pub fn trusted(url: impl Into<String>) -> Href {
        Href(Some(url.into()))
    }

    /// An invalid URL, rendered as `#`.
    pub fn invalid() -> Href {
        Href(None)
    }

    /// Whether the URL passed the check (or was trusted).
    pub fn is_valid(&self) -> bool {
        self.0.is_some()
    }

    /// The URL, or `"#"` when invalid.
    pub fn as_str(&self) -> &str {
        self.0.as_deref().unwrap_or("#")
    }
}

impl From<&str> for Href {
    fn from(s: &str) -> Href {
        Href::new(s)
    }
}

impl From<String> for Href {
    fn from(s: String) -> Href {
        Href::new(s)
    }
}

/// How a multi-URL attribute separates its URLs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // used by the element builder (Task 4)
pub(crate) enum UrlList {
    /// `srcset`: comma-separated `url [descriptor]` candidates.
    Srcset,
    /// `ping`: whitespace-separated URLs.
    SpaceSeparated,
}

/// Keeps only the safe URLs of a multi-URL attribute value.
#[allow(dead_code)] // used by the element builder (Task 4)
pub(crate) fn filter_url_list(value: &str, kind: UrlList) -> String {
    match kind {
        UrlList::Srcset => value
            .split(',')
            .map(str::trim)
            .filter(|c| c.split_whitespace().next().is_some_and(is_safe))
            .collect::<Vec<_>>()
            .join(", "),
        UrlList::SpaceSeparated => value
            .split_whitespace()
            .filter(|u| is_safe(u))
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// Classifies `url` the way a browser would parse its scheme: leading
/// whitespace and C0 controls are ignored and tab/LF/CR are removed anywhere.
fn is_safe(url: &str) -> bool {
    let normalised: String = url
        .trim_start_matches(|c: char| c <= ' ')
        .chars()
        .filter(|c| !matches!(c, '\t' | '\n' | '\r'))
        .collect();
    match normalised.find([':', '/', '?', '#']) {
        Some(i) if normalised.as_bytes()[i] == b':' => {
            let scheme = &normalised[..i];
            ["http", "https", "mailto", "tel"]
                .iter()
                .any(|allowed| scheme.eq_ignore_ascii_case(allowed))
        }
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_urls_are_kept() {
        for ok in [
            "/a",
            "a/b",
            "../x",
            "?q=1",
            "#top",
            "/a:b",
            "?x=y:z",
            "https://e.com",
            "HTTP://e.com",
            "mailto:a@b.c",
            "tel:+1",
            "//cdn.example/x.js",
        ] {
            assert_eq!(Href::new(ok).as_str(), ok, "{ok}");
        }
    }

    #[test]
    fn unsafe_schemes_become_hash() {
        for bad in [
            "javascript:alert(1)",
            " JaVaScRiPt:alert(1)",
            "java\tscript:x",
            "\u{0}javascript:x",
            "data:text/html,<b>",
            "vbscript:x",
            "file:///etc",
        ] {
            assert_eq!(Href::new(bad).as_str(), "#", "{bad:?}");
        }
        assert_eq!(
            Href::trusted("javascript:void(0)").as_str(),
            "javascript:void(0)"
        );
    }

    #[test]
    fn url_lists_drop_unsafe_entries() {
        assert_eq!(
            filter_url_list("a.png 1x, javascript:x 2x, b.png 3x", UrlList::Srcset),
            "a.png 1x, b.png 3x"
        );
        assert_eq!(
            filter_url_list("/p1 javascript:x /p2", UrlList::SpaceSeparated),
            "/p1 /p2"
        );
    }
}
