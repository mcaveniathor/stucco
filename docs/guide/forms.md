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
    .child(FormActions::submit("Create order").cancel("/orders"));
```

Controls cover text, email, password, number, search, telephone, URL and date inputs, plus `Textarea`, `Select`, `Checkbox` and `RadioGroup`. Inputs accept prefix and suffix adornments, such as a currency sign. Group related controls in a `Fieldset`, whose legend names the group.

- `required()` on a field adds the `required` attribute and a marker; in a form where most fields are required, mark the few optional ones with `optional()` instead, which adds "(optional)" to the label.
- `disabled()` and `readonly()` (on `Input` and `Textarea`; `Select` has `disabled()`) show values that can't be changed. A disabled control is not submitted; a read-only one is.
- `FormActions` ends a form: the primary submit button first (so Enter submits with it), any secondary buttons, and a cancel link that leaves without submitting.

For an amount or other value users type in their own format, prefer a text input with `.attr("inputmode", "decimal")` to a number input: browsers discard what a number input can't parse, so the invalid text could never be shown back with its error.

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

### Repeated values, checkboxes and malformed input

A form body can carry a name more than once, or not at all, and anything a client likes:

- `text` takes the first value of a repeated name. `single` refuses a repeated name with an error (a forged or duplicated field), and `list` takes every value (checkbox groups, multi-selects).
- Browsers send nothing for an unchecked checkbox, so `flag("name")` is `true` when any value arrived and `false` otherwise; a missing checkbox is never an error.
- Parsing keeps the raw text. When `parse` or `and_then` fails, the state still holds what the user typed ("12.x"), and `bind` shows it back with the error. Keep parse errors ("Enter the total as an amount, like 12.50") apart from rule errors ("Enter a total of $1,000,000.00 or less") so each says what to fix.

## Initial and submitted values

The same `FormState` fills a form in both cases. For an edit form, build it from the stored record with `with_value`; for a submission, it comes from `FormState::from_urlencoded` (or `Submission`). `is_submitted()` tells them apart, and checkboxes follow suit: a checkbox is checked when its value is among the state's values.

```rust
let initial = FormState::new()
    .with_value("customer", &order.customer)
    .with_value("total", format!("{}.{:02}", cents / 100, cents % 100));
let initial = if order.priority { initial.with_value("priority", "on") } else { initial };
assert!(!initial.is_submitted());
```

When a submission fails, render it, not the stored record: the user should see what they typed.

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

## Server failures and conflicts

Not every failed save is the user's mistake. Report these as form-level errors (`FormState::with_form_error`), keep the submitted values, and use a status that says what happened:

| Situation | Status | Message |
| --- | --- | --- |
| A field is invalid | 422 | Per field, linked from the summary |
| Someone else saved the record first | 409 | "Someone else changed this order while you were editing…", with the submitted values kept |
| Storage or another service failed | 500 | "The order couldn't be saved because of a problem on our side. Your changes are still below; try again." |

For conflicts, put a version in the form as a hidden input (`HiddenInput::new("version", …)`), compare it when saving, and on a mismatch re-render with the record's current version so that saving again is a deliberate overwrite. The orders example does this.

## Feedback after a save

Attach a one-shot message to the redirect and show it on the page it leads to:

```rust
use stucco::prelude::*;

// After saving:
IntoResponse::into_response(
    SeeOther::new("/orders/42").flash(Flash::success("Order 42 was saved.")),
)

// On the order's page:
async fn show(flash: IncomingFlash, page: PageCx) -> (IncomingFlash, Document) {
    let notice = flash.get().map(|f| Notice::success(f.message().to_owned()));
    let document = page.title("Order 42").main((notice, /* the order */));
    (flash, document) // returning the flash deletes its cookie
}
```

The message travels in a short-lived `HttpOnly`, `SameSite=Lax` cookie that the showing page deletes. It is not signed, so treat it as display text: stucco escapes it, but never put secrets in it or act on it. Map `flash.level()` to the notice's tone (`Notice::success`, `info`, `warning`, `danger`). A notice for an outcome is announced (`role="status"`, or `role="alert"` for warnings and errors); call `quiet()` on notices that describe the page instead, such as "This order is archived".

Send the user somewhere predictable: back to the record they changed, or to the list they came from. If that location comes from the request (a `back` parameter), check it is one of your own paths before redirecting; `Href` blocks dangerous schemes, not other sites.

## Passwords and secrets

- `Input::password` and anything marked `sensitive()` never render a value, so a failed sign-in or settings form doesn't send the secret back to the browser.
- The `FormState` still holds the submitted secret, and its `Debug` output shows every value. Call `without_values(&["password", "token"])` before logging, storing or keeping the state, and don't log raw request bodies.
- Keep secrets out of flash messages, activity records and error messages.

## CSRF

`Form::csrf` adds a `_csrf` token field to a POST form (and `Confirmation::csrf` to a confirmation page); `FormState` drops that field so it is never echoed back. Issuing and checking tokens belongs to your application: use your framework's CSRF defence or an established library. The orders example refuses cross-site posts by checking the `Sec-Fetch-Site` and `Origin` headers instead of using tokens.

Never change anything in response to a GET. A GET may show a confirmation page; the change itself is a POST.
