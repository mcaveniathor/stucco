use crate::model::Order;
use stucco::{Capabilities, CollectionQuery, ColumnKind, ColumnSpec, Filter, Window};
use stucco_redb::{CollectionMapping, ScanRequest};
pub(crate) struct OrdersMapping {
    pub pages: bool,
}
pub(crate) fn columns() -> Vec<ColumnSpec> {
    vec![
        ColumnSpec::new("id", ColumnKind::Number),
        ColumnSpec::new("customer", ColumnKind::Text),
        ColumnSpec::new(
            "status",
            ColumnKind::Enumeration(["pending", "paid", "shipped"].map(String::from).to_vec()),
        ),
        ColumnSpec::new("total_cents", ColumnKind::Number),
        ColumnSpec::new("created", ColumnKind::Date),
    ]
}
impl CollectionMapping<Order> for OrdersMapping {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            sortable: vec!["id".into(), "customer".into()],
            filterable: vec!["status".into(), "total_cents".into(), "created".into()],
            searchable: true,
            total_count: self.pages,
            offset: self.pages,
        }
    }
    fn columns(&self) -> Vec<ColumnSpec> {
        columns()
    }
    fn scan_request(&self, q: &CollectionQuery) -> ScanRequest {
        ScanRequest {
            index: (q.sort.as_deref() == Some("customer")).then(|| "customer".into()),
            direction: q.direction,
            window: q.window.clone(),
            per_page: q.per_page,
            scope: q.clone().with_window(Window::default()).to_query_string(),
            offset_mode: self.pages,
        }
    }
    fn matches(&self, row: &Order, q: &CollectionQuery) -> bool {
        if !q.search.is_empty()
            && !row
                .customer
                .to_lowercase()
                .contains(&q.search.to_lowercase())
        {
            return false;
        }
        q.filters
            .iter()
            .all(|(key, filter)| match (key.as_str(), filter) {
                ("status", Filter::Enumeration(status)) => &row.status == status,
                ("total_cents", Filter::Number { min, max }) => {
                    min.is_none_or(|n| row.total_cents as f64 >= n)
                        && max.is_none_or(|n| row.total_cents as f64 <= n)
                }
                ("created", Filter::Date { min, max }) => {
                    min.as_ref().is_none_or(|n| &row.created >= n)
                        && max.as_ref().is_none_or(|n| &row.created <= n)
                }
                _ => true,
            })
    }
}
