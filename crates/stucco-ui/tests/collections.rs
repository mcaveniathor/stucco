#![cfg(feature = "collections")]
use stucco_core::{
    Capabilities, CollectionPage, CollectionQuery, ColumnKind, ColumnSpec, Direction, Filter,
    Window, to_html,
};
use stucco_ui::collections::{Col, DataTable};
#[derive(Debug)]
struct Record {
    name: String,
    status: String,
}
fn caps() -> Capabilities {
    Capabilities {
        sortable: vec!["name".into()],
        filterable: vec!["status".into()],
        searchable: true,
        total_count: true,
        offset: true,
    }
}
#[test]
fn server_controls_preserve_query_and_use_real_table_headers() {
    let rows = vec![Record {
        name: "<Ada>".into(),
        status: "paid".into(),
    }];
    let page = CollectionPage {
        rows,
        next: None,
        prev: None,
        total: Some(1),
    };
    let q = CollectionQuery::default()
        .with_sort("name", Direction::Desc)
        .with_filter("status", Some(Filter::Enumeration("paid".into())));
    let caps = caps();
    let html = to_html(
        &DataTable::from_page(&page, "Orders")
            .action("/orders?mode=pages")
            .query(&q)
            .capabilities(&caps)
            .column(
                Col::text("name", "Customer", |r: &Record| r.name.clone())
                    .sortable()
                    .searchable(),
            )
            .column(
                Col::enumeration(
                    "status",
                    "Status",
                    vec![("paid".into(), "Paid".into())],
                    |r: &Record| r.status.clone(),
                )
                .filter(),
            ),
    );
    assert!(html.contains("aria-sort=\"descending\""));
    assert!(html.contains("&lt;Ada&gt;"));
    assert!(html.contains("method=\"get\""));
    assert!(html.contains("name=\"mode\" value=\"pages\""));
    assert!(html.contains("f.status=paid"));
    assert!(html.contains("Search orders"));
}
#[test]
fn empty_and_unsupported_operations_have_useful_fallbacks() {
    let page = CollectionPage::<Record> {
        rows: vec![],
        next: None,
        prev: None,
        total: None,
    };
    let q = CollectionQuery::default().with_window(Window::Offset { page: u64::MAX });
    let caps = Capabilities::default();
    let html = to_html(
        &DataTable::from_page(&page, "Orders")
            .query(&q)
            .capabilities(&caps)
            .column(
                Col::text("name", "Name", |r: &Record| r.name.clone())
                    .sortable()
                    .searchable(),
            ),
    );
    assert!(html.contains("No results"));
    assert!(html.contains("Clear filters"));
    assert!(!html.contains("name=\"q\""));
    assert!(!html.contains("aria-sort"));
}
#[test]
fn date_and_numeric_ranges_render_native_inputs() {
    let caps = Capabilities {
        filterable: vec!["created".into(), "amount".into()],
        ..Default::default()
    };
    let q = CollectionQuery::default();
    let columns = vec![
        ColumnSpec::new("created", ColumnKind::Date),
        ColumnSpec::new("amount", ColumnKind::Number),
    ];
    let view = stucco_ui::collections::CollectionView::new("/orders", &q, &caps, &columns);
    let html = to_html(&stucco_ui::collections::FilterBar::new(&view));
    assert!(html.contains("type=\"date\""));
    assert!(html.contains("type=\"number\""));
    assert!(html.contains("name=\"f.amount.min\""));
    assert!(html.contains("name=\"f.created.max\""));
}
