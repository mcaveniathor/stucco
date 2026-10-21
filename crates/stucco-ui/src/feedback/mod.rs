//! Empty and status feedback.
//!
//! ```
//! use stucco_ui::feedback::{EmptyState, LiveRegion};
//! use stucco_core::{el, to_html};
//! let empty = EmptyState::new("No orders").description("Try another filter")
//!     .actions(el::a().href("/orders").text("Reset"));
//! assert!(to_html(&empty).contains("No orders"));
//! assert!(to_html(&LiveRegion::new().child("Showing 25 orders")).contains("aria-live"));
//! ```
mod flash;
mod readiness;
pub use flash::FlashMessages;
pub use readiness::{ReadinessPanel, Requirement};

use crate::{passthrough::apply, typography::Heading};
use stucco_core::{Attrs, Cx, Render, Slot, el};
/// Feedback styles.
pub static FEEDBACK: stucco_core::Asset = stucco_core::Asset {
    name: "feedback",
    css: Some(include_str!("../../css/feedback.css")),
    behavior: None,
    deps: &[],
};
stucco_core::register_asset!(FEEDBACK);
/// An actionable empty state.
#[derive(Debug)]
pub struct EmptyState<'a> {
    attrs: Attrs,
    title: String,
    description: Option<Slot<'a>>,
    actions: Option<Slot<'a>>,
}
impl<'a> EmptyState<'a> {
    /// Creates an empty state with an h2.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            attrs: Attrs::default(),
            title: title.into(),
            description: None,
            actions: None,
        }
    }
    /// Sets explanatory content.
    pub fn description(mut self, r: impl Render + 'a) -> Self {
        self.description = Some(Slot::new(r));
        self
    }
    /// Sets recovery actions.
    pub fn actions(mut self, r: impl Render + 'a) -> Self {
        self.actions = Some(Slot::new(r));
        self
    }
}
passthrough!(EmptyState<'_>);
impl Render for EmptyState<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FEEDBACK);
        apply(
            el::div()
                .class("st-empty-state")
                .child(Heading::new(2, &self.title))
                .child(self.description.as_ref())
                .child(self.actions.as_ref()),
            &self.attrs,
            &[],
        )
        .render(cx);
    }
}
/// A polite atomic status region; initial server text is a readable snapshot.
#[derive(Debug, Default)]
pub struct LiveRegion<'a> {
    attrs: Attrs,
    children: Vec<Slot<'a>>,
    assertive: bool,
}
impl<'a> LiveRegion<'a> {
    /// An empty status region.
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends status contents.
    pub fn child(mut self, r: impl Render + 'a) -> Self {
        self.children.push(Slot::new(r));
        self
    }
    /// Announces urgent errors with `role="alert"` and `aria-live="assertive"`.
    /// Initial server text remains readable without JavaScript; announcement of
    /// initial content depends on the browser and assistive technology.
    pub fn assertive(mut self) -> Self {
        self.assertive = true;
        self
    }
}
passthrough!(LiveRegion<'_>);
impl Render for LiveRegion<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FEEDBACK);
        apply(
            el::div()
                .attr("role", if self.assertive { "alert" } else { "status" })
                .aria(
                    "live",
                    if self.assertive {
                        "assertive"
                    } else {
                        "polite"
                    },
                )
                .aria("atomic", "true")
                .children(self.children.iter()),
            &self.attrs,
            &["role", "aria-live", "aria-atomic"],
        )
        .render(cx);
    }
}
