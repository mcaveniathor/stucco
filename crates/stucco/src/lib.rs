//! stucco: server-rendered, themeable, accessible UI components.
//!
//! Render components into a [`Page`] backed by a [`Bundle`]:
//!
//! ```
//! use stucco::prelude::*;
//!
//! let bundle = Bundle::new(Preset::Slate);
//! let html = Page::new(&bundle, "Hello")
//!     .body(el::main().child(el::h1().text("Hello")))
//!     .render();
//! assert!(html.contains("<h1>Hello</h1>"));
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub use stucco_core::{
    Asset, AssetFile, AssetRequirements, Attrs, Behavior, Bundle, Cx, Delivery, FormState, Href,
    Meta, Page, Raw, Render, RenderFn, RenderedFragment, Slot, behavior, check_component_css, el,
    register_asset, render_fn, render_fragment, to_html,
};

/// Themes: colours, scales, presets and contrast validation.
pub mod theme {
    pub use stucco_theme::{
        BuiltTheme, Color, ContrastFailure, ContrastReport, Density, Fonts, Preset, Radius, Scale,
        Scheme, Scope, Theme, TypeScale, contrast,
    };
}

/// The most common imports.
pub mod prelude {
    pub use crate::theme::{Preset, Theme};
    pub use crate::{Bundle, Cx, Page, Render, el};
}
