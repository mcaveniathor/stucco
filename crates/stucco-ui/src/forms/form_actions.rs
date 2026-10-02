//! The row of buttons that ends a form.

use stucco_core::{Attrs, Cx, Href, Render, Slot, el};

use super::FORMS;
use crate::Variant;
use crate::actions::{Button, ButtonLink};
use crate::passthrough::apply;

/// The buttons at the end of a form: one primary submit button, any
/// secondary actions, and a cancel link.
///
/// The primary button comes first in the markup, so it is the one pressing
/// Enter in a text field submits with, and the first the keyboard reaches.
/// Cancel is a link, not a button: it leaves without submitting, and works
/// without JavaScript. Send it somewhere predictable, usually the page the
/// user came from.
///
/// ```
/// use stucco_core::to_html;
/// use stucco_ui::forms::FormActions;
///
/// let html = to_html(&FormActions::submit("Save order").cancel("/orders/42"));
/// assert!(html.starts_with(r#"<div class="st-form-actions"><button class="st-button" type="submit""#));
/// assert!(html.contains(r#"data-variant="primary""#));
/// assert!(html.contains(r#"href="/orders/42""#) && html.contains(">Cancel<"));
/// ```
#[derive(Debug)]
pub struct FormActions<'a> {
    attrs: Attrs,
    primary: Slot<'a>,
    secondary: Vec<Slot<'a>>,
    cancel: Option<(String, Href)>,
}

impl<'a> FormActions<'a> {
    /// A primary submit button labelled `label`.
    pub fn submit(label: impl Render + 'a) -> Self {
        FormActions::new(Button::new(label).submit().variant(Variant::Primary))
    }

    /// Uses `primary` as the main action (for a danger button, say).
    pub fn new(primary: impl Render + 'a) -> Self {
        FormActions {
            attrs: Attrs::default(),
            primary: Slot::new(primary),
            secondary: Vec::new(),
            cancel: None,
        }
    }

    /// Adds a secondary action after the primary one.
    pub fn secondary(mut self, action: impl Render + 'a) -> Self {
        self.secondary.push(Slot::new(action));
        self
    }

    /// A "Cancel" link to `href`.
    pub fn cancel(self, href: impl Into<Href>) -> Self {
        self.cancel_with("Cancel", href)
    }

    /// A cancel link with its own label ("Back to order", say).
    pub fn cancel_with(mut self, label: impl Into<String>, href: impl Into<Href>) -> Self {
        self.cancel = Some((label.into(), href.into()));
        self
    }
}

passthrough!(FormActions<'_>);

impl Render for FormActions<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FORMS);
        let row =
            el::div()
                .class("st-form-actions")
                .child(&self.primary)
                .children(self.secondary.iter())
                .child(self.cancel.as_ref().map(|(label, href)| {
                    ButtonLink::new(label, href.clone()).variant(Variant::Ghost)
                }));
        apply(row, &self.attrs, &[]).render(cx);
    }
}
