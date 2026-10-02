//! Semantic data display.
//!
//! ```
//! use stucco_ui::data::{Card, Panel, Table, Row, ResultCount};
//! use stucco_core::to_html;
//! let table = Table::new("Orders").header(Row::new().header("Customer"))
//!     .row(Row::new().cell("Ada"));
//! let panel = Panel::new("Recent orders").body(Card::new().child(table))
//!     .footer(ResultCount::new(1, Some(1), Some(0)));
//! assert!(to_html(&panel).contains("<caption>Orders</caption>"));
//! ```
mod activity;
mod panel;
mod record;
mod result_count;
mod table;
pub use activity::{Activity, ActivityList, Timestamp};
pub use panel::{Card, Panel};
pub use record::{DescriptionList, StatusBadge};
pub use result_count::ResultCount;
pub use table::{Row, Table};
/// Data display stylesheet.
pub static DATA: stucco_core::Asset = stucco_core::Asset {
    name: "data",
    css: Some(include_str!("../../css/data.css")),
    behavior: None,
    deps: &[],
};
stucco_core::register_asset!(DATA);
