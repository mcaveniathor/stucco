#![cfg(feature = "ssr")]

mod common;

use common::assert_parity;
use leptos::prelude::*;
use stucco_core::to_html;
use stucco_leptos::{NavLink, Tabs};
use stucco_ui::navigation::{NavLink as StaticNavLink, Tabs as StaticTabs};

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
