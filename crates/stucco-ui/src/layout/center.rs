use stucco_core::{Attrs, Cx, Render, Slot};

use super::{LAYOUT, Tag};
use crate::passthrough::apply;
use crate::{Measure, Space};

/// Content centred horizontally within a maximum width.
///
/// ```
/// use stucco_ui::{Measure, layout::Center};
/// let html = stucco_core::to_html(&Center::new().max(Measure::Md).child("text"));
/// assert!(html.contains(r#"data-max="md""#));
/// ```
#[derive(Debug, Default)]
pub struct Center<'a> {
    attrs: Attrs,
    tag: Tag,
    max: Option<Measure>,
    gutters: Option<Space>,
    intrinsic: bool,
    children: Vec<Slot<'a>>,
}

impl Center<'_> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-max", "data-gutters", "data-intrinsic"];

    /// A centred column at the reading measure (`Measure::Prose`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Maximum width.
    pub fn max(mut self, max: Measure) -> Self {
        self.max = Some(max);
        self
    }

    /// Minimum space on both sides.
    pub fn gutters(mut self, gutters: Space) -> Self {
        self.gutters = Some(gutters);
        self
    }

    /// Also centres children narrower than the measure.
    pub fn intrinsic(mut self) -> Self {
        self.intrinsic = true;
        self
    }
}

container_methods!(Center);

impl Render for Center<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let mut el = self
            .tag
            .element()
            .class("st-center")
            .data("max", self.max.unwrap_or(Measure::Prose).as_str());
        if let Some(g) = self.gutters {
            el = el.data("gutters", g.as_str());
        }
        let el = el
            .bool_attr("data-intrinsic", self.intrinsic)
            .children(self.children.iter());
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}
