# Forms

Build forms from labelled fields, read a submission, validate it, and show it again with every value kept and every error explained.

## Fields

A `Field` wraps a control with its label, an optional hint and any errors, and connects them for assistive technology:

```rust
use stucco::forms::{Field, Form, Input, Select};
use stucco::actions::Button;
use stucco::Variant;

let form = Form::post("/orders")
    .csrf(&token)
    .child(Field::new("Customer", Input::text("customer").id("order-customer")).required())
    .child(
        Field::new("Quantity", Input::number("quantity").min("1"))
            .hint("Between 1 and 99."),
    )
    .child(Field::new(
        "Status",
        Select::new("status").options([("pending", "Pending"), ("paid", "Paid")]),
    ))
    .child(Button::new("Create order").submit().variant(Variant::Primary));
```

Controls cover text, email, password, number, search, telephone, URL and date inputs, plus `Textarea`, `Select`, `Checkbox` and `RadioGroup`. Inputs accept prefix and suffix adornments, such as a currency sign.

## Reading a submission

In an Axum handler, `Submission` reads an `application/x-www-form-urlencoded` body into a `FormState`. It answers `415 Unsupported Media Type` for other content types, and the body limit from the standard layers still applies. The CSRF field is left out of the state, so a token is never echoed back into a form.

```rust
use stucco_tower::{SeeOther, Submission};

async fn create(Submission(form): Submission) -> axum::response::Response {
    // validate, then redirect or re-render (below)
}
```

Outside Axum, `FormState::from_urlencoded(&body)` does the same parsing.

## Validating

`Validator` checks values and records one message per field. Each check returns the value only if it passed, and `finish` either builds your typed result or hands back the state with its errors:

```rust
use stucco::Validator;

struct Order { customer: String, quantity: u32 }

let mut v = Validator::new(form);
let customer = v.text("customer")
    .required("Enter a customer name")
    .max_chars(80, "Use 80 characters or fewer")
    .get();
let quantity = v.text("quantity")
    .required("Enter a quantity")
    .parse::<u32>("Enter a whole number")
    .check(|q| (1..=99).contains(q), "Enter a quantity from 1 to 99")
    .get();
let result = v.finish(|| Some(Order { customer: customer?, quantity: quantity? }));
```

Values are trimmed, and an empty value counts as missing: it fails `required` and passes every other check, so optional fields need no special handling. `one_of` checks a select's value against the options you offered, since a client can send anything. `error` and `form_error` record problems found elsewhere, such as a duplicate in your database.

Write messages that say how to fix the problem: "Enter a quantity from 1 to 99", not "Invalid quantity".

## Showing errors

When validation fails, render the same form with the state bound to each field and an `ErrorSummary` at the top, and respond with `422 Unprocessable Content`:

```rust
use axum::response::IntoResponse;
use stucco::prelude::*;
use stucco::server::SeeOther;

// In a handler that takes `page: PageCx`.
match result {
    Ok(order) => {
        save(order).await;
        IntoResponse::into_response(SeeOther::new("/orders"))
    }
    Err(state) => {
        let form = Form::post("/orders")
            .child(ErrorSummary::new(&state).field("customer", "order-customer"))
            .child(Field::new("Customer", Input::text("customer").id("order-customer")).bind(&state));
        page.title("New order")
            .main(form)
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .into_response()
    }
}
```

`FragmentResponse` and `SeeOther` also have an inherent `into_response` that returns a plain `http` response, which is why the axum conversion is spelled out for `SeeOther` here.

- `bind` puts the submitted value back in the control and shows the field's errors under it. Passwords are never redisplayed.
- The summary lists form-wide errors first, then each field error as a link to its control, and takes focus when the page loads so keyboard and screen reader users hear what went wrong.
- After a successful submission, `SeeOther` answers `303 See Other`, so reloading the next page doesn't submit the form twice.

## CSRF

`Form::csrf` adds the token field to a POST form. Issuing and verifying tokens is up to your application today; a signed double-submit layer for `stucco-tower` is planned.

## Revision conflicts

When an atomic save rejects a stale revision, record the failure on the submitted
`FormState` and respond with `409 Conflict`. `with_conflict` stores opaque
submitted/current revisions without changing any submitted values and makes
`has_errors()` true. Bind controls to that state and keep the **submitted**
revision in the form:

```rust
use stucco::{FormState, ReturnContext};
use stucco::forms::{ConflictNotice, Field, Form, HiddenInput, Input};

let submitted = FormState::from_urlencoded(b"customer=My+edit&revision=v1");
// The storage transaction rejected expected revision v1 and observed v2.
let state = submitted.with_conflict("v1", "v2");
let context = ReturnContext::parse("/orders?page=2", &["/orders"], &["page"]).unwrap();
let form = Form::post("/orders/42/edit")
    .child(ConflictNotice::new(&state, context.carry("/orders/42/edit").unwrap()))
    .child(HiddenInput::new("revision", state.conflict().unwrap().submitted()))
    .child(HiddenInput::new(ReturnContext::FIELD, context.as_str()))
    .child(Field::new("Customer", Input::text("customer")).bind(&state));
```

`ConflictNotice` explains the failure, takes focus on initial page load, and
provides an ordinary GET link explicitly labelled as discarding entries and
reloading. It renders nothing without a conflict. No client behavior silently
refreshes the revision or overwrites edits. `ErrorSummary` also reports a
conflict; if both are present, set `ConflictNotice::focus(false)` so only the
summary requests focus.

The handler must compare the expected revision and save atomically in storage,
recheck authorization, and retain the original revision after any validation
failure. Setting `with_conflict` does not perform a save or compare revisions.
Only a deliberate reload or reconciliation should adopt a newer revision.
