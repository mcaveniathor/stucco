use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Href, Render, el};

/// The trail from a section to the current page, in a `<nav>` named
/// "Breadcrumb". Earlier steps are links; the last is the current page,
/// marked `aria-current="page"` and not linked.
///
/// Label each step with the page's name ("Order 42"), not a generic word
/// ("Details"), so the trail makes sense when read out of context.
/// Separators are drawn by CSS, so screen readers don't announce them.
///
/// ```
/// use stucco_core::to_html;
/// use stucco_ui::navigation::Breadcrumbs;
///
/// let html = to_html(&Breadcrumbs::new().link("Orders", "/orders").current("Order 42"));
/// assert!(html.contains(r#"<nav class="st-breadcrumbs" aria-label="Breadcrumb">"#));
/// assert!(html.contains(r#"<li><a href="/orders">Orders</a></li>"#));
/// assert!(html.contains(r#"<li><span aria-current="page">Order 42</span></li>"#));
/// ```
#[derive(Clone, Debug, Default)]
pub struct Breadcrumbs {
    attrs: Attrs,
    steps: Vec<(String, Option<Href>)>,
    label: Option<String>,
}

impl Breadcrumbs {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["aria-label"];

    /// An empty trail.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a linked step.
    pub fn link(mut self, label: impl Into<String>, href: impl Into<Href>) -> Self {
        self.steps.push((label.into(), Some(href.into())));
        self
    }

    /// Adds the current page, which ends the trail.
    pub fn current(mut self, label: impl Into<String>) -> Self {
        self.steps.push((label.into(), None));
        self
    }

    /// Names the navigation landmark (default "Breadcrumb").
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }
}

passthrough!(Breadcrumbs);

impl Render for Breadcrumbs {
    fn render(&self, cx: &mut Cx) {
        if self.steps.is_empty() {
            return;
        }
        cx.require(&super::NAVIGATION);
        let list = el::ol().children(self.steps.iter().map(|(label, href)| {
            el::li().child(match href {
                Some(href) => el::a().href(href.clone()).text(label),
                None => el::span().aria("current", "page").text(label),
            })
        }));
        let nav = el::nav()
            .class("st-breadcrumbs")
            .aria("label", self.label.as_deref().unwrap_or("Breadcrumb"))
            .child(list);
        apply(nav, &self.attrs, Self::RESERVED).render(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::to_html;

    #[test]
    fn an_empty_trail_renders_nothing_and_labels_escape() {
        assert_eq!(to_html(&Breadcrumbs::new()), "");
        let html = to_html(
            &Breadcrumbs::new()
                .label("You are here")
                .link("A & B", "javascript:alert(1)")
                .current("<C>"),
        );
        assert_eq!(
            html,
            concat!(
                r#"<nav class="st-breadcrumbs" aria-label="You are here"><ol>"#,
                r##"<li><a href="#">A &amp; B</a></li>"##,
                r#"<li><span aria-current="page">&lt;C&gt;</span></li></ol></nav>"#
            )
        );
    }
}
