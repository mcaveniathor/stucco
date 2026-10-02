use super::*;
use stucco_core::{FormState, to_html};

#[test]
fn field_wires_label_hint_error_and_invalid_state() {
    let html = to_html(
        &Field::new("Email", Input::email("email"))
            .hint("Work address")
            .error("Required"),
    );
    assert!(
        html.contains(r#"<label class="st-label" for="field-1">Email</label>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"<input id="field-1" class="st-input" type="email" name="email" aria-describedby="field-1-hint field-1-error" aria-invalid="true">"#),
        "{html}"
    );
    assert!(html.contains(r#"<p id="field-1-hint" class="st-field-hint">Work address</p>"#));
    assert!(html.contains(r#"<p id="field-1-error" class="st-field-error">"#));
}

#[test]
fn two_fields_get_distinct_ids_and_explicit_ids_are_kept() {
    let html = to_html(&(
        Field::new("A", Input::text("a")),
        Field::new("B", Input::text("b").id("bee")).hint("h"),
    ));
    assert!(
        html.contains(r#"for="field-1""#) && html.contains(r#"for="bee""#),
        "{html}"
    );
    assert!(html.contains(r#"id="bee-hint""#) && html.contains(r#"aria-describedby="bee-hint""#));
    assert!(!html.contains("aria-invalid"));
}

#[test]
fn binding_redisplays_values_but_never_sensitive_ones() {
    let state = FormState::new()
        .with_value("email", "a@b.c\"")
        .with_value("password", "hunter2")
        .with_error("password", "Too short");
    let email = to_html(&Field::new("Email", Input::email("email")).bind(&state));
    assert!(email.contains(r#"value="a@b.c&quot;""#), "{email}");
    let password = to_html(&Field::new("Password", Input::password("password")).bind(&state));
    assert!(
        !password.contains("hunter2") && !password.contains("value="),
        "{password}"
    );
    assert!(password.contains("Too short"));
    let notes = to_html(&Field::new("Notes", Textarea::new("password").sensitive()).bind(&state));
    assert!(!notes.contains("hunter2"), "{notes}");
}

#[test]
fn textareas_put_values_in_content() {
    let html = to_html(&Textarea::new("bio").rows(4).value("<hi>"));
    assert_eq!(
        html,
        r#"<textarea class="st-textarea" name="bio" rows="4">&lt;hi&gt;</textarea>"#
    );
}

#[test]
fn selects_mark_the_selected_option_and_escape_labels() {
    let html = to_html(
        &Select::new("plan")
            .placeholder("Choose…")
            .option("free", "Free <tier>")
            .option("pro", "Pro")
            .selected("pro"),
    );
    assert!(
        html.contains(r#"<option value="" disabled>Choose…</option>"#),
        "{html}"
    );
    assert!(html.contains(
        r#"<option value="free">Free &lt;tier&gt;</option><option value="pro" selected>Pro</option>"#
    ));
}

#[test]
fn bound_selects_select_the_submitted_value() {
    let state = FormState::new().with_value("plan", "free");
    let html = to_html(
        &Field::new(
            "Plan",
            Select::new("plan")
                .option("free", "Free")
                .option("pro", "Pro"),
        )
        .bind(&state),
    );
    assert!(
        html.contains(r#"<option value="free" selected>Free</option>"#),
        "{html}"
    );
}

#[test]
fn adornments_wrap_the_input() {
    let html = to_html(&Input::url("site").prefix("https://"));
    assert!(
        html.starts_with(
            r#"<div class="st-input-group"><span class="st-input-adornment">https://</span><input"#
        ),
        "{html}"
    );
}

#[test]
fn required_fields_mark_the_label_and_control() {
    let html = to_html(&Field::new("Name", Input::text("name")).required());
    assert!(
        html.contains(r#"<span class="st-required" aria-hidden="true">*</span>"#),
        "{html}"
    );
    assert!(html.contains(" required"));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "reserved attribute")]
fn field_wiring_cannot_be_overridden() {
    let _ = to_html(&Field::new("x", Input::text("x")).attr("aria-describedby", "y"));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "reserved attribute")]
fn control_names_cannot_be_overridden() {
    let _ = to_html(&Input::text("x").attr("name", "y"));
}

#[test]
fn checkboxes_bind_by_value() {
    let state = FormState::new()
        .with_value("tags", "a")
        .with_value("tags", "c");
    let a = to_html(&Checkbox::new("tags", "A").value("a").bind(&state));
    assert_eq!(
        a,
        r#"<label class="st-checkbox"><input type="checkbox" name="tags" value="a" checked>A</label>"#
    );
    assert!(!to_html(&Checkbox::new("tags", "B").value("b").bind(&state)).contains(" checked"));
    assert!(to_html(&Checkbox::new("terms", "I agree")).contains(r#"value="on""#));
}

#[test]
fn radio_groups_are_fieldsets_with_described_errors() {
    let state = FormState::new()
        .with_value("plan", "pro")
        .with_error("plan", "Pick one");
    let html = to_html(
        &RadioGroup::new("plan", "Plan")
            .option("free", "Free")
            .option("pro", "Pro <best>")
            .bind(&state),
    );
    assert!(
        html.starts_with(r#"<fieldset class="st-radio-group" role="radiogroup" aria-describedby="radios-1-error" aria-invalid="true"><legend class="st-legend">Plan</legend>"#),
        "{html}"
    );
    assert!(html.contains(r#"<label class="st-radio"><input type="radio" name="plan" value="pro" checked>Pro &lt;best&gt;</label>"#));
    assert!(html.contains(r#"id="radios-1-error""#) && html.contains("Pick one"));
}

#[test]
fn radio_group_hints_and_required() {
    let html = to_html(
        &RadioGroup::new("size", "Size")
            .option("s", "S")
            .hint("Pick a size")
            .required(),
    );
    assert!(
        html.contains(r#"aria-describedby="radios-1-hint""#)
            && html.contains(r#"<input type="radio" name="size" value="s" required>"#),
        "{html}"
    );
}

#[test]
fn checkboxes_can_be_required_and_described() {
    let state = FormState::new().with_error("terms", "You must agree");
    let html = to_html(
        &Checkbox::new("terms", "I agree")
            .required()
            .hint("Read them first")
            .bind(&state),
    );
    assert!(html.starts_with(r#"<div class="st-checkbox-field"><label class="st-checkbox"><input id="checkbox-1" type="checkbox""#), "{html}");
    assert!(html.contains(r#" required aria-describedby="checkbox-1-hint checkbox-1-error" aria-invalid="true">I agree</label>"#), "{html}");
    assert!(html.contains(r#"<p id="checkbox-1-hint" class="st-field-hint">Read them first</p>"#));
    assert!(html.contains("You must agree"));
}

#[test]
fn checkbox_passthrough_goes_to_the_input() {
    let html = to_html(
        &Checkbox::new("terms", "I agree")
            .id("terms")
            .data("track", "t"),
    );
    assert_eq!(
        html,
        r#"<label class="st-checkbox"><input id="terms" type="checkbox" name="terms" value="on" data-track="t">I agree</label>"#
    );
}

#[test]
fn adornments_are_announced_with_the_field() {
    let html =
        to_html(&Field::new("Quantity", Input::number("q").suffix("seats")).hint("Whole seats"));
    assert!(
        html.contains(r#"<span id="field-1-suffix" class="st-input-adornment">seats</span>"#),
        "{html}"
    );
    assert!(
        html.contains(r#"aria-describedby="field-1-suffix field-1-hint""#),
        "{html}"
    );
}

#[test]
fn textareas_keep_a_leading_newline() {
    let html = to_html(&Textarea::new("bio").value("\nsecond line"));
    assert!(
        html.contains("<textarea class=\"st-textarea\" name=\"bio\">\n\nsecond line</textarea>"),
        "{html:?}"
    );
}

#[test]
fn unmatched_selections_keep_the_placeholder_selected() {
    let html = to_html(
        &Select::new("plan")
            .placeholder("Choose…")
            .option("free", "Free")
            .selected("gone"),
    );
    assert!(
        html.contains(r#"<option value="" disabled selected>Choose…</option>"#),
        "{html}"
    );
    assert!(!html.contains(r#"value="free" selected"#));
}
