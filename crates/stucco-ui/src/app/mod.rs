//! Page shells and page identity.
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
