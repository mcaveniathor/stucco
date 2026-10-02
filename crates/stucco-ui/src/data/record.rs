//! Showing one record: its properties and its status.

use crate::{Tone, passthrough::apply};
use stucco_core::{Attrs, Cx, Render, Slot, el};

/// A record's status as a short label, such as "Paid" or "Archived".
///
/// The text is always shown, and the tone only tints it, so the status
/// reads the same without colour. The application maps its states to
/// labels and tones.
///
/// ```
/// use stucco_core::to_html;
/// use stucco_ui::{Tone, data::StatusBadge};
///
/// let html = to_html(&StatusBadge::new("Paid").tone(Tone::Success));
/// assert_eq!(html, r#"<span class="st-badge" data-tone="success">Paid</span>"#);
/// ```
#[derive(Clone, Debug)]
pub struct StatusBadge {
    attrs: Attrs,
    label: String,
    tone: Tone,
}

impl StatusBadge {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-tone"];

    /// A neutral badge reading `label`.
    pub fn new(label: impl Into<String>) -> Self {
        StatusBadge {
            attrs: Attrs::default(),
            label: label.into(),
            tone: Tone::Default,
        }
    }

    /// Tints the badge (default: neutral).
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

passthrough!(StatusBadge);

impl Render for StatusBadge {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::DATA);
        apply(
            el::span()
                .class("st-badge")
                .data("tone", self.tone.as_str())
                .text(&self.label),
            &self.attrs,
            Self::RESERVED,
        )
        .render(cx);
    }
}

/// A record's properties as name–value pairs (`<dl>`), laid out in columns
/// where there is room and stacked on narrow screens.
///
/// Long values wrap rather than overflow. Pass an explicit "Not set" (or
/// similar) for empty values: a blank reads as missing data.
///
/// ```
/// use stucco_core::to_html;
/// use stucco_ui::data::DescriptionList;
///
/// let html = to_html(&DescriptionList::new().item("Customer", "Ada").item("Total", "$12.00"));
/// assert!(html.contains("<dt>Customer</dt><dd>Ada</dd>"));
/// ```
#[derive(Debug, Default)]
pub struct DescriptionList<'a> {
    attrs: Attrs,
    items: Vec<(Slot<'a>, Slot<'a>)>,
}

impl<'a> DescriptionList<'a> {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a property.
    pub fn item(mut self, term: impl Render + 'a, detail: impl Render + 'a) -> Self {
        self.items.push((Slot::new(term), Slot::new(detail)));
        self
    }
}

passthrough!(DescriptionList<'_>);

impl Render for DescriptionList<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::DATA);
        let list = el::dl()
            .class("st-description-list")
            .children(self.items.iter().map(|(term, detail)| {
                el::div()
                    .class("st-description-item")
                    .child(el::dt().child(term))
                    .child(el::dd().child(detail))
            }));
        apply(list, &self.attrs, &[]).render(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::to_html;

    #[test]
    fn description_lists_group_each_pair() {
        let html = to_html(
            &DescriptionList::new()
                .item("Note", "<b>bold</b>")
                .item("Status", StatusBadge::new("Open")),
        );
        assert_eq!(
            html,
            concat!(
                r#"<dl class="st-description-list">"#,
                r#"<div class="st-description-item"><dt>Note</dt><dd>&lt;b&gt;bold&lt;/b&gt;</dd></div>"#,
                r#"<div class="st-description-item"><dt>Status</dt><dd><span class="st-badge" data-tone="default">Open</span></dd></div>"#,
                "</dl>"
            )
        );
    }
}
