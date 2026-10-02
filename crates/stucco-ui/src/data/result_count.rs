use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Render, el};
/// Honest count text for offset or cursor collections.
#[derive(Clone, Debug)]
pub struct ResultCount {
    attrs: Attrs,
    shown: usize,
    total: Option<u64>,
    offset: Option<u64>,
}
impl ResultCount {
    /// Creates a result count. Unknown cursor position uses a local count.
    pub fn new(shown: usize, total: Option<u64>, offset: Option<u64>) -> Self {
        Self {
            attrs: Attrs::default(),
            shown,
            total,
            offset,
        }
    }
}
passthrough!(ResultCount);
impl Render for ResultCount {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::DATA);
        let text = if self.shown == 0 {
            "No results".into()
        } else if let (Some(total), Some(offset)) = (self.total, self.offset) {
            format!(
                "Showing {}–{} of {total}",
                offset.saturating_add(1),
                offset.saturating_add(self.shown as u64).min(total)
            )
        } else {
            match self.shown {
                1 => "Showing 1 result".into(),
                n => format!("Showing {n} results"),
            }
        };
        apply(
            el::p().class("st-result-count").text(text),
            &self.attrs,
            &[],
        )
        .render(cx);
    }
}
