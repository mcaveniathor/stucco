use stucco_core::{Attrs, Cx, Render, Slot};

use super::{LAYOUT, Tag};
use crate::Space;
use crate::passthrough::apply;

/// Main-axis distribution of a [`Cluster`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Justify {
    /// Pack at the start.
    Start,
    /// Centre.
    Center,
    /// Pack at the end.
    End,
    /// Spread with space between.
    Between,
}

/// Cross-axis alignment of a [`Cluster`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    /// Top.
    Start,
    /// Middle (default).
    Center,
    /// Bottom.
    End,
    /// Text baselines.
    Baseline,
    /// Fill the row height.
    Stretch,
}

/// Inline items that wrap, with even gaps: tags, buttons, toolbars.
///
/// ```
/// use stucco_ui::layout::{Cluster, Justify};
/// let html = stucco_core::to_html(&Cluster::new().justify(Justify::Between).child("a"));
/// assert!(html.contains(r#"data-justify="between""#));
/// ```
#[derive(Debug, Default)]
pub struct Cluster<'a> {
    attrs: Attrs,
    tag: Tag,
    space: Option<Space>,
    justify: Option<Justify>,
    align: Option<Align>,
    children: Vec<Slot<'a>>,
}

impl Cluster<'_> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-space", "data-justify", "data-align"];

    /// An empty cluster with `Space::S3` gaps.
    pub fn new() -> Self {
        Self::default()
    }

    /// Gap between items.
    pub fn space(mut self, space: Space) -> Self {
        self.space = Some(space);
        self
    }

    /// Main-axis distribution.
    pub fn justify(mut self, justify: Justify) -> Self {
        self.justify = Some(justify);
        self
    }

    /// Cross-axis alignment.
    pub fn align(mut self, align: Align) -> Self {
        self.align = Some(align);
        self
    }
}

container_methods!(Cluster);

impl Render for Cluster<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let mut el = self
            .tag
            .element()
            .class("st-cluster")
            .data("space", self.space.unwrap_or(Space::S3).as_str());
        if let Some(j) = self.justify {
            el = el.data(
                "justify",
                match j {
                    Justify::Start => "start",
                    Justify::Center => "center",
                    Justify::End => "end",
                    Justify::Between => "between",
                },
            );
        }
        if let Some(a) = self.align {
            el = el.data(
                "align",
                match a {
                    Align::Start => "start",
                    Align::Center => "center",
                    Align::End => "end",
                    Align::Baseline => "baseline",
                    Align::Stretch => "stretch",
                },
            );
        }
        apply(
            el.children(self.children.iter()),
            &self.attrs,
            Self::RESERVED,
        )
        .render(cx);
    }
}
