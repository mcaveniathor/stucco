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

#[test]
fn rows_without_a_source_get_no_filter_bar() {
    let rows = ["Ada", "Grace"];
    let col = || Col::text("name", "Name", |n: &&str| (*n).into()).sortable();
    let plain = to_html(&DataTable::new(&rows, "Names").column(col()));
    assert!(!plain.contains("st-filter-bar"));
    assert!(
        !plain.contains("<a href"),
        "no sort links without capabilities"
    );
    let empty: [&str; 0] = [];
    let none = to_html(&DataTable::new(&empty, "Names").column(col()));
    assert!(none.contains("No results") && !none.contains("Clear filters"));
    let caps = Capabilities {
        sortable: vec!["name".into()],
        ..Capabilities::default()
    };
    let sourced = to_html(
        &DataTable::new(&rows, "Names")
            .capabilities(&caps)
            .column(col()),
    );
    assert!(sourced.contains("st-filter-bar"));
}

/// An empty page of an offset collection, `page` of `total` rows, filtered.
fn empty_offset_page(page: u64, total: u64) -> String {
    let empty = CollectionPage::<Record> {
        rows: vec![],
        next: None,
        prev: None,
        total: Some(total),
    };
    let q = CollectionQuery::default()
        .with_filter("status", Some(Filter::Enumeration("paid".into())))
        .with_window(Window::Offset { page });
    let caps = caps();
    to_html(
        &DataTable::from_page(&empty, "Orders")
            .action("/orders")
            .query(&q)
            .capabilities(&caps)
            .column(Col::text("name", "Name", |r: &Record| r.name.clone()))
            .column(
                Col::enumeration(
                    "status",
                    "Status",
                    vec![("paid".into(), "Paid".into())],
                    |r: &Record| r.status.clone(),
                )
                .filter(),
            ),
    )
}

#[test]
fn an_empty_page_past_the_end_links_back_keeping_the_filters() {
    // Past the last page: links back to the pages with rows.
    let html = empty_offset_page(9, 45);
    assert!(html.contains("No results"), "{html}");
    assert!(html.contains("st-pagination"), "{html}");
    assert!(html.contains("aria-label=\"Page 1\""), "{html}");
    let last = html
        .split("aria-label=\"Page ")
        .filter_map(|s| s.split('"').next())
        .filter_map(|n| n.parse::<u64>().ok())
        .max();
    assert!(last.is_some_and(|n| n >= 1), "{html}");
    for link in html
        .split("href=\"")
        .skip(1)
        .filter_map(|s| s.split('"').next())
    {
        if link.contains("page=") {
            assert!(
                link.contains("f.status=paid"),
                "a page link drops the filter: {link}"
            );
        }
    }

    // No rows at all, on a later page: one link, to page 1, never page 0.
    let html = empty_offset_page(3, 0);
    assert!(html.contains("aria-label=\"Page 1\""), "{html}");
    assert!(!html.contains("aria-label=\"Page 0\""), "{html}");

    // An empty first page has nowhere to go.
    let html = empty_offset_page(1, 0);
    assert!(!html.contains("st-pagination"), "{html}");
}

#[test]
fn an_empty_cursor_page_keeps_its_previous_link() {
    let empty = CollectionPage::<Record> {
        rows: vec![],
        next: None,
        prev: stucco_core::Cursor::new("c1"),
        total: None,
    };
    let caps = Capabilities {
        offset: false,
        total_count: false,
        ..caps()
    };
    let html = to_html(
        &DataTable::from_page(&empty, "Orders")
            .action("/orders")
            .query(&CollectionQuery::default())
            .capabilities(&caps)
            .column(Col::text("name", "Name", |r: &Record| r.name.clone())),
    );
    assert!(html.contains("No results"), "{html}");
    assert!(html.contains(">Previous</a>"), "{html}");
}
