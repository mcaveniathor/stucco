use stucco_core::{Attrs, Cx, Render, Slot};

use super::{LAYOUT, Tag};
use crate::Measure;
use crate::passthrough::apply;

/// The page-width wrapper: centred, with gutters that grow with the viewport.
///
/// ```
/// use stucco_ui::layout::Container;
/// let html = stucco_core::to_html(&Container::new().child("page"));
/// assert!(html.contains(r#"data-size="xl""#));
/// ```
#[derive(Debug, Default)]
pub struct Container<'a> {
    attrs: Attrs,
    tag: Tag,
    size: Option<Measure>,
    children: Vec<Slot<'a>>,
}

impl Container<'_> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-size"];

    /// A container at `Measure::Xl`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Maximum width.
    pub fn size(mut self, size: Measure) -> Self {
        self.size = Some(size);
        self
    }
}

container_methods!(Container);

impl Render for Container<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let el = self
            .tag
            .element()
            .class("st-container")
            .data("size", self.size.unwrap_or(Measure::Xl).as_str())
            .children(self.children.iter());
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}
