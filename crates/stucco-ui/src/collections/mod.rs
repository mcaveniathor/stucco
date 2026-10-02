//! Server collections: native GET controls and typed table columns.
//!
//! ```
//! use stucco_ui::collections::{Col, DataTable, CollectionView, SearchForm,
//!     FilterBar, SortControl, PageSizeSelect, CollectionToolbar};
//! use stucco_core::{Capabilities, CollectionPage, CollectionQuery, ColumnSpec, ColumnKind, to_html};
//! let query = CollectionQuery::default();
//! let caps = Capabilities { sortable: vec!["name".into()], searchable: true, ..Capabilities::default() };
//! let columns = [ColumnSpec::new("name", ColumnKind::Text)];
//! let view = CollectionView::new("/orders", &query, &caps, &columns).label("orders");
//! let _controls = (SearchForm::new(&view), FilterBar::new(&view),
//!     SortControl::new(&view), PageSizeSelect::new(&view), CollectionToolbar::new(&view, 1, None));
//! let page = CollectionPage { rows: vec!["Ada"], next: None, prev: None, total: None };
//! let table = DataTable::from_page(&page, "Orders").query(&query).capabilities(&caps)
//!     .action("/orders").column(Col::text("name", "Customer", |row: &&str| row.to_string())
//!     .sortable().searchable());
//! assert!(to_html(&table).contains("Ada"));
//! ```
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
