//! Rendering core for stucco: the [`Render`] trait, safe markup, identity,
//! asset requirements, pages and fragments.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod aria;
mod asset;
mod attrs;
mod base_css;
pub mod behavior;
mod bundle;
pub mod el;
pub mod escape;
mod hash;
mod href;
mod identity;
mod render;

pub use asset::{Asset, AssetRef, AssetRequirements, Behavior, is_registered, registered_assets};
pub use attrs::Attrs;
pub use base_css::{BASE_CSS, LAYERS_CSS, RESET_CSS, check_component_css};
pub use bundle::{AssetFile, Bundle};
pub use href::Href;
#[doc(hidden)]
pub use inventory;
pub use render::{Cx, Raw, Render, RenderFn, Slot, render_fn, to_html};
