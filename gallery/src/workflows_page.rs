//! Responsive collection and recoverable form workflow examples.
use crate::shell::{component_page, section};
use stucco::forms::HiddenInput;
use stucco::prelude::*;
use stucco::{Capabilities, FormState, Slot};

/// Static workflow examples. The GET form previews submitted values in its URL.
pub fn page(bundle: &Bundle, latest: bool, base: &str) -> String {
    let queue_path = format!("{base}collections.html");
    let workflow_path = format!("{base}workflows.html");
    let context = ReturnContext::parse(
        &format!("{queue_path}?q=Ada&sort=name&dir=desc&page=2"),
        &[&queue_path],
        &["q", "sort", "dir", "page"],
    )
    .unwrap();
    let rows = [
        (42, "Ada & Co", "Review"),
        (
            43,
            "Grace and a long customer name that wraps on small screens",
            "Ready",
        ),
    ];
    let query = CollectionQuery::default();
    let caps = Capabilities {
        sortable: vec!["name".into()],
        ..Capabilities::default()
    };
    let table = DataTable::new(&rows, "Review queue")
        .responsive()
        .query(&query)
        .capabilities(&caps)
        .action(workflow_path.clone())
        .column(Col::custom(
            "open",
            "Open record",
            |r: &(u64, &str, &str)| {
                Slot::new(ButtonLink::new(
                    format!("Open {}", r.0),
                    context
                        .carry(&format!("{workflow_path}?record={}#edit", r.0))
                        .unwrap(),
                ))
            },
        ))
        .column(Col::text("name", "Customer", |r: &(u64, &str, &str)| r.1.into()).sortable())
        .column(Col::text("status", "Status", |r: &(u64, &str, &str)| {
            r.2.into()
        }));
    let state = if latest {
        FormState::new()
            .with_value("customer", "Ada (latest saved)")
            .with_value("revision", "v2")
    } else {
        FormState::new()
            .with_value("customer", "Ada (my unsaved edit)")
            .with_value("revision", "v1")
            .with_conflict("v1", "v2")
    };
    let reload = context
        .carry(&format!("{base}workflows-latest.html#edit"))
        .unwrap();
    let form = Form::get(workflow_path.clone()).child(
        Stack::new()
            .space(Space::S4)
            .child(ConflictNotice::new(&state, reload.clone()).focus(false))
            .child(HiddenInput::new(
                "revision",
                state.value("revision").unwrap(),
            ))
            .child(HiddenInput::new(ReturnContext::FIELD, context.as_str()))
            .child(Field::new("Customer", Input::text("customer").id("customer-edit")).bind(&state))
            .child(Button::new("Preview submission").submit())
            .child(Link::new("Back to queue", context.href())),
    );
    component_page(
        bundle,
        "Workflow components",
        "Static examples: the form previews its submission in the URL. Applications own persistence and authorization.",
        [
            section("Responsive collection", table),
            section("Saved record editing", el::div().id("edit").child(form)),
            section(
                "Redirect feedback",
                FlashMessages::new([
                    Flash::new(FlashLevel::Success, "Order saved."),
                    Flash::new(
                        FlashLevel::Error,
                        "The order could not be deleted. Try again.",
                    ),
                ]),
            ),
            section(
                "Action prerequisites",
                ReadinessPanel::new("Ready to publish")
                    .requirement(Requirement::ready("Customer details"))
                    .requirement(
                        Requirement::blocked("Review", "A reviewer has not approved this record.")
                            .guidance("A reviewer can complete this step.")
                            .remedy("Review latest record", reload),
                    ),
            ),
        ],
    )
}
