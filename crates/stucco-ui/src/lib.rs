//! Components for stucco: layout primitives, typography, actions, form
//! controls and icons. Enable families with cargo features.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[macro_use]
mod passthrough;
mod common;

pub use common::{ColorScheme, Measure, Size, Space, Tone, Variant};

use stucco_core::Asset;

/// Every asset registered by the enabled component families.
pub fn ui_assets() -> Vec<&'static Asset> {
    #[allow(unused_mut)]
    let mut assets: Vec<&'static Asset> = Vec::new();
    assets
}
