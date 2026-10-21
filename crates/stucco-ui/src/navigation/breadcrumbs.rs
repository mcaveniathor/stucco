use super::NAVIGATION;
use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Href, Render, el};

/// The trail from the top of the site to this page, for
/// [`PageHeader::breadcrumbs`](crate::app::PageHeader::breadcrumbs).
///
/// The current page is text marked `aria-current="page"`. The separators
/// are drawn by CSS, so screen readers don't read them.
///
/// ```
/// use stucco_ui::navigation::Breadcrumbs;
/// let trail = Breadcrumbs::new().link("Orders", "/orders").current("Order 42");
/// assert!(stucco_core::to_html(&trail).contains(r#"aria-current="page">Order 42<"#));
/// ```
#[derive(Debug)]
pub struct Breadcrumbs {
    attrs: Attrs,
    label: String,
    links: Vec<(String, Href)>,
    current: Option<String>,
}

impl Default for Breadcrumbs {
    fn default() -> Self {
        Breadcrumbs {
            attrs: Attrs::default(),
            label: "Breadcrumb".to_owned(),
            links: Vec::new(),
            current: None,
        }
    }
}

impl Breadcrumbs {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["aria-label"];

    /// An empty trail labelled "Breadcrumb".
    pub fn new() -> Self {
        Self::default()
    }

    /// A page above this one.
    pub fn link(mut self, label: impl Into<String>, href: impl Into<Href>) -> Self {
        self.links.push((label.into(), href.into()));
        self
    }

    /// This page, last in the trail.
    pub fn current(mut self, label: impl Into<String>) -> Self {
        self.current = Some(label.into());
        self
    }

    /// Names the landmark when a page has more than one trail.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

passthrough!(Breadcrumbs);

impl Render for Breadcrumbs {
    fn render(&self, cx: &mut Cx) {
        cx.require(&NAVIGATION);
        let links = self
            .links
            .iter()
            .map(|(label, href)| el::li().child(el::a().href(href.clone()).text(label.clone())));
        let current = self
            .current
            .as_ref()
            .map(|label| el::li().child(el::span().aria("current", "page").text(label.clone())));
        let nav = el::nav()
            .class("st-breadcrumbs")
            .aria("label", self.label.clone())
            .child(el::ol().children(links).child(current));
        apply(nav, &self.attrs, Self::RESERVED).render(cx);
    }
}
