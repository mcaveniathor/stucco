use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, FormState, Href, Render, el};

/// A focusable revision conflict explanation with an explicit reload link.
/// Renders nothing unless [`FormState::with_conflict`] was called. Bind the
/// fields to the same state and keep its submitted revision in the form.
/// Reloading is a normal GET that discards the local edits; no script replaces
/// field values or silently updates the revision. Use `.focus(false)` when an
/// [`super::ErrorSummary`] already takes focus on the same page.
#[derive(Debug)]
pub struct ConflictNotice<'s> {
    attrs: Attrs,
    state: &'s FormState,
    reload: Href,
    focus: bool,
}
impl<'s> ConflictNotice<'s> {
    /// Creates a notice whose link deliberately loads the latest saved record.
    pub fn new(state: &'s FormState, reload: impl Into<Href>) -> Self {
        Self {
            attrs: Attrs::default(),
            state,
            reload: reload.into(),
            focus: true,
        }
    }
    /// Whether the notice takes focus on page load (default true).
    pub fn focus(mut self, focus: bool) -> Self {
        self.focus = focus;
        self
    }
}
passthrough!(ConflictNotice<'_>);
impl Render for ConflictNotice<'_> {
    fn render(&self, cx: &mut Cx) {
        if self.state.conflict().is_none() {
            return;
        }
        cx.require(&super::FORMS);
        let id = cx.id("conflict");
        apply(el::div().class("st-conflict-notice").attr("role", "group")
            .aria("labelledby", &id).attr("tabindex", "-1").bool_attr("autofocus", self.focus)
            .child(el::h2().id(id).text("This record has changed"))
            .child(el::p().text("Your changes were not saved. Review your entries before reloading the latest version."))
            .child(el::p().text("Reloading discards the entries in this form."))
            .child(el::a().href(self.reload.clone()).text("Discard entries and reload latest version")),
            &self.attrs, &["role", "aria-labelledby", "tabindex", "autofocus"]).render(cx);
    }
}
