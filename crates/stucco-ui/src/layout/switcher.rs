use stucco_core::{Attrs, Cx, Render, Slot};

use super::{LAYOUT, Tag};
use crate::passthrough::apply;
use crate::{Measure, Space};

/// Items in a row that switch to a column when the container is narrower
/// than the threshold; more than `limit` items always stack.
///
/// ```
/// use stucco_ui::{Measure, layout::Switcher};
/// let html = stucco_core::to_html(&Switcher::new().threshold(Measure::Md).child("a").child("b"));
/// assert!(html.contains(r#"data-threshold="md""#));
/// ```
#[derive(Debug, Default)]
pub struct Switcher<'a> {
    attrs: Attrs,
    tag: Tag,
    threshold: Option<Measure>,
    space: Option<Space>,
    limit: Option<u8>,
    children: Vec<Slot<'a>>,
}

impl Switcher<'_> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-threshold", "data-space", "data-limit"];

    /// A switcher at `Measure::Lg` with `Space::S4` gaps.
    pub fn new() -> Self {
        Self::default()
    }

    /// Container width below which items stack.
    pub fn threshold(mut self, threshold: Measure) -> Self {
        self.threshold = Some(threshold);
        self
    }

    /// Gap between items.
    pub fn space(mut self, space: Space) -> Self {
        self.space = Some(space);
        self
    }

    /// Stacks whenever there are more than `limit` items (clamped to 2–6).
    pub fn limit(mut self, limit: u8) -> Self {
        self.limit = Some(limit.clamp(2, 6));
        self
    }
}

container_methods!(Switcher);

impl Render for Switcher<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let mut el = self
            .tag
            .element()
            .class("st-switcher")
            .data("threshold", self.threshold.unwrap_or(Measure::Lg).as_str())
            .data("space", self.space.unwrap_or(Space::S4).as_str());
        if let Some(limit) = self.limit {
            el = el.data("limit", limit.to_string());
        }
        apply(
            el.children(self.children.iter()),
            &self.attrs,
            Self::RESERVED,
        )
        .render(cx);
    }
}
