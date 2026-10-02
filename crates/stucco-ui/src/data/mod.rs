//! Semantic data display.
mod panel;
mod result_count;
mod table;
pub use panel::{Card, Panel};
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
