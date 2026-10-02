use crate::{passthrough::apply, typography::Heading};
use stucco_core::{Attrs, Cx, Render, Slot, el};
/// Page identity with optional context and actions.
#[derive(Debug)]
pub struct PageHeader<'a> {
    attrs: Attrs,
    title: String,
    level: u8,
    description: Option<Slot<'a>>,
    actions: Option<Slot<'a>>,
    breadcrumbs: Option<Slot<'a>>,
}
impl<'a> PageHeader<'a> {
    /// Creates an h1 page heading.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            attrs: Attrs::default(),
            title: title.into(),
            level: 1,
            description: None,
            actions: None,
            breadcrumbs: None,
        }
    }
    /// Sets heading level.
    pub fn level(mut self, n: u8) -> Self {
        self.level = n.clamp(1, 6);
        self
    }
    /// Sets descriptive content.
    pub fn description(mut self, r: impl Render + 'a) -> Self {
        self.description = Some(Slot::new(r));
        self
    }
    /// Sets actions.
    pub fn actions(mut self, r: impl Render + 'a) -> Self {
        self.actions = Some(Slot::new(r));
        self
    }
    /// Sets breadcrumbs.
    pub fn breadcrumbs(mut self, r: impl Render + 'a) -> Self {
        self.breadcrumbs = Some(Slot::new(r));
        self
    }
}
passthrough!(PageHeader<'_>);
impl Render for PageHeader<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::APP);
        apply(
            el::div()
                .class("st-page-header")
                .child(self.breadcrumbs.as_ref())
                .child(Heading::new(self.level, &self.title))
                .child(self.description.as_ref())
                .child(self.actions.as_ref()),
            &self.attrs,
            &[],
        )
        .render(cx);
    }
}
/// A section heading using PageHeader's slots and an h2 default.
#[derive(Debug)]
pub struct SectionHeader<'a>(PageHeader<'a>);
impl<'a> SectionHeader<'a> {
    /// Creates an h2 section heading.
    pub fn new(title: impl Into<String>) -> Self {
        Self(PageHeader::new(title).level(2))
    }
    /// Sets explanatory content.
    pub fn description(mut self, r: impl Render + 'a) -> Self {
        self.0 = self.0.description(r);
        self
    }
    /// Sets heading actions.
    pub fn actions(mut self, r: impl Render + 'a) -> Self {
        self.0 = self.0.actions(r);
        self
    }
    /// Sets heading level.
    pub fn level(mut self, n: u8) -> Self {
        self.0 = self.0.level(n);
        self
    }
}
impl Render for SectionHeader<'_> {
    fn render(&self, cx: &mut Cx) {
        self.0.render(cx);
    }
}
