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
