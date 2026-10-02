use stucco_core::{Attrs, Cx, Render, Slot};

use super::{LAYOUT, Tag};
use crate::Space;
use crate::passthrough::apply;

/// Children in a column with consistent space between them.
///
/// ```
/// use stucco_ui::{Space, layout::Stack};
/// let html = stucco_core::to_html(&Stack::new().space(Space::S6).child("a").child("b"));
/// assert!(html.starts_with(r#"<div class="st-stack" data-space="6">"#));
/// ```
#[derive(Debug, Default)]
pub struct Stack<'a> {
    attrs: Attrs,
    tag: Tag,
    space: Option<Space>,
    recursive: bool,
    children: Vec<Slot<'a>>,
}

impl Stack<'_> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-space", "data-recursive"];

    /// An empty stack with `Space::S4` between children.
    pub fn new() -> Self {
        Self::default()
    }

    /// Space between children.
    pub fn space(mut self, space: Space) -> Self {
        self.space = Some(space);
        self
    }

    /// Applies the spacing to nested descendants too.
    pub fn recursive(mut self) -> Self {
        self.recursive = true;
        self
    }
}

container_methods!(Stack);

impl Render for Stack<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let el = self
            .tag
            .element()
            .class("st-stack")
            .data("space", self.space.unwrap_or(Space::S4).as_str())
            .bool_attr("data-recursive", self.recursive)
            .children(self.children.iter());
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}
