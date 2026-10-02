use crate::{layout::SkipLink, passthrough::apply};
use stucco_core::{Attrs, Cx, Render, Slot, el};
/// Application structure owning the main landmark and skip link.
/// Footer slots render directly; pass Footer to create that landmark.
#[derive(Debug, Default)]
pub struct AppShell<'a> {
    attrs: Attrs,
    header: Option<Slot<'a>>,
    sidebar: Option<Slot<'a>>,
    main: Option<Slot<'a>>,
    footer: Option<Slot<'a>>,
}
impl<'a> AppShell<'a> {
    /// An empty shell with main#main.
    pub fn new() -> Self {
        Self::default()
    }
    /// Sets top-level header contents.
    pub fn header(mut self, r: impl Render + 'a) -> Self {
        self.header = Some(Slot::new(r));
        self
    }
    /// Sets sidebar contents, inside a native disclosure.
    pub fn sidebar(mut self, r: impl Render + 'a) -> Self {
        self.sidebar = Some(Slot::new(r));
        self
    }
    /// Sets main contents (must not contain another main landmark).
    pub fn main(mut self, r: impl Render + 'a) -> Self {
        self.main = Some(Slot::new(r));
        self
    }
    /// Sets footer slot, without adding a nested footer wrapper.
    pub fn footer(mut self, r: impl Render + 'a) -> Self {
        self.footer = Some(Slot::new(r));
        self
    }
}
passthrough!(AppShell<'_>);
impl Render for AppShell<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::APP);
        let header = self
            .header
            .as_ref()
            .map(|h| el::header().class("st-app-header").child(h));
        let sidebar = self.sidebar.as_ref().map(|s| {
            el::aside().class("st-app-sidebar").child(
                el::details()
                    .bool_attr("open", true)
                    .child(el::summary().text("Navigation"))
                    .child(s),
            )
        });
        apply(
            el::div()
                .class("st-app-shell")
                .child(SkipLink::new())
                .child(header)
                .child(
                    el::div().class("st-app-body").child(sidebar).child(
                        el::main()
                            .id("main")
                            .class("st-app-main")
                            .child(self.main.as_ref()),
                    ),
                )
                .child(self.footer.as_ref()),
            &self.attrs,
            &[],
        )
        .render(cx);
    }
}
