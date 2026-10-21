use super::NAVIGATION;
use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Href, Render, el};

/// Where the user is in a multi-page flow, such as a checkout.
///
/// Finished steps link back; the current one is marked
/// `aria-current="step"`; later ones are text. Each step's state is in
/// hidden text as well as its style.
///
/// ```
/// use stucco_ui::navigation::Steps;
/// let steps = Steps::new("Checkout").step("Cart", "/cart").step("Pay", "/pay").current(1);
/// assert!(stucco_core::to_html(&steps).contains(r#"aria-current="step">Pay<"#));
/// ```
#[derive(Debug)]
pub struct Steps {
    attrs: Attrs,
    label: String,
    steps: Vec<(String, Option<Href>)>,
    current: usize,
}

impl Steps {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["aria-label"];

    /// A flow named `label`, starting at its first step.
    pub fn new(label: impl Into<String>) -> Self {
        Steps {
            attrs: Attrs::default(),
            label: label.into(),
            steps: Vec::new(),
            current: 0,
        }
    }

    /// A step that links back to `href` once it is done.
    pub fn step(mut self, label: impl Into<String>, href: impl Into<Href>) -> Self {
        self.steps.push((label.into(), Some(href.into())));
        self
    }

    /// A step that never links, even when done.
    pub fn step_disabled(mut self, label: impl Into<String>) -> Self {
        self.steps.push((label.into(), None));
        self
    }

    /// The zero-based current step; past the end means the last.
    pub fn current(mut self, index: usize) -> Self {
        self.current = index;
        self
    }
}

passthrough!(Steps);

impl Render for Steps {
    fn render(&self, cx: &mut Cx) {
        if self.steps.is_empty() {
            return;
        }
        cx.require(&NAVIGATION);
        let current = self.current.min(self.steps.len() - 1);
        let hidden = |text: &'static str| el::span().class("st-sr-only").text(text);
        let items = self.steps.iter().enumerate().map(|(i, (label, href))| {
            let marker = el::span()
                .class("st-steps-marker")
                .aria("hidden", "true")
                .text((i + 1).to_string());
            let (state, body) = if i < current {
                let body = match href {
                    Some(href) => el::a()
                        .href(href.clone())
                        .text(label.clone())
                        .child(hidden(" (completed)")),
                    None => el::span().text(label.clone()).child(hidden(" (completed)")),
                };
                ("done", body)
            } else if i == current {
                (
                    "current",
                    el::span().aria("current", "step").text(label.clone()),
                )
            } else {
                (
                    "todo",
                    el::span()
                        .text(label.clone())
                        .child(hidden(" (not started)")),
                )
            };
            el::li().data("state", state).child(marker).child(body)
        });
        let nav = el::nav()
            .class("st-steps")
            .aria("label", self.label.clone())
            .child(el::ol().children(items));
        apply(nav, &self.attrs, Self::RESERVED).render(cx);
    }
}
