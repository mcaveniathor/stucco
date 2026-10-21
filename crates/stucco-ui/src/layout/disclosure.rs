use super::LAYOUT;
use crate::{Icon, passthrough::apply};
use stucco_core::{Attrs, Cx, Render, Slot, el};

// Lucide `chevron-down` (ISC; see LICENSE-LUCIDE).
const CHEVRON_DOWN: Icon = Icon::custom("chevron-down", r#"<path d="m6 9 6 6 6-6" />"#);

/// A section the user can show and hide: a native `<details>`.
///
/// The browser handles the keyboard and the open state, and opens it when
/// find-in-page matches its content.
///
/// ```
/// use stucco_ui::layout::Disclosure;
/// let html = stucco_core::to_html(&Disclosure::new("Shipping").child("By courier"));
/// assert!(html.starts_with(r#"<details class="st-disclosure">"#));
/// ```
#[derive(Debug)]
pub struct Disclosure<'a> {
    attrs: Attrs,
    summary: String,
    children: Vec<Slot<'a>>,
    open: bool,
    heading: Option<u8>,
    name: Option<String>,
}

impl<'a> Disclosure<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["open", "name"];

    /// A closed section headed `summary`.
    pub fn new(summary: impl Into<String>) -> Self {
        Disclosure {
            attrs: Attrs::default(),
            summary: summary.into(),
            children: Vec::new(),
            open: false,
            heading: None,
            name: None,
        }
    }

    /// Adds content shown when open.
    pub fn child(mut self, child: impl Render + 'a) -> Self {
        self.children.push(Slot::new(child));
        self
    }

    /// Starts open.
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Wraps the summary in a heading (2–6), so heading navigation finds it.
    pub fn heading(mut self, level: u8) -> Self {
        self.heading = Some(level.clamp(2, 6));
        self
    }
}

passthrough!(Disclosure<'_>);

fn heading(level: u8) -> stucco_core::el::Element<'static> {
    match level {
        2 => el::h2(),
        3 => el::h3(),
        4 => el::h4(),
        5 => el::h5(),
        _ => el::h6(),
    }
}

impl Render for Disclosure<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let label = match self.heading {
            Some(level) => heading(level).text(self.summary.clone()),
            None => el::span().text(self.summary.clone()),
        };
        let mut details = el::details().class("st-disclosure");
        if let Some(name) = &self.name {
            details = details.attr("name", name.clone());
        }
        let details = details
            .bool_attr("open", self.open)
            .child(el::summary().child(label).child(CHEVRON_DOWN))
            .child(
                el::div()
                    .class("st-disclosure-body")
                    .children(&self.children),
            );
        apply(details, &self.attrs, Self::RESERVED).render(cx);
    }
}

/// A stack of [`Disclosure`] sections. With [`exclusive`](Self::exclusive),
/// opening one closes the others, natively.
///
/// ```
/// use stucco_ui::layout::Accordion;
/// let faq = Accordion::new().item("Shipping", "By courier").item("Returns", "30 days");
/// assert_eq!(stucco_core::to_html(&faq).matches("<details").count(), 2);
/// ```
#[derive(Debug, Default)]
pub struct Accordion<'a> {
    attrs: Attrs,
    items: Vec<(String, Slot<'a>, bool)>,
    exclusive: bool,
    heading: Option<u8>,
}

impl<'a> Accordion<'a> {
    /// An empty accordion.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a closed section.
    pub fn item(mut self, summary: impl Into<String>, content: impl Render + 'a) -> Self {
        self.items.push((summary.into(), Slot::new(content), false));
        self
    }

    /// Adds a section that starts open.
    pub fn item_open(mut self, summary: impl Into<String>, content: impl Render + 'a) -> Self {
        self.items.push((summary.into(), Slot::new(content), true));
        self
    }

    /// At most one section open at a time.
    pub fn exclusive(mut self) -> Self {
        self.exclusive = true;
        self
    }

    /// Wraps every summary in a heading (2–6).
    pub fn heading(mut self, level: u8) -> Self {
        self.heading = Some(level.clamp(2, 6));
        self
    }
}

passthrough!(Accordion<'_>);

impl Render for Accordion<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let name = self.exclusive.then(|| cx.id("accordion"));
        let items = self
            .items
            .iter()
            .map(|(summary, content, open)| Disclosure {
                attrs: Attrs::default(),
                summary: summary.clone(),
                children: vec![Slot::new(content)],
                open: *open,
                heading: self.heading,
                name: name.clone(),
            });
        let root = el::div().class("st-accordion").children(items);
        apply(root, &self.attrs, &[]).render(cx);
    }
}
