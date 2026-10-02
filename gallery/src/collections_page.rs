//! Server-side collection and shell examples.
use crate::shell::{component_page, section};
use stucco::app::{AppShell, Footer, PageHeader, SectionHeader};
use stucco::collections::{Col, DataTable};
use stucco::data::{Card, Panel, ResultCount, Row, Table};
use stucco::navigation::NavLink;
use stucco::navigation::Pagination;
use stucco::{Bundle, Capabilities, CollectionPage, CollectionQuery, Cursor, Page, Size, Window};

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
