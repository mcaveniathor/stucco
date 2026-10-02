//! The landing page.

use stucco::actions::ButtonLink;
use stucco::data::Card;
use stucco::layout::{Cluster, Grid, Stack};
use stucco::typography::{Heading, Text};
use stucco::{Measure, Size, Space, Tone, Variant, el};

use crate::shell::{Section, Site};

const FEATURES: [(&str, &str); 6] = [
    (
        "Works without JavaScript",
        "Search, filters, sorting, pagination and forms run on ordinary links and form posts. \
         Scripts only enhance pages that already work.",
    ),
    (
        "Accessible by construction",
        "Labels, hints and errors are wired to their controls, text is escaped, URLs are checked, \
         and every theme passes WCAG contrast checks before it builds.",
    ),
    (
        "Themes from a single seed",
        "Fourteen presets, a theme builder, and Theme::seeded(n) for a complete, contrast-checked \
         theme from one number. Every option can be seeded on its own.",
    ),
    (
        "Typed collections",
        "Declare columns once to get a table, search, typed filters, sorting and cursor or \
         numbered pagination, all validated on the server.",
    ),
    (
        "Forms that validate",
        "Parse a submission, check it with Validator, and re-render the form with every value \
         kept and every error linked from a summary.",
    ),
    (
        "Fits your server",
        "Components render plain HTML. stucco-tower adds Axum responses, asset serving and \
         standard middleware, and stucco-redb adds embedded storage.",
    ),
];

const EXAMPLE: &str = r#"use stucco::prelude::*;
use stucco::actions::Button;
use stucco::layout::{Container, Stack};
use stucco::typography::{Heading, Text};

let bundle = Bundle::new(Theme::seeded(42).build()?);
let content = Container::new().child(
    Stack::new()
        .child(Heading::new(1, "Hello, stucco"))
        .child(Text::new("A page rendered entirely in Rust."))
        .child(Button::new("Continue")),
);
let html = Page::new(&bundle, "Hello, stucco")
    .body(el::main().id("main").child(content))
    .render();"#;

/// The landing page.
pub fn page(site: &Site) -> String {
    let hero = el::section()
        .class("site-hero")
        .aria("labelledby", "hero-title")
        .child(
            Stack::new()
                .space(Space::S6)
                .child(Heading::new(1, "Server-rendered UI for Rust").id("hero-title").size(Size::Xl3))
                .child(
                    Text::new(
                        "stucco builds accessible, themeable interfaces from semantic HTML. Compose \
                         layouts, forms, tables and application shells with ordinary Rust builders.",
                    )
                    .size(Size::Lg)
                    .tone(Tone::Muted),
                )
                .child(
                    Cluster::new()
                        .space(Space::S3)
                        .child(
                            ButtonLink::new("Get started", site.url("guide/getting-started.html"))
                                .variant(Variant::Primary),
                        )
                        .child(ButtonLink::new("Try the theme playground", site.url("playground.html")))
                        .child(
                            ButtonLink::new("Browse components", site.url("gallery/"))
                                .variant(Variant::Ghost),
                        ),
                )
                .child(
                    el::pre()
                        .class("site-install")
                        .child(el::code().text("cargo add stucco")),
                ),
        );
    let features = el::section()
        .class("site-band")
        .aria("labelledby", "features-title")
        .child(
            Stack::new()
                .space(Space::S6)
                .child(Heading::new(2, "What you get").id("features-title"))
                .child(
                    Grid::new()
                        .min(Measure::Xs)
                        .space(Space::S4)
                        .children(FEATURES.map(|(title, body)| {
                            Card::new().child(
                                Stack::new()
                                    .space(Space::S2)
                                    .child(Heading::new(3, title).size(Size::Lg))
                                    .child(Text::new(body).tone(Tone::Muted)),
                            )
                        })),
                ),
        );
    let example = el::section()
        .class("site-band")
        .aria("labelledby", "example-title")
        .child(
            Stack::new()
                .space(Space::S4)
                .child(Heading::new(2, "A page in a few lines").id("example-title"))
                .child(
                    Text::new(
                        "Components implement Render. A Bundle supplies hashed, themed assets and a \
                         Page produces the document. Serve it with any framework.",
                    )
                    .tone(Tone::Muted),
                )
                .child(
                    el::pre()
                        .class("site-code")
                        .child(el::code().class("language-rust").text(EXAMPLE)),
                )
                .child(
                    Cluster::new()
                        .space(Space::S3)
                        .child(ButtonLink::new("Read the guide", site.url("guide/")))
                        .child(
                            ButtonLink::new("API reference", site.url("api/stucco/"))
                                .variant(Variant::Ghost),
                        ),
                ),
        );
    let themes = el::section()
        .class("site-band site-band-tinted")
        .aria("labelledby", "themes-title")
        .child(
            Stack::new()
                .space(Space::S4)
                .child(Heading::new(2, "Make it yours").id("themes-title"))
                .child(
                    Text::new(
                        "Pick a preset from the Theme menu, roll a random theme, or open the \
                         playground to adjust every option and export Rust, CSS or JSON.",
                    )
                    .tone(Tone::Muted),
                )
                .child(
                    Cluster::new()
                        .space(Space::S3)
                        .child(
                            ButtonLink::new("Open the playground", site.url("playground.html"))
                                .variant(Variant::Primary),
                        )
                        .child(ButtonLink::new(
                            "See seeded themes",
                            site.url("gallery/themes.html"),
                        )),
                ),
        );
    site.page(
        "stucco",
        "Server-rendered, themeable, accessible UI components for Rust.",
        Section::Home,
        None,
        el::div()
            .class("site-landing")
            .child(hero)
            .child(features)
            .child(example)
            .child(themes),
    )
}
