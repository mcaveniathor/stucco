//! Shared data and validation for the Orders reference application.
//!
//! Prices come from the server's order, never from the submitted draft. The
//! server store persists tenant-scoped orders in redb or optional SQLite; this
//! module remains independent of storage on the WASM target.

use serde::{Deserialize, Serialize};

/// A public order snapshot.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Order {
    pub id: u64,
    pub revision: u64,
    pub customer: String,
    pub items: [LineItem; 2],
}

/// Number of records on a collection page.
pub const PAGE_SIZE: u32 = 5;

/// A customer-search result in descending ID order.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OrderPage {
    pub orders: Vec<Order>,
    pub query: String,
    pub page: u32,
    pub total_pages: u32,
    pub total_count: u64,
}

/// A server-priced line item whose quantity can be edited.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LineItem {
    pub id: u64,
    pub name: String,
    pub unit_price_cents: u64,
    pub quantity: u32,
}

/// Exact submitted text, retained when validation or a revision check fails.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct OrderDraft {
    pub customer: String,
    pub quantity_one: String,
    pub quantity_two: String,
    pub expected_revision: String,
}

/// A submitted field, including an error that applies to the whole order.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Field {
    Customer,
    QuantityOne,
    QuantityTwo,
    Revision,
    Order,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct FieldError {
    pub field: Field,
    pub message: String,
}

/// Validated input. Server services still validate every submitted draft.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ValidatedOrder {
    pub customer: String,
    pub quantity_one: u32,
    pub quantity_two: u32,
    pub expected_revision: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct TotalError;

impl std::fmt::Display for TotalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The order total is too large")
    }
}

impl std::error::Error for TotalError {}

impl Order {
    /// Editable values with the revision this snapshot was loaded from.
    pub fn draft(&self) -> OrderDraft {
        OrderDraft {
            customer: self.customer.clone(),
            quantity_one: self.items[0].quantity.to_string(),
            quantity_two: self.items[1].quantity.to_string(),
            expected_revision: self.revision.to_string(),
        }
    }

    /// Calculates integer cents, checking both multiplication and addition.
    pub fn total_cents(&self) -> Result<u64, TotalError> {
        self.items.iter().try_fold(0_u64, |total, item| {
            item.unit_price_cents
                .checked_mul(u64::from(item.quantity))
                .and_then(|subtotal| total.checked_add(subtotal))
                .ok_or(TotalError)
        })
    }
}

/// Formats integer cents without floating-point rounding.
pub fn money(cents: u64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// Checks the same text rules on native and WASM targets without changing it.
pub fn validate(draft: &OrderDraft) -> Result<ValidatedOrder, Vec<FieldError>> {
    let mut errors = Vec::new();
    let customer = draft.customer.trim();
    if customer.is_empty() {
        errors.push(field_error(Field::Customer, "Enter a customer name"));
    } else if customer.chars().count() > 80 {
        errors.push(field_error(Field::Customer, "Use 80 characters or fewer"));
    }

    let quantity_one = quantity(&draft.quantity_one, Field::QuantityOne, &mut errors);
    let quantity_two = quantity(&draft.quantity_two, Field::QuantityTwo, &mut errors);
    let expected_revision = unsigned(&draft.expected_revision).filter(|value| *value > 0);
    if expected_revision.is_none() {
        errors.push(field_error(
            Field::Revision,
            "Reload the order to obtain a valid revision",
        ));
    }

    match (quantity_one, quantity_two, expected_revision) {
        (Some(quantity_one), Some(quantity_two), Some(expected_revision)) if errors.is_empty() => {
            Ok(ValidatedOrder {
                customer: customer.to_owned(),
                quantity_one,
                quantity_two,
                expected_revision,
            })
        }
        _ => Err(errors),
    }
}

fn field_error(field: Field, message: &str) -> FieldError {
    FieldError {
        field,
        message: message.to_owned(),
    }
}

pub(crate) fn unsigned(value: &str) -> Option<u64> {
    let value = value.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok()
}

fn quantity(value: &str, field: Field, errors: &mut Vec<FieldError>) -> Option<u32> {
    match unsigned(value) {
        Some(quantity @ 1..=999) => Some(quantity as u32),
        _ => {
            errors.push(field_error(field, "Enter a whole quantity from 1 to 999"));
            None
        }
    }
}

/// A public failure loading the order; internal implementation details stay private.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct LoadError;

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("The order could not be loaded. Please try again.")
    }
}

impl std::error::Error for LoadError {}

/// Every rejected save carries the original draft, including its revision.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum SaveError {
    Access {
        draft: OrderDraft,
        reason: crate::auth::AccessError,
    },
    Validation {
        draft: OrderDraft,
        errors: Vec<FieldError>,
    },
    Conflict {
        draft: OrderDraft,
        current: Box<Order>,
    },
    NotFound {
        draft: OrderDraft,
    },
    Unavailable {
        draft: OrderDraft,
    },
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Access { .. } => "Your session or access changed. Sign in again.",
            Self::Validation { .. } => "Check the submitted order",
            Self::Conflict { .. } => "The order changed. Review it before saving again.",
            Self::NotFound { .. } => "The order no longer exists. Your edits have not been saved.",
            Self::Unavailable { .. } => "The order could not be saved. Please try again.",
        })
    }
}

impl std::error::Error for SaveError {}

/// Every rejected deletion preserves the submitted revision text.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum DeleteError {
    Access {
        expected_revision: String,
        reason: crate::auth::AccessError,
    },
    Validation {
        expected_revision: String,
        message: String,
    },
    Conflict {
        expected_revision: String,
        current: Box<Order>,
    },
    NotFound {
        expected_revision: String,
    },
    Unavailable {
        expected_revision: String,
    },
}

impl std::fmt::Display for DeleteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Access { .. } => "Your session or access changed. Sign in again.",
            Self::Validation { message, .. } => message,
            Self::Conflict { .. } => "The order changed. Review it before deleting.",
            Self::NotFound { .. } => "The order no longer exists.",
            Self::Unavailable { .. } => "The order could not be deleted. Please try again.",
        })
    }
}

impl std::error::Error for DeleteError {}

#[cfg(feature = "ssr")]
pub use crate::store::OrderStore;

/// A new unsaved order using the trusted catalogue, with one of each item.
/// ID zero is a draft placeholder; only the server assigns stored IDs.
pub fn new_order() -> Order {
    Order {
        id: 0,
        revision: 1,
        customer: String::new(),
        items: [
            LineItem {
                id: 1,
                name: "Notebook".into(),
                unit_price_cents: 1_200,
                quantity: 1,
            },
            LineItem {
                id: 2,
                name: "Desk organizer".into(),
                unit_price_cents: 3_500,
                quantity: 1,
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Order {
        let mut order = new_order();
        order.id = 1;
        order.customer = "Ada Lovelace".into();
        order.items[0].quantity = 2;
        order
    }

    #[test]
    fn validation_keeps_submitted_text_and_names_each_invalid_field() {
        let draft = OrderDraft {
            customer: "  ".into(),
            quantity_one: "1.5".into(),
            quantity_two: "-1".into(),
            expected_revision: "old".into(),
        };
        let original = draft.clone();
        let fields: Vec<Field> = validate(&draft)
            .unwrap_err()
            .into_iter()
            .map(|error| error.field)
            .collect();
        assert_eq!(draft, original);
        assert_eq!(
            fields,
            [
                Field::Customer,
                Field::QuantityOne,
                Field::QuantityTwo,
                Field::Revision
            ]
        );
    }

    #[test]
    fn validation_accepts_boundaries_and_rejects_invalid_numbers() {
        let mut draft = seed().draft();
        draft.customer = format!(" {} ", "é".repeat(80));
        draft.quantity_one = " 1 ".into();
        draft.quantity_two = "999".into();
        draft.expected_revision = u64::MAX.to_string();
        let input = validate(&draft).unwrap();
        assert_eq!(input.customer, "é".repeat(80));
        assert_eq!((input.quantity_one, input.quantity_two), (1, 999));
        assert_eq!(input.expected_revision, u64::MAX);
        for invalid in [
            "",
            "0",
            "1000",
            "-1",
            "+1",
            "1.0",
            "1e2",
            "１２",
            "18446744073709551616",
        ] {
            draft.quantity_one = invalid.into();
            assert!(
                validate(&draft)
                    .unwrap_err()
                    .iter()
                    .any(|e| e.field == Field::QuantityOne),
                "{invalid}"
            );
        }
        draft = seed().draft();
        draft.customer = "é".repeat(81);
        assert_eq!(validate(&draft).unwrap_err()[0].field, Field::Customer);
        draft = seed().draft();
        for invalid in ["", "0", "-1", "+1", "1.0", "18446744073709551616"] {
            draft.expected_revision = invalid.into();
            assert_eq!(validate(&draft).unwrap_err()[0].field, Field::Revision);
        }
    }

    #[test]
    fn totals_check_multiplication_and_addition_without_rounding() {
        let mut order = seed();
        assert_eq!(order.total_cents(), Ok(5_900));
        assert_eq!(money(0), "$0.00");
        assert_eq!(money(107), "$1.07");
        assert_eq!(money(u64::MAX), "$184467440737095516.15");
        order.items[0].unit_price_cents = u64::MAX;
        assert_eq!(order.total_cents(), Err(TotalError));
        order.items[0].quantity = 1;
        assert_eq!(order.total_cents(), Err(TotalError));
        order.items[1].unit_price_cents = 0;
        assert_eq!(order.total_cents(), Ok(u64::MAX));
    }

    #[test]
    fn new_orders_have_unassigned_ids_and_the_trusted_catalogue() {
        let order = new_order();
        assert_eq!((order.id, order.revision), (0, 1));
        assert!(order.customer.is_empty());
        assert_eq!(order.items.map(|item| item.quantity), [1, 1]);
        assert_eq!(new_order().total_cents(), Ok(4_700));
        assert_eq!(new_order().draft().expected_revision, "1");
    }
}
