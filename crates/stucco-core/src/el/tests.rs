use crate::{Attrs, el, to_html};

#[test]
fn builds_nested_markup_with_escaping() {
    let html = to_html(
        &el::section()
            .class("hero")
            .id("intro")
            .data("state", "open")
            .child(el::h1().text("A & B"))
            .child_if(false, el::p().text("hidden"))
            .children(["x", "y"].map(|s| el::li().text(s))),
    );
    assert_eq!(
        html,
        r#"<section id="intro" class="hero" data-state="open"><h1>A &amp; B</h1><li>x</li><li>y</li></section>"#
    );
}

#[test]
fn class_appends_and_attrs_merge() {
    let attrs = Attrs::default().class("b c").attr("hx-get", "/more");
    let html = to_html(
        &el::div()
            .class("a b")
            .attrs(&attrs)
            .bool_attr("hidden", true)
            .bool_attr("inert", false),
    );
    assert_eq!(html, r#"<div class="a b c" hx-get="/more" hidden></div>"#);
}

#[test]
fn url_attributes_are_checked_including_lists() {
    assert_eq!(
        to_html(
            &el::img()
                .src("javascript:x")
                .attr("srcset", "a.png 1x, javascript:x 2x")
        ),
        r##"<img src="#" srcset="a.png 1x">"##
    );
    assert_eq!(
        to_html(&el::a().attr("href", "/ok").text("ok")),
        r#"<a href="/ok">ok</a>"#
    );
}

#[test]
fn repeated_attributes_keep_the_last_value() {
    assert_eq!(
        to_html(&el::div().attr("title", "a").attr("title", "b")),
        r#"<div title="b"></div>"#
    );
}

#[test]
fn trusted_attr_is_the_only_way_to_set_handlers() {
    assert_eq!(
        to_html(&el::button().trusted_attr("onclick", "go()")),
        r#"<button onclick="go()"></button>"#
    );
}

#[test]
fn script_and_style_text_is_raw_but_cannot_close_the_element() {
    assert_eq!(
        to_html(&el::style().text("ul > li {}")),
        "<style>ul > li {}</style>"
    );
    assert_eq!(
        to_html(&el::script().text("a < b && c</script><b>")),
        r"<script>a < b && c<\/script><b></script>"
    );
}

#[test]
fn attribute_names_are_case_insensitive() {
    assert_eq!(
        to_html(&el::div().attr("title", "a").attr("TITLE", "b")),
        r#"<div title="b"></div>"#
    );
    assert_eq!(
        to_html(
            &el::div()
                .bool_attr("hidden", true)
                .bool_attr("Hidden", false)
        ),
        "<div></div>"
    );
    assert_eq!(
        Attrs::default()
            .attr("ROLE", "x")
            .reserved_conflicts(&["role"]),
        ["role"]
    );
}

#[test]
fn without_removes_names_including_id_and_class() {
    let a = Attrs::default()
        .id("x")
        .class("c")
        .attr("role", "r")
        .attr("title", "t");
    assert_eq!(
        to_html(&el::div().attrs(&a.without(&["id", "role"]))),
        r#"<div class="c" title="t"></div>"#
    );
}

#[test]
fn custom_elements_are_validated() {
    assert_eq!(to_html(&el::custom("st-tabs")), "<st-tabs></st-tabs>");
}

#[test]
fn reserved_conflicts_are_reported() {
    let attrs = Attrs::default().attr("role", "x").aria("controls", "y");
    assert_eq!(
        attrs.reserved_conflicts(&["role", "aria-controls", "id"]),
        ["role", "aria-controls"]
    );
}

#[cfg(debug_assertions)]
mod refusals {
    use crate::el;

    #[test]
    #[should_panic(expected = "refused attribute")]
    fn mixed_case_handler() {
        let _ = el::div().attr("OnClick", "x");
    }

    #[test]
    #[should_panic(expected = "refused attribute")]
    fn upper_case_handler() {
        let _ = el::img().attr("ONLOAD", "x");
    }

    #[test]
    #[should_panic(expected = "refused attribute")]
    fn srcdoc() {
        let _ = el::iframe().attr("srcdoc", "<b>");
    }

    #[test]
    #[should_panic(expected = "invalid attribute name")]
    fn spaces() {
        let _ = el::div().attr("a b", "x");
    }

    #[test]
    #[should_panic(expected = "unknown aria attribute")]
    fn aria_typo() {
        let _ = el::div().aria("lable", "x");
    }

    #[test]
    #[should_panic(expected = "invalid custom element")]
    fn custom_without_hyphen() {
        let _ = el::custom("tabs");
    }
}

#[test]
fn open_is_not_mistaken_for_an_event_handler() {
    assert_eq!(
        to_html(&el::details().bool_attr("open", true)),
        "<details open></details>"
    );
}

#[test]
fn passthrough_attrs_get_the_tag_specific_url_check() {
    let attrs = Attrs::default().attr("data", "javascript:alert(1)");
    assert_eq!(
        to_html(&el::object().attrs(&attrs)),
        r##"<object data="#"></object>"##
    );
    let trusted = Attrs::default().trusted_attr("data", "javascript:void(0)");
    assert_eq!(
        to_html(&el::object().attrs(&trusted)),
        r#"<object data="javascript:void(0)"></object>"#
    );
}

#[cfg(not(debug_assertions))]
mod release_drops {
    use crate::{el, to_html};

    #[test]
    fn refused_and_invalid_attributes_are_dropped() {
        let html = to_html(
            &el::iframe()
                .attr("OnLoad", "x")
                .attr("srcdoc", "<b>")
                .attr("a b", "x")
                .attr("title", "kept"),
        );
        assert_eq!(html, r#"<iframe title="kept"></iframe>"#);
    }

    #[test]
    fn invalid_custom_elements_render_a_div() {
        assert_eq!(to_html(&el::custom("tabs")), "<div></div>");
    }
}

#[test]
fn script_text_cannot_enter_the_double_escaped_state() {
    let html = to_html(&el::script().text(r#"{"note":"<!--<script>"}"#));
    assert!(
        !html.contains("<!--") && !html.contains("<script>\"}"),
        "{html}"
    );
    assert!(html.ends_with("</script>"));
    let upper = to_html(&el::script().text("<!-- <SCRIPT x"));
    assert!(
        !upper.contains("<!--") && !upper.to_ascii_lowercase().contains("<script x"),
        "{upper}"
    );
}
