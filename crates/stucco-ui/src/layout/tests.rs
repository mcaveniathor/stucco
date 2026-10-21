use super::*;
use crate::{Measure, Space};
use stucco_core::to_html;

#[test]
fn stack_spaces_children_with_data_attributes() {
    let html = to_html(&Stack::new().space(Space::S6).child("a").child("<b>"));
    assert_eq!(
        html,
        r#"<div class="st-stack" data-space="6">a&lt;b&gt;</div>"#
    );
}

#[test]
fn layout_parameters_never_use_inline_styles() {
    let html = to_html(&(
        Cluster::new()
            .justify(Justify::Between)
            .align(Align::Center),
        Grid::new().min(Measure::Md).space(Space::S2),
        Center::new().max(Measure::Lg).intrinsic(),
        Container::new().size(Measure::Xl),
    ));
    assert!(!html.contains("style="), "{html}");
    for needle in [
        r#"data-justify="between""#,
        r#"data-min="md""#,
        r#"data-max="lg""#,
        "data-intrinsic",
        r#"data-size="xl""#,
    ] {
        assert!(html.contains(needle), "{needle}");
    }
}

#[test]
fn as_tag_changes_the_element_and_passthrough_merges() {
    let html = to_html(
        &Stack::new()
            .as_tag(Tag::Ul)
            .class("x")
            .aria("label", "Items"),
    );
    assert_eq!(
        html,
        r#"<ul class="st-stack x" data-space="4" aria-label="Items"></ul>"#
    );
}

#[test]
fn layout_stylesheet_covers_every_value() {
    let css = LAYOUT.css.unwrap();
    for s in ["0", "1", "2", "3", "4", "5", "6", "8", "10", "12"] {
        assert!(
            css.contains(&format!(".st-stack[data-space=\"{s}\"]")),
            "stack {s}"
        );
        assert!(
            css.contains(&format!(".st-cluster[data-space=\"{s}\"]")),
            "cluster {s}"
        );
    }
    for m in ["xs", "sm", "md", "lg", "xl", "prose"] {
        assert!(
            css.contains(&format!(".st-grid[data-min=\"{m}\"]")),
            "grid {m}"
        );
        assert!(
            css.contains(&format!(".st-center[data-max=\"{m}\"]")),
            "center {m}"
        );
    }
}

#[test]
fn sidebar_dom_order_matches_visual_order() {
    assert_eq!(
        to_html(&Sidebar::new("nav", "content")),
        r#"<div class="st-sidebar" data-side="left" data-side-width="xs" data-space="4"><div class="st-sidebar-side">nav</div><div class="st-sidebar-main">content</div></div>"#
    );
    assert_eq!(
        to_html(&Sidebar::new("nav", "content").right()),
        r#"<div class="st-sidebar" data-side="right" data-side-width="xs" data-space="4"><div class="st-sidebar-main">content</div><div class="st-sidebar-side">nav</div></div>"#
    );
    assert!(!LAYOUT.css.unwrap().contains(" order:"));
}

#[test]
fn separators_have_the_right_semantics() {
    assert_eq!(to_html(&Separator::new()), r#"<hr class="st-separator">"#);
    assert_eq!(
        to_html(&Separator::new().decorative()),
        r#"<div class="st-separator" role="none"></div>"#
    );
}

#[test]
fn skip_link_targets_main_by_default() {
    assert_eq!(
        to_html(&SkipLink::new()),
        r##"<a class="st-skip-link" href="#main">Skip to main content</a>"##
    );
}

#[test]
fn visually_hidden_text_stays_in_the_accessibility_tree() {
    assert_eq!(
        to_html(&VisuallyHidden::new("a <b>")),
        r#"<span class="st-sr-only">a &lt;b&gt;</span>"#
    );
}

#[test]
fn surface_levels_and_padding() {
    assert_eq!(
        to_html(
            &Surface::new()
                .level(Level::Raised)
                .padding(Space::S6)
                .child("x")
        ),
        r#"<div class="st-surface" data-level="raised" data-padding="6" data-border>x</div>"#
    );
}

#[test]
fn theme_scope_sets_scheme_and_named_theme() {
    let html = to_html(
        &ThemeScope::new()
            .scheme(crate::ColorScheme::Dark)
            .named("brand")
            .child("x"),
    );
    assert_eq!(
        html,
        r#"<div class="st-theme-scope" data-theme="dark" data-st-theme="brand">x</div>"#
    );
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "invalid theme name")]
fn theme_scope_validates_names() {
    let _ = ThemeScope::new().named("Bad Name");
}

#[test]
fn switcher_limit_is_clamped() {
    assert!(to_html(&Switcher::new().limit(9)).contains(r#"data-limit="6""#));
    assert!(to_html(&Switcher::new().limit(0)).contains(r#"data-limit="2""#));
}

#[test]
fn of_takes_every_child_at_once() {
    let one_by_one = Stack::new()
        .space(Space::S2)
        .child("a")
        .child(stucco_core::el::b().text("b"));
    let at_once = Stack::of(("a", stucco_core::el::b().text("b"))).space(Space::S2);
    assert_eq!(to_html(&at_once), to_html(&one_by_one));
    assert_eq!(
        to_html(&Cluster::of("x")),
        to_html(&Cluster::new().child("x"))
    );
}

#[test]
fn disclosure_is_native_details() {
    let html = to_html(&Disclosure::new("Shipping <details>").child("Body"));
    assert!(
        html.starts_with(r#"<details class="st-disclosure"><summary>"#),
        "{html}"
    );
    assert!(html.contains("Shipping &lt;details&gt;"), "{html}");
    assert!(html.contains(r#"aria-hidden="true""#), "{html}");
    assert!(
        html.contains(r#"<div class="st-disclosure-body">Body</div></details>"#),
        "{html}"
    );
    let open = to_html(&Disclosure::new("x").open(true));
    assert!(
        open.starts_with(r#"<details class="st-disclosure" open>"#),
        "{open}"
    );
}

#[test]
fn disclosure_heading_clamps() {
    assert!(to_html(&Disclosure::new("x").heading(1)).contains("<summary><h2>x</h2>"));
    assert!(to_html(&Disclosure::new("x").heading(9)).contains("<summary><h6>x</h6>"));
    assert!(to_html(&Disclosure::new("x").heading(3)).contains("<summary><h3>x</h3>"));
}

/// Every `name="…"` value in `html`.
fn names(html: &str) -> Vec<&str> {
    html.split(r#" name=""#)
        .skip(1)
        .map(|r| &r[..r.find('"').unwrap()])
        .collect()
}

#[test]
fn exclusive_accordion_shares_one_name() {
    let html = to_html(
        &Accordion::new()
            .item("A", "a")
            .item("B", "b")
            .item_open("C", "c")
            .exclusive(),
    );
    assert_eq!(html.matches("<details").count(), 3, "{html}");
    let names = names(&html);
    assert_eq!(names.len(), 3, "{html}");
    assert!(
        names.iter().all(|n| *n == names[0] && !n.is_empty()),
        "{html}"
    );
    assert_eq!(html.matches(" open").count(), 1, "{html}");
}

#[test]
fn exclusive_accordions_get_distinct_names() {
    let a = Accordion::new().item("A", "a").exclusive();
    let b = Accordion::new().item("B", "b").exclusive();
    let html = to_html(&(a, b));
    let names = names(&html);
    assert_eq!(names.len(), 2, "{html}");
    assert_ne!(names[0], names[1], "{html}");
}

#[test]
fn non_exclusive_accordion_has_no_name() {
    let html = to_html(&Accordion::new().item("A", "a").heading(4));
    assert!(!html.contains(" name="), "{html}");
    assert!(html.contains("<summary><h4>A</h4>"), "{html}");
}

#[test]
fn empty_accordion_is_an_empty_wrapper() {
    assert_eq!(
        to_html(&Accordion::new()),
        r#"<div class="st-accordion"></div>"#
    );
}
