//! Server-side collection and shell examples.
use crate::shell::{component_page, section};
use stucco::actions::Button;
use stucco::app::{AppShell, Footer, PageHeader, SectionHeader};
use stucco::collections::{Col, DataTable};
use stucco::data::{Card, Panel, ResultCount, Row, Table};
use stucco::forms::Form;
use stucco::layout::Cluster;
use stucco::navigation::NavLink;
use stucco::navigation::Pagination;
use stucco::{
    Bundle, Capabilities, CollectionPage, CollectionQuery, Cursor, Filter, Page, Size, Slot,
    Window, el,
};

/// An order row for the selectable table.
struct Order {
    id: u32,
    customer: &'static str,
    status: &'static str,
    total: f64,
    note: &'static str,
}

/// Tables, typed controls, empty states and both pagination styles.
pub fn page(bundle: &Bundle) -> String {
    let page = CollectionPage {
        rows: vec!["Ada", "Grace"],
        next: Cursor::new("next"),
        prev: None,
        total: None,
    };
    let query = CollectionQuery::default();
    let caps = Capabilities {
        sortable: vec!["name".into()],
        filterable: vec!["name".into()],
        searchable: true,
        ..Capabilities::default()
    };
    let empty: CollectionPage<&str> = CollectionPage {
        rows: vec![],
        next: None,
        prev: None,
        total: Some(0),
    };
    let orders = CollectionPage {
        rows: vec![
            Order {
                id: 1042,
                customer: "Ada Lovelace",
                status: "paid",
                total: 1234.5,
                note: "Deliver to the side entrance; the front door is being replaced this week.",
            },
            Order {
                id: 1043,
                customer: "Grace Hopper",
                status: "paid",
                total: 89.0,
                note: "",
            },
        ],
        next: None,
        prev: None,
        total: Some(2),
    };
    let order_caps = Capabilities {
        sortable: vec!["customer".into(), "total".into()],
        filterable: vec!["status".into(), "total".into()],
        searchable: true,
        total_count: true,
        offset: true,
    };
    let filtered = CollectionQuery::default()
        .with_search("a")
        .with_filter("status", Some(Filter::Enumeration("paid".into())))
        .with_filter(
            "total",
            Some(Filter::Number {
                min: Some(50.0),
                max: None,
            }),
        );
    let no_match: CollectionPage<Order> = CollectionPage {
        rows: vec![],
        next: None,
        prev: None,
        total: Some(0),
    };
    let order_columns = || {
        vec![
            Col::number("id", "ID", |o: &Order| f64::from(o.id))
                .href(|o: &Order| format!("#order-{}", o.id)),
            Col::text("customer", "Customer", |o: &Order| o.customer.into())
                .sortable()
                .searchable(),
            Col::enumeration(
                "status",
                "Status",
                vec![("paid".into(), "Paid".into())],
                |o: &Order| o.status.into(),
            )
            .filter(),
            Col::number("total", "Total", |o: &Order| o.total)
                .display(|o: &Order| format!("${:.2}", o.total))
                .sortable()
                .filter(),
            Col::text("note", "Note", |o: &Order| {
                if o.note.is_empty() {
                    "No note".into()
                } else {
                    o.note.into()
                }
            })
            .wrap(),
            Col::actions("Actions", |o: &Order| {
                Slot::new(
                    el::a().href(format!("#edit-{}", o.id)).text("Edit").child(
                        el::span()
                            .class("st-sr-only")
                            .text(format!(" order {}", o.id)),
                    ),
                )
            }),
        ]
    };
    let numbered = CollectionQuery {
        window: Window::Offset { page: 3 },
        ..query.clone()
    };
    component_page(
        bundle,
        "Data and collections",
        "Native tables and GET navigation, with no behavior module.",
        [
            section(
                "Tables and panels",
                Panel::new("Customer overview")
                    .description("A labelled region")
                    .body(
                        Card::new()
                            .header("Current customers")
                            .child(
                                Table::new("Customer summary")
                                    .header(Row::new().header("Name").header("Status"))
                                    .row(Row::new().header("Ada").cell("paid")),
                            )
                            .footer(ResultCount::new(1, Some(1), Some(0))),
                    ),
            ),
            section(
                "Cursor collection",
                DataTable::from_page(&page, "Customers")
                    .query(&query)
                    .capabilities(&caps)
                    .action("collections.html")
                    .column(
                        Col::text("name", "Name", |name: &&str| (*name).into())
                            .sortable()
                            .searchable()
                            .filter(),
                    ),
            ),
            section(
                "Filtered, selectable rows with actions",
                (
                    Form::get("collections.html").id("bulk").child(
                        Cluster::new()
                            .child(el::span().text("With the selected orders:"))
                            .child(
                                Button::new("Archive")
                                    .submit()
                                    .name("action")
                                    .value("archive"),
                            ),
                    ),
                    DataTable::from_page(&orders, "Orders")
                        .query(&filtered)
                        .capabilities(&order_caps)
                        .action("collections.html")
                        .row_id(|o: &Order| o.id.to_string())
                        .selectable("bulk", "id", |o| format!("order {}", o.id))
                        .columns(order_columns()),
                ),
            ),
            section(
                "No matching results",
                DataTable::from_page(&no_match, "Orders")
                    .query(&filtered)
                    .capabilities(&order_caps)
                    .action("collections.html")
                    .columns(order_columns()),
            ),
            section(
                "Empty and unsupported operations",
                DataTable::from_page(&empty, "Empty collection")
                    .column(Col::text("name", "Name", |name: &&str| (*name).into()).sortable()),
            ),
            section(
                "Numbered navigation",
                Pagination::new("collections.html?mode=pages", &numbered).total(Some(250)),
            ),
        ],
    )
}
/// A shell with one main landmark, a disclosure sidebar, and footer.
pub fn app_page(bundle: &Bundle) -> String {
    Page::new(bundle, "Application shell")
        .body(
            AppShell::new()
                .header(
                    PageHeader::new("Application shell").description("A server-rendered workspace"),
                )
                .nav_label("Gallery")
                .link(NavLink::new("Gallery", "index.html"))
                .link(NavLink::new("Collections", "collections.html"))
                .main(
                    Panel::new("Workspace").body(
                        SectionHeader::new("Recent activity")
                            .level(3)
                            .size(Size::Md)
                            .description("No recent changes."),
                    ),
                )
                .footer(Footer::new().child("Stucco gallery")),
        )
        .render()
}
