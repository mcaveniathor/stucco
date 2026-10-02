//! Server collections: native GET controls and typed table columns.
mod column;
mod data_table;
mod toolbar;
pub use column::Col;
pub use data_table::DataTable;
pub use toolbar::{
    CollectionToolbar, CollectionView, FilterBar, PageSizeSelect, SearchForm, SortControl,
};
/// Collection styles (no behavior script in server mode).
pub static COLLECTIONS: stucco_core::Asset = stucco_core::Asset {
    name: "collections",
    css: Some(include_str!("../../css/collections.css")),
    behavior: None,
    deps: &[],
};
stucco_core::register_asset!(COLLECTIONS);
