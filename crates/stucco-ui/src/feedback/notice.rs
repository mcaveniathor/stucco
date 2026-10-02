//! Inline notices: the outcome of an action, or a state worth pointing out.

use crate::{Tone, passthrough::apply};
use stucco_core::{Attrs, Cx, Render, Slot, el};

/// A success, warning, error or informational message shown inline, such as
/// the confirmation after a form saved or the reason a record is read-only.
///
/// The tone is spelled out for screen readers ("Success:", "Error:") and the
/// title and message carry the meaning; colour only reinforces it.
///
/// A notice shown because of something the user just did is announced: the
/// success and info tones as a polite `role="status"`, the warning and danger
/// tones as `role="alert"`. Call [`Notice::quiet`] for standing notices that
/// describe the page rather than an outcome, so they are not announced on
/// every visit.
///
/// ```
/// use stucco_core::to_html;
/// use stucco_ui::feedback::Notice;
///
/// let html = to_html(&Notice::success("Order 42 was saved."));
/// assert!(html.contains(r#"role="status""#) && html.contains("Success:"));
/// let html = to_html(&Notice::danger("The order could not be saved.").title("Something went wrong"));
/// assert!(html.contains(r#"role="alert""#));
/// let html = to_html(&Notice::info("This order is archived.").quiet());
/// assert!(!html.contains("role="));
/// ```
#[derive(Debug)]
pub struct Notice<'a> {
    attrs: Attrs,
    tone: Tone,
    title: Option<String>,
    message: Slot<'a>,
    actions: Option<Slot<'a>>,
    announce: bool,
}

impl<'a> Notice<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["role", "data-tone"];

    /// A notice with `tone`. Tones other than success, warning, danger and
    /// info read as information.
    pub fn new(tone: Tone, message: impl Render + 'a) -> Self {
        Notice {
            attrs: Attrs::default(),
            tone,
            title: None,
            message: Slot::new(message),
            actions: None,
            announce: true,
        }
    }

    /// Something finished as intended.
    pub fn success(message: impl Render + 'a) -> Self {
        Notice::new(Tone::Success, message)
    }

    /// Something the user should know.
    pub fn info(message: impl Render + 'a) -> Self {
        Notice::new(Tone::Info, message)
    }

    /// Something may need attention.
    pub fn warning(message: impl Render + 'a) -> Self {
        Notice::new(Tone::Warning, message)
    }

    /// Something failed.
    pub fn danger(message: impl Render + 'a) -> Self {
        Notice::new(Tone::Danger, message)
    }

    /// A short, bold summary before the message.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Follow-up actions, such as a link to the record or a retry button.
    pub fn actions(mut self, actions: impl Render + 'a) -> Self {
        self.actions = Some(Slot::new(actions));
        self
    }

    /// Not announced: for notices that describe the page, not an outcome.
    pub fn quiet(mut self) -> Self {
        self.announce = false;
        self
    }

    fn tone(&self) -> (&'static str, &'static str) {
        match self.tone {
            Tone::Success => ("success", "Success: "),
            Tone::Warning => ("warning", "Warning: "),
            Tone::Danger => ("danger", "Error: "),
            _ => ("info", "Information: "),
        }
    }
}

passthrough!(Notice<'_>);

impl Render for Notice<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::FEEDBACK);
        let (tone, prefix) = self.tone();
        let mut root = el::div().class("st-notice").data("tone", tone);
        if self.announce {
            let urgent = matches!(tone, "warning" | "danger");
            root = root.attr("role", if urgent { "alert" } else { "status" });
        }
        let body = el::div()
            .class("st-notice-body")
            .child(
                self.title
                    .as_ref()
                    .map(|t| el::p().class("st-notice-title").text(t)),
            )
            .child(
                el::div()
                    .class("st-notice-message")
                    .child(el::span().class("st-sr-only").text(prefix))
                    .child(&self.message),
            )
            .child(
                self.actions
                    .as_ref()
                    .map(|a| el::div().class("st-notice-actions").child(a)),
            );
        apply(root.child(body), &self.attrs, Self::RESERVED).render(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::to_html;

    #[test]
    fn notices_name_their_tone_and_escape_content() {
        let html = to_html(
            &Notice::warning("Stock is <low>")
                .title("Check stock")
                .actions(el::a().href("/stock").text("Review"))
                .class("extra"),
        );
        assert_eq!(
            html,
            concat!(
                r#"<div class="st-notice extra" data-tone="warning" role="alert">"#,
                r#"<div class="st-notice-body"><p class="st-notice-title">Check stock</p>"#,
                r#"<div class="st-notice-message"><span class="st-sr-only">Warning: </span>Stock is &lt;low&gt;</div>"#,
                r#"<div class="st-notice-actions"><a href="/stock">Review</a></div></div></div>"#,
            )
        );
    }

    #[test]
    fn other_tones_read_as_information() {
        let html = to_html(&Notice::new(Tone::Accent, "Hi"));
        assert!(html.contains(r#"data-tone="info" role="status""#));
        assert!(html.contains("Information: "));
        assert!(to_html(&Notice::danger("x")).contains("Error: "));
    }
}
