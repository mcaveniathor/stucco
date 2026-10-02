//! Server-rendered navigation.
mod pagination;
pub use pagination::Pagination;
/// Navigation stylesheet.
pub static NAVIGATION: stucco_core::Asset = stucco_core::Asset {
    name: "navigation",
    css: Some(include_str!("../../css/navigation.css")),
    behavior: None,
    deps: &[],
};
stucco_core::register_asset!(NAVIGATION);
