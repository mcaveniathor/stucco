//! Complete application compositions backed by shared, deterministic fixtures.
use stucco::prelude::*;
use stucco::{ColorScheme, layout::ThemeScope, theme::Direction};

/// Atelier orders, with links to every screen and direction.
pub fn page(bundle: &Bundle) -> String {
    screen_page(bundle, Direction::Atelier, "orders")
}

/// One screen in both schemes, rendered completely before any script loads.
pub fn screen_page(bundle: &Bundle, direction: Direction, screen: &str) -> String {
    let nav = el::nav()
        .aria("label", "Showcase directions")
        .class("gallery-showcase-nav")
        .children(
            Direction::ALL
                .map(|d| Link::new(d.name(), format!("showcase-{}-{screen}.html", d.name()))),
        );
    let screens = el::nav()
        .aria("label", "Showcase screens")
        .class("gallery-showcase-nav")
        .children(
            ["orders", "editorial", "project"]
                .map(|s| Link::new(s, format!("showcase-{}-{s}.html", direction.name()))),
        );
    let preview = |scheme, label| {
        ThemeScope::new()
            .named(direction.name())
            .scheme(scheme)
            .child(stucco_showcase::screen(screen, "#recipe", label))
    };
    crate::shell::component_page(
        bundle,
        &format!("{} / {screen}", direction.name()),
        "One composition, seven directions. All data is illustrative; both schemes are rendered in Rust.",
        [
            crate::shell::section("Explore", (nav, screens)),
            crate::shell::section(
                "Light",
                preview(ColorScheme::Light, "Illustrative orders in light"),
            ),
            crate::shell::section(
                "Dark",
                preview(ColorScheme::Dark, "Illustrative orders in dark"),
            ),
            crate::shell::section(
                "Build this screen",
                el::div()
                    .id("recipe")
                    .child(el::pre().text(format!("Direction::{direction:?}.theme()")))
                    .child(Link::new(
                        "Read the copyable screen recipes",
                        "https://github.com/mcaveniathor/stucco/tree/main/examples/showcase",
                    )),
            ),
        ],
    )
}

/// Opt-in typography demonstration; all ordinary pages retain system fonts.
#[cfg(feature = "atelier-fonts")]
pub fn fonts_page(bundle: &Bundle) -> String {
    crate::shell::component_page(
        bundle,
        "Optional Atelier fonts",
        "Locally bundled Newsreader and DM Sans; system fonts remain the fallback.",
        [crate::shell::section(
            "Reading specimen",
            ThemeScope::new()
                .named("atelier")
                .child(stucco_showcase::with_atelier_fonts(
                    stucco_showcase::editorial_preview("showcase.html#recipe"),
                )),
        )],
    )
}
