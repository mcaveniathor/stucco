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
mod collection;
pub mod el;
pub mod escape;
mod form_state;
mod fragment;
mod hash;
mod href;
mod identity;
mod page;
mod render;
mod validate;

pub use asset::{Asset, AssetRef, AssetRequirements, Behavior, is_registered, registered_assets};
pub use attrs::Attrs;
pub use base_css::{BASE_CSS, LAYERS_CSS, RESET_CSS, check_component_css};
pub use bundle::{AssetFile, Bundle};
pub use collection::{
    Capabilities, CollectionPage, CollectionQuery, ColumnKind, ColumnSpec, Cursor, Direction,
    Filter, Window,
};
pub use form_state::FormState;
pub use fragment::{RenderedFragment, render_fragment};
pub use href::Href;
#[doc(hidden)]
pub use inventory;
pub use page::{Delivery, Meta, Page};
pub use render::{Cx, Raw, Render, RenderFn, Slot, render_fn, to_html};
pub use validate::{Check, Validator};
