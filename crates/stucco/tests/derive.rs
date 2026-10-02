//! `#[derive(Columns)]`, end to end.
#![cfg(feature = "derive")]

use stucco::prelude::*;
use stucco::{CollectionPage, ColumnKind, to_html};

fn money(order: &Order) -> String {
    format!(
        "${}.{:02}",
        order.total_cents / 100,
        order.total_cents % 100
    )
}

fn dollars(order: &Order) -> f64 {
    order.total_cents as f64 / 100.0
}

#[derive(Columns)]
struct Order {
    #[col(label = "ID", sortable)]
    id: u64,
    #[col(sortable, searchable)]
    customer: String,
    #[col(enumeration("pending", "paid"), filter)]
    status: String,
    #[col(key = "total", label = "Total", value = dollars, display = money, filter)]
    total_cents: u64,
    #[col(date)]
    created: String,
    #[col(skip)]
    #[allow(dead_code)]
    notes: Vec<String>,
}

#[test]
fn derived_columns_match_the_fields() {
    let specs = Order::column_specs();
    let keys: Vec<&str> = specs.iter().map(|s| s.key.as_str()).collect();
    assert_eq!(keys, ["id", "customer", "status", "total", "created"]);
    assert_eq!(specs[0].kind, ColumnKind::Number);
    assert_eq!(specs[1].kind, ColumnKind::Text);
    assert_eq!(
        specs[2].kind,
        ColumnKind::Enumeration(vec!["pending".into(), "paid".into()])
    );
    assert_eq!(specs[4].kind, ColumnKind::Date);
}

#[test]
fn a_table_renders_from_derived_columns() {
    let page = CollectionPage {
        rows: vec![Order {
            id: 7,
            customer: "<Ada>".into(),
            status: "paid".into(),
            total_cents: 4800,
            created: "2026-10-02".into(),
            notes: vec![],
        }],
        next: None,
        prev: None,
        total: None,
    };
    let html = to_html(&DataTable::from_page(&page, "Orders").columns(Order::columns()));
    for heading in [">ID<", ">Customer<", ">Status<", ">Total<", ">Created<"] {
        assert!(html.contains(heading), "missing {heading}");
    }
    assert!(!html.contains("Notes"));
    assert!(html.contains("&lt;Ada&gt;"));
    assert!(html.contains(r#"data-value="paid""#));
    assert!(html.contains("$48.00"));
    // The filter works in dollars, from `value`, not in cents.
    assert!(!html.contains(">4800<"));
}

fn customer_page(customer: &Customer) -> String {
    format!("/customers/{}", customer.id)
}

#[derive(Columns)]
struct Customer {
    #[col(sortable)]
    id: u64,
    #[col(searchable, link = customer_page)]
    name: String,
}

#[test]
fn linked_columns_keep_their_kind_and_controls() {
    let specs = Customer::column_specs();
    assert_eq!(specs[1].kind, ColumnKind::Text);
    let rows = [Customer {
        id: 3,
        name: "Grace <Hopper>".into(),
    }];
    let html = to_html(&DataTable::new(&rows, "Customers").columns(Customer::columns()));
    assert!(
        html.contains(r#"<a href="/customers/3">Grace &lt;Hopper&gt;</a>"#),
        "{html}"
    );
}
