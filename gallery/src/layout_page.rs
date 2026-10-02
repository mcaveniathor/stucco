//! Layout primitives.

use stucco::layout::{
    Center, Cluster, Grid, Justify, Level, Separator, Sidebar, Stack, Surface, Switcher, ThemeScope,
};
use stucco::typography::Text;
use stucco::{Bundle, ColorScheme, Measure, Render, Space, Tone};

use crate::shell::{component_page, section};

fn tile(label: &str) -> Surface<'static> {
    Surface::new()
        .level(Level::Raised)
        .padding(Space::S3)
        .child(Text::new(label.to_owned()).inline())
}

/// The layout page.
pub fn page(bundle: &Bundle) -> String {
    let sections: Vec<Box<dyn Render>> = vec![
        section(
            "Stack",
            Stack::new()
                .space(Space::S3)
                .children(["First", "Second", "Third"].map(tile)),
        ),
        section(
            "Cluster",
            Cluster::new()
                .justify(Justify::Start)
                .children(["Rust", "HTML", "CSS", "Accessibility", "Themes"].map(tile)),
        ),
        section(
            "Grid",
            Grid::new()
                .min(Measure::Xs)
                .space(Space::S3)
                .children(["One", "Two", "Three", "Four", "Five", "Six"].map(tile)),
        ),
        section(
            "Sidebar",
            Sidebar::new(
                tile("Side panel"),
                tile("Main content stretches to fill the rest"),
            ),
        ),
        section(
            "Switcher",
            Switcher::new()
                .threshold(Measure::Md)
                .children(["Left", "Middle", "Right"].map(tile)),
        ),
        section(
            "Center",
            Center::new()
                .max(Measure::Sm)
                .child(tile("Centred within 30rem")),
        ),
        section(
            "Container",
            Text::new(
                "This page sits in a Container: centred, with gutters that grow with the viewport.",
            )
            .tone(Tone::Muted),
        ),
        section(
            "Separator",
            Stack::new()
                .child(Text::new("Above"))
                .child(Separator::new())
                .child(Text::new("Below")),
        ),
        section(
            "Surface and ThemeScope",
            Cluster::new()
                .space(Space::S4)
                .child(tile("Raised surface"))
                .child(
                    ThemeScope::new()
                        .scheme(ColorScheme::Dark)
                        .child(tile("Forced dark scheme")),
                )
                .child(
                    ThemeScope::new()
                        .scheme(ColorScheme::Light)
                        .named("iris")
                        .child(tile("Iris theme, light")),
                ),
        ),
    ];
    component_page(
        bundle,
        "Layout",
        "Composable primitives that are responsive on their own, with no inline styles.",
        sections,
    )
}
