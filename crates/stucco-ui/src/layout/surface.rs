use stucco_core::{Attrs, Cx, Render, Slot};

use super::{LAYOUT, Tag};
use crate::passthrough::apply;
use crate::{Size, Space};

/// How far a [`Surface`] sits above the page.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Level {
    /// The page background.
    #[default]
    Flat,
    /// A panel (`--st-surface`).
    Raised,
    /// A floating layer with a shadow.
    Overlay,
}

/// A background plane with optional padding, border and rounding.
///
/// ```
/// use stucco_ui::layout::{Level, Surface};
/// let html = stucco_core::to_html(&Surface::new().level(Level::Raised).child("x"));
/// assert!(html.contains(r#"data-level="raised""#));
/// ```
#[derive(Debug)]
pub struct Surface<'a> {
    attrs: Attrs,
    tag: Tag,
    level: Level,
    padding: Option<Space>,
    border: bool,
    radius: Option<Size>,
    children: Vec<Slot<'a>>,
}

impl Default for Surface<'_> {
    fn default() -> Self {
        Surface {
            attrs: Attrs::default(),
            tag: Tag::Div,
            level: Level::Flat,
            padding: None,
            border: true,
            radius: None,
            children: Vec::new(),
        }
    }
}

impl Surface<'_> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] =
        &["data-level", "data-padding", "data-border", "data-radius"];

    /// A flat, bordered surface.
    pub fn new() -> Self {
        Self::default()
    }

    /// Elevation.
    pub fn level(mut self, level: Level) -> Self {
        self.level = level;
        self
    }

    /// Inner padding.
    pub fn padding(mut self, padding: Space) -> Self {
        self.padding = Some(padding);
        self
    }

    /// Whether to draw a border (default `true`).
    pub fn border(mut self, border: bool) -> Self {
        self.border = border;
        self
    }

    /// Corner rounding (`Xs`/`Sm` → small, `Md` → medium, larger → large).
    pub fn radius(mut self, radius: Size) -> Self {
        self.radius = Some(radius);
        self
    }
}

container_methods!(Surface);

impl Render for Surface<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let level = match self.level {
            Level::Flat => "flat",
            Level::Raised => "raised",
            Level::Overlay => "overlay",
        };
        let mut el = self.tag.element().class("st-surface").data("level", level);
        if let Some(p) = self.padding {
            el = el.data("padding", p.as_str());
        }
        el = el.bool_attr("data-border", self.border);
        if let Some(r) = self.radius {
            let r = match r {
                Size::Xs | Size::Sm => "sm",
                Size::Md => "md",
                _ => "lg",
            };
            el = el.data("radius", r);
        }
        apply(
            el.children(self.children.iter()),
            &self.attrs,
            Self::RESERVED,
        )
        .render(cx);
    }
}
