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
    Href, Meta, Page, Raw, Render, RenderFn, RenderedFragment, Slot, Validator, WithBody, behavior,
    check_component_css, el, register_asset, render_fn, render_fragment, to_html,
};

pub use stucco_core::{
    Capabilities, Collection, CollectionPage, CollectionQuery, ColumnKind, ColumnSpec, Cursor,
    Direction, Filter, Window,
};
pub use stucco_ui::*;

/// Serving stucco from axum and Tower (the `stucco-tower` crate): asset
/// serving, page and fragment responses, request negotiation, form
/// submissions and the standard layers.
#[cfg(feature = "axum")]
pub use stucco_tower as server;

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

/// The everyday imports: the page and theme types, the common components
/// of each enabled feature, their shared enums, and with the `axum` feature
/// the server setup.
///
/// ```
/// use stucco::prelude::*;
///
/// let bundle = Bundle::new(Preset::Slate);
/// let html = Page::new(&bundle, "Hello")
///     .main(
///         Stack::new()
///             .space(Space::S4)
///             .child(Heading::new(1, "Hello"))
///             .child(Button::new("Continue").variant(Variant::Primary)),
///     )
///     .render();
/// assert!(html.contains(r#"<main id="main">"#));
/// ```
pub mod prelude {
    pub use crate::theme::{Preset, Random, Seeded, Theme};
    pub use crate::{
        Attrs, Bundle, Cx, Href, Measure, Page, PageExt, Render, Size, Space, Tone, Variant, el,
    };

    #[cfg(feature = "actions")]
    pub use crate::actions::{Button, ButtonLink, IconButton};
    #[cfg(feature = "app")]
    pub use crate::app::{AppShell, Footer, PageHeader};
    #[cfg(feature = "collections")]
    pub use crate::collections::{Col, DataTable};
    #[cfg(feature = "data")]
    pub use crate::data::{Card, Panel, Row, Table};
    #[cfg(feature = "feedback")]
    pub use crate::feedback::{EmptyState, LiveRegion};
    #[cfg(feature = "forms")]
    pub use crate::forms::{
        Checkbox, ErrorSummary, Field, Fieldset, Form, Input, RadioGroup, Select, Textarea,
    };
    #[cfg(feature = "layout")]
    pub use crate::layout::{
        Center, Cluster, Container, Grid, Separator, Sidebar, Stack, Switcher,
    };
    #[cfg(feature = "navigation")]
    pub use crate::navigation::{NavLink, Pagination};
    #[cfg(feature = "typography")]
    pub use crate::typography::{Code, Heading, Kbd, Link, Text};
    #[cfg(feature = "collections")]
    pub use crate::{Collection, CollectionQuery};
    #[cfg(feature = "axum")]
    pub use stucco_tower::{CollectionSource, Document, PageCx, StuccoRouter};
}
