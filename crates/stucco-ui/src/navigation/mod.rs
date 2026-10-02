//! Server-rendered navigation.
//!
//! ```
//! use stucco_ui::navigation::Pagination;
//! use stucco_core::{CollectionQuery, Cursor, to_html};
//! let nav = Pagination::new("/orders", &CollectionQuery::default())
//!     .cursors(Cursor::new("next"), None);
//! assert!(to_html(&nav).contains("after=next"));
//! ```
mod breadcrumbs;
mod nav_link;
mod pagination;
mod tabs;
pub use breadcrumbs::Breadcrumbs;
pub use nav_link::NavLink;
pub use pagination::Pagination;
pub use tabs::Tabs;
/// Navigation stylesheet.
pub static NAVIGATION: stucco_core::Asset = stucco_core::Asset {
    name: "navigation",
    css: Some(include_str!("../../css/navigation.css")),
    behavior: None,
    deps: &[],
};
stucco_core::register_asset!(NAVIGATION);
