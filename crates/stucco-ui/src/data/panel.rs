use crate::{passthrough::apply, typography::Heading};
use stucco_core::{Attrs, Cx, Render, Slot, el};
/// A general-purpose bordered content surface.
#[derive(Debug, Default)]
pub struct Card<'a> {
    attrs: Attrs,
    header: Option<Slot<'a>>,
    body: Vec<Slot<'a>>,
    footer: Option<Slot<'a>>,
}
impl<'a> Card<'a> {
    /// An empty card.
    pub fn new() -> Self {
        Self::default()
    }
    /// Sets its header.
    pub fn header(mut self, r: impl Render + 'a) -> Self {
        self.header = Some(Slot::new(r));
        self
    }
    /// Appends content.
    pub fn child(mut self, r: impl Render + 'a) -> Self {
        self.body.push(Slot::new(r));
        self
    }
    /// Sets its footer.
    pub fn footer(mut self, r: impl Render + 'a) -> Self {
        self.footer = Some(Slot::new(r));
        self
    }
}
passthrough!(Card<'_>);
impl Render for Card<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::DATA);
        apply(
            el::div()
                .class("st-card")
                .child(self.header.as_ref())
                .children(self.body.iter())
                .child(self.footer.as_ref()),
            &self.attrs,
            &[],
        )
        .render(cx);
    }
}
/// A section labelled by its heading.
#[derive(Debug)]
pub struct Panel<'a> {
    attrs: Attrs,
    title: String,
    level: u8,
    description: Option<Slot<'a>>,
    actions: Option<Slot<'a>>,
    body: Option<Slot<'a>>,
    footer: Option<Slot<'a>>,
}
impl<'a> Panel<'a> {
    /// Creates a panel with an h2 heading.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            attrs: Attrs::default(),
            title: title.into(),
            level: 2,
            description: None,
            actions: None,
            body: None,
            footer: None,
        }
    }
    /// Sets the heading level, clamped to 1–6.
    pub fn level(mut self, n: u8) -> Self {
        self.level = n.clamp(1, 6);
        self
    }
    /// Sets explanatory content.
    pub fn description(mut self, r: impl Render + 'a) -> Self {
        self.description = Some(Slot::new(r));
        self
    }
    /// Sets heading actions.
    pub fn actions(mut self, r: impl Render + 'a) -> Self {
        self.actions = Some(Slot::new(r));
        self
    }
    /// Sets main panel content.
    pub fn body(mut self, r: impl Render + 'a) -> Self {
        self.body = Some(Slot::new(r));
        self
    }
    /// Sets footer content.
    pub fn footer(mut self, r: impl Render + 'a) -> Self {
        self.footer = Some(Slot::new(r));
        self
    }
}
passthrough!(Panel<'_>);
impl Render for Panel<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::DATA);
        let id = cx.id("panel");
        apply(
            el::section()
                .class("st-panel")
                .aria("labelledby", &id)
                .child(
                    el::div()
                        .class("st-panel-header")
                        .child(Heading::new(self.level, &self.title).id(id))
                        .child(self.description.as_ref())
                        .child(self.actions.as_ref()),
                )
                .child(self.body.as_ref())
                .child(self.footer.as_ref()),
            &self.attrs,
            &["aria-labelledby"],
        )
        .render(cx);
    }
}
