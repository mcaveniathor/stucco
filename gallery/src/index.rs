//! The gallery home page.

use stucco::layout::{Container, SkipLink, Stack};
use stucco::typography::{Heading, Link, Text};
use stucco::{Bundle, Page, Size, Space, Tone, el};

const PAGES: [(&str, &str, &str); 5] = [
    (
        "palette.html",
        "Palettes",
        "Every preset's scales and roles, light and dark.",
    ),
    (
        "layout.html",
        "Layout",
        "Stack, Cluster, Grid, Sidebar, Switcher and friends.",
    ),
    (
        "typography.html",
        "Typography",
        "Headings, text, links, code and keys.",
    ),
    (
        "actions.html",
        "Actions",
        "Buttons, button links and icon buttons.",
    ),
    (
        "forms.html",
        "Forms",
        "Fields, controls, validation and binding.",
    ),
];

/// Links to every gallery page.
pub fn index_page(bundle: &Bundle) -> String {
    let links = el::ul().children(PAGES.map(|(href, title, blurb)| {
        el::li().child(
            Stack::new()
                .space(Space::S1)
                .child(Link::new(title, href))
                .child(Text::new(blurb).tone(Tone::Muted).size(Size::Sm)),
        )
    }));
    Page::new(bundle, "stucco gallery")
        .body((
            SkipLink::new(),
            el::main().id("main").child(
                Container::new().class("g-page").child(
                    Stack::new()
                        .space(Space::S6)
                        .child(Heading::new(1, "stucco gallery"))
                        .child(
                            Text::new(
                                "Components, themes and test fixtures for the stucco UI library.",
                            )
                            .tone(Tone::Muted)
                            .size(Size::Lg),
                        )
                        .child(links),
                ),
            ),
        ))
        .render()
}
