//! Page shells and page identity.
//!
//! ```
//! use stucco_ui::app::{AppShell, PageHeader, SectionHeader, Footer};
//! use stucco_ui::navigation::NavLink;
//! use stucco_core::{el, to_html};
//! let shell = AppShell::new().header(PageHeader::new("Orders"))
//!     .link(NavLink::new("Orders", "/orders").current(true))
//!     .sidebar(el::nav().aria("label", "Reports").child("Weekly"))
//!     .main(SectionHeader::new("Recent orders"))
//!     .footer(Footer::new().child("Example"));
//! assert_eq!(to_html(&shell).matches("<main ").count(), 1);
//! ```
mod footer;
mod header;
mod shell;
pub use footer::Footer;
pub use header::{PageHeader, SectionHeader};
pub use shell::AppShell;
/// Application-shell styles.
pub static APP: stucco_core::Asset = stucco_core::Asset {
    name: "app",
    css: Some(include_str!("../../css/app.css")),
    behavior: None,
    deps: &[],
};
stucco_core::register_asset!(APP);
