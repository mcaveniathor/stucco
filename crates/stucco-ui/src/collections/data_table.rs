use super::{Col, CollectionToolbar, CollectionView};
use crate::{
    data::{Row, Table},
    feedback::EmptyState,
    navigation::Pagination,
    passthrough::apply,
};
use stucco_core::{
    Attrs, Capabilities, CollectionPage, CollectionQuery, Cx, Direction, Href, Render, el,
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
    /// Adds a column.
    pub fn column(mut self, col: Col<'a, T>) -> Self {
        self.columns.push(col);
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
        let mut root = el::div()
            .class("st-data-table")
            .child(CollectionToolbar::new(&view, self.rows.len(), total));
        if self.rows.is_empty() {
            root = root.child(
                EmptyState::new("No results")
                    .description("Try adjusting your search or filters.")
                    .actions(
                        el::a()
                            .href(query.clone().reset().link(&self.action))
                            .text("Reset filters"),
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
                header = header.header_with_attrs(content, attrs);
            }
            let mut table = Table::new(&self.caption).header(header);
            for record in self.rows {
                let mut row = Row::new();
                for column in &self.columns {
                    row = row.cell((column.render)(record));
                }
                table = table.row(row);
            }
            root = root.child(table);
        }
        if let Some(page) = self.page {
            root = root.child(
                Pagination::new(self.action.clone(), query)
                    .cursors(page.next.clone(), page.prev.clone())
                    .total(total.filter(|_| caps.offset)),
            );
        }
        apply(root, &self.attrs, &[]).render(cx);
    }
}
