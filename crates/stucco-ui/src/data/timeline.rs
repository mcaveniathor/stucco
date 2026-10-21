use super::DATA;
use crate::{Icon, passthrough::apply};
use stucco_core::{Attrs, Cx, Render, Slot, el};

/// One activity in a [`Timeline`]. Content and the visible timestamp wrap.
#[derive(Debug)]
pub struct TimelineEntry<'a> {
    datetime: String,
    time: String,
    content: Slot<'a>,
    icon: Option<Icon>,
}
impl<'a> TimelineEntry<'a> {
    /// A machine-readable HTML timestamp, its human-readable label, and content.
    /// Ordering and time-zone presentation are the caller's responsibility.
    pub fn new(
        datetime: impl Into<String>,
        time: impl Into<String>,
        content: impl Render + 'a,
    ) -> Self {
        Self {
            datetime: datetime.into(),
            time: time.into(),
            content: Slot::new(content),
            icon: None,
        }
    }
    /// Optional decorative icon. The content must explain the event in text.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

/// An ordered activity history. Supply entries in the intended reading order.
#[derive(Debug)]
pub struct Timeline<'a> {
    attrs: Attrs,
    label: String,
    entries: Vec<TimelineEntry<'a>>,
}
impl<'a> Timeline<'a> {
    /// A named ordered list; an empty timeline shows “No activity yet”.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            attrs: Attrs::default(),
            label: label.into(),
            entries: Vec::new(),
        }
    }
    /// Appends an activity.
    pub fn entry(mut self, entry: TimelineEntry<'a>) -> Self {
        self.entries.push(entry);
        self
    }
}
passthrough!(Timeline<'_>);
impl Render for Timeline<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&DATA);
        let list = el::ol()
            .class("st-timeline")
            .aria("label", self.label.clone())
            .children(self.entries.iter().map(|e| {
                el::li()
                    .child(
                        el::span()
                            .class("st-timeline-mark")
                            .aria("hidden", "true")
                            .child(e.icon),
                    )
                    .child(
                        el::div()
                            .class("st-timeline-content")
                            .child(
                                el::time()
                                    .attr("datetime", e.datetime.clone())
                                    .text(e.time.clone()),
                            )
                            .child(&e.content),
                    )
            }))
            .child_if(self.entries.is_empty(), el::li().text("No activity yet"));
        apply(list, &self.attrs, &["aria-label"]).render(cx);
    }
}
