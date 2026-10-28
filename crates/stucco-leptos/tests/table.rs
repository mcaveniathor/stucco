#![cfg(feature = "ssr")]

mod common;

use common::assert_parity;
use leptos::prelude::*;
use stucco_core::{Attrs, to_html};
use stucco_leptos::{Cell, CellKind, HeaderCell, Row, Table, TableBody, TableHead};
use stucco_ui::data::{Row as StaticRow, Table as StaticTable};

fn render(view: impl FnOnce() -> String) -> String {
    Owner::new().with(view)
}

fn number() -> Attrs {
    Attrs::default().data("kind", "number")
}

fn static_orders(cards: bool) -> String {
    let table = StaticTable::new("Orders")
        .header(
            StaticRow::new()
                .header("Customer")
                .header_with_attrs("Total", number()),
        )
        .row(
            StaticRow::new()
                .header("Ada")
                .cell_with_attrs("$12", number()),
        );
    to_html(&if cards {
        table.card_labels(["Customer", "Total"])
    } else {
        table
    })
}

fn orders(cards: bool) -> String {
    render(|| {
        view! {
            <Table caption="Orders" cards=cards>
                <TableHead>
                    <Row>
                        <HeaderCell label="Customer">"Customer"</HeaderCell>
                        <HeaderCell kind=CellKind::Number label="Total">"Total"</HeaderCell>
                    </Row>
                </TableHead>
                <TableBody>
                    <Row>
                        <HeaderCell label="Customer">"Ada"</HeaderCell>
                        <Cell kind=CellKind::Number label="Total">"$12"</Cell>
                    </Row>
                </TableBody>
            </Table>
        }
        .to_html()
    })
}

#[test]
fn table_matches_static_markup() {
    assert_parity(&orders(false), &static_orders(false));
}

#[test]
fn cards_table_matches_static_markup() {
    let html = orders(true);
    assert_parity(&html, &static_orders(true));
    assert!(html.contains("st-table-cards"), "{html}");
    assert!(html.contains(r#"role="columnheader""#), "{html}");
}

#[test]
fn labels_are_ignored_unless_cards_are_on() {
    assert!(!orders(false).contains("st-table-card-label"));
}

#[test]
fn a_body_cell_without_a_label_gets_an_empty_card_label() {
    let html = render(|| {
        view! {
            <Table caption="Orders" cards=true>
                <TableBody><Row><Cell>"x"</Cell></Row></TableBody>
            </Table>
        }
        .to_html()
    });
    let expected = to_html(
        &StaticTable::new("Orders")
            .row(StaticRow::new().cell("x"))
            .card_labels(Vec::<String>::new()),
    );
    assert_parity(&html, &expected);
}

#[test]
fn empty_table_has_caption_and_region() {
    let html = render(|| view! { <Table caption="Orders"><TableBody/></Table> }.to_html());
    assert_parity(&html, &to_html(&StaticTable::new("Orders")));
    assert!(html.contains(r#"role="region""#), "{html}");
    assert!(html.contains(r#"aria-label="Orders""#), "{html}");
    assert!(html.contains(r#"tabindex="0""#), "{html}");
}

#[test]
fn cells_accept_arbitrary_views() {
    let html = render(|| {
        view! {
            <Table caption="Orders">
                <TableBody><Row><Cell><a href="/o/1">"View"</a><b>"!"</b></Cell></Row></TableBody>
            </Table>
        }
        .to_html()
    });
    assert!(
        html.contains(r#"<a href="/o/1">View</a><b>!</b>"#),
        "{html}"
    );
}

#[test]
fn caption_and_cells_are_escaped() {
    let html = render(|| {
        view! {
            <Table caption="<Orders> & more">
                <TableBody><Row><Cell>"<script>"</Cell></Row></TableBody>
            </Table>
        }
        .to_html()
    });
    assert!(html.contains("&lt;Orders&gt; &amp; more"), "{html}");
    assert!(html.contains("&lt;script&gt;"), "{html}");
    assert!(!html.contains("<script>"), "{html}");
}

#[test]
fn header_cell_renders_aria_sort_and_date_kind() {
    let html = render(|| {
        view! {
            <Table caption="Orders">
                <TableHead><Row>
                    <HeaderCell aria_sort="ascending" kind=CellKind::Date>"Placed"</HeaderCell>
                </Row></TableHead>
            </Table>
        }
        .to_html()
    });
    assert!(html.contains(r#"aria-sort="ascending""#), "{html}");
    assert!(html.contains(r#"data-kind="date""#), "{html}");
    assert!(html.contains(r#"scope="col""#), "{html}");
}

#[test]
fn rows_can_come_from_a_for_loop_in_order() {
    let html = render(|| {
        view! {
            <Table caption="Numbers">
                <TableBody>
                    <For each=move || vec![1, 2, 3] key=|n| *n
                        children=move |n| view! { <Row><Cell>{format!("n{n}")}</Cell></Row> }/>
                </TableBody>
            </Table>
        }
        .to_html()
    });
    let (a, b, c) = (html.find("n1"), html.find("n2"), html.find("n3"));
    assert!(a < b && b < c && a.is_some(), "{html}");
}
