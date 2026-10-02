use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Order {
    pub id: u64,
    pub customer: String,
    pub status: String,
    pub total_cents: u64,
    pub created: String,
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
