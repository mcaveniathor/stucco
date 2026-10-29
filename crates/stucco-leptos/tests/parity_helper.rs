mod common;

use common::{align_ids, assert_parity, normalize};

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

#[test]
fn normalize_lowercases_attribute_names_but_not_values() {
    assert_eq!(
        normalize(r#"<svg viewBox="0 0 24 24" ARIA-Hidden="True">"#),
        r#"<svg aria-hidden="True" viewbox="0 0 24 24">"#
    );
}

#[test]
fn align_ids_renames_ids_and_their_references_in_order() {
    let leptos = r#"<label for="p">L</label><meter id="p" aria-describedby="p-text"></meter><span id="p-text">t</span>"#;
    let stucco = r#"<label for="meter-1">L</label><meter id="meter-1" aria-describedby="metertext-2"></meter><span id="metertext-2">t</span>"#;
    assert_eq!(align_ids(leptos, stucco), stucco);
}

#[test]
fn align_ids_leaves_markup_without_ids_alone() {
    assert_eq!(align_ids("<p>x</p>", "<p>x</p>"), "<p>x</p>");
}
