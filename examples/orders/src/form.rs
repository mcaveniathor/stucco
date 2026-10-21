//! The order form: validation and rendering.
use crate::model::{Order, STATUSES};
use stucco::forms::HiddenInput;
use stucco::prelude::*;
use stucco::{FormState, Validator};

/// A validated submission.
pub(crate) struct OrderInput {
    pub customer: String,
    pub status: String,
    pub total_cents: u64,
    pub created: String,
}

impl OrderInput {
    pub(crate) fn into_order(self, id: u64) -> Order {
        Order {
            id,
            customer: self.customer,
            status: self.status,
            total_cents: self.total_cents,
            created: self.created,
        }
    }
}

const TOTAL: &str = "Enter an amount like 12.50";
const DATE: &str = "Enter a date like 2026-09-14";

/// Checks a submission, keeping its values and errors on failure.
pub(crate) fn validate(state: FormState) -> Result<OrderInput, FormState> {
    let mut v = Validator::new(state);
    let customer = v
        .text("customer")
        .required("Enter a customer name")
        .max_chars(80, "Use 80 characters or fewer")
        .get();
    let status = v
        .text("status")
        .required("Choose a status")
        .one_of(&STATUSES, "Choose a status")
        .get();
    let total = v
        .text("total")
        .required(TOTAL)
        .and_then(|s| cents(&s).ok_or_else(|| TOTAL.to_owned()))
        .get();
    let created = v
        .text("created")
        .required(DATE)
        .check(|s| is_date(s), DATE)
        .get();
    v.finish(|| {
        Some(OrderInput {
            customer: customer?,
            status: status?,
            total_cents: total?,
            created: created?,
        })
    })
}

/// `12`, `12.5` or `12.50` dollars as cents.
fn cents(text: &str) -> Option<u64> {
    let (whole, frac) = text.split_once('.').unwrap_or((text, ""));
    let digits = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if whole.is_empty() || !digits(whole) || !digits(frac) || frac.len() > 2 {
        return None;
    }
    if text.contains('.') && frac.is_empty() {
        return None;
    }
    let frac = format!("{frac:0<2}").parse::<u64>().ok()?;
    whole
        .parse::<u64>()
        .ok()?
        .checked_mul(100)?
        .checked_add(frac)
}

/// `YYYY-MM-DD` with a plausible month and day.
fn is_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let number = |s: &str, len: usize| {
        (s.len() == len && s.bytes().all(|b| b.is_ascii_digit()))
            .then(|| s.parse::<u32>().ok())
            .flatten()
    };
    matches!(
        parts[..],
        [y, m, d] if number(y, 4).is_some()
            && number(m, 2).is_some_and(|m| (1..=12).contains(&m))
            && number(d, 2).is_some_and(|d| (1..=31).contains(&d))
    )
}

/// A state holding `order`'s values, for an edit form.
pub(crate) fn state_of(order: &Order) -> FormState {
    FormState::new()
        .with_value("customer", order.customer.clone())
        .with_value("status", order.status.clone())
        .with_value(
            "total",
            format!("{}.{:02}", order.total_cents / 100, order.total_cents % 100),
        )
        .with_value("created", order.created.clone())
}

/// The form posting to `action`. Edits carry the revision they started from.
pub(crate) fn order_form<'a>(
    action: String,
    state: &'a FormState,
    csrf: &str,
    revision: Option<&str>,
    submit: &str,
) -> Form<'a> {
    let mut form = Form::post(action).csrf(csrf);
    if let Some(revision) = revision {
        form = form.child(HiddenInput::new("revision", revision));
    }
    form.child(
        ErrorSummary::new(state)
            .field("customer", "order-customer")
            .field("status", "order-status")
            .field("total", "order-total")
            .field("created", "order-created"),
    )
    .child(
        Field::new("Customer", Input::text("customer").id("order-customer"))
            .required()
            .bind(state),
    )
    .child(
        Field::new(
            "Status",
            Select::new("status")
                .id("order-status")
                .placeholder("Choose a status")
                .options(STATUSES.iter().map(|s| (*s, *s))),
        )
        .required()
        .bind(state),
    )
    .child(
        Field::new(
            "Total",
            Input::text("total")
                .id("order-total")
                .attr("inputmode", "decimal")
                .prefix("$"),
        )
        .hint("Dollars and cents, like 12.50")
        .required()
        .bind(state),
    )
    .child(
        Field::new("Created", Input::date("created").id("order-created"))
            .required()
            .bind(state),
    )
    .child(
        Button::new(submit.to_owned())
            .submit()
            .variant(Variant::Primary),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cents_accepts_whole_and_two_place_amounts_only() {
        assert_eq!(cents("12"), Some(1200));
        assert_eq!(cents("12.5"), Some(1250));
        assert_eq!(cents("0.07"), Some(7));
        for bad in [
            "12.555",
            "-1",
            "1e3",
            ".5",
            "12.",
            "",
            "1.2.3",
            "99999999999999999999",
        ] {
            assert_eq!(cents(bad), None, "{bad}");
        }
    }

    #[test]
    fn dates_need_the_iso_shape() {
        assert!(is_date("2026-09-14"));
        for bad in [
            "2026-13-01",
            "2026-9-14",
            "2026-09-32",
            "26-09-14",
            "2026-09-14x",
            "",
        ] {
            assert!(!is_date(bad), "{bad}");
        }
    }
}
