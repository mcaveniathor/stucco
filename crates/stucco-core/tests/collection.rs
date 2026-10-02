use stucco_core::{
    Capabilities, CollectionQuery, ColumnKind, ColumnSpec, Cursor, Direction, Filter, Href, Window,
};

fn declarations() -> (Capabilities, Vec<ColumnSpec>) {
    (
        Capabilities {
            sortable: vec!["name".into()],
            filterable: vec!["status".into(), "amount".into(), "date".into()],
            searchable: true,
            total_count: true,
            offset: true,
        },
        vec![
            ColumnSpec::new("name", ColumnKind::Text),
            ColumnSpec::new(
                "status",
                ColumnKind::Enumeration(vec!["paid".into(), "pending".into()]),
            ),
            ColumnSpec::new("amount", ColumnKind::Number),
            ColumnSpec::new("date", ColumnKind::Date),
        ],
    )
}

#[test]
fn query_round_trips_and_changes_reset_position() {
    let (caps, columns) = declarations();
    let query = CollectionQuery::parse(
        "sort=name&dir=desc&q=Ada+%26+%E2%99%A5&f.status=paid&per=2&after=YWJj",
        &caps,
        &columns,
    );
    assert_eq!(query.search, "Ada & ♥");
    assert_eq!(query.direction, Direction::Desc);
    assert_eq!(query.filters["status"], Filter::Enumeration("paid".into()));
    assert_eq!(
        CollectionQuery::parse(&query.to_query_string(), &caps, &columns),
        query
    );
    assert_eq!(
        query.clone().with_search("new").window,
        Window::Offset { page: 1 }
    );
    assert_eq!(query.clone().with_per_page(0).per_page, 25);
    assert_eq!(
        query.with_sort("name", Direction::Asc).window,
        Window::Offset { page: 1 }
    );
}

#[test]
fn invalid_and_unsupported_input_degrades_safely() {
    let (caps, columns) = declarations();
    let query = CollectionQuery::parse(
        "sort=unknown&dir=no&page=0&per=999&after=a&before=b&q=%ZZ&f.status=secret&f.amount.min=NaN&f.date.min=2025-02-29",
        &caps,
        &columns,
    );
    assert!(query.sort.is_none());
    assert!(query.search.is_empty());
    assert!(query.filters.is_empty());
    assert_eq!(query.per_page, 100);
    assert_eq!(query.window, Window::Offset { page: 1 });
    let query = CollectionQuery::parse(
        "f.amount.min=10&f.amount.max=2&f.date.min=2024-02-29&f.date.max=2024-03-01",
        &caps,
        &columns,
    );
    assert!(!query.filters.contains_key("amount"));
    assert!(query.filters.contains_key("date"));
    assert_eq!(
        CollectionQuery::parse("q=ok&q=%ZZ&page=18446744073709551616", &caps, &columns).search,
        "ok"
    );
    assert_eq!(
        CollectionQuery::parse(&"x".repeat(16385), &caps, &columns),
        CollectionQuery::default()
    );
}

#[test]
fn cursor_and_count_limits_are_bounded() {
    assert!(Cursor::new("a/b").is_none());
    assert!(Cursor::new("").is_none());
    assert!(Cursor::new(&"a".repeat(4097)).is_none());
    let (mut caps, columns) = declarations();
    caps.offset = false;
    caps.searchable = false;
    let q = CollectionQuery::parse("page=8&q=secret", &caps, &columns);
    assert_eq!(q.window, Window::Offset { page: 1 });
    assert!(q.search.is_empty());
}

#[test]
fn collection_links_preserve_unrelated_query_and_fragment() {
    let q = CollectionQuery::default().with_search("Ada & Co");
    let href = q.link(&Href::new("/orders?mode=pages&q=old&after=old#results"));
    assert_eq!(
        href.as_str(),
        "/orders?mode=pages&q=Ada+%26+Co&per=25#results"
    );
    assert_eq!(q.link(&Href::invalid()), Href::invalid());
}

#[test]
fn default_sort_direction_survives_navigation() {
    let (caps, columns) = declarations();
    let q = CollectionQuery::parse("dir=desc", &caps, &columns);
    let next = q
        .clone()
        .with_window(Window::After(Cursor::new("anchor").unwrap()));
    assert_eq!(
        CollectionQuery::parse(&next.to_query_string(), &caps, &columns).direction,
        Direction::Desc
    );
}
#[test]
fn malformed_duplicate_filters_do_not_erase_valid_values() {
    let (caps, columns) = declarations();
    let q = CollectionQuery::parse(
        "f.status=paid&f.status=unknown&f.amount.min=10&f.amount.min=NaN&f.date.min=2024-02-29&f.date.min=invalid",
        &caps,
        &columns,
    );
    assert_eq!(
        q.filters.get("status"),
        Some(&Filter::Enumeration("paid".into()))
    );
    assert_eq!(
        q.filters.get("amount"),
        Some(&Filter::Number {
            min: Some(10.0),
            max: None
        })
    );
    assert!(q.filters.contains_key("date"));
}
