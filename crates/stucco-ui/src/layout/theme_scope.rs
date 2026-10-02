use stucco_core::{Attrs, Cx, Render, Slot};

use super::{LAYOUT, Tag};
use crate::ColorScheme;
use crate::passthrough::apply;

/// Repaints its subtree in a forced colour scheme and/or a named theme
/// registered with `Bundle::with_theme`.
///
/// ```
/// use stucco_ui::{ColorScheme, layout::ThemeScope};
/// let html = stucco_core::to_html(&ThemeScope::new().scheme(ColorScheme::Dark).child("x"));
/// assert!(html.contains(r#"data-theme="dark""#));
/// ```
#[derive(Debug, Default)]
pub struct ThemeScope<'a> {
    attrs: Attrs,
    tag: Tag,
    scheme: Option<ColorScheme>,
    named: Option<&'static str>,
    children: Vec<Slot<'a>>,
}

impl ThemeScope<'_> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-theme", "data-st-theme"];

    /// A scope that only sets the theme's background and text colours.
    pub fn new() -> Self {
        Self::default()
    }

    /// Forces light or dark for the subtree.
    pub fn scheme(mut self, scheme: ColorScheme) -> Self {
        self.scheme = Some(scheme);
        self
    }

    /// Applies a named theme; `name` must match `[a-z0-9-]+` (debug panic
    /// "invalid theme name", ignored in release).
    pub fn named(mut self, name: &'static str) -> Self {
        let valid = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        debug_assert!(valid, "invalid theme name: {name:?}");
        if valid {
            self.named = Some(name);
        }
        self
    }
}

container_methods!(ThemeScope);

impl Render for ThemeScope<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let mut el = self.tag.element().class("st-theme-scope");
        if let Some(s) = self.scheme {
            el = el.data("theme", s.as_str());
        }
        if let Some(n) = self.named {
            el = el.data("st-theme", n);
        }
        apply(
            el.children(self.children.iter()),
            &self.attrs,
            Self::RESERVED,
        )
        .render(cx);
    }
}
