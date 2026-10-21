use super::LiveRegion;
use stucco_core::{Cx, Flash, FlashLevel, Render, el};

/// Plain-text redirect feedback, with a visible severity label and a live region
/// per message. Errors use `role="alert"`; other levels use `role="status"`.
/// An empty collection renders nothing. Initial content is readable without JS.
///
/// ```
/// use stucco_core::{Flash, FlashLevel, to_html};
/// use stucco_ui::feedback::FlashMessages;
/// let message = Flash::new(FlashLevel::Success, "Order saved.");
/// let html = to_html(&FlashMessages::new([message]));
/// assert!(html.contains("Success: "));
/// ```
#[derive(Clone, Debug, Default)]
pub struct FlashMessages {
    messages: Vec<Flash>,
}
impl FlashMessages {
    /// Takes zero or more messages. A cloned `Option<Flash>` from a request
    /// context can be passed directly.
    pub fn new(messages: impl IntoIterator<Item = Flash>) -> Self {
        Self {
            messages: messages.into_iter().collect(),
        }
    }
}
impl Render for FlashMessages {
    fn render(&self, cx: &mut Cx) {
        for message in &self.messages {
            let (level, label) = match message.level {
                FlashLevel::Info => ("info", "Info: "),
                FlashLevel::Success => ("success", "Success: "),
                FlashLevel::Warning => ("warning", "Warning: "),
                FlashLevel::Error => ("error", "Error: "),
            };
            let mut region = LiveRegion::new()
                .class("st-flash")
                .data("level", level)
                .child(
                    el::p()
                        .child(el::strong().text(label))
                        .text(&message.message),
                );
            if message.level == FlashLevel::Error {
                region = region.assertive();
            }
            region.render(cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::to_html;
    #[test]
    fn messages_are_escaped_labelled_and_announced_at_their_severity() {
        assert_eq!(to_html(&FlashMessages::default()), "");
        let html = to_html(&FlashMessages::new([
            Flash::new(FlashLevel::Info, "Note"),
            Flash::new(FlashLevel::Success, "Saved"),
            Flash::new(FlashLevel::Warning, "Check"),
            Flash::new(FlashLevel::Error, "<script>&bad</script>"),
        ]));
        assert_eq!(html.matches("role=\"status\"").count(), 3);
        assert_eq!(html.matches("role=\"alert\"").count(), 1);
        assert_eq!(html.matches("aria-live=\"assertive\"").count(), 1);
        assert_eq!(html.matches("aria-atomic=\"true\"").count(), 4);
        assert!(html.contains("<strong>Success: </strong>Saved"));
        assert!(html.contains("&lt;script&gt;&amp;bad&lt;/script&gt;"));
    }
}
