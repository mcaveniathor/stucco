//! The shared frame of component pages: skip link, main landmark, container.

use stucco::layout::{Container, SkipLink, Stack};
use stucco::typography::{Heading, Link, Text};
use stucco::{Bundle, Page, Render, Size, Space, Tone, el};

/// A component page titled `title`, introduced by `lead`, with `sections`.
pub fn component_page<'a>(
    bundle: &Bundle,
    title: &str,
    lead: &str,
    sections: impl IntoIterator<Item = Box<dyn Render + 'a>>,
) -> String {
    let content = Stack::new()
        .space(Space::S10)
        .child(
            Stack::new()
                .space(Space::S3)
                .child(Link::new("← Gallery", "index.html"))
                .child(Heading::new(1, title.to_owned()))
                .child(Text::new(lead.to_owned()).tone(Tone::Muted).size(Size::Lg)),
        )
        .children(sections);
    Page::new(bundle, format!("{title} — stucco gallery"))
        .body((
            SkipLink::new(),
            el::main()
                .id("main")
                .child(Container::new().class("g-page").child(content)),
        ))
        .render()
}

/// A titled section of a component page.
pub fn section<'a>(title: &str, body: impl Render + 'a) -> Box<dyn Render + 'a> {
    Box::new(
        el::section().child(
            Stack::new()
                .space(Space::S4)
                .child(Heading::new(2, title.to_owned()))
                .child(body),
        ),
    )
}
