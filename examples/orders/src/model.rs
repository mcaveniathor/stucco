use serde::{Deserialize, Serialize};
use stucco::collections::Columns;

/// One stored order; its fields are also the table's columns.
#[derive(Clone, Debug, Deserialize, Serialize, Columns)]
pub(crate) struct Order {
    #[col(label = "ID", sortable)]
    pub id: u64,
    #[col(sortable, searchable)]
    pub customer: String,
    #[col(enumeration("pending", "paid", "shipped"), filter)]
    pub status: String,
    #[col(key = "total", label = "Total", value = dollars, display = money, filter)]
    pub total_cents: u64,
    #[col(date, filter)]
    pub created: String,
}

fn dollars(order: &Order) -> f64 {
    order.total_cents as f64 / 100.0
}

fn money(order: &Order) -> String {
    format!(
        "${}.{:02}",
        order.total_cents / 100,
        order.total_cents % 100
    )
}
pub(crate) fn seed() -> Vec<Order> {
    (1..=67)
        .map(|id| Order {
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
            created: format!("2026-09-{:02}", (id - 1) % 28 + 1),
        })
        .collect()
}
