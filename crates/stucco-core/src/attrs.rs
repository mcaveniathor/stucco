//! Attribute sets and the attribute safety policy (spec §4.4).

use crate::aria::ARIA_ATTRIBUTES;
use crate::href::{Href, UrlList, filter_url_list};
use crate::{Cx, escape::escape_attr};

/// A set of attributes: an id, classes and other attributes in insertion
/// order. Components keep one for passthrough attributes and apply it to
/// their root element.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attrs {
    pub(crate) id: Option<String>,
    pub(crate) classes: Vec<String>,
    /// `(name, Some(value))` or `(name, None)` for boolean attributes.
    pub(crate) list: Vec<(String, Option<String>)>,
    /// Names set through `trusted_attr`; merges re-check every other value.
    trusted: Vec<String>,
}

/// Attributes whose value is a single URL.
const URL_ATTRIBUTES: &[&str] = &[
    "href",
    "src",
    "action",
    "formaction",
    "poster",
    "cite",
    "xlink:href",
];

impl Attrs {
    /// Appends space-separated classes, skipping duplicates.
    pub fn class(mut self, classes: impl AsRef<str>) -> Attrs {
        for class in classes.as_ref().split_whitespace() {
            if !self.classes.iter().any(|c| c == class) {
                self.classes.push(class.to_owned());
            }
        }
        self
    }

    /// Sets the id, replacing any previous one.
    pub fn id(mut self, id: impl Into<String>) -> Attrs {
        self.id = Some(id.into());
        self
    }

    /// Sets an attribute under the safety policy: event handlers (`on*`) and
    /// `srcdoc` are refused, and URL-bearing attributes are checked.
    pub fn attr(mut self, name: &str, value: impl Into<String>) -> Attrs {
        self.set_checked(None, name, value.into());
        self
    }

    /// Sets any validly named attribute without the refusal or URL checks.
    /// The value is still escaped. This is a trust decision.
    pub fn trusted_attr(mut self, name: &str, value: impl Into<String>) -> Attrs {
        if valid_name(name) {
            self.set(name, Some(value.into()));
            if !self.trusted.iter().any(|t| t == name) {
                self.trusted.push(name.to_owned());
            }
        }
        self
    }

    /// Adds (`on = true`) or removes a boolean attribute.
    pub fn bool_attr(mut self, name: &str, on: bool) -> Attrs {
        if valid_name(name) && !refused(name) {
            if on {
                self.set(name, None);
            } else {
                self.list.retain(|(n, _)| n != name);
            }
        }
        self
    }

    /// Sets `data-{name}`; `name` must match `[a-z0-9-]+`.
    pub fn data(mut self, name: &str, value: impl Into<String>) -> Attrs {
        if valid_suffix(name) {
            self.set(&format!("data-{name}"), Some(value.into()));
        }
        self
    }

    /// Sets `aria-{name}`; `name` must be a WAI-ARIA 1.2 attribute.
    pub fn aria(mut self, name: &str, value: impl Into<String>) -> Attrs {
        if valid_suffix(name) {
            debug_assert!(
                ARIA_ATTRIBUTES.contains(&name),
                "unknown aria attribute: aria-{name}"
            );
            self.set(&format!("aria-{name}"), Some(value.into()));
        }
        self
    }

    /// Names in this set (including `id` and `class`) that appear in
    /// `reserved`, in insertion order.
    pub fn reserved_conflicts(&self, reserved: &[&str]) -> Vec<String> {
        let mut out = Vec::new();
        if self.id.is_some() && reserved.contains(&"id") {
            out.push("id".to_owned());
        }
        if !self.classes.is_empty() && reserved.contains(&"class") {
            out.push("class".to_owned());
        }
        out.extend(
            self.list
                .iter()
                .filter(|(n, _)| reserved.contains(&n.as_str()))
                .map(|(n, _)| n.clone()),
        );
        out
    }

    /// Merges `other` into this set: classes append, id and attributes replace.
    pub(crate) fn merge(&mut self, other: &Attrs) {
        if let Some(id) = &other.id {
            self.id = Some(id.clone());
        }
        for class in &other.classes {
            if !self.classes.contains(class) {
                self.classes.push(class.clone());
            }
        }
        for (name, value) in &other.list {
            self.set(name, value.clone());
        }
        for name in &other.trusted {
            if !self.trusted.contains(name) {
                self.trusted.push(name.clone());
            }
        }
    }

    /// Merges `other` onto an element's attributes, re-applying the policy
    /// with the element's tag to every value not set through `trusted_attr`
    /// (so passthrough cannot skip tag-specific checks such as `<object data>`).
    pub(crate) fn merge_for_tag(&mut self, other: &Attrs, tag: &str) {
        let checked = Attrs {
            list: Vec::new(),
            ..other.clone()
        };
        self.merge(&checked);
        for (name, value) in &other.list {
            match value {
                Some(v) if !other.trusted.contains(name) => {
                    self.set_checked(Some(tag), name, v.clone())
                }
                _ => self.set(name, value.clone()),
            }
        }
    }

    /// Applies the policy for `attr()`; `tag` enables element-specific rules
    /// (`data` is a URL on `<object>`).
    pub(crate) fn set_checked(&mut self, tag: Option<&str>, name: &str, value: String) {
        if !valid_name(name) || refused(name) {
            return;
        }
        let lower = name.to_ascii_lowercase();
        match lower.as_str() {
            "id" => self.id = Some(value),
            "class" => *self = std::mem::take(self).class(value),
            "srcset" => self.set(name, Some(filter_url_list(&value, UrlList::Srcset))),
            "ping" => self.set(name, Some(filter_url_list(&value, UrlList::SpaceSeparated))),
            "data" if tag == Some("object") => {
                self.set(name, Some(Href::new(value).as_str().to_owned()))
            }
            n if URL_ATTRIBUTES.contains(&n) => {
                self.set(name, Some(Href::new(value).as_str().to_owned()))
            }
            _ => self.set(name, Some(value)),
        }
    }

    fn set(&mut self, name: &str, value: Option<String>) {
        self.trusted.retain(|t| t != name);
        match self.list.iter_mut().find(|(n, _)| n == name) {
            Some(slot) => slot.1 = value,
            None => self.list.push((name.to_owned(), value)),
        }
    }

    /// Writes ` id="…" class="…" name="value" name` (claiming the id).
    pub(crate) fn render(&self, cx: &mut Cx) {
        let mut out = String::new();
        if let Some(id) = &self.id {
            cx.claim_id(id);
            out.push_str(" id=\"");
            escape_attr(id, &mut out);
            out.push('"');
        }
        if !self.classes.is_empty() {
            out.push_str(" class=\"");
            escape_attr(&self.classes.join(" "), &mut out);
            out.push('"');
        }
        for (name, value) in &self.list {
            out.push(' ');
            out.push_str(name);
            if let Some(value) = value {
                out.push_str("=\"");
                escape_attr(value, &mut out);
                out.push('"');
            }
        }
        cx.raw(&out);
    }
}

/// ASCII letter first, then letters, digits, `-`, `_`, `:`, `.`.
fn valid_name(name: &str) -> bool {
    let ok = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | ':' | '.'));
    debug_assert!(ok, "invalid attribute name: {name:?}");
    ok
}

/// `[a-z0-9-]+` for `data-*` and `aria-*` suffixes.
fn valid_suffix(name: &str) -> bool {
    let ok = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    debug_assert!(ok, "invalid attribute name: {name:?}");
    ok
}

/// Event handlers (`on*`) and HTML-bearing attributes, in
/// any case.
fn refused(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    let refused = lower.starts_with("on") || lower == "srcdoc";
    debug_assert!(!refused, "refused attribute: {name} (use trusted_attr)");
    refused
}
