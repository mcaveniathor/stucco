use super::*;
use stucco_core::to_html;

#[test]
fn post_forms_carry_csrf_tokens() {
    let html = to_html(&Form::post("/orders").csrf("t\"k").child("x"));
    assert_eq!(
        html,
        r#"<form class="st-form" method="post" action="/orders"><input type="hidden" name="_csrf" value="t&quot;k">x</form>"#
    );
}

#[test]
fn get_forms_and_multipart() {
    assert_eq!(
        to_html(&Form::get("/search")),
        r#"<form class="st-form" method="get" action="/search"></form>"#
    );
    assert!(to_html(&Form::post("/up").multipart()).contains(r#"enctype="multipart/form-data""#));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "multipart")]
fn multipart_get_forms_panic_in_debug() {
    let _ = Form::get("/x").multipart();
}

#[test]
fn field_errors_render_only_when_present() {
    assert_eq!(to_html(&FieldError::new("e", &[])), "");
    assert_eq!(
        to_html(&FieldError::new(
            "e",
            &["Too <short>".into(), "Required".into()]
        )),
        r#"<p id="e" class="st-field-error"><span class="st-sr-only">Error: </span>Too &lt;short&gt;<br>Required</p>"#
    );
}

#[test]
fn hints_have_ids() {
    assert_eq!(
        to_html(&FieldHint::new("h", "Use <8 chars")),
        r#"<p id="h" class="st-field-hint">Use &lt;8 chars</p>"#
    );
}

#[test]
fn fieldsets_have_legends() {
    assert_eq!(
        to_html(&Fieldset::new("Shipping & billing").child("x")),
        r#"<fieldset class="st-fieldset"><legend class="st-legend">Shipping &amp; billing</legend>x</fieldset>"#
    );
}

#[test]
fn hidden_inputs_escape_values() {
    assert_eq!(
        to_html(&HiddenInput::new("next", "/a?b=1&c=\"2\"")),
        r#"<input type="hidden" name="next" value="/a?b=1&amp;c=&quot;2&quot;">"#
    );
}
