//! A full-page confirmation for actions that change or destroy data.

use crate::actions::Button;
use crate::forms::{Form, FormActions, HiddenInput};
use crate::passthrough::apply;
use crate::{Variant, typography::Heading};
use stucco_core::{Attrs, Cx, Href, Render, Slot, el};

/// A page that asks before acting: what will be affected, what will
/// happen, and an explicit confirm button beside a way back.
///
/// It is an ordinary POST form, so it works without JavaScript: link to the
/// page that shows it (`GET /orders/42/delete`), and handle the action where
/// the form posts (`POST /orders/42/delete`). A dialog can offer the same
/// form in place as an enhancement; keep this page as its fallback.
///
/// Name the action and its scope in the title and button ("Delete order
/// 42", "Archive 3 orders"), not "Are you sure?" and "OK". For actions that
/// can't be undone, say so in a consequence and call [`Confirmation::danger`].
///
/// The application still checks, when the form arrives, that the user may
/// do this and that the record still exists; the page only asks.
///
/// ```
/// use stucco_core::to_html;
/// use stucco_ui::app::Confirmation;
///
/// let page = Confirmation::new("Delete order 42?", "/orders/42/delete", "Delete order")
///     .target("Order 42 for Ada Lovelace, $12.00")
///     .consequence("The order and its history are removed permanently.")
///     .danger()
///     .csrf("token")
///     .cancel("/orders/42");
/// let html = to_html(&page);
/// assert!(html.contains(r#"<form class="st-form" method="post" action="/orders/42/delete">"#));
/// assert!(html.contains(r#"data-variant="danger""#) && html.contains(">Delete order<"));
/// assert!(html.contains(r#"href="/orders/42""#));
/// ```
#[derive(Debug)]
pub struct Confirmation<'a> {
    attrs: Attrs,
    title: String,
    level: u8,
    action: Href,
    confirm: String,
    target: Option<Slot<'a>>,
    consequences: Vec<Slot<'a>>,
    hidden: Vec<HiddenInput>,
    csrf: Option<String>,
    cancel: Option<(String, Href)>,
    danger: bool,
}

impl<'a> Confirmation<'a> {
    /// Asks `title`; the button labelled `confirm` posts to `action`.
    pub fn new(
        title: impl Into<String>,
        action: impl Into<Href>,
        confirm: impl Into<String>,
    ) -> Self {
        Confirmation {
            attrs: Attrs::default(),
            title: title.into(),
            level: 1,
            action: action.into(),
            confirm: confirm.into(),
            target: None,
            consequences: Vec::new(),
            hidden: Vec::new(),
            csrf: None,
            cancel: None,
            danger: false,
        }
    }

    /// What the action applies to: a sentence, or a short list of records.
    pub fn target(mut self, target: impl Render + 'a) -> Self {
        self.target = Some(Slot::new(target));
        self
    }

    /// Something that will happen, listed under "This will:".
    pub fn consequence(mut self, consequence: impl Render + 'a) -> Self {
        self.consequences.push(Slot::new(consequence));
        self
    }

    /// Styles the confirm button as destructive.
    pub fn danger(mut self) -> Self {
        self.danger = true;
        self
    }

    /// A hidden value posted with the form (selected ids, a record version,
    /// a return location).
    pub fn hidden(mut self, name: &str, value: impl Into<String>) -> Self {
        self.hidden.push(HiddenInput::new(name, value));
        self
    }

    /// The application's CSRF token, posted as `_csrf`.
    pub fn csrf(mut self, token: &str) -> Self {
        self.csrf = Some(token.to_owned());
        self
    }

    /// A "Cancel" link to `href`, usually the page the user came from.
    pub fn cancel(self, href: impl Into<Href>) -> Self {
        self.cancel_with("Cancel", href)
    }

    /// A cancel link with its own label.
    pub fn cancel_with(mut self, label: impl Into<String>, href: impl Into<Href>) -> Self {
        self.cancel = Some((label.into(), href.into()));
        self
    }

    /// The heading level (default 1, for a page of its own).
    pub fn level(mut self, level: u8) -> Self {
        self.level = level.clamp(1, 6);
        self
    }
}

passthrough!(Confirmation<'_>);

impl Render for Confirmation<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::APP);
        let variant = if self.danger {
            Variant::Danger
        } else {
            Variant::Primary
        };
        let mut actions =
            FormActions::new(Button::new(self.confirm.as_str()).submit().variant(variant));
        if let Some((label, href)) = &self.cancel {
            actions = actions.cancel_with(label.as_str(), href.clone());
        }
        let mut form = Form::post(self.action.clone()).children(self.hidden.iter());
        if let Some(token) = &self.csrf {
            form = form.csrf(token);
        }
        let consequences = (!self.consequences.is_empty()).then(|| {
            el::div()
                .class("st-confirmation-consequences")
                .child(el::p().text("This will:"))
                .child(el::ul().children(self.consequences.iter().map(|c| el::li().child(c))))
        });
        let root = el::section()
            .class("st-confirmation")
            .child(Heading::new(self.level, self.title.as_str()))
            .child(
                self.target
                    .as_ref()
                    .map(|t| el::div().class("st-confirmation-target").child(t)),
            )
            .child(consequences)
            .child(form.child(actions));
        apply(root, &self.attrs, &[]).render(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::to_html;

    #[test]
    fn confirmations_post_hidden_values_and_escape_text() {
        let html = to_html(
            &Confirmation::new("Archive <2> orders?", "/orders/archive", "Archive 2 orders")
                .hidden("id", "1")
                .hidden("id", "2")
                .consequence("They leave the default list.")
                .cancel_with("Back to orders", "/orders?q=a"),
        );
        assert!(html.starts_with(r#"<section class="st-confirmation"><h1"#));
        assert!(html.contains("Archive &lt;2&gt; orders?"));
        assert!(html.contains(
            r#"<input type="hidden" name="id" value="1"><input type="hidden" name="id" value="2">"#
        ));
        assert!(html.contains("<p>This will:</p><ul><li>They leave the default list.</li></ul>"));
        assert!(html.contains(r#"data-variant="primary""#) && !html.contains("_csrf"));
        assert!(html.contains(r#"href="/orders?q=a""#) && html.contains(">Back to orders<"));
    }
}
