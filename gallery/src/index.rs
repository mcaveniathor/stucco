//! The gallery home page.

use stucco::data::Card;
use stucco::layout::{Container, Grid, SkipLink, Stack};
use stucco::typography::{Heading, Link, Text};
use stucco::{Bundle, Measure, Page, Size, Space, Tone, el};

const PAGES: [(&str, &str, &str); 11] = [
    (
        "collections.html",
        "Data and collections",
        "Tables, panels and server collection controls.",
    ),
    (
        "app.html",
        "Application shells",
        "Page headings, navigation and footer composition.",
    ),
    (
        "palette.html",
        "Palettes",
        "Every preset's scales and roles, light and dark.",
    ),
    (
        "themes.html",
        "Seeded themes",
        "Whole themes generated from a single number.",
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
    (
        "records.html",
        "Records and feedback",
        "Notices, badges, properties, history and confirmations.",
    ),
    (
        "overlays.html",
        "Overlays",
        "Dialogs, menus, tooltips and toasts.",
    ),
    (
        "tabs.html",
        "Tabs",
        "Sections of a page, each with its own URL.",
    ),
];

/// Links to every gallery page.
pub fn index_page(bundle: &Bundle) -> String {
    let links = Grid::new()
        .min(Measure::Xs)
        .space(Space::S4)
        .children(PAGES.map(|(href, title, blurb)| {
            Card::new().class("g-card").child(
                Stack::new()
                    .space(Space::S1)
                    .child(
                        Heading::new(2, Link::new(title, href).class("g-card-link")).size(Size::Lg),
                    )
                    .child(Text::new(blurb).tone(Tone::Muted)),
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
