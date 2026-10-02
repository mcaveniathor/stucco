use super::{ActiveFilters, Col, CollectionView, FilterBar};
use crate::{
    data::{ResultCount, Row, Table},
    feedback::{EmptyState, LiveRegion},
    navigation::Pagination,
    passthrough::apply,
};
use stucco_core::{
    Attrs, Capabilities, Collection, CollectionPage, CollectionQuery, ColumnKind, Cx, Direction,
    Href, Render, Slot, Window, el,
};
/// A function of a row giving text (its id, its name).
struct RowText<'a, T>(Box<dyn Fn(&T) -> String + 'a>);
impl<T> std::fmt::Debug for RowText<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("RowText(..)")
    }
}
/// Row checkboxes for bulk actions; see [`DataTable::selectable`].
#[derive(Debug)]
struct Selection<'a, T> {
    form: String,
    name: String,
    label: RowText<'a, T>,
    selected: &'a [String],
}
/// Server-rendered collection table with native GET controls.
///
/// With a query and capabilities (from [`DataTable::from_collection`]) it
/// shows a filter bar, the active filters with links that remove each one,
/// sortable headings, a result count and pagination, all as ordinary links
/// and GET forms. It tells three kinds of empty apart: an empty collection
/// (see [`DataTable::empty`]), a search or filter with no matches (with a
/// link that clears them), and a page past the end (with a link back to
/// the first page). A failed load is the application's to report; render a
/// notice in place of the table, with a 5xx status.
#[derive(Debug)]
pub struct DataTable<'a, T> {
    attrs: Attrs,
    rows: &'a [T],
    page: Option<&'a CollectionPage<T>>,
    caption: String,
    columns: Vec<Col<'a, T>>,
    query: Option<&'a CollectionQuery>,
    caps: Option<&'a Capabilities>,
    action: Href,
    row_id: Option<RowText<'a, T>>,
    selection: Option<Selection<'a, T>>,
    empty: Option<Slot<'a>>,
}
impl<'a, T: 'a> DataTable<'a, T> {
    /// Creates a table over already-loaded rows.
    pub fn new(rows: &'a [T], caption: impl Into<String>) -> Self {
        Self {
            attrs: Attrs::default(),
            rows,
            page: None,
            caption: caption.into(),
            columns: vec![],
            query: None,
            caps: None,
            action: Href::new(""),
            row_id: None,
            selection: None,
            empty: None,
        }
    }
    /// Uses a page's rows and metadata without duplicated state.
    pub fn from_page(page: &'a CollectionPage<T>, caption: impl Into<String>) -> Self {
        Self::new(&page.rows, caption).page(page)
    }
    /// Uses a loaded collection's rows, query and capabilities.
    pub fn from_collection(collection: &'a Collection<T>, caption: impl Into<String>) -> Self {
        Self::from_page(&collection.page, caption)
            .query(&collection.query)
            .capabilities(&collection.capabilities)
    }
    /// Adds a column.
    pub fn column(mut self, col: Col<'a, T>) -> Self {
        self.columns.push(col);
        self
    }
    /// Adds several columns, such as a row type's
    /// [`Columns::columns`](super::Columns::columns).
    pub fn columns(mut self, cols: impl IntoIterator<Item = Col<'a, T>>) -> Self {
        self.columns.extend(cols);
        self
    }
    /// Current query.
    pub fn query(mut self, q: &'a CollectionQuery) -> Self {
        self.query = Some(q);
        self
    }
    /// Backend capabilities.
    pub fn capabilities(mut self, caps: &'a Capabilities) -> Self {
        self.caps = Some(caps);
        self
    }
    /// Sets rows and pagination metadata together.
    pub fn page(mut self, page: &'a CollectionPage<T>) -> Self {
        self.rows = &page.rows;
        self.page = Some(page);
        self
    }
    /// GET endpoint.
    pub fn action(mut self, action: impl Into<Href>) -> Self {
        self.action = action.into();
        self
    }
    /// Each row's stable identifier, rendered as `data-row-id` on its
    /// `<tr>` and submitted by [`DataTable::selectable`]'s checkboxes. Use
    /// the record's key, not its position, so it survives sorting and
    /// paging.
    pub fn row_id(mut self, id: impl Fn(&T) -> String + 'a) -> Self {
        self.row_id = Some(RowText(Box::new(id)));
        self
    }
    /// Adds a checkbox to each row, submitted as `name=<row id>` with the
    /// form whose id is `form`. Put that form (a bulk action's buttons)
    /// anywhere on the page; the checkboxes join it through their `form`
    /// attribute, so selection needs no JavaScript.
    ///
    /// `label` names each row for its checkbox ("Select order 42"). Only
    /// the rows on this page can be selected; the receiving handler gets
    /// their ids, and must check again that each still exists and that the
    /// user may act on it. Requires [`DataTable::row_id`].
    ///
    /// ```
    /// use stucco_core::to_html;
    /// use stucco_ui::collections::{Col, DataTable};
    ///
    /// let rows = [(7, "Ada")];
    /// let table = DataTable::new(&rows, "Orders")
    ///     .row_id(|r: &(u32, &str)| r.0.to_string())
    ///     .selectable("bulk", "id", |r| format!("order {}", r.0))
    ///     .column(Col::text("customer", "Customer", |r: &(u32, &str)| r.1.to_string()));
    /// let html = to_html(&table);
    /// assert!(html.contains(r#"<tr data-row-id="7">"#));
    /// assert!(html.contains(r#"form="bulk" name="id" value="7" aria-label="Select order 7""#));
    /// ```
    pub fn selectable(mut self, form: &str, name: &str, label: impl Fn(&T) -> String + 'a) -> Self {
        self.selection = Some(Selection {
            form: form.to_owned(),
            name: name.to_owned(),
            label: RowText(Box::new(label)),
            selected: &[],
        });
        self
    }
    /// Rows whose checkboxes start checked, by id (after a bulk action
    /// failed, say).
    pub fn selected(mut self, ids: &'a [String]) -> Self {
        if let Some(selection) = &mut self.selection {
            selection.selected = ids;
        }
        self
    }
    /// What to show when the collection itself is empty (not merely
    /// filtered to nothing): usually an [`EmptyState`] explaining what
    /// belongs here, with the action that adds the first record.
    pub fn empty(mut self, content: impl Render + 'a) -> Self {
        self.empty = Some(Slot::new(content));
        self
    }
}
passthrough!(<T> DataTable<'_, T>);
/// The `data-kind` for cells of a column that needs special alignment.
fn cell_kind<T>(col: &Col<'_, T>) -> Option<&'static str> {
    match col.spec.kind {
        _ if col.actions => Some("actions"),
        ColumnKind::Number => Some("number"),
        ColumnKind::Date => Some("date"),
        _ => None,
    }
}
impl<T> Render for DataTable<'_, T> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::COLLECTIONS);
        let default_q = CollectionQuery::default();
        let default_caps = Capabilities::default();
        let query = self.query.unwrap_or(&default_q);
        let source = self.caps.unwrap_or(&default_caps);
        let specs: Vec<_> = self.columns.iter().map(|c| c.spec.clone()).collect();
        let caps = Capabilities {
            sortable: self
                .columns
                .iter()
                .filter(|c| c.sortable && source.sortable.contains(&c.spec.key))
                .map(|c| c.spec.key.clone())
                .collect(),
            filterable: self
                .columns
                .iter()
                .filter(|c| c.filterable && source.filterable.contains(&c.spec.key))
                .map(|c| c.spec.key.clone())
                .collect(),
            searchable: source.searchable && self.columns.iter().any(|c| c.searchable),
            total_count: source.total_count,
            offset: source.offset,
        };
        let view =
            CollectionView::new(self.action.clone(), query, &caps, &specs).label(&self.caption);
        let total = self.page.and_then(|p| p.total).filter(|_| caps.total_count);
        // Rows handed over without a source's capabilities have nothing to
        // search, filter or page, so they get no filter bar.
        let filters = self.caps.map(|_| {
            self.columns.iter().fold(FilterBar::new(&view), |bar, c| {
                bar.column_label(c.spec.key.clone(), c.label.clone())
            })
        });
        let offset_pages = total.filter(|_| caps.offset);
        let pagination = |page: &CollectionPage<T>| {
            Pagination::new(self.action.clone(), query)
                .label(format!("{} pagination", self.caption))
                .cursors(page.next.clone(), page.prev.clone())
                .total(offset_pages)
        };
        let active = self.caps.map(|_| {
            self.columns
                .iter()
                .fold(ActiveFilters::new(&view), |chips, c| {
                    chips.column_label(c.spec.key.clone(), c.label.clone())
                })
        });
        let mut root = el::div()
            .class("st-data-table")
            .child(filters)
            .child(active);
        if self.rows.is_empty() {
            let noun = self.caption.to_lowercase();
            // Past the start (a later numbered page, or a cursor page with
            // rows before it) there are rows elsewhere: offer the way back,
            // not "no matches". A cursor alone proves nothing; it may be
            // stale or forged.
            let later_page = matches!(query.window, Window::Offset { page } if page > 1)
                && self.page.is_none_or(|p| p.total != Some(0));
            let past_start = later_page || self.page.is_some_and(|p| p.prev.is_some());
            root = if past_start {
                root.child(
                    EmptyState::new("Nothing on this page")
                        .description(el::p().text(format!("There are no more {noun} here.")))
                        .actions(
                            el::a()
                                .href(
                                    query
                                        .clone()
                                        .with_window(Window::default())
                                        .link(&self.action),
                                )
                                .text("Go to the first page"),
                        ),
                )
            } else if query.is_filtered() {
                root.child(
                    EmptyState::new(format!("No matching {noun}"))
                        .description(el::p().text("Try another search, or remove some filters."))
                        .actions(
                            el::a()
                                .href(query.clone().reset().link(&self.action))
                                .text("Clear search and filters"),
                        ),
                )
            } else {
                match &self.empty {
                    Some(empty) => root.child(empty),
                    None => root.child(EmptyState::new(format!("No {noun} yet"))),
                }
            };
            // An empty page past the end still links back to the pages that
            // have rows, keeping the search and filters.
            if let Some(page) = self.page {
                root = root.child(pagination(page));
            }
        } else {
            debug_assert!(
                self.selection.is_none() || self.row_id.is_some(),
                "DataTable::selectable needs DataTable::row_id"
            );
            let selection = self.selection.as_ref().filter(|_| self.row_id.is_some());
            let mut header = Row::new();
            if selection.is_some() {
                header = header.header_with_attrs(
                    el::span().class("st-sr-only").text("Select"),
                    Attrs::default().data("kind", "select"),
                );
            }
            for c in &self.columns {
                let sortable = caps.sortable.contains(&c.spec.key);
                let active = sortable && query.sort.as_deref() == Some(&c.spec.key);
                let direction = if active {
                    query.direction.reversed()
                } else {
                    Direction::Asc
                };
                let content = if sortable {
                    el::a()
                        .href(
                            query
                                .clone()
                                .with_sort(&c.spec.key, direction)
                                .link(&self.action),
                        )
                        .text(&c.label)
                } else {
                    el::span().text(&c.label)
                };
                let attrs = if active {
                    Attrs::default().aria(
                        "sort",
                        if query.direction == Direction::Asc {
                            "ascending"
                        } else {
                            "descending"
                        },
                    )
                } else {
                    Attrs::default()
                };
                let attrs = match cell_kind(c) {
                    Some(kind) => attrs.data("kind", kind),
                    None => attrs,
                };
                let attrs = if sortable {
                    attrs.data("sortable", "true")
                } else {
                    attrs
                };
                header = header.header_with_attrs(content, attrs);
            }
            let mut table = Table::new(&self.caption).header(header);
            for record in self.rows {
                let mut row = Row::new();
                let id = self.row_id.as_ref().map(|id| (id.0)(record));
                if let Some(id) = &id {
                    row = row.data("row-id", id.clone());
                }
                if let (Some(selection), Some(id)) = (selection, &id) {
                    let checkbox = el::input()
                        .class("st-row-select")
                        .attr("type", "checkbox")
                        .attr("form", selection.form.clone())
                        .attr("name", selection.name.clone())
                        .attr("value", id.clone())
                        .aria("label", format!("Select {}", (selection.label.0)(record)))
                        .bool_attr("checked", selection.selected.contains(id));
                    row = row.cell_with_attrs(checkbox, Attrs::default().data("kind", "select"));
                }
                for column in &self.columns {
                    let value = (column.render)(record);
                    let mut attrs = Attrs::default();
                    if let Some(kind) = cell_kind(column) {
                        attrs = attrs.data("kind", kind);
                    }
                    if column.wrap {
                        attrs = attrs.data("wrap", "true");
                    }
                    row = row.cell_with_attrs(value, attrs);
                }
                table = table.row(row);
            }
            let offset = match (caps.offset, &query.window) {
                (true, Window::Offset { page }) => Some(
                    page.saturating_sub(1)
                        .saturating_mul(u64::from(query.per_page)),
                ),
                _ => None,
            };
            let mut footer = el::div()
                .class("st-data-table-footer")
                .child(LiveRegion::new().child(ResultCount::new(self.rows.len(), total, offset)));
            if let Some(page) = self.page {
                footer = footer.child(pagination(page));
            }
            root = root.child(table).child(footer);
        }
        apply(root, &self.attrs, &[]).render(cx);
    }
}
