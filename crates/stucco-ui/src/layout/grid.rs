use stucco_core::{Attrs, Cx, Render, Slot};

use super::{LAYOUT, Tag};
use crate::passthrough::apply;
use crate::{Measure, Space};

/// As many equal columns as fit, each at least the minimum width.
///
/// ```
/// use stucco_ui::{Measure, layout::Grid};
/// let html = stucco_core::to_html(&Grid::new().min(Measure::Md).child("card"));
/// assert!(html.contains(r#"data-min="md""#));
/// ```
#[derive(Debug, Default)]
pub struct Grid<'a> {
    attrs: Attrs,
    tag: Tag,
    min: Option<Measure>,
    space: Option<Space>,
    children: Vec<Slot<'a>>,
}

impl Grid<'_> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-min", "data-space"];

    /// An empty grid with `Measure::Sm` columns and `Space::S4` gaps.
    pub fn new() -> Self {
        Self::default()
    }

    /// Minimum column width.
    pub fn min(mut self, min: Measure) -> Self {
        self.min = Some(min);
        self
    }

    /// Gap between cells.
    pub fn space(mut self, space: Space) -> Self {
        self.space = Some(space);
        self
    }
}

container_methods!(Grid);

impl Render for Grid<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let el = self
            .tag
            .element()
            .class("st-grid")
            .data("min", self.min.unwrap_or(Measure::Sm).as_str())
            .data("space", self.space.unwrap_or(Space::S4).as_str())
            .children(self.children.iter());
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}
