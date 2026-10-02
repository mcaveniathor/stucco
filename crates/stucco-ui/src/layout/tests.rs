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
