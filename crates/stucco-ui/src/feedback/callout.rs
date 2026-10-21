use super::FEEDBACK;
use crate::{Icon, Tone, passthrough::apply};
use stucco_core::{Attrs, Cx, Render, Slot, el};

// Lucide `info`, `circle-check`, `triangle-alert` and `circle-alert`
// (ISC; see LICENSE-LUCIDE).
const INFO: Icon = Icon::custom(
    "info",
    r#"<circle cx="12" cy="12" r="10" /><path d="M12 16v-4" /><path d="M12 8h.01" />"#,
);
const CIRCLE_CHECK: Icon = Icon::custom(
    "circle-check",
    r#"<circle cx="12" cy="12" r="10" /><path d="m16 9-5.5 5.5L8 12" />"#,
);
const TRIANGLE_ALERT: Icon = Icon::custom(
    "triangle-alert",
    r#"<path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3" /><path d="M12 9v4" /><path d="M12 17h.01" />"#,
);
const CIRCLE_ALERT: Icon = Icon::custom(
    "circle-alert",
    r#"<circle cx="12" cy="12" r="10" /><line x1="12" x2="12" y1="8" y2="12" /><line x1="12" x2="12.01" y1="16" y2="16" />"#,
);

/// A note that is part of the page, such as "Payment overdue".
///
/// It is static content, so it has no live role; announce changes with
/// [`FlashMessages`](super::FlashMessages) or [`LiveRegion`](super::LiveRegion)
/// instead. The tone is also given in hidden text ("Warning: ").
///
/// ```
/// use stucco_ui::{Tone, feedback::Callout};
/// let html = stucco_core::to_html(&Callout::new(Tone::Warning).title("Payment overdue"));
/// assert!(html.contains("Warning: </span>Payment overdue"));
/// ```
#[derive(Debug)]
pub struct Callout<'a> {
    attrs: Attrs,
    tone: Tone,
    title: Option<String>,
    children: Vec<Slot<'a>>,
    actions: Option<Slot<'a>>,
}

impl<'a> Callout<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-tone"];

    /// An empty callout in `tone`.
    pub fn new(tone: Tone) -> Self {
        Callout {
            attrs: Attrs::default(),
            tone,
            title: None,
            children: Vec::new(),
            actions: None,
        }
    }

    /// A short title.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Adds body content.
    pub fn child(mut self, child: impl Render + 'a) -> Self {
        self.children.push(Slot::new(child));
        self
    }

    /// Links or buttons that act on the note.
    pub fn actions(mut self, actions: impl Render + 'a) -> Self {
        self.actions = Some(Slot::new(actions));
        self
    }
}

passthrough!(Callout<'_>);

impl Render for Callout<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FEEDBACK);
        let (icon, prefix) = match self.tone {
            Tone::Info | Tone::Accent => (INFO, Some("Note: ")),
            Tone::Success => (CIRCLE_CHECK, Some("Success: ")),
            Tone::Warning => (TRIANGLE_ALERT, Some("Warning: ")),
            Tone::Danger => (CIRCLE_ALERT, Some("Important: ")),
            Tone::Default | Tone::Muted => (INFO, None),
        };
        let prefix = prefix.map(|p| el::span().class("st-sr-only").text(p));
        let mut body = el::div().class("st-callout-body");
        match &self.title {
            Some(title) => {
                body = body.child(
                    el::p()
                        .class("st-callout-title")
                        .child(prefix)
                        .text(title.clone()),
                );
            }
            None => body = body.child(prefix),
        }
        let actions = self
            .actions
            .as_ref()
            .map(|a| el::div().class("st-callout-actions").child(a));
        let root = el::div()
            .class("st-callout")
            .data("tone", self.tone.as_str())
            .child(icon)
            .child(body.children(&self.children).child(actions));
        apply(root, &self.attrs, Self::RESERVED).render(cx);
    }
}
