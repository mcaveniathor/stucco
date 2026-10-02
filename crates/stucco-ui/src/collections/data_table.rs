use super::{Col, CollectionView, FilterBar};
use crate::{
    data::{ResultCount, Row, Table},
    feedback::{EmptyState, LiveRegion},
    navigation::Pagination,
    passthrough::apply,
};
use stucco_core::{
    Attrs, Capabilities, Collection, CollectionPage, CollectionQuery, ColumnKind, Cx, Direction,
    Href, Render, Window, el,
};
/// Server-rendered collection table with native GET controls.
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
}
passthrough!(<T> DataTable<'_, T>);
/// The `data-kind` for cells of a column kind that needs special alignment.
fn kind_attr(kind: &ColumnKind) -> Option<&'static str> {
    match kind {
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
        let filters = self.columns.iter().fold(FilterBar::new(&view), |bar, c| {
            bar.column_label(c.spec.key.clone(), c.label.clone())
        });
        let mut root = el::div().class("st-data-table").child(filters);
        if self.rows.is_empty() {
            root = root.child(
                EmptyState::new("No results")
                    .description("Try adjusting your search or filters.")
                    .actions(
                        el::a()
                            .href(query.clone().reset().link(&self.action))
                            .text("Clear filters"),
                    ),
            );
        } else {
            let mut header = Row::new();
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
                let attrs = match kind_attr(&c.spec.kind) {
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
                for column in &self.columns {
                    let value = (column.render)(record);
                    row = match kind_attr(&column.spec.kind) {
                        Some(kind) => {
                            row.cell_with_attrs(value, Attrs::default().data("kind", kind))
                        }
                        None => row.cell(value),
                    };
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
                footer = footer.child(
                    Pagination::new(self.action.clone(), query)
                        .label(format!("{} pagination", self.caption))
                        .cursors(page.next.clone(), page.prev.clone())
                        .total(total.filter(|_| caps.offset)),
                );
            }
            root = root.child(table).child(footer);
        }
        apply(root, &self.attrs, &[]).render(cx);
    }
}
