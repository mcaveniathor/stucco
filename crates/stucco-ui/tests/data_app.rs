#![cfg(all(feature = "data", feature = "app", feature = "feedback"))]
use stucco_core::{el, to_html};
use stucco_ui::{
    app::{AppShell, Footer, PageHeader},
    data::{Panel, ResultCount, Row, Table},
    feedback::{EmptyState, LiveRegion},
    navigation::NavLink,
};
#[test]
fn table_preserves_native_semantics_and_escapes_values() {
    let html = to_html(
        &Table::new("Orders")
            .header(Row::new().header("Customer"))
            .row(Row::new().cell("<script>")),
    );
    assert!(html.contains("<caption>Orders</caption>"));
    assert!(html.contains("scope=\"col\""));
    assert!(html.contains("&lt;script&gt;"));
    assert!(html.contains("<tbody>"));
}
#[test]
fn shell_owns_one_main_and_panel_wires_its_heading() {
    let html = to_html(
        &AppShell::new()
            .header(PageHeader::new("Orders"))
            .main(Panel::new("Details").body("Body"))
            .footer(Footer::new().child("Footer")),
    );
    assert_eq!(html.matches("<main").count(), 1);
    assert!(html.contains("href=\"#main\""));
    assert!(html.contains("aria-labelledby=\"panel-1\""));
    assert!(html.contains("id=\"panel-1\""));
    assert_eq!(html.matches("<footer").count(), 1);
}
#[test]
fn shell_marks_which_regions_it_has() {
    let bare = to_html(&AppShell::new().main("Body"));
    assert!(bare.contains(r#"<div class="st-app-body">"#));
    assert!(!bare.contains("st-app-nav"));
    let full = to_html(
        &AppShell::new()
            .nav_label("Primary")
            .link(NavLink::new("Orders", "/orders").current(true))
            .links([NavLink::new("Customers", "/customers")])
            .sidebar("Outline")
            .main("Body"),
    );
    assert!(full.contains(r#"<div class="st-app-body" data-nav data-sidebar>"#));
    assert!(full.contains(r#"<nav class="st-app-nav" aria-label="Primary"><a href="/orders" aria-current="page">Orders</a><a href="/customers">Customers</a></nav>"#));
    // Primary links come before the sidebar and main in reading order.
    let (nav, side, main) = (
        full.find("st-app-nav").unwrap(),
        full.find("st-app-sidebar").unwrap(),
        full.find("<main").unwrap(),
    );
    assert!(nav < side && side < main);
}
#[test]
fn empty_counts_and_feedback_are_accessible() {
    let html = to_html(&ResultCount::new(0, Some(0), Some(0)));
    assert!(html.contains("No results"));
    assert!(!html.contains("1–0"));
    let html = to_html(&ResultCount::new(3, None, None));
    assert!(html.contains("Showing 3 results"));
    let html = to_html(&EmptyState::new("Nothing").actions(el::a().href("/orders").text("Reset")));
    assert!(html.contains("Reset"));
    assert!(to_html(&LiveRegion::new().child("3 results")).contains("aria-live=\"polite\""));
}
