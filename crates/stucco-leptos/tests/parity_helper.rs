mod common;

use common::{assert_parity, normalize};

#[test]
fn normalize_drops_markers_and_sorts_attributes() {
    assert_eq!(
        normalize(r#"<a href="/x" class="c"><!>Hi<!-- note --></a>"#),
        r#"<a class="c" href="/x">Hi</a>"#
    );
}

#[test]
fn normalize_collapses_whitespace_between_tags_but_not_in_text() {
    assert_eq!(
        normalize("<ol>\n  <li>One  two</li>\n  <li>Three</li>\n</ol>"),
        "<ol><li>One  two</li><li>Three</li></ol>"
    );
}

#[test]
fn normalize_keeps_quoted_angle_brackets_inside_attributes() {
    assert_eq!(
        normalize(r#"<span title="a > b" id="x">t</span>"#),
        r#"<span id="x" title="a > b">t</span>"#
    );
}

#[test]
fn assert_parity_accepts_equivalent_markup() {
    assert_parity(
        r#"<a href="/x" class="c">Hi</a>"#,
        r#"<a class="c" href="/x"><!>Hi</a>"#,
    );
}

#[test]
#[should_panic(expected = "markup differs")]
fn assert_parity_panics_with_both_strings_on_difference() {
    assert_parity("<p>one</p>", "<p>two</p>");
}

#[test]
fn normalize_treats_self_closed_svg_shapes_like_empty_pairs() {
    assert_eq!(
        normalize(r#"<svg><path d="M0 0" /><circle r="3"></circle></svg>"#),
        normalize(r#"<svg><path d="M0 0"></path><circle r="3" /></svg>"#)
    );
}
