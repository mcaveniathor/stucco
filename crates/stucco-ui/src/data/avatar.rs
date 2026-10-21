use super::DATA;
use crate::{Size, passthrough::apply};
use stucco_core::{Attrs, Cx, Href, Render, el};

/// A person's picture, or their initials when there is none.
///
/// It is labelled with the name. Use [`decorative`](Self::decorative) when
/// the name is written beside it, so screen readers don't hear it twice. A
/// picture that fails to load shows the initials underneath.
///
/// ```
/// use stucco_ui::data::Avatar;
/// let html = stucco_core::to_html(&Avatar::new("Ada Lovelace"));
/// assert!(html.contains(">AL</span>"));
/// ```
#[derive(Debug)]
pub struct Avatar {
    attrs: Attrs,
    name: String,
    image: Option<Href>,
    size: &'static str,
    decorative: bool,
}

impl Avatar {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] =
        &["role", "aria-label", "aria-hidden", "data-size"];

    /// Initials for `name`, labelled with it.
    pub fn new(name: impl Into<String>) -> Self {
        Avatar {
            attrs: Attrs::default(),
            name: name.into(),
            image: None,
            size: "md",
            decorative: false,
        }
    }

    /// A picture shown over the initials.
    pub fn image(mut self, src: impl Into<Href>) -> Self {
        self.image = Some(src.into());
        self
    }

    /// Small, medium (default) or large; other sizes use the nearest.
    pub fn size(mut self, size: Size) -> Self {
        self.size = match size {
            Size::Xs | Size::Sm => "sm",
            Size::Md => "md",
            _ => "lg",
        };
        self
    }

    /// Hides the avatar from assistive technology.
    pub fn decorative(mut self) -> Self {
        self.decorative = true;
        self
    }
}

passthrough!(Avatar);

/// The first character of the first and last words, uppercased.
pub(crate) fn initials(name: &str) -> String {
    let mut words = name.split_whitespace();
    let first = words.next().and_then(|w| w.chars().next());
    let last = words.last().and_then(|w| w.chars().next());
    first
        .into_iter()
        .chain(last)
        .flat_map(char::to_uppercase)
        .collect()
}

impl Render for Avatar {
    fn render(&self, cx: &mut Cx) {
        cx.require(&DATA);
        let mut root = el::span().class("st-avatar").data("size", self.size).child(
            el::span()
                .class("st-avatar-initials")
                .aria("hidden", "true")
                .text(initials(&self.name)),
        );
        if self.decorative {
            root = root.aria("hidden", "true");
        } else if self.image.is_none() {
            root = root.attr("role", "img").aria("label", self.name.clone());
        }
        if let Some(src) = &self.image {
            let alt = if self.decorative {
                String::new()
            } else {
                self.name.clone()
            };
            root = root.child(el::img().src(src.clone()).attr("alt", alt));
        }
        apply(root, &self.attrs, Self::RESERVED).render(cx);
    }
}
