use super::*;
use crate::{Icon, Variant};
use stucco_core::to_html;

#[test]
fn buttons_default_to_type_button() {
    assert_eq!(
        to_html(&Button::new("Save")),
        r#"<button class="st-button" type="button" data-variant="secondary" data-size="md">Save</button>"#
    );
    assert!(
        to_html(&Button::new("Go").submit().variant(Variant::Primary))
            .contains(r#"type="submit" data-variant="primary""#)
    );
}

#[test]
fn loading_buttons_stay_focusable_and_announce_busy() {
    let html = to_html(&Button::new("Save").loading());
    assert!(
        html.contains(r#"aria-busy="true" aria-disabled="true""#),
        "{html}"
    );
    assert!(!html.contains(" disabled"));
}

#[test]
fn button_links_are_anchors_with_checked_urls() {
    let html = to_html(&ButtonLink::new("Docs <here>", "javascript:x").variant(Variant::Primary));
    assert_eq!(
        html,
        r##"<a class="st-button" href="#" data-variant="primary" data-size="md">Docs &lt;here&gt;</a>"##
    );
}

#[test]
fn icon_buttons_have_accessible_names() {
    let html = to_html(&IconButton::new(Icon::custom("x", ""), "Close <dialog>"));
    assert!(
        html.contains(r#"aria-label="Close &lt;dialog&gt;""#),
        "{html}"
    );
    assert!(html.contains("data-icon-only") && html.contains("aria-hidden=\"true\""));
}

#[test]
fn leading_icons_are_decorative() {
    let html = to_html(&Button::new("Next").icon(Icon::custom("arrow", "<path/>")));
    assert!(
        html.contains("aria-hidden=\"true\"") && html.ends_with("Next</button>"),
        "{html}"
    );
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "reserved attribute")]
fn type_cannot_be_overridden_by_passthrough() {
    let _ = to_html(&Button::new("x").attr("type", "submit"));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "icon button label")]
fn icon_buttons_need_a_label() {
    let _ = IconButton::new(Icon::custom("x", ""), "  ");
}
