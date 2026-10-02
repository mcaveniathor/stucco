use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Render, el};

use super::NavLink;

/// Tabs across sections of one page, such as an order's Details, Items and
/// History. Each tab is a link to its own URL (`?tab=items`, or a path), and
/// the server renders the current one, so tabs work without script, can be
/// bookmarked and keep the back button.
///
/// ```
/// use stucco_ui::navigation::{NavLink, Tabs};
/// use stucco_core::to_html;
/// let html = to_html(
///     &Tabs::new("Order sections")
///         .tab(NavLink::new("Details", "?tab=details").current(true))
///         .tab(NavLink::new("History", "?tab=history")),
/// );
/// assert!(html.starts_with(r#"<nav class="st-tabs" aria-label="Order sections">"#));
/// assert!(html.contains(r#"aria-current="page">Details</a>"#));
/// ```
#[derive(Debug)]
pub struct Tabs {
    attrs: Attrs,
    label: String,
    tabs: Vec<NavLink>,
}

impl Tabs {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["aria-label"];

    /// Tabs named `label` for assistive technology, such as "Order sections".
    pub fn new(label: impl Into<String>) -> Self {
        Tabs {
            attrs: Attrs::default(),
            label: label.into(),
            tabs: Vec::new(),
        }
    }

    /// Adds a tab; mark the one being shown with `NavLink::current`.
    pub fn tab(mut self, tab: NavLink) -> Self {
        self.tabs.push(tab);
        self
    }
}

passthrough!(Tabs);

impl Render for Tabs {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::NAVIGATION);
        let nav = el::nav()
            .class("st-tabs")
            .aria("label", &self.label)
            .children(self.tabs.iter());
        apply(nav, &self.attrs, Self::RESERVED).render(cx);
    }
}
