use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Render, Slot, el};
/// A page footer landmark.
#[derive(Debug, Default)]
pub struct Footer<'a> {
    attrs: Attrs,
    children: Vec<Slot<'a>>,
}
impl<'a> Footer<'a> {
    /// An empty footer.
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends content.
    pub fn child(mut self, r: impl Render + 'a) -> Self {
        self.children.push(Slot::new(r));
        self
    }
}
passthrough!(Footer<'_>);
impl Render for Footer<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::APP);
        apply(
            el::footer()
                .class("st-footer")
                .children(self.children.iter()),
            &self.attrs,
            &[],
        )
        .render(cx);
    }
}
