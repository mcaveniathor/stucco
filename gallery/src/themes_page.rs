//! Seeded themes: `Theme::seeded(n)` for a spread of seeds, each in a light
//! and a dark panel with the same sample interface.

use stucco::actions::Button;
use stucco::data::{Card, Row, Table};
use stucco::forms::{Field, Input, Select};
use stucco::layout::{Cluster, Grid, Stack};
use stucco::theme::Theme;
use stucco::typography::{Heading, Text};
use stucco::{Attrs, Bundle, Measure, Render, Size, Space, Tone, Variant, el};

use crate::shell::{component_page, section};

/// The seeds shown, with their named-theme scopes.
pub const SEEDS: [(u64, &str); 12] = [
    (1, "seed-1"),
    (2, "seed-2"),
    (3, "seed-3"),
    (5, "seed-5"),
    (8, "seed-8"),
    (13, "seed-13"),
    (21, "seed-21"),
    (34, "seed-34"),
    (42, "seed-42"),
    (99, "seed-99"),
    (256, "seed-256"),
    (1024, "seed-1024"),
];

/// The built theme for a gallery seed.
pub fn seeded(seed: u64) -> stucco::theme::BuiltTheme {
    Theme::seeded(seed)
        .build()
        .expect("seeded themes always build")
}

/// One section per seed.
pub fn page(bundle: &Bundle) -> String {
    component_page(
        bundle,
        "Seeded themes",
        "Theme::seeded(n) derives colours, fonts, scale, radius, density and a style \
         personality from one number. The same seed always gives the same theme.",
        SEEDS.map(|(seed, name)| {
            section(
                &format!("Theme::seeded({seed})"),
                el::div()
                    .class("g-panels")
                    .attr("data-st-theme", name)
                    .child(sample(seed, "light", "Light"))
                    .child(sample(seed, "dark", "Dark")),
            )
        }),
    )
}

fn sample(seed: u64, scheme: &'static str, label: &'static str) -> impl Render + 'static {
    let orders = [
        ("1042", "Paid", "$48.00"),
        ("1043", "Pending", "$12.50"),
        ("1044", "Shipped", "$230.10"),
    ];
    let mut table = Table::new(format!(
        "Recent orders, seed {seed}, {}",
        label.to_lowercase()
    ))
    .header(
        Row::new()
            .header("Order")
            .header("Status")
            .header_with_attrs("Total", Attrs::default().data("kind", "number")),
    );
    for (id, status, total) in orders {
        table = table.row(
            Row::new()
                .cell(id)
                .cell(el::span().class("st-tag").text(status))
                .cell_with_attrs(total, Attrs::default().data("kind", "number")),
        );
    }
    el::div()
        .class("g-panel")
        .attr("data-theme", scheme)
        .child(
            // The app header's own class, so the header style shows here.
            el::div()
                .class("st-app-header g-sample-header")
                .child(Heading::new(3, label).size(Size::Lg)),
        )
        .child(
            Stack::new()
                .space(Space::S4)
                .child(
                    Stack::new()
                        .space(Space::S1)
                        .child(Text::new("Body text in the theme's sans stack."))
                        .child(Text::new("Muted text for hints and captions.").tone(Tone::Muted)),
                )
                .child(
                    Cluster::new()
                        .space(Space::S2)
                        .child(Button::new("Save changes").variant(Variant::Primary))
                        .child(Button::new("Cancel"))
                        .child(Button::new("Delete").variant(Variant::Danger)),
                )
                .child(
                    Grid::new()
                        .min(Measure::Xs)
                        .space(Space::S3)
                        .child(Field::new(
                            "Name",
                            Input::text("name").value("Ada Lovelace"),
                        ))
                        .child(
                            Field::new("Email", Input::email("email").value("ada@example"))
                                .error("Enter a valid email address."),
                        )
                        .child(Field::new(
                            "Plan",
                            Select::new("plan")
                                .options([("team", "Team"), ("solo", "Solo")])
                                .selected("team"),
                        )),
                )
                .child(Card::new().child(Text::new("A card on the page.").tone(Tone::Muted)))
                .child(table),
        )
}
