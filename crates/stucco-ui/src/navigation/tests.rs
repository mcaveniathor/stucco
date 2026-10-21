use super::*;
use stucco_core::to_html;

#[test]
fn breadcrumbs_mark_the_current_page() {
    let html = to_html(
        &Breadcrumbs::new()
            .link("Home", "/")
            .link("Orders", "/orders")
            .current("Order 42"),
    );
    assert!(
        html.starts_with(r#"<nav class="st-breadcrumbs" aria-label="Breadcrumb"><ol>"#),
        "{html}"
    );
    assert_eq!(html.matches("<a href=").count(), 2, "{html}");
    assert!(
        html.contains(r#"<span aria-current="page">Order 42</span>"#),
        "{html}"
    );
}

#[test]
fn breadcrumbs_label_overrides() {
    let html = to_html(&Breadcrumbs::new().label("Location").current("Here"));
    assert!(html.contains(r#"aria-label="Location""#), "{html}");
}

#[test]
fn steps_states_and_hidden_text() {
    let html = to_html(
        &Steps::new("Checkout")
            .step("Cart", "/cart")
            .step("Shipping", "/shipping")
            .step("Payment", "/pay")
            .current(1),
    );
    assert!(
        html.starts_with(r#"<nav class="st-steps" aria-label="Checkout"><ol>"#),
        "{html}"
    );
    let done = html.find(r#"data-state="done""#).unwrap();
    let current = html.find(r#"data-state="current""#).unwrap();
    let todo = html.find(r#"data-state="todo""#).unwrap();
    assert!(done < current && current < todo, "{html}");
    assert!(
        html.contains(r#"<a href="/cart">Cart<span class="st-sr-only"> (completed)</span></a>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<span aria-current="step">Shipping</span>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"Payment<span class="st-sr-only"> (not started)</span>"#),
        "{html}"
    );
    assert!(!html.contains(r#"href="/pay""#), "{html}");
    assert!(!html.contains(r#"href="/shipping""#), "{html}");
    for n in 1..=3 {
        assert!(
            html.contains(&format!(
                r#"<span class="st-steps-marker" aria-hidden="true">{n}</span>"#
            )),
            "{html}"
        );
    }
}

#[test]
fn disabled_done_step_is_text() {
    let html = to_html(
        &Steps::new("Flow")
            .step_disabled("Locked")
            .step("Next", "/n")
            .current(1),
    );
    assert!(
        html.contains(r#"Locked<span class="st-sr-only"> (completed)</span>"#),
        "{html}"
    );
    assert_eq!(html.matches("<a ").count(), 0, "{html}");
}

#[test]
fn out_of_range_current_clamps_to_last() {
    let html = to_html(
        &Steps::new("Flow")
            .step("A", "/a")
            .step("B", "/b")
            .current(9),
    );
    assert!(
        html.contains(r#"<span aria-current="step">B</span>"#),
        "{html}"
    );
}

#[test]
fn empty_steps_render_nothing() {
    assert_eq!(to_html(&Steps::new("Flow")), "");
}
