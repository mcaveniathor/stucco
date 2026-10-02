//! Small structural primitives: Separator, VisuallyHidden and SkipLink.

use stucco_core::{Attrs, Cx, Href, Render, Slot, el};

use super::LAYOUT;
use crate::passthrough::apply;

/// A thematic break (`<hr>`), or a purely visual divider.
///
/// ```
/// use stucco_ui::layout::Separator;
/// assert_eq!(stucco_core::to_html(&Separator::new()), r#"<hr class="st-separator">"#);
/// ```
#[derive(Debug, Default, Clone)]
pub struct Separator {
    attrs: Attrs,
    decorative: bool,
}

impl Separator {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["role"];

    /// A thematic break.
    pub fn new() -> Self {
        Self::default()
    }

    /// A visual divider with no meaning (hidden from assistive technology).
    pub fn decorative(mut self) -> Self {
        self.decorative = true;
        self
    }
}

passthrough!(Separator);

impl Render for Separator {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        if self.decorative {
            let el = el::div().class("st-separator").attr("role", "none");
            apply(el, &self.attrs, Self::RESERVED).render(cx);
        } else {
            el::hr()
                .class("st-separator")
                .attrs(&self.attrs.without(Self::RESERVED))
                .render(cx);
        }
    }
}

/// Content read by screen readers but not shown.
///
/// ```
/// use stucco_ui::layout::VisuallyHidden;
/// let html = stucco_core::to_html(&VisuallyHidden::new("(opens in a new tab)"));
/// assert!(html.starts_with(r#"<span class="st-sr-only">"#));
/// ```
#[derive(Debug)]
pub struct VisuallyHidden<'a> {
    attrs: Attrs,
    content: Slot<'a>,
}

impl<'a> VisuallyHidden<'a> {
    /// Hides `content` visually.
    pub fn new(content: impl Render + 'a) -> Self {
        VisuallyHidden {
            attrs: Attrs::default(),
            content: Slot::new(content),
        }
    }
}

passthrough!(VisuallyHidden<'_>);

impl Render for VisuallyHidden<'_> {
    fn render(&self, cx: &mut Cx) {
        let el = el::span().class("st-sr-only").child(&self.content);
        apply(el, &self.attrs, &[]).render(cx);
    }
}

/// A link to the main content, shown when focused; put it first in `<body>`.
///
/// ```
/// use stucco_ui::layout::SkipLink;
/// assert!(stucco_core::to_html(&SkipLink::new()).contains(r##"href="#main""##));
/// ```
#[derive(Debug, Clone)]
pub struct SkipLink {
    attrs: Attrs,
    target: String,
    text: String,
}

impl Default for SkipLink {
    fn default() -> Self {
        SkipLink {
            attrs: Attrs::default(),
            target: "#main".to_owned(),
            text: "Skip to main content".to_owned(),
        }
    }
}

impl SkipLink {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["href"];

    /// A skip link to `#main`.
    pub fn new() -> Self {
        Self::default()
    }

    /// The fragment to skip to (default `#main`).
    pub fn target(mut self, target: &str) -> Self {
        self.target = target.to_owned();
        self
    }

    /// The link text.
    pub fn text(mut self, text: &str) -> Self {
        self.text = text.to_owned();
        self
    }
}

passthrough!(SkipLink);

impl Render for SkipLink {
    fn render(&self, cx: &mut Cx) {
        cx.require(&LAYOUT);
        let el = el::a()
            .class("st-skip-link")
            .href(Href::new(self.target.clone()))
            .text(self.text.clone());
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}
