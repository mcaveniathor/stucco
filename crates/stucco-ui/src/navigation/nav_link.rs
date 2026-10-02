use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Href, Render, el};

/// A navigation link that can mark the current page.
///
/// Use it in `AppShell::link` for the primary navigation, or in your own
/// `<nav>` in a sidebar. The current page gets `aria-current="page"`, which
/// the theme's nav style draws.
///
/// ```
/// use stucco_ui::navigation::NavLink;
/// use stucco_core::to_html;
/// assert_eq!(
///     to_html(&NavLink::new("Orders", "/orders").current(true)),
///     r#"<a href="/orders" aria-current="page">Orders</a>"#
/// );
/// ```
#[derive(Clone, Debug)]
pub struct NavLink {
    attrs: Attrs,
    label: String,
    href: Href,
    current: bool,
}

impl NavLink {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["href", "aria-current"];

    /// A link labelled `label` to `href` (unsafe URLs render as `#`).
    pub fn new(label: impl Into<String>, href: impl Into<Href>) -> Self {
        NavLink {
            attrs: Attrs::default(),
            label: label.into(),
            href: href.into(),
            current: false,
        }
    }

    /// Marks the link as the current page.
    pub fn current(mut self, current: bool) -> Self {
        self.current = current;
        self
    }
}

passthrough!(NavLink);

impl Render for NavLink {
    fn render(&self, cx: &mut Cx) {
        let a = el::a().href(self.href.clone()).text(&self.label);
        let a = if self.current {
            a.aria("current", "page")
        } else {
            a
        };
        apply(a, &self.attrs, Self::RESERVED).render(cx);
    }
}
