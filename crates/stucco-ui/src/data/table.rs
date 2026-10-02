use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Render, Slot, el};
/// A semantic table inside a keyboard-focusable overflow region.
#[derive(Debug)]
pub struct Table<'a> {
    attrs: Attrs,
    caption: String,
    header: Option<Row<'a>>,
    rows: Vec<Row<'a>>,
}
impl<'a> Table<'a> {
    /// An empty table with a required accessible caption.
    pub fn new(caption: impl Into<String>) -> Self {
        Self {
            attrs: Attrs::default(),
            caption: caption.into(),
            header: None,
            rows: vec![],
        }
    }
    /// Column headings.
    pub fn header(mut self, row: Row<'a>) -> Self {
        self.header = Some(row);
        self
    }
    /// Appends a row.
    pub fn row(mut self, row: Row<'a>) -> Self {
        self.rows.push(row);
        self
    }
}
passthrough!(Table<'_>);
impl Render for Table<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::DATA);
        let heading = self
            .header
            .as_ref()
            .map(|r| el::thead().child(r.element(true)));
        let body = el::tbody().children(self.rows.iter().map(|r| r.element(false)));
        let table = el::table()
            .class("st-table")
            .child(el::caption().text(&self.caption))
            .child(heading)
            .child(body);
        apply(
            el::div()
                .class("st-table-scroll")
                .attr("role", "region")
                .aria("label", &self.caption)
                .attr("tabindex", "0")
                .child(table),
            &self.attrs,
            &["role", "aria-label", "tabindex"],
        )
        .render(cx);
    }
}
/// A table row with data cells or scoped headings.
#[derive(Debug, Default)]
pub struct Row<'a> {
    cells: Vec<(bool, Slot<'a>)>,
    attrs: Attrs,
}
impl<'a> Row<'a> {
    /// An empty row.
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends a data cell.
    pub fn cell(mut self, value: impl Render + 'a) -> Self {
        self.cells.push((false, Slot::new(value)));
        self
    }
    /// Appends a heading (column scope in a table header, row scope in the body).
    pub fn header(mut self, value: impl Render + 'a) -> Self {
        self.cells.push((true, Slot::new(value)));
        self
    }
    fn element(&self, header: bool) -> stucco_core::el::Element<'_> {
        apply(
            el::tr().children(self.cells.iter().map(|(heading, value)| {
                if *heading {
                    el::th()
                        .attr("scope", if header { "col" } else { "row" })
                        .child(value)
                } else {
                    el::td().child(value)
                }
            })),
            &self.attrs,
            &[],
        )
    }
}
passthrough!(Row<'_>);
impl Render for Row<'_> {
    fn render(&self, cx: &mut Cx) {
        self.element(false).render(cx);
    }
}
