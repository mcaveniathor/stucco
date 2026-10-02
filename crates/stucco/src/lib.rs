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

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub use stucco_core::{
    Asset, AssetFile, AssetRequirements, Attrs, Behavior, Bundle, Check, Cx, Delivery, FormState,
    Href, Meta, Page, Raw, Render, RenderFn, RenderedFragment, Slot, Validator, behavior,
    check_component_css, el, register_asset, render_fn, render_fragment, to_html,
};

pub use stucco_core::{
    Capabilities, CollectionPage, CollectionQuery, ColumnKind, ColumnSpec, Cursor, Direction,
    Filter, Window,
};
pub use stucco_ui::*;

/// Themes: colours, scales, presets and contrast validation.
pub mod theme {
    pub use stucco_theme::spec;
    pub use stucco_theme::{
        Backdrop, BuiltTheme, ButtonDepth, ButtonShape, Color, ContrastFailure, ContrastReport,
        ControlStyle, CornerStyle, Density, Elevation, Finish, FocusStyle, Fonts, HeaderEdge,
        HeaderStyle, HeadingFont, HeadingWeight, IconWeight, LabelStyle, Leading, LineWeight,
        LinkStyle, Material, Motion, NavStyle, Palette, PanelStyle, Pattern, Preset, Radius,
        Random, Relief, RuleStyle, Scale, Scheme, Scope, SeedRng, Seeded, ShadowStyle, ShellLayout,
        TableStyle, TagStyle, Theme, TypeScale, contrast, random_seed,
    };
}

/// The most common imports.
pub mod prelude {
    pub use crate::theme::{Preset, Random, Seeded, Theme};
    pub use crate::{Bundle, Cx, Page, Render, el};
}
