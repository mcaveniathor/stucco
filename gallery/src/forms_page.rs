//! Form primitives, shown as a submitted form with errors.

use stucco::actions::Button;
use stucco::forms::{
    Checkbox, ErrorSummary, Field, Fieldset, Form, FormActions, Input, RadioGroup, Select, Textarea,
};
use stucco::layout::{Grid, Stack};
use stucco::{Bundle, FormState, Measure, Render, Space};

use crate::shell::{component_page, section};

/// The password in the example submission; it must never appear in the page.
pub const SUBMITTED_PASSWORD: &str = "correct horse battery staple";

/// The forms page.
pub fn page(bundle: &Bundle) -> String {
    let state = FormState::new()
        .with_value("name", "Ada Lovelace")
        .with_value("email", "ada@example")
        .with_error("email", "Enter a valid email address.")
        .with_value("password", SUBMITTED_PASSWORD)
        .with_value("plan", "team")
        .with_value("terms", "on")
        .with_error("billing", "Choose a billing period.");
    let form = Form::post("#").csrf("demo-token").child(
        Stack::new()
            .space(Space::S6)
            .child(
                // A real failed submission lets the summary take focus; here
                // it would pull focus away from the rest of the gallery page.
                ErrorSummary::new(&state)
                    .field("email", "email")
                    .focus(false),
            )
            .child(
                Grid::new()
                    .min(Measure::Xs)
                    .space(Space::S4)
                    .child(
                        Field::new("Name", Input::text("name").autocomplete("name"))
                            .required()
                            .bind(&state),
                    )
                    .child(
                        Field::new(
                            "Email",
                            Input::email("email").autocomplete("email").id("email"),
                        )
                        .hint("We only use it for receipts.")
                        .bind(&state),
                    )
                    .child(
                        Field::new(
                            "Password",
                            Input::password("password").autocomplete("new-password"),
                        )
                        .hint("At least 12 characters.")
                        .bind(&state),
                    )
                    .child(
                        Field::new("Website", Input::url("website").prefix("https://"))
                            .bind(&state),
                    )
                    .child(
                        Field::new(
                            "Quantity",
                            Input::number("quantity").min("1").suffix("seats"),
                        )
                        .bind(&state),
                    )
                    .child(
                        Field::new(
                            "Plan",
                            Select::new("plan")
                                .placeholder("Choose a plan…")
                                .option("solo", "Solo")
                                .option("team", "Team")
                                .option("enterprise", "Enterprise"),
                        )
                        .bind(&state),
                    ),
            )
            .child(
                Field::new("Bio", Textarea::new("bio").rows(3))
                    .optional()
                    .bind(&state),
            )
            .child(
                Fieldset::new("Preferences").child(
                    Stack::new()
                        .space(Space::S4)
                        .child(
                            RadioGroup::new("billing", "Billing")
                                .option("monthly", "Monthly")
                                .option("yearly", "Yearly")
                                .bind(&state),
                        )
                        .child(Checkbox::new("terms", "I agree to the terms").bind(&state)),
                ),
            )
            .child(
                FormActions::submit("Create account")
                    .secondary(Button::new("Reset").reset())
                    .cancel("index.html"),
            ),
    );
    let fixed = Form::post("#").child(
        Grid::new()
            .min(Measure::Xs)
            .space(Space::S4)
            .child(
                Field::new(
                    "Account ID",
                    Input::text("account").value("acct_8f2c").readonly(),
                )
                .hint("Read-only: submitted, but not editable."),
            )
            .child(
                Field::new(
                    "Region",
                    Select::new("region")
                        .option("eu", "Europe")
                        .selected("eu")
                        .disabled(),
                )
                .hint("Disabled: neither editable nor submitted."),
            )
            .child(Field::new(
                "Archived note",
                Textarea::new("archived")
                    .rows(2)
                    .value("Kept for the record.")
                    .readonly(),
            ))
            .child(Field::new("Coupon", Input::text("coupon").disabled()).optional()),
    );
    let sections: Vec<Box<dyn Render>> = vec![
        section("A submitted form with errors", form),
        section("Read-only and disabled fields", fixed),
    ];
    component_page(
        bundle,
        "Forms",
        "Fields wire labels, hints and errors automatically and redisplay submitted values — never passwords.",
        sections,
    )
}
