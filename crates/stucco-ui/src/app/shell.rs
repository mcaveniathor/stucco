use crate::navigation::NavLink;
use crate::{layout::SkipLink, passthrough::apply};
use stucco_core::{Attrs, Cx, Render, Slot, el};

/// Application structure owning the main landmark and skip link.
/// Footer slots render directly; pass Footer to create that landmark.
///
/// Navigation comes in two kinds. [`AppShell::link`] adds a flat list of
/// top-level links, the primary navigation. [`AppShell::sidebar`] holds
/// anything else, such as section links or a nested outline, inside a
/// disclosure. The theme's `ShellLayout` places them: the links at the top
/// of the side column, or in a row under the header, with the sidebar
/// always a column beside the content.
///
/// ```
/// use stucco_ui::app::AppShell;
/// use stucco_ui::navigation::NavLink;
/// use stucco_core::to_html;
/// let html = to_html(
///     &AppShell::new()
///         .link(NavLink::new("Orders", "/orders").current(true))
///         .link(NavLink::new("Customers", "/customers"))
///         .main("…"),
/// );
/// assert!(html.contains(r#"<nav class="st-app-nav" aria-label="Main">"#));
/// assert!(html.contains(r#"aria-current="page""#));
/// ```
#[derive(Debug)]
pub struct AppShell<'a> {
    attrs: Attrs,
    header: Option<Slot<'a>>,
    links: Vec<NavLink>,
    nav_label: String,
    sidebar: Option<Slot<'a>>,
    main: Option<Slot<'a>>,
    footer: Option<Slot<'a>>,
}

impl Default for AppShell<'_> {
    fn default() -> Self {
        AppShell {
            attrs: Attrs::default(),
            header: None,
            links: Vec::new(),
            nav_label: "Main".into(),
            sidebar: None,
            main: None,
            footer: None,
        }
    }
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
    /// Adds a link to the primary navigation.
    pub fn link(mut self, link: NavLink) -> Self {
        self.links.push(link);
        self
    }
    /// Adds links to the primary navigation.
    pub fn links(mut self, links: impl IntoIterator<Item = NavLink>) -> Self {
        self.links.extend(links);
        self
    }
    /// Names the primary navigation landmark (default "Main").
    pub fn nav_label(mut self, label: impl Into<String>) -> Self {
        self.nav_label = label.into();
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
        let nav = (!self.links.is_empty()).then(|| {
            el::nav()
                .class("st-app-nav")
                .aria("label", &self.nav_label)
                .children(self.links.iter())
        });
        let sidebar = self.sidebar.as_ref().map(|s| {
            el::aside().class("st-app-sidebar").child(
                el::details()
                    .bool_attr("open", true)
                    .child(el::summary().text("Navigation"))
                    .child(s),
            )
        });
        // The body's grid areas depend on which regions are present.
        let body = el::div()
            .class("st-app-body")
            .bool_attr("data-nav", nav.is_some())
            .bool_attr("data-sidebar", sidebar.is_some());
        apply(
            el::div()
                .class("st-app-shell")
                .child(SkipLink::new())
                .child(header)
                .child(
                    body.child(nav).child(sidebar).child(
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
