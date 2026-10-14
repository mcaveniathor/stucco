#[cfg(any(feature = "collections", feature = "forms", feature = "feedback"))]
use stucco_core::to_html;

#[cfg(feature = "collections")]
#[test]
fn responsive_collection_renders_controls_once_and_keeps_headers() {
    use stucco_core::{Capabilities, CollectionQuery, Slot, el};
    use stucco_ui::collections::{Col, DataTable};
    let records = ["<Ada>"];
    let caps = Capabilities {
        sortable: vec!["name".into()],
        ..Capabilities::default()
    };
    let q = CollectionQuery::default();
    let html = to_html(
        &DataTable::new(&records, "Queue")
            .responsive()
            .query(&q)
            .capabilities(&caps)
            .action("/orders")
            .column(Col::text("name", "Customer & name", |r: &&str| r.to_string()).sortable())
            .column(Col::custom("action", "Action", |_: &&str| {
                Slot::new(el::button().id("only-button").text("Review"))
            })),
    );
    assert!(html.contains("st-table-cards"));
    assert!(html.contains("role=\"table\""));
    assert!(html.contains("role=\"columnheader\""));
    assert!(html.contains("scope=\"col\""));
    assert!(html.contains("role=\"cell\""));
    assert!(html.contains("sort=name"));
    assert!(html.contains("&lt;Ada&gt;"));
    assert!(html.contains("aria-hidden=\"true\">Customer &amp; name"));
    assert_eq!(html.matches("id=\"only-button\"").count(), 1);
}

#[cfg(feature = "forms")]
#[test]
fn conflicts_retain_submitted_fields_and_revision_until_deliberate_reload() {
    use stucco_core::FormState;
    use stucco_ui::forms::{ConflictNotice, ErrorSummary, Field, HiddenInput, Input};
    let state = FormState::from_urlencoded(b"name=My+edit&revision=v1&tags=a&tags=b")
        .with_conflict("v1", "v2");
    assert!(state.has_errors());
    assert_eq!(state.conflict().unwrap().submitted(), "v1");
    assert_eq!(state.conflict().unwrap().current(), "v2");
    assert_eq!(state.values("tags"), ["a", "b"]);
    let html = to_html(&(
        ConflictNotice::new(&state, "/edit?return_to=%2Fqueue"),
        Field::new("Name", Input::text("name")).bind(&state),
        HiddenInput::new("revision", state.value("revision").unwrap()),
    ));
    assert!(html.contains("value=\"My edit\""));
    assert!(html.contains("name=\"revision\" value=\"v1\""));
    assert!(!html.contains("v2"));
    assert!(html.contains("autofocus"));
    assert!(html.contains("href=\"/edit?return_to=%2Fqueue\""));
    assert!(to_html(&ErrorSummary::new(&state)).contains("Your changes were not saved"));
    assert!(!to_html(&ConflictNotice::new(&state, "/edit").focus(false)).contains("autofocus"));
    assert_eq!(
        to_html(&ConflictNotice::new(&FormState::new(), "/edit")),
        ""
    );
}

#[cfg(feature = "feedback")]
#[test]
fn readiness_explains_mixed_checks_without_color_and_escapes_content() {
    use stucco_ui::feedback::{ReadinessPanel, Requirement};
    let html = to_html(
        &ReadinessPanel::new("Publish <draft>")
            .requirement(Requirement::ready("Details"))
            .requirement(
                Requirement::blocked("Approval", "Needs <review>")
                    .guidance("Ask an editor")
                    .remedy("Review", "/edit"),
            ),
    );
    assert!(html.contains("Publish &lt;draft&gt;"));
    assert!(html.contains("1 of 2 checks complete."));
    assert!(html.contains("Action needed"));
    assert!(html.contains("Needs &lt;review&gt;"));
    assert!(html.contains("Ask an editor"));
    assert!(html.contains("href=\"/edit\""));
    assert!(html.contains("aria-labelledby=\"readiness-1\""));
    assert!(to_html(&ReadinessPanel::new("Publish")).contains("No checks available."));
    assert!(
        to_html(&ReadinessPanel::new("Publish").requirement(Requirement::ready("Details")))
            .contains("1 of 1 checks complete.")
    );
}
