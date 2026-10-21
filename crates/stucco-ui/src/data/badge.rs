use super::DATA;
use crate::{Icon, Tone, passthrough::apply};
use stucco_core::{Attrs, Cx, Render, el};

/// A short status label, such as "Paid" or "Overdue".
///
/// The label carries the meaning; the tone only reinforces it.
///
/// ```
/// use stucco_ui::{Tone, data::Badge};
/// let html = stucco_core::to_html(&Badge::new("Paid").tone(Tone::Success));
/// assert!(html.contains(r#"data-tone="success""#));
/// ```
#[derive(Debug)]
pub struct Badge {
    attrs: Attrs,
    label: String,
    tone: Tone,
    icon: Option<Icon>,
}

impl Badge {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-tone"];

    /// A neutral badge.
    pub fn new(label: impl Into<String>) -> Self {
        Badge {
            attrs: Attrs::default(),
            label: label.into(),
            tone: Tone::Default,
            icon: None,
        }
    }

    /// Colour.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// A decorative icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

passthrough!(Badge);

impl Render for Badge {
    fn render(&self, cx: &mut Cx) {
        cx.require(&DATA);
        apply(
            el::span()
                .class("st-badge")
                .data("tone", self.tone.as_str())
                .child(self.icon)
                .text(self.label.clone()),
            &self.attrs,
            Self::RESERVED,
        )
        .render(cx);
    }
}
