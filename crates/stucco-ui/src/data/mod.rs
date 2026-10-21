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
mod avatar;
mod badge;
mod description_list;
mod panel;
mod result_count;
mod sparkline;
mod stat;
mod table;
mod timeline;
pub use crate::Trend;
pub use avatar::Avatar;
pub use badge::Badge;
pub use description_list::DescriptionList;
pub use panel::{Card, Panel};
pub use result_count::ResultCount;
pub use sparkline::Sparkline;
pub use stat::Stat;
pub use table::{Row, Table};
pub use timeline::{Timeline, TimelineEntry};
/// Data display stylesheet.
pub static DATA: stucco_core::Asset = stucco_core::Asset {
    name: "data",
    css: Some(include_str!("../../css/data.css")),
    behavior: None,
    deps: &[],
};
stucco_core::register_asset!(DATA);
#[cfg(test)]
mod tests;
