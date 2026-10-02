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

/// Rows that know their table columns: a [`DataTable`] takes
/// `T::columns()`, and a query parser takes `T::column_specs()`, so the two
/// can't disagree.
///
/// The `stucco` crate's `derive` feature derives it from a struct's named
/// fields, one column per field in order. A field's kind comes from its type
/// (numbers for numeric primitives, text for strings) or from `#[col(...)]`:
///
/// | Option | Effect |
/// | --- | --- |
/// | `text`, `number`, `date` | The column kind |
/// | `enumeration("a", "b")` | A fixed set of values, shown as tags |
/// | `key = "k"` | The query key (default: the field name) |
/// | `label = "L"` | The heading (default: the field name as words) |
/// | `value = path` | A `fn(&Row) -> f64` (numbers) or `-> String` that sorts and filters, in place of the field |
/// | `display = path` | A `fn(&Row) -> String` for how cells read |
/// | `sortable`, `searchable`, `filter` | Enable that control |
/// | `skip` | No column for this field |
///
/// ```ignore
/// #[derive(Columns)]
/// struct Order {
///     #[col(label = "ID", sortable)]
///     id: u64,
///     #[col(sortable, searchable)]
///     customer: String,
///     #[col(enumeration("pending", "paid", "shipped"), filter)]
///     status: String,
/// }
/// ```
pub trait Columns: Sized {
    /// The columns, in order.
    fn columns<'a>() -> Vec<Col<'a, Self>>
    where
        Self: 'a;

    /// Each column's key and kind, for `CollectionQuery::parse` and
    /// `CollectionSource::load`.
    fn column_specs() -> Vec<stucco_core::ColumnSpec> {
        Self::columns().iter().map(|c| c.spec().clone()).collect()
    }
}
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
