use crate::model::Order;
use stucco::collections::Columns;
use stucco::{Capabilities, CollectionQuery, ColumnSpec, Filter, Window};
use stucco_redb::{CollectionMapping, ScanRequest};
pub(crate) struct OrdersMapping {
    pub pages: bool,
}
impl CollectionMapping<Order> for OrdersMapping {
    fn capabilities(&self) -> Capabilities {
        Capabilities {
            sortable: vec!["id".into(), "customer".into()],
            filterable: vec!["status".into(), "total".into(), "created".into()],
            searchable: true,
            total_count: self.pages,
            offset: self.pages,
        }
    }
    fn columns(&self) -> Vec<ColumnSpec> {
        Order::column_specs()
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
                ("total", Filter::Number { min, max }) => {
                    let dollars = row.total_cents as f64 / 100.0;
                    min.is_none_or(|n| dollars >= n) && max.is_none_or(|n| dollars <= n)
                }
                ("created", Filter::Date { min, max }) => {
                    min.as_ref().is_none_or(|n| &row.created >= n)
                        && max.as_ref().is_none_or(|n| &row.created <= n)
                }
                _ => true,
            })
    }
}
