//! The element builder: one function per HTML element (spec §4.4).

use crate::{Attrs, Cx, Href, Render, Slot};

mod tags;
#[cfg(test)]
mod tests;

pub use tags::*;

/// An HTML element with children.
#[derive(Debug)]
pub struct Element<'a> {
    tag: &'static str,
    custom: Option<String>,
    attrs: Attrs,
    children: Vec<Slot<'a>>,
}

/// A void HTML element (no children, no closing tag).
#[derive(Debug, Clone)]
pub struct VoidElement {
    tag: &'static str,
    attrs: Attrs,
}

/// A custom element (`st-tabs`). The name must be lowercase ASCII, start with
/// a letter and contain a hyphen; an invalid name panics in debug builds and
/// renders a `div` in release builds.
pub fn custom<'a>(tag: &str) -> Element<'a> {
    let valid = tag.chars().next().is_some_and(|c| c.is_ascii_lowercase())
        && tag.contains('-')
        && tag
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '.' | '_'));
    debug_assert!(valid, "invalid custom element name: {tag:?}");
    let mut el = Element::new("div");
    if valid {
        el.custom = Some(tag.to_owned());
    }
    el
}

impl<'a> Element<'a> {
    pub(crate) fn new(tag: &'static str) -> Element<'a> {
        Element {
            tag,
            custom: None,
            attrs: Attrs::default(),
            children: Vec::new(),
        }
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Render + 'a) -> Element<'a> {
        self.children.push(Slot::new(child));
        self
    }

    /// Appends a child when `cond` holds.
    pub fn child_if(self, cond: bool, child: impl Render + 'a) -> Element<'a> {
        if cond { self.child(child) } else { self }
    }

    /// Appends every item as a child.
    pub fn children<R: Render + 'a>(mut self, items: impl IntoIterator<Item = R>) -> Element<'a> {
        self.children.extend(items.into_iter().map(Slot::new));
        self
    }

    /// Appends text. It is HTML-escaped, except inside `<script>` and
    /// `<style>`, where it is written as-is with `</` neutralised as `<\/` so
    /// the element cannot be closed early. Script content is code: never put
    /// untrusted text there.
    pub fn text(self, text: impl Into<String>) -> Element<'a> {
        let text = text.into();
        if matches!(self.tag, "script" | "style") && self.custom.is_none() {
            self.child(crate::Raw::trusted(text.replace("</", r"<\/")))
        } else {
            self.child(text)
        }
    }

    fn tag_name(&self) -> &str {
        self.custom.as_deref().unwrap_or(self.tag)
    }
}

impl VoidElement {
    pub(crate) fn new(tag: &'static str) -> VoidElement {
        VoidElement {
            tag,
            attrs: Attrs::default(),
        }
    }

    fn tag_name(&self) -> &str {
        self.tag
    }
}

macro_rules! attr_methods {
    ($ty:ty) => {
        impl $ty {
            /// Appends space-separated classes, skipping duplicates.
            pub fn class(mut self, classes: impl AsRef<str>) -> Self {
                self.attrs = self.attrs.class(classes);
                self
            }
            /// Sets the id, replacing any previous one.
            pub fn id(mut self, id: impl Into<String>) -> Self {
                self.attrs = self.attrs.id(id);
                self
            }
            /// Sets an attribute under the safety policy.
            pub fn attr(mut self, name: &str, value: impl Into<String>) -> Self {
                let tag = self.tag_name().to_owned();
                self.attrs.set_checked(Some(&tag), name, value.into());
                self
            }
            /// Sets any validly named attribute without refusal or URL checks.
            pub fn trusted_attr(mut self, name: &str, value: impl Into<String>) -> Self {
                self.attrs = self.attrs.trusted_attr(name, value);
                self
            }
            /// Adds or removes a boolean attribute.
            pub fn bool_attr(mut self, name: &str, on: bool) -> Self {
                self.attrs = self.attrs.bool_attr(name, on);
                self
            }
            /// Sets `data-{name}`.
            pub fn data(mut self, name: &str, value: impl Into<String>) -> Self {
                self.attrs = self.attrs.data(name, value);
                self
            }
            /// Sets `aria-{name}`.
            pub fn aria(mut self, name: &str, value: impl Into<String>) -> Self {
                self.attrs = self.attrs.aria(name, value);
                self
            }
            /// Sets `href`.
            pub fn href(self, href: impl Into<Href>) -> Self {
                self.trusted_attr("href", href.into().as_str())
            }
            /// Sets `src`.
            pub fn src(self, src: impl Into<Href>) -> Self {
                self.trusted_attr("src", src.into().as_str())
            }
            /// Sets `action`.
            pub fn action(self, action: impl Into<Href>) -> Self {
                self.trusted_attr("action", action.into().as_str())
            }
            /// Merges `attrs`: classes append, id and other attributes replace.
            pub fn attrs(mut self, attrs: &Attrs) -> Self {
                let tag = self.tag_name().to_owned();
                self.attrs.merge_for_tag(attrs, &tag);
                self
            }
        }
    };
}
attr_methods!(Element<'_>);
attr_methods!(VoidElement);

impl Render for Element<'_> {
    fn render(&self, cx: &mut Cx) {
        let tag = self.tag_name();
        cx.raw("<");
        cx.raw(tag);
        self.attrs.render(cx);
        cx.raw(">");
        for child in &self.children {
            child.render(cx);
        }
        cx.raw("</");
        cx.raw(tag);
        cx.raw(">");
    }
}

impl Render for VoidElement {
    fn render(&self, cx: &mut Cx) {
        cx.raw("<");
        cx.raw(self.tag);
        self.attrs.render(cx);
        cx.raw(">");
    }
}
