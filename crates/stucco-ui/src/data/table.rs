use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Render, Slot, el};
/// A semantic table inside a keyboard-focusable overflow region.
#[derive(Debug)]
pub struct Table<'a> {
    attrs: Attrs,
    caption: String,
    header: Option<Row<'a>>,
    rows: Vec<Row<'a>>,
    card_labels: Option<Vec<String>>,
}
impl<'a> Table<'a> {
    /// An empty table with a required accessible caption.
    pub fn new(caption: impl Into<String>) -> Self {
        Self {
            attrs: Attrs::default(),
            caption: caption.into(),
            header: None,
            rows: vec![],
            card_labels: None,
        }
    }
    /// Presents rows as labelled cards below 48rem of available container width.
    /// Supply one label per column, in order, and use rows without cell spans.
    /// Cells and controls render only once;
    /// explicit table roles retain semantics when CSS changes their display.
    /// Header links remain visible and keyboard accessible in the card layout.
    /// Without CSS this is an ordinary table. Missing labels render as empty text.
    pub fn card_labels(mut self, labels: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.card_labels = Some(labels.into_iter().map(Into::into).collect());
        self
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
        let responsive = self.card_labels.is_some();
        let heading = self.header.as_ref().map(|r| {
            let head = el::thead().child(r.element(true, self.card_labels.as_deref()));
            if responsive {
                head.attr("role", "rowgroup")
            } else {
                head
            }
        });
        let mut body = el::tbody().children(
            self.rows
                .iter()
                .map(|r| r.element(false, self.card_labels.as_deref())),
        );
        if responsive {
            body = body.attr("role", "rowgroup");
        }
        let mut table = el::table().class("st-table");
        if responsive {
            table = table.attr("role", "table");
        }
        let table = table
            .child(el::caption().text(&self.caption))
            .child(heading)
            .child(body);
        let mut wrapper = el::div().class("st-table-scroll");
        if responsive {
            wrapper = wrapper.class("st-table-cards");
        }
        apply(
            wrapper
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
    cells: Vec<(bool, Slot<'a>, Attrs)>,
    attrs: Attrs,
}
impl<'a> Row<'a> {
    /// An empty row.
    pub fn new() -> Self {
        Self::default()
    }
    /// Appends a data cell.
    pub fn cell(mut self, value: impl Render + 'a) -> Self {
        self.cells.push((false, Slot::new(value), Attrs::default()));
        self
    }
    /// Appends a data cell with attributes, such as `data-kind="number"`.
    pub fn cell_with_attrs(mut self, value: impl Render + 'a, attrs: Attrs) -> Self {
        self.cells.push((false, Slot::new(value), attrs));
        self
    }
    /// Appends a heading (column scope in a table header, row scope in the body).
    pub fn header(mut self, value: impl Render + 'a) -> Self {
        self.cells.push((true, Slot::new(value), Attrs::default()));
        self
    }
    /// Appends a scoped heading with per-cell attributes, such as aria-sort.
    pub fn header_with_attrs(mut self, value: impl Render + 'a, attrs: Attrs) -> Self {
        self.cells.push((true, Slot::new(value), attrs));
        self
    }
    fn element<'s>(
        &'s self,
        header: bool,
        labels: Option<&'s [String]>,
    ) -> stucco_core::el::Element<'s> {
        let mut row = el::tr().children(self.cells.iter().enumerate().map(
            |(index, (heading, value, attrs))| {
                let mut cell = if *heading {
                    el::th().attr("scope", if header { "col" } else { "row" })
                } else {
                    el::td()
                };
                if labels.is_some() {
                    cell = cell.attr(
                        "role",
                        if *heading {
                            if header { "columnheader" } else { "rowheader" }
                        } else {
                            "cell"
                        },
                    );
                }
                // Keep the ordinary table's attribute behavior unchanged.
                let attrs = if *heading {
                    attrs.without(&["scope"])
                } else {
                    attrs.clone()
                };
                cell = apply(cell, &attrs, if labels.is_some() { &["role"] } else { &[] });
                if !header {
                    if let Some(labels) = labels {
                        cell = cell
                            .child(
                                el::span()
                                    .class("st-table-card-label")
                                    .aria("hidden", "true")
                                    .text(labels.get(index).map_or("", String::as_str)),
                            )
                            .child(el::div().class("st-table-card-value").child(value));
                        return cell;
                    }
                }
                cell.child(value)
            },
        ));
        if labels.is_some() {
            row = row.attr("role", "row");
        }
        apply(
            row,
            &self.attrs,
            if labels.is_some() { &["role"] } else { &[] },
        )
    }
}
passthrough!(Row<'_>);
impl Render for Row<'_> {
    fn render(&self, cx: &mut Cx) {
        self.element(false, None).render(cx);
    }
}
