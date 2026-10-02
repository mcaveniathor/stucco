//! Typography (feature `typography`): headings, text, links, inline code and
//! keyboard keys.

use stucco_core::{Asset, Attrs, Cx, Href, Render, Slot, el, register_asset};

use crate::passthrough::apply;
use crate::{Size, Tone};

#[cfg(test)]
mod tests;

/// The typography family's stylesheet.
pub static TYPOGRAPHY: Asset = Asset {
    name: "st-typography",
    css: Some(include_str!("../../css/typography.css")),
    behavior: None,
    deps: &[],
};
register_asset!(TYPOGRAPHY);

/// A heading whose semantic level (1–6) and visual size are independent.
///
/// ```
/// use stucco_ui::{Size, typography::Heading};
/// let html = stucco_core::to_html(&Heading::new(2, "Orders").size(Size::Xl));
/// assert_eq!(html, r#"<h2 class="st-heading" data-size="xl">Orders</h2>"#);
/// ```
#[derive(Debug)]
pub struct Heading<'a> {
    attrs: Attrs,
    level: u8,
    size: Option<Size>,
    content: Slot<'a>,
}

impl<'a> Heading<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-size"];

    /// A level-`level` heading (debug panic "heading level" outside 1–6;
    /// clamped in release).
    pub fn new(level: u8, content: impl Render + 'a) -> Self {
        debug_assert!(
            (1..=6).contains(&level),
            "heading level must be 1–6, got {level}"
        );
        Heading {
            attrs: Attrs::default(),
            level: level.clamp(1, 6),
            size: None,
            content: Slot::new(content),
        }
    }

    /// Visual size (default: 1→3xl, 2→2xl, 3→xl, 4→lg, 5→md, 6→sm).
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }
}

passthrough!(Heading<'_>);

impl Render for Heading<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&TYPOGRAPHY);
        let default = [Size::Xl3, Size::Xl2, Size::Xl, Size::Lg, Size::Md, Size::Sm];
        let size = self.size.unwrap_or(default[usize::from(self.level) - 1]);
        let el = match self.level {
            1 => el::h1(),
            2 => el::h2(),
            3 => el::h3(),
            4 => el::h4(),
            5 => el::h5(),
            _ => el::h6(),
        }
        .class("st-heading")
        .data("size", size.as_str())
        .child(&self.content);
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}

/// Font weight for [`Text`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weight {
    /// Regular.
    Normal,
    /// Medium.
    Medium,
    /// Bold.
    Bold,
}

/// A paragraph (or inline span) of styled text.
///
/// ```
/// use stucco_ui::{Tone, typography::Text};
/// let html = stucco_core::to_html(&Text::new("Saved").tone(Tone::Success));
/// assert!(html.contains(r#"data-tone="success""#));
/// ```
#[derive(Debug)]
pub struct Text<'a> {
    attrs: Attrs,
    content: Slot<'a>,
    size: Size,
    tone: Tone,
    weight: Option<Weight>,
    inline: bool,
}

impl<'a> Text<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["data-size", "data-tone", "data-weight"];

    /// Body-size text in a `<p>`.
    pub fn new(content: impl Render + 'a) -> Self {
        Text {
            attrs: Attrs::default(),
            content: Slot::new(content),
            size: Size::Md,
            tone: Tone::Default,
            weight: None,
            inline: false,
        }
    }

    /// Size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Colour.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }

    /// Weight.
    pub fn weight(mut self, weight: Weight) -> Self {
        self.weight = Some(weight);
        self
    }

    /// Renders a `<span>` instead of a `<p>`.
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }
}

passthrough!(Text<'_>);

impl Render for Text<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&TYPOGRAPHY);
        let mut el = if self.inline { el::span() } else { el::p() }
            .class("st-text")
            .data("size", self.size.as_str());
        if self.tone != Tone::Default {
            el = el.data("tone", self.tone.as_str());
        }
        if let Some(w) = self.weight {
            let w = match w {
                Weight::Normal => "normal",
                Weight::Medium => "medium",
                Weight::Bold => "bold",
            };
            el = el.data("weight", w);
        }
        apply(el.child(&self.content), &self.attrs, Self::RESERVED).render(cx);
    }
}

/// A styled link. External links open in a new tab safely and say so to
/// screen readers.
///
/// ```
/// use stucco_ui::typography::Link;
/// let html = stucco_core::to_html(&Link::new("Docs", "/docs"));
/// assert_eq!(html, r#"<a class="st-link" href="/docs">Docs</a>"#);
/// ```
#[derive(Debug)]
pub struct Link<'a> {
    attrs: Attrs,
    content: Slot<'a>,
    href: Href,
    external: bool,
}

impl<'a> Link<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["href", "target", "rel"];

    /// A link to `href` (unsafe URLs render as `#`).
    pub fn new(content: impl Render + 'a, href: impl Into<Href>) -> Self {
        Link {
            attrs: Attrs::default(),
            content: Slot::new(content),
            href: href.into(),
            external: false,
        }
    }

    /// Opens in a new tab (`rel="noopener noreferrer"`), announced to screen
    /// readers.
    pub fn external(mut self) -> Self {
        self.external = true;
        self
    }
}

passthrough!(Link<'_>);

impl Render for Link<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&TYPOGRAPHY);
        let mut el = el::a()
            .class("st-link")
            .href(self.href.clone())
            .child(&self.content);
        if self.external {
            el = el
                .attr("target", "_blank")
                .attr("rel", "noopener noreferrer")
                .child(el::span().class("st-sr-only").text(" (opens in a new tab)"));
        }
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}

/// Inline code.
///
/// ```
/// use stucco_ui::typography::Code;
/// assert_eq!(stucco_core::to_html(&Code::new("cargo run")), r#"<code class="st-code">cargo run</code>"#);
/// ```
#[derive(Debug, Clone)]
pub struct Code {
    attrs: Attrs,
    text: String,
}

impl Code {
    /// `text` as inline code.
    pub fn new(text: impl Into<String>) -> Self {
        Code {
            attrs: Attrs::default(),
            text: text.into(),
        }
    }
}

passthrough!(Code);

impl Render for Code {
    fn render(&self, cx: &mut Cx) {
        cx.require(&TYPOGRAPHY);
        let el = el::code().class("st-code").text(self.text.clone());
        apply(el, &self.attrs, &[]).render(cx);
    }
}

/// A keyboard shortcut: keys joined with `+`.
///
/// ```
/// use stucco_ui::typography::Kbd;
/// assert!(stucco_core::to_html(&Kbd::new(&["Ctrl", "K"])).contains("<kbd>Ctrl</kbd>+<kbd>K</kbd>"));
/// ```
#[derive(Debug, Clone)]
pub struct Kbd {
    attrs: Attrs,
    keys: Vec<String>,
}

impl Kbd {
    /// The keys pressed together.
    pub fn new(keys: &[&str]) -> Self {
        Kbd {
            attrs: Attrs::default(),
            keys: keys.iter().map(|k| (*k).to_owned()).collect(),
        }
    }
}

passthrough!(Kbd);

impl Render for Kbd {
    fn render(&self, cx: &mut Cx) {
        cx.require(&TYPOGRAPHY);
        let mut el = el::kbd().class("st-kbd");
        for (i, key) in self.keys.iter().enumerate() {
            if i > 0 {
                el = el.text("+");
            }
            el = el.child(el::kbd().text(key.clone()));
        }
        apply(el, &self.attrs, &[]).render(cx);
    }
}
