//! The error summary shown above a form that failed validation.

use stucco_core::{Attrs, Cx, FormState, Href, Render, el};

use super::FORMS;
use crate::passthrough::apply;

/// Lists a submission's errors at the top of the form and takes focus when
/// the page loads, so keyboard and screen reader users learn what went wrong
/// without hunting for it. Renders nothing when the state has no errors.
///
/// Form errors come first. Field errors follow, linked to their controls in
/// the order fields are registered with [`ErrorSummary::field`]; give those
/// controls an explicit `.id()` so the link has a target. Errors for fields
/// that were not registered are listed last, unlinked. Write messages that
/// make sense on their own ("Enter a customer name", not "Required").
///
/// ```
/// use stucco_core::{FormState, to_html};
/// use stucco_ui::forms::ErrorSummary;
///
/// let state = FormState::new().with_error("customer", "Enter a customer name");
/// let html = to_html(&ErrorSummary::new(&state).field("customer", "order-customer"));
/// assert!(html.contains(r##"<a href="#order-customer">Enter a customer name</a>"##));
/// assert_eq!(to_html(&ErrorSummary::new(&FormState::new())), "");
/// ```
#[derive(Debug)]
pub struct ErrorSummary<'s> {
    attrs: Attrs,
    state: &'s FormState,
    title: String,
    fields: Vec<(String, String)>,
    focus: bool,
}

impl<'s> ErrorSummary<'s> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["tabindex", "autofocus", "aria-labelledby"];

    /// A summary of `state`'s errors titled "There is a problem".
    pub fn new(state: &'s FormState) -> Self {
        ErrorSummary {
            attrs: Attrs::default(),
            state,
            title: "There is a problem".to_owned(),
            fields: Vec::new(),
            focus: true,
        }
    }

    /// Replaces the heading text.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Whether the summary takes focus when the page loads (default true).
    /// Turn it off when the summary is not the reason the page was shown,
    /// such as a pattern library page.
    pub fn focus(mut self, focus: bool) -> Self {
        self.focus = focus;
        self
    }

    /// Links errors for field `name` to the control with id `control_id`.
    pub fn field(mut self, name: impl Into<String>, control_id: impl Into<String>) -> Self {
        self.fields.push((name.into(), control_id.into()));
        self
    }
}

passthrough!(ErrorSummary<'_>);

impl Render for ErrorSummary<'_> {
    fn render(&self, cx: &mut Cx) {
        if !self.state.has_errors() {
            return;
        }
        cx.require(&FORMS);
        let title_id = cx.id("errors");
        let mut list = el::ul().class("st-error-summary-list");
        for message in self.state.form_errors() {
            list = list.child(el::li().text(message.clone()));
        }
        for (name, control_id) in &self.fields {
            for message in self.state.errors(name) {
                let link = el::a()
                    .href(Href::new(format!("#{control_id}")))
                    .text(message.clone());
                list = list.child(el::li().child(link));
            }
        }
        for (name, messages) in self.state.field_errors() {
            if self.fields.iter().any(|(n, _)| n == name) {
                continue;
            }
            for message in messages {
                list = list.child(el::li().text(message.clone()));
            }
        }
        let el = el::div()
            .class("st-error-summary")
            .attr("role", "group")
            .aria("labelledby", title_id.clone())
            .attr("tabindex", "-1")
            .bool_attr("autofocus", self.focus)
            .child(
                el::h2()
                    .class("st-error-summary-title")
                    .id(title_id)
                    .text(self.title.clone()),
            )
            .child(list);
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::to_html;

    #[test]
    fn lists_form_errors_then_linked_then_unlinked_field_errors() {
        let state = FormState::new()
            .with_error("b", "B is <wrong>")
            .with_error("a", "A one")
            .with_error("a", "A two")
            .with_error("z", "Z unregistered")
            .with_form_error("Saved by someone else");
        let html = to_html(
            &ErrorSummary::new(&state)
                .field("b", "b-id")
                .field("a", "a-id")
                .field("missing", "m-id")
                .class("extra"),
        );
        let id = "errors-1";
        assert_eq!(
            html,
            format!(
                concat!(
                    r#"<div class="st-error-summary extra" role="group" aria-labelledby="{id}" tabindex="-1" autofocus>"#,
                    r#"<h2 id="{id}" class="st-error-summary-title">There is a problem</h2>"#,
                    r#"<ul class="st-error-summary-list"><li>Saved by someone else</li>"#,
                    r##"<li><a href="#b-id">B is &lt;wrong&gt;</a></li>"##,
                    r##"<li><a href="#a-id">A one</a></li><li><a href="#a-id">A two</a></li>"##,
                    r#"<li>Z unregistered</li></ul></div>"#,
                ),
                id = id
            )
        );
    }

    #[test]
    fn only_form_errors_still_render_with_a_custom_title() {
        let state = FormState::new().with_form_error("Try again");
        let html = to_html(&ErrorSummary::new(&state).title("Could not save"));
        assert!(html.contains(">Could not save</h2>") && html.contains("<li>Try again</li>"));
        let quiet = to_html(&ErrorSummary::new(&state).focus(false));
        assert!(quiet.contains(r#"tabindex="-1""#) && !quiet.contains("autofocus"));
    }
}
