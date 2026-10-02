use stucco_core::{Attrs, Cx, Render, Slot, el};

use super::LAYOUT;
use crate::passthrough::apply;
use crate::{Measure, Space};

/// A fixed-width side panel beside a fluid main area; stacks when the main
/// area would be narrower than half the container.
///
/// ```
/// use stucco_ui::layout::Sidebar;
/// let html = stucco_core::to_html(&Sidebar::new("nav", "content"));
/// assert!(html.contains(r#"data-side="left""#));
/// ```
#[derive(Debug)]
pub struct Sidebar<'a> {
    attrs: Attrs,
    side: Slot<'a>,
    main: Slot<'a>,
    right: bool,
    side_width: Measure,
    space: Space,
}

impl<'a> Sidebar<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-side", "data-side-width", "data-space"];

    /// `side` beside `main` (side on the left by default).
    pub fn new(side: impl Render + 'a, main: impl Render + 'a) -> Self {
        Sidebar {
            attrs: Attrs::default(),
            side: Slot::new(side),
            main: Slot::new(main),
            right: false,
            side_width: Measure::Xs,
            space: Space::S4,
        }
    }

    /// Shows the side panel on the right (after the main area in reading
    /// order, matching what is seen).
    pub fn right(mut self) -> Self {
        self.right = true;
        self
    }

    /// Width of the side panel.
    pub fn side_width(mut self, width: Measure) -> Self {
        self.side_width = width;
        self
    }

    /// Gap between the panels.
    pub fn space(mut self, space: Space) -> Self {
        self.space = space;
        self
    }
}

passthrough!(Sidebar<'_>);

impl Render for Sidebar<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let el = el::div()
            .class("st-sidebar")
            .data("side", if self.right { "right" } else { "left" })
            .data("side-width", self.side_width.as_str())
            .data("space", self.space.as_str());
        let side = el::div().class("st-sidebar-side").child(&self.side);
        let main = el::div().class("st-sidebar-main").child(&self.main);
        // DOM order follows visual order so reading and focus order match.
        let el = if self.right {
            el.child(main).child(side)
        } else {
            el.child(side).child(main)
        };
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}
