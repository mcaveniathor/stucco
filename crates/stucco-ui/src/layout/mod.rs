//! Layout primitives (feature `layout`): small composable building blocks
//! that are responsive on their own. Parameters are data attributes, so
//! layouts work under a strict CSP.

use stucco_core::el::{self, Element};
use stucco_core::{Asset, register_asset};

/// Generates `.child()`, `.children()` and `.as_tag()` (plus passthrough
/// methods) for a layout struct with `children: Vec<Slot<'a>>` and `tag: Tag`
/// fields.
macro_rules! container_methods {
    ($ty:ident) => {
        impl<'a> $ty<'a> {
            /// Appends a child.
            pub fn child(mut self, child: impl stucco_core::Render + 'a) -> Self {
                self.children.push(stucco_core::Slot::new(child));
                self
            }
            /// Appends every item as a child.
            pub fn children<R: stucco_core::Render + 'a>(
                mut self,
                items: impl IntoIterator<Item = R>,
            ) -> Self {
                self.children
                    .extend(items.into_iter().map(stucco_core::Slot::new));
                self
            }
            /// Renders as a different element (e.g. `Tag::Ul` for a list).
            pub fn as_tag(mut self, tag: $crate::layout::Tag) -> Self {
                self.tag = tag;
                self
            }
        }
        passthrough!($ty<'_>);
    };
}

mod center;
mod cluster;
mod container;
mod grid;
mod stack;

pub use center::Center;
pub use cluster::{Align, Cluster, Justify};
pub use container::Container;
pub use grid::Grid;
pub use stack::Stack;

#[cfg(test)]
mod tests;

/// The layout family's stylesheet.
pub static LAYOUT: Asset = Asset {
    name: "st-layout",
    css: Some(include_str!("../../css/layout.css")),
    behavior: None,
    deps: &[],
};
register_asset!(LAYOUT);

/// The element a layout primitive renders as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tag {
    /// `<div>` (default).
    #[default]
    Div,
    /// `<section>`.
    Section,
    /// `<ul>`.
    Ul,
    /// `<ol>`.
    Ol,
    /// `<nav>`.
    Nav,
    /// `<header>`.
    Header,
    /// `<footer>`.
    Footer,
    /// `<article>`.
    Article,
    /// `<aside>`.
    Aside,
}

impl Tag {
    pub(crate) fn element<'a>(self) -> Element<'a> {
        match self {
            Tag::Div => el::div(),
            Tag::Section => el::section(),
            Tag::Ul => el::ul(),
            Tag::Ol => el::ol(),
            Tag::Nav => el::nav(),
            Tag::Header => el::header(),
            Tag::Footer => el::footer(),
            Tag::Article => el::article(),
            Tag::Aside => el::aside(),
        }
    }
}
