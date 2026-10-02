//! Theme engine for stucco: OKLCH colour scales, semantic roles, token CSS,
//! presets and WCAG contrast validation.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod color;
mod css;
mod personality;
mod presets;
mod roles;
mod scale;
mod seed;
pub mod spec;
mod theme;

pub use color::{Color, contrast};
pub use personality::{
    ButtonDepth, ButtonShape, ControlStyle, CornerStyle, Elevation, Finish, FocusStyle,
    HeaderStyle, HeadingFont, HeadingWeight, IconWeight, LabelStyle, Leading, LineWeight,
    LinkStyle, Motion, NavStyle, PanelStyle, RuleStyle, ShellLayout, TableStyle, TagStyle,
};
pub use presets::Preset;
pub use scale::{Scale, Scheme};
pub use seed::{Palette, Random, SeedRng, Seeded, random_seed};
pub use theme::{
    BuiltTheme, ContrastFailure, ContrastReport, Density, Fonts, Radius, Scope, Theme, TypeScale,
};
