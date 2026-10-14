//! Bounded, allowlisted return destinations for server workflows.
use crate::{CollectionQuery, Href};
use std::collections::BTreeMap;

/// A local collection destination carried through detail pages and forms.
///
/// Validate incoming values again on every request. This type constrains URLs;
/// it does not authorize access to a route or validate application filters.
/// Paths are canonical root-relative ASCII paths, without encoded segments,
/// dot segments, repeated slashes, backslashes or fragments.
///
/// ```
/// use stucco_core::ReturnContext;
/// let context = ReturnContext::parse("/orders?q=Ada&page=2", &["/orders"], &["q", "page"]).unwrap();
/// assert_eq!(context.as_str(), "/orders?page=2&q=Ada");
/// assert!(context.carry("/orders/42#edit").unwrap().as_str().ends_with("#edit"));
/// assert!(ReturnContext::parse("//example.com", &["/orders"], &[]).is_none());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReturnContext(String);

impl ReturnContext {
    /// Query/form field used to carry a destination.
    pub const FIELD: &'static str = "return_to";

    /// Validates a decoded destination against exact paths and query keys.
    /// Unknown query keys (including nested `return_to`) are discarded. The
    /// last occurrence of a permitted key wins. Invalid paths and values over
    /// 16 KiB are rejected; callers should choose a known local fallback.
    pub fn parse(raw: &str, paths: &[&str], keys: &[&str]) -> Option<Self> {
        if raw.len() > 16384 || raw.contains('#') || raw.chars().any(char::is_control) {
            return None;
        }
        let (path, query) = raw.split_once('?').unwrap_or((raw, ""));
        if !local_path(path) || !paths.contains(&path) {
            return None;
        }
        let fields: BTreeMap<_, _> = form_urlencoded::parse(query.as_bytes())
            .filter(|(key, _)| key != Self::FIELD && keys.contains(&key.as_ref()))
            .collect();
        let mut serializer = form_urlencoded::Serializer::new(String::new());
        serializer.extend_pairs(fields);
        let query = serializer.finish();
        let destination = if query.is_empty() {
            path.into()
        } else {
            format!("{path}?{query}")
        };
        (destination.len() <= 16384).then_some(Self(destination))
    }

    /// Builds context from a trusted collection path and its validated query.
    /// Incoming `return_to` values must instead pass through [`Self::parse`].
    pub fn collection(path: &str, query: &CollectionQuery) -> Option<Self> {
        if !local_path(path) {
            return None;
        }
        let href = query.link(&Href::new(path));
        let value = href.as_str();
        (value.len() <= 16384).then(|| Self(value.to_owned()))
    }

    /// The validated destination, suitable for a local redirect or hidden field.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The destination as a safe link.
    pub fn href(&self) -> Href {
        Href::new(self.0.clone())
    }

    /// Adds context to a local detail/edit/action link, replacing any previous
    /// `return_to` while retaining its other query parameters and fragment.
    /// Invalid, external or oversized links return `None`.
    pub fn carry(&self, link: &str) -> Option<Href> {
        if link.len() > 16384 || link.chars().any(char::is_control) {
            return None;
        }
        let (url, fragment) = link
            .split_once('#')
            .map_or((link, None), |(u, f)| (u, Some(f)));
        let (path, query) = url.split_once('?').unwrap_or((url, ""));
        if !local_path(path) {
            return None;
        }
        let mut serializer = form_urlencoded::Serializer::new(String::new());
        serializer.extend_pairs(
            form_urlencoded::parse(query.as_bytes()).filter(|(k, _)| k != Self::FIELD),
        );
        serializer.append_pair(Self::FIELD, &self.0);
        let mut result = format!("{path}?{}", serializer.finish());
        if let Some(fragment) = fragment {
            result.push('#');
            result.push_str(fragment);
        }
        Some(Href::new(result))
    }
}

fn local_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.contains("//")
        && path
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"/-_.~".contains(&c))
        && !path.split('/').any(|part| part == "." || part == "..")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_only_allowlisted_state_and_replaces_nested_context() {
        let context = ReturnContext::parse(
            "/orders?q=A%26B&page=2&page=3&secret=x&return_to=%2Fevil",
            &["/orders"],
            &["q", "page", "return_to"],
        )
        .unwrap();
        assert_eq!(context.as_str(), "/orders?page=3&q=A%26B");
        let carried = context
            .carry("/orders/42?tab=history&return_to=old#edit")
            .unwrap();
        assert_eq!(
            carried.as_str(),
            "/orders/42?tab=history&return_to=%2Forders%3Fpage%3D3%26q%3DA%2526B#edit"
        );
        let encoded = carried
            .as_str()
            .split_once('?')
            .unwrap()
            .1
            .split('#')
            .next()
            .unwrap();
        let decoded = form_urlencoded::parse(encoded.as_bytes())
            .find(|(k, _)| k == ReturnContext::FIELD)
            .unwrap()
            .1;
        assert_eq!(
            ReturnContext::parse(&decoded, &["/orders"], &["q", "page"]),
            Some(context)
        );
    }
    #[test]
    fn rejects_redirect_tricks_even_if_allowlisted() {
        for path in [
            "https://evil.test",
            "//evil.test",
            "/\\evil.test",
            "/%2f%2fevil",
            "/orders/../evil",
            "/./orders",
            " /orders",
            "/orders\n",
            "/orders#x",
            "/orders//x",
            "orders",
            "",
        ] {
            assert!(
                ReturnContext::parse(path, &[path], &[]).is_none(),
                "{path:?}"
            );
            assert!(
                ReturnContext::collection(path, &CollectionQuery::default()).is_none(),
                "{path:?}"
            );
        }
        assert!(ReturnContext::parse("/other", &["/orders"], &[]).is_none());
        assert!(
            ReturnContext::parse(
                &format!("/orders?q={}", "x".repeat(16384)),
                &["/orders"],
                &["q"]
            )
            .is_none()
        );
        let context = ReturnContext::parse("/orders", &["/orders"], &[]).unwrap();
        for link in [
            "//evil.test",
            "https://evil.test",
            "/orders/../evil",
            "/orders\r\nLocation: evil",
        ] {
            assert!(context.carry(link).is_none());
        }
    }
    #[test]
    fn canonicalization_stays_bounded_and_round_trips_unicode() {
        let c = ReturnContext::parse(
            "/orders?q=café&f.status=needs+review",
            &["/orders"],
            &["q", "f.status"],
        )
        .unwrap();
        assert_eq!(
            ReturnContext::parse(c.as_str(), &["/orders"], &["q", "f.status"]),
            Some(c)
        );
        assert!(
            ReturnContext::parse(
                &format!("/orders?q={}", "é".repeat(4000)),
                &["/orders"],
                &["q"]
            )
            .is_none()
        );
    }
    #[test]
    fn collection_query_keeps_sort_filters_and_page() {
        let q = CollectionQuery::default()
            .with_search("Ada")
            .with_sort("name", crate::Direction::Desc)
            .with_window(crate::Window::Offset { page: 3 });
        let c = ReturnContext::collection("/orders", &q).unwrap();
        assert_eq!(c.href(), q.link(&Href::new("/orders")));
        assert!(c.as_str().contains("page=3"));
    }
}
