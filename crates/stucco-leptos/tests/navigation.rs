#![cfg(feature = "ssr")]

mod common;

use common::assert_parity;
use leptos::prelude::*;
use stucco_core::{CollectionQuery, Cursor, Href, Window, to_html};
use stucco_leptos::{
    BreadcrumbCurrent, BreadcrumbLink, Breadcrumbs, CursorPagination, NavLink, Pagination,
    StepItem, Steps, Tabs,
};
use stucco_ui::navigation::{
    Breadcrumbs as StaticBreadcrumbs, NavLink as StaticNavLink, Pagination as StaticPagination,
    Steps as StaticSteps, Tabs as StaticTabs,
};

fn render(view: impl FnOnce() -> String) -> String {
    Owner::new().with(view)
}

#[test]
fn nav_link_matches_static_markup() {
    let html = render(|| view! { <NavLink label="Orders" href="/orders" current=true/> }.to_html());
    assert_parity(
        &html,
        &to_html(&StaticNavLink::new("Orders", "/orders").current(true)),
    );

    let html = render(|| view! { <NavLink label="Orders" href="/orders"/> }.to_html());
    assert_parity(&html, &to_html(&StaticNavLink::new("Orders", "/orders")));
    assert!(!html.contains("aria-current"), "{html}");
}

#[test]
fn nav_link_escapes_label() {
    let html = render(|| view! { <NavLink label="<b>&" href="/x"/> }.to_html());
    assert!(html.contains("&lt;b&gt;&amp;"), "{html}");
    assert!(!html.contains("<b>"), "{html}");
}

#[test]
fn nav_link_href_follows_the_static_allow_list() {
    for url in [
        "/orders",
        "?tab=items",
        "#top",
        "https://a.test/",
        "mailto:a@b.test",
        "tel:+1555",
        "HTTP://A.TEST",
        "javascript:alert(1)",
        " \tjava\nscript:alert(1)",
        "JaVaScRiPt:alert(1)",
        "data:text/html,x",
        "vbscript:x",
    ] {
        let html = render(|| {
            let href = url.to_owned();
            view! { <NavLink label="x" href=href/> }.to_html()
        });
        assert_parity(&html, &to_html(&StaticNavLink::new("x", url)));
    }
    let html = render(|| view! { <NavLink label="x" href="javascript:alert(1)"/> }.to_html());
    assert!(html.contains(r##"href="#""##), "{html}");
}

#[test]
fn tabs_match_static_markup() {
    let html = render(|| {
        view! {
            <Tabs label="Order sections">
                <NavLink label="Details" href="?tab=details" current=true/>
                <NavLink label="History" href="?tab=history"/>
            </Tabs>
        }
        .to_html()
    });
    let expected = to_html(
        &StaticTabs::new("Order sections")
            .tab(StaticNavLink::new("Details", "?tab=details").current(true))
            .tab(StaticNavLink::new("History", "?tab=history")),
    );
    assert_parity(&html, &expected);
}

#[test]
fn current_signal_initial_value_is_rendered() {
    let html = render(|| {
        let flag = RwSignal::new(false);
        view! { <NavLink label="Orders" href="/orders" current=Signal::derive(move || flag.get())/> }
            .to_html()
    });
    assert!(!html.contains("aria-current"), "{html}");
    let html = render(|| {
        let flag = RwSignal::new(true);
        view! { <NavLink label="Orders" href="/orders" current=Signal::derive(move || flag.get())/> }
            .to_html()
    });
    assert!(html.contains(r#"aria-current="page""#), "{html}");
}

#[test]
fn breadcrumbs_match_static_markup() {
    let html = render(|| {
        view! {
            <Breadcrumbs>
                <BreadcrumbLink href="/orders">"Orders"</BreadcrumbLink>
                <BreadcrumbCurrent>"Order 42"</BreadcrumbCurrent>
            </Breadcrumbs>
        }
        .to_html()
    });
    let expected = to_html(
        &StaticBreadcrumbs::new()
            .link("Orders", "/orders")
            .current("Order 42"),
    );
    assert_parity(&html, &expected);
    assert!(html.contains(r#"aria-label="Breadcrumb""#), "{html}");
}

#[test]
fn breadcrumb_label_is_overridable() {
    let html = render(|| {
        view! { <Breadcrumbs label="Trail"><BreadcrumbCurrent>"Here"</BreadcrumbCurrent></Breadcrumbs> }
            .to_html()
    });
    assert_parity(
        &html,
        &to_html(&StaticBreadcrumbs::new().label("Trail").current("Here")),
    );
}

#[test]
fn breadcrumb_link_sanitizes_href() {
    let html = render(|| {
        view! {
            <Breadcrumbs>
                <BreadcrumbLink href="javascript:alert(1)">"Bad"</BreadcrumbLink>
            </Breadcrumbs>
        }
        .to_html()
    });
    assert!(html.contains(r##"href="#""##), "{html}");
    assert!(!html.contains("javascript"), "{html}");
}

#[test]
fn breadcrumbs_with_only_the_current_page_have_one_item() {
    let html = render(|| {
        view! { <Breadcrumbs><BreadcrumbCurrent>"Home"</BreadcrumbCurrent></Breadcrumbs> }.to_html()
    });
    assert_eq!(html.matches("<li>").count(), 1, "{html}");
    assert_parity(&html, &to_html(&StaticBreadcrumbs::new().current("Home")));
}

fn step(label: &str, href: Option<&str>) -> StepItem {
    StepItem {
        label: label.to_owned(),
        href: href.map(str::to_owned),
    }
}

fn checkout_steps() -> Vec<StepItem> {
    vec![
        step("Cart", Some("/cart")),
        step("Pay", Some("/pay")),
        step("Done", Some("/done")),
    ]
}

#[test]
fn steps_match_static_markup() {
    for current in [0, 1, 2] {
        let html = render(|| {
            view! { <Steps label="Checkout" steps=checkout_steps() current=current/> }.to_html()
        });
        let expected = to_html(
            &StaticSteps::new("Checkout")
                .step("Cart", "/cart")
                .step("Pay", "/pay")
                .step("Done", "/done")
                .current(current),
        );
        assert_parity(&html, &expected);
    }
}

#[test]
fn steps_default_to_the_first_step() {
    let html = render(|| view! { <Steps label="Checkout" steps=checkout_steps()/> }.to_html());
    assert!(html.contains(r#"aria-current="step""#), "{html}");
    assert!(html.contains(r#"data-state="current""#), "{html}");
}

#[test]
fn a_finished_step_without_a_link_renders_text() {
    let steps = vec![step("Cart", None), step("Pay", Some("/pay"))];
    let html = render(|| view! { <Steps label="Checkout" steps=steps current=1/> }.to_html());
    let expected = to_html(
        &StaticSteps::new("Checkout")
            .step_disabled("Cart")
            .step("Pay", "/pay")
            .current(1),
    );
    assert_parity(&html, &expected);
    assert!(!html.contains("<a "), "{html}");
}

#[test]
fn steps_with_no_items_render_nothing() {
    let html = render(|| view! { <Steps label="Checkout" steps=Vec::new()/> }.to_html());
    assert!(!html.contains("<nav"), "{html}");
    assert_eq!(to_html(&StaticSteps::new("Checkout")), "");
}

#[test]
fn current_past_the_end_means_the_last_step() {
    let html =
        render(|| view! { <Steps label="Checkout" steps=checkout_steps() current=99/> }.to_html());
    let expected = to_html(
        &StaticSteps::new("Checkout")
            .step("Cart", "/cart")
            .step("Pay", "/pay")
            .step("Done", "/done")
            .current(99),
    );
    assert_parity(&html, &expected);
}

#[test]
fn hidden_step_text_is_translatable() {
    let html = render(|| {
        view! {
            <Steps label="Paiement" steps=checkout_steps() current=1
                completed_label=" (terminé)" not_started_label=" (à venir)"/>
        }
        .to_html()
    });
    assert!(html.contains("(terminé)"), "{html}");
    assert!(html.contains("(à venir)"), "{html}");
    assert!(!html.contains("(completed)"), "{html}");
}

#[test]
fn step_hrefs_are_sanitized() {
    let steps = vec![step("Cart", Some("javascript:alert(1)")), step("Pay", None)];
    let html = render(|| view! { <Steps label="Checkout" steps=steps current=1/> }.to_html());
    assert!(!html.contains("javascript"), "{html}");
}

fn page_query(page: u64) -> CollectionQuery {
    CollectionQuery::default()
        .with_per_page(1)
        .with_window(Window::Offset { page })
}

fn page_link(page: u64) -> String {
    page_query(page)
        .link(&Href::new("/orders"))
        .as_str()
        .to_owned()
}

fn numbered(page: u64, total_pages: u64) -> String {
    render(|| {
        view! {
            <Pagination page=page total_pages=total_pages
                href=Callback::new(page_link)/>
        }
        .to_html()
    })
}

fn static_numbered(page: u64, total_pages: u64) -> String {
    to_html(&StaticPagination::new("/orders", &page_query(page)).total(Some(total_pages)))
}

#[test]
fn numbered_pagination_matches_static_windowing() {
    for (page, total) in [(1, 2), (5, 10), (1, 10), (10, 10), (3, 5), (2, 3), (4, 4)] {
        assert_parity(&numbered(page, total), &static_numbered(page, total));
    }
}

#[test]
fn single_page_on_page_one_renders_nothing() {
    assert!(!numbered(1, 1).contains("<nav"));
    assert_eq!(static_numbered(1, 1), "");
}

#[test]
fn zero_total_pages_renders_nothing() {
    assert!(!numbered(1, 0).contains("<nav"));
    assert_eq!(static_numbered(1, 0), "");
}

#[test]
fn page_zero_is_treated_as_page_one() {
    assert_parity(&numbered(0, 5), &static_numbered(1, 5));
}

#[test]
fn page_past_the_end_still_links_back() {
    let html = numbered(20, 10);
    assert_parity(&html, &static_numbered(20, 10));
    assert!(html.contains("Page 1"), "{html}");
    assert!(!html.contains("aria-current"), "{html}");
}

#[test]
fn current_page_is_marked_and_every_link_is_labelled() {
    let html = numbered(3, 5);
    assert_eq!(html.matches(r#"aria-current="page""#).count(), 1, "{html}");
    for n in 1..=5 {
        assert!(
            html.contains(&format!(r#"aria-label="Page {n}""#)),
            "{html}"
        );
    }
}

#[test]
fn page_label_is_translatable() {
    let html = render(|| {
        view! {
            <Pagination page=1u64 total_pages=3u64 href=Callback::new(page_link)
                label="Pages" page_label=Callback::new(|n| format!("Seite {n}"))/>
        }
        .to_html()
    });
    assert!(html.contains(r#"aria-label="Seite 2""#), "{html}");
    assert!(html.contains(r#"aria-label="Pages""#), "{html}");
}

#[test]
fn pagination_hrefs_are_sanitized() {
    let html = render(|| {
        view! {
            <Pagination page=1u64 total_pages=3u64
                href=Callback::new(|_| "javascript:alert(1)".to_owned())/>
        }
        .to_html()
    });
    assert!(!html.contains("javascript"), "{html}");
}

fn cursor_link(window: Window) -> String {
    CollectionQuery::default()
        .with_window(window)
        .link(&Href::new("/orders"))
        .as_str()
        .to_owned()
}

fn cursor_case(previous: Option<&str>, next: Option<&str>) -> (String, String) {
    let previous_href = previous.map(|c| cursor_link(Window::Before(Cursor::new(c).unwrap())));
    let next_href = next.map(|c| cursor_link(Window::After(Cursor::new(c).unwrap())));
    let html = render(|| {
        view! { <CursorPagination previous_href=previous_href next_href=next_href/> }.to_html()
    });
    let expected = to_html(
        &StaticPagination::new("/orders", &CollectionQuery::default())
            .cursors(next.and_then(Cursor::new), previous.and_then(Cursor::new)),
    );
    (html, expected)
}

#[test]
fn cursor_pagination_matches_static_markup() {
    for (previous, next) in [
        (Some("abc"), None),
        (None, Some("def")),
        (Some("abc"), Some("def")),
    ] {
        let (html, expected) = cursor_case(previous, next);
        assert_parity(&html, &expected);
    }
}

#[test]
fn cursor_pagination_without_cursors_renders_nothing() {
    let (html, expected) = cursor_case(None, None);
    assert!(!html.contains("<nav"), "{html}");
    assert_eq!(expected, "");
}

#[test]
fn cursor_pagination_labels_are_translatable_and_hrefs_sanitized() {
    let html = render(|| {
        view! {
            <CursorPagination previous_href="javascript:x" next_href="/n"
                previous_label="Zurück" next_label="Weiter"/>
        }
        .to_html()
    });
    assert!(html.contains("Zurück") && html.contains("Weiter"), "{html}");
    assert!(!html.contains("javascript"), "{html}");
}
