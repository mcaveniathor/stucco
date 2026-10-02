//! Rendering core for stucco: the [`Render`] trait, safe markup, identity,
//! asset requirements, pages and fragments.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod aria;
mod attrs;
pub mod el;
pub mod escape;
mod href;
mod identity;
mod render;

pub use attrs::Attrs;
pub use href::Href;
pub use render::{Cx, Raw, Render, RenderFn, Slot, render_fn, to_html};
