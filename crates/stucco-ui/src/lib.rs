//! Components for stucco: layout primitives, typography, actions, form
//! controls and icons. Enable families with cargo features.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[macro_use]
mod passthrough;
#[cfg(feature = "actions")]
pub mod actions;
#[cfg(feature = "app")]
pub mod app;
mod common;
#[cfg(feature = "data")]
pub mod data;
#[cfg(feature = "feedback")]
pub mod feedback;
#[cfg(feature = "forms")]
pub mod forms;
pub mod icon;
#[cfg(feature = "layout")]
pub mod layout;
#[cfg(feature = "navigation")]
pub mod navigation;
#[cfg(feature = "typography")]
pub mod typography;

pub use common::{ColorScheme, Measure, Size, Space, Tone, Variant};
pub use icon::{Icon, LabelledIcon};

use stucco_core::Asset;

/// Every asset registered by the enabled component families.
pub fn ui_assets() -> Vec<&'static Asset> {
    #[allow(unused_mut)]
    let mut assets: Vec<&'static Asset> = vec![&icon::ICON];
    #[cfg(feature = "layout")]
    assets.push(&layout::LAYOUT);
    #[cfg(feature = "actions")]
    assets.push(&actions::ACTIONS);
    #[cfg(feature = "forms")]
    assets.push(&forms::FORMS);
    #[cfg(feature = "typography")]
    assets.push(&typography::TYPOGRAPHY);
    #[cfg(feature = "data")]
    assets.push(&data::DATA);
    #[cfg(feature = "app")]
    assets.push(&app::APP);
    #[cfg(feature = "feedback")]
    assets.push(&feedback::FEEDBACK);
    assets
}
