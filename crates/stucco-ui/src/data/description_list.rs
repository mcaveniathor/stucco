use super::DATA;
use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Render, Slot, el};

/// Terms and their details, such as the fields of a record.
///
/// Each term sits above its detail in a narrow container and beside it from
/// 32rem of container width.
///
/// ```
/// use stucco_ui::data::DescriptionList;
/// let html = stucco_core::to_html(&DescriptionList::new().item("Customer", "Ada"));
/// assert!(html.contains("<dt>Customer</dt><dd>Ada</dd>"));
/// ```
#[derive(Debug, Default)]
pub struct DescriptionList<'a> {
    attrs: Attrs,
    items: Vec<(String, Slot<'a>)>,
}

impl<'a> DescriptionList<'a> {
    /// An empty list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a term and its detail.
    pub fn item(mut self, term: impl Into<String>, detail: impl Render + 'a) -> Self {
        self.items.push((term.into(), Slot::new(detail)));
        self
    }
}

passthrough!(DescriptionList<'_>);

impl Render for DescriptionList<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&DATA);
        let list = el::dl()
            .class("st-dl")
            .children(self.items.iter().map(|(term, detail)| {
                el::div()
                    .class("st-dl-item")
                    .child(el::dt().text(term.clone()))
                    .child(el::dd().child(detail))
            }));
        apply(list, &self.attrs, &[]).render(cx);
    }
}
