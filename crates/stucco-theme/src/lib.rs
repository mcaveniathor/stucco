//! Theme engine for stucco: OKLCH colour scales, semantic roles, token CSS,
//! presets and WCAG contrast validation.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod color;
mod css;
mod presets;
mod roles;
mod scale;
mod theme;

pub use color::{Color, contrast};
pub use presets::Preset;
pub use scale::{Scale, Scheme};
pub use theme::{
    BuiltTheme, ContrastFailure, ContrastReport, Density, Fonts, Radius, Scope, Theme, TypeScale,
};
