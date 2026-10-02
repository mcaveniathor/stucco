use serde::{Deserialize, Serialize};
use stucco::collections::Columns;
use stucco::{Tone, Validator};

/// One stored order; the fields marked as columns are the list's columns.
#[derive(Clone, Debug, Deserialize, Serialize, Columns)]
pub(crate) struct Order {
    #[col(label = "ID", sortable)]
    pub id: u64,
    #[col(sortable, searchable)]
    pub customer: String,
    #[col(enumeration("pending", "paid", "shipped", "archived"), filter)]
    pub status: String,
    #[col(key = "total", label = "Total", value = dollars, display = money, filter)]
    pub total_cents: u64,
    #[col(date, filter)]
    pub created: String,
    #[col(skip)]
    pub priority: bool,
    #[col(skip)]
    pub note: String,
    /// Bumped by every save, so an edit based on an older copy is caught.
    #[col(skip)]
    pub version: u64,
    /// The status to go back to when an archived order is restored.
    #[col(skip)]
    pub restore_status: Option<String>,
    /// Newest last.
    #[col(skip)]
    pub history: Vec<Event>,
}

/// Something that happened to an order.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Event {
    /// RFC 3339, UTC.
    pub at: String,
    pub actor: String,
    pub action: String,
    pub detail: Option<String>,
}

/// The statuses an order can be edited into; "archived" is set only by
/// archiving.
pub(crate) const EDITABLE_STATUSES: [(&str, &str); 3] = [
    ("pending", "Pending"),
    ("paid", "Paid"),
    ("shipped", "Shipped"),
];

impl Order {
    pub fn label(&self) -> String {
        format!("Order {}", self.id)
    }

    pub fn archived(&self) -> bool {
        self.status == "archived"
    }

    pub fn status_label(&self) -> &'static str {
        status_label(&self.status)
    }

    pub fn status_tone(&self) -> Tone {
        match self.status.as_str() {
            "paid" => Tone::Success,
            "shipped" => Tone::Info,
            "pending" => Tone::Warning,
            _ => Tone::Muted,
        }
    }

    pub fn record(&mut self, action: impl Into<String>, detail: Option<String>) {
        self.history.push(Event {
            at: crate::time::now_rfc3339(),
            // No sign-in in this example; a real app records the user.
            actor: "Demo user".into(),
            action: action.into(),
            detail,
        });
    }
}

pub(crate) fn status_label(status: &str) -> &'static str {
    match status {
        "pending" => "Pending",
        "paid" => "Paid",
        "shipped" => "Shipped",
        "archived" => "Archived",
        _ => "Unknown",
    }
}

fn dollars(order: &Order) -> f64 {
    order.total_cents as f64 / 100.0
}

pub(crate) fn money(order: &Order) -> String {
    format_cents(order.total_cents)
}

pub(crate) fn format_cents(cents: u64) -> String {
    format!("${}.{:02}", cents / 100, cents % 100)
}

/// What a valid order form submits.
pub(crate) struct OrderInput {
    pub customer: String,
    pub status: String,
    pub total_cents: u64,
    pub created: String,
    pub priority: bool,
    pub note: String,
}

impl OrderInput {
    /// The submitted fields, checked. Parsing problems ("12.x") and rules
    /// ("at most $1,000,000") are reported per field; the submitted text is
    /// kept on the state either way.
    pub fn validate(v: &mut Validator) -> Option<OrderInput> {
        let customer = v
            .single("customer", "Enter one customer name")
            .required("Enter the customer’s name")
            .max_chars(80, "Use 80 characters or fewer for the customer’s name")
            .get();
        let statuses: Vec<&str> = EDITABLE_STATUSES.iter().map(|(v, _)| *v).collect();
        let status = v
            .single("status", "Choose one status")
            .required("Choose a status")
            .one_of(&statuses, "Choose pending, paid or shipped")
            .get();
        let total_cents = v
            .single("total", "Enter one total")
            .required("Enter the order total")
            .and_then(|t| {
                parse_cents(&t).ok_or_else(|| "Enter the total as an amount, like 12.50".into())
            })
            .check(
                |c| *c <= 100_000_000,
                "Enter a total of $1,000,000.00 or less",
            )
            .get();
        let created = v
            .single("created", "Enter one date")
            .required("Enter the order date")
            .check(|d| valid_date(d), "Enter a real date, like 2026-09-04")
            .get();
        let note = v
            .text("note")
            .max_chars(500, "Keep the note to 500 characters or fewer")
            .get();
        // An unchecked box sends nothing: absent means "no", never an error.
        let priority = v.flag("priority");
        Some(OrderInput {
            customer: customer?,
            status: status?,
            total_cents: total_cents?,
            created: created?,
            priority,
            note: note.unwrap_or_default(),
        })
    }

    /// The fields that differ from `order`, by label.
    pub fn changes(&self, order: &Order) -> Vec<&'static str> {
        [
            ("customer", self.customer != order.customer),
            ("status", self.status != order.status),
            ("total", self.total_cents != order.total_cents),
            ("date", self.created != order.created),
            ("priority", self.priority != order.priority),
            ("note", self.note != order.note),
        ]
        .into_iter()
        .filter_map(|(name, changed)| changed.then_some(name))
        .collect()
    }

    pub fn apply(self, order: &mut Order) {
        order.customer = self.customer;
        order.status = self.status;
        order.total_cents = self.total_cents;
        order.created = self.created;
        order.priority = self.priority;
        order.note = self.note;
    }
}

/// "12", "12.5", "12.50", "$1,200.00" → cents.
fn parse_cents(text: &str) -> Option<u64> {
    let text: String = text
        .trim()
        .trim_start_matches('$')
        .chars()
        .filter(|c| *c != ',')
        .collect();
    let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
    if whole.is_empty() && fraction.is_empty()
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !fraction.chars().all(|c| c.is_ascii_digit())
        || fraction.len() > 2
        || whole.len() > 12
    {
        return None;
    }
    let whole: u64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let fraction: u64 = format!("{fraction:0<2}").parse().ok()?;
    whole.checked_mul(100)?.checked_add(fraction)
}

/// A real `YYYY-MM-DD` date.
fn valid_date(text: &str) -> bool {
    let parts: Vec<&str> = text.split('-').collect();
    let [y, m, d] = parts.as_slice() else {
        return false;
    };
    let (Ok(y), Ok(m), Ok(d)) = (y.parse::<u32>(), m.parse::<u32>(), d.parse::<u32>()) else {
        return false;
    };
    if text.len() != 10 || !(1..=9999).contains(&y) || !(1..=12).contains(&m) {
        return false;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    (1..=days[m as usize - 1]).contains(&d)
}

pub(crate) fn seed() -> Vec<Order> {
    (1..=67)
        .map(|id| {
            let created = format!("2026-09-{:02}", (id - 1) % 28 + 1);
            Order {
                id,
                customer: [
                    "Ada Lovelace",
                    "Grace Hopper",
                    "Alan Turing",
                    "Katherine Johnson",
                ][(id as usize - 1) % 4]
                    .into(),
                status: ["pending", "paid", "shipped"][(id as usize - 1) % 3].into(),
                total_cents: 1000 + id * 137,
                priority: id % 10 == 0,
                note: String::new(),
                version: 1,
                restore_status: None,
                history: vec![Event {
                    at: format!("{created}T09:00:00Z"),
                    actor: "Seed data".into(),
                    action: "created the order".into(),
                    detail: None,
                }],
                created,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn amounts_parse_to_cents() {
        assert_eq!(parse_cents("12"), Some(1200));
        assert_eq!(parse_cents(" $1,200.5 "), Some(120050));
        assert_eq!(parse_cents(".99"), Some(99));
        for bad in ["", ".", "12.345", "-1", "1e3", "12.x", "9999999999999"] {
            assert_eq!(parse_cents(bad), None, "{bad}");
        }
    }

    #[test]
    fn dates_must_exist() {
        assert!(valid_date("2028-02-29"));
        for bad in ["2026-02-29", "2026-13-01", "2026-1-01", "x", "2026-09-31"] {
            assert!(!valid_date(bad), "{bad}");
        }
    }
}
