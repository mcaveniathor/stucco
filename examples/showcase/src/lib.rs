//! Copyable presentation recipes shared by the gallery, site and orders example.
//! This workspace-only crate contains no HTTP, storage or application queries.

use stucco::prelude::*;
use stucco::theme::{Direction, Theme};
use stucco::{Asset, register_asset};

/// Styles for the showcase compositions, kept separate from library defaults.
pub static SHOWCASE: Asset = Asset {
    name: "showcase",
    css: Some(include_str!("../css/showcase.css")),
    behavior: None,
    deps: &[],
};
register_asset!(SHOWCASE);

/// Atelier: warm plaster, terracotta, serif headings and quiet data surfaces.
/// Ordinary text and controls retain the platform's own sans serif font.
pub fn atelier() -> Theme {
    Direction::Atelier.theme()
}

/// An application masthead, with real navigation supplied by its caller.
pub fn masthead() -> impl Render {
    styled(
        el::div()
            .class("showcase-masthead")
            .child(el::span().class("showcase-brand").text("atelier"))
            .child(Text::new("Order studio").size(Size::Sm).tone(Tone::Muted)),
    )
}

/// A page title, context and one bounded primary action.
pub fn orders_header(level: u8, action: impl Render + 'static) -> impl Render {
    styled(
        el::div()
            .class("showcase-page-heading")
            .child(
                el::div()
                    .child(Text::new("THE WORKSPACE").class("showcase-eyebrow"))
                    .child(Heading::new(level, "Orders").display())
                    .child(
                        Text::new("Good work, from first order to final delivery.")
                            .tone(Tone::Muted),
                    ),
            )
            .child(el::div().class("showcase-primary-action").child(action)),
    )
}

/// A customer and secondary context, with a decorative initial avatar.
pub fn identity(name: &str, detail: &str) -> impl Render + use<> {
    styled(
        el::div()
            .class("showcase-identity")
            .child(Avatar::new(name).size(Size::Sm).decorative())
            .child(
                el::div()
                    .child(el::span().class("showcase-identity-name").text(name))
                    .child(
                        Text::new(detail.to_owned())
                            .size(Size::Sm)
                            .tone(Tone::Muted),
                    ),
            ),
    )
}

/// Status text always accompanies its semantic tone.
pub fn status(value: &str) -> Badge {
    Badge::new(value)
        .tone(match value {
            "pending" => Tone::Warning,
            "paid" => Tone::Info,
            "shipped" => Tone::Success,
            _ => Tone::Default,
        })
        .data("value", value)
}

/// Three metrics explicitly scoped to the rows currently on screen.
pub fn page_metrics(rows: usize, pending: usize, total: &str) -> impl Render + use<> {
    styled(
        el::div()
            .class("showcase-metrics")
            .attr("role", "group")
            .aria("label", "Current page summary")
            .child(Stat::new("Orders on this page", rows.to_string()))
            .child(Stat::new(
                "Awaiting payment on this page",
                pending.to_string(),
            ))
            .child(Stat::new("Value on this page", total)),
    )
}

/// Deterministic illustrative content; never presented as live business data.
pub const ORDERS: [(&str, &str, &str, &str); 5] = [
    (
        "Ada Lovelace",
        "North House · London",
        "pending",
        "$1,248.00",
    ),
    ("Grace Hopper", "Studio Forma · New York", "paid", "$860.50"),
    (
        "Alexandra Montgomery-Sutherland",
        "Montgomery & Partners · Edinburgh",
        "shipped",
        "$12,840.00",
    ),
    (
        "Katherine Johnson",
        "Fieldwork · Hampton",
        "paid",
        "$2,190.00",
    ),
    ("Renée Dubois", "Maison Dubois · Lyon", "pending", "$74.25"),
];

/// The same orders composition on the landing page and full gallery showcase.
/// Actions link to the recipe: this is an explicitly labelled static preview.
pub fn orders_preview(recipe_url: &str, caption: &str) -> impl Render + use<> {
    let rows: Vec<_> = ORDERS
        .iter()
        .enumerate()
        .map(|(i, (name, detail, state, total))| {
            Row::new()
                .cell(format!("ST–{:04}", 1042 + i))
                .cell(identity(name, detail))
                .cell(status(state))
                .cell_with_attrs(*total, stucco::Attrs::default().data("kind", "number"))
        })
        .collect();
    let table = Table::new(caption)
        .header(
            Row::new()
                .header("Order")
                .header("Customer")
                .header("Status")
                .header_with_attrs("Total", stucco::Attrs::default().data("kind", "number")),
        )
        .card_labels(["Order", "Customer", "Status", "Total"]);
    let table = rows.into_iter().fold(table, |table, row| table.row(row));
    styled(
        el::div()
            .class("showcase-workspace")
            .child(masthead())
            .child(orders_header(
                2,
                ButtonLink::new("Build this workspace", recipe_url).variant(Variant::Primary),
            ))
            .child(page_metrics(5, 2, "$17,212.75"))
            .child(el::div().class("showcase-table").child(table))
            .child(
                Text::new("Illustrative data · 5 orders · Built with Stucco components")
                    .class("showcase-caption")
                    .size(Size::Sm)
                    .tone(Tone::Muted),
            ),
    )
}

/// A long-form reading recipe with a bounded measure and ordinary HTML.
pub fn editorial_preview(recipe_url: &str) -> impl Render + use<> {
    styled(el::article().class("showcase-workspace showcase-editorial")
        .child(Text::new("FIELD NOTES / 07").class("showcase-eyebrow"))
        .child(Heading::new(2, "Make room for good work.").display())
        .child(Text::new("The best tools give a team space to think. A few deliberate choices can make an ordinary working day feel calmer.").size(Size::Lg))
        .child(identity("Renée Dubois", "21 October 2026 · 4 minute read"))
        .child(el::div().class("showcase-reading")
            .child(Heading::new(3, "Start with what matters"))
            .child(Text::new("A workspace is a collection of decisions. Which task deserves attention? What has already been settled? Where should a new person begin? Clear hierarchy answers these questions before a single button is pressed."))
            .child(el::blockquote().text("Give the important things room, and let the supporting details be quiet."))
            .child(Text::new("We began with orders: a familiar list of people, amounts and promises. Names need room to wrap. Numbers need to align. A status needs words, even when its color feels obvious."))
            .child(Heading::new(3, "Three decisions to keep"))
            .child(el::ol().child(el::li().text("Use one clear primary action for each stage of work."))
                .child(el::li().text("Keep reading surfaces calm and reserve depth for emphasis."))
                .child(el::li().text("Make every state useful, including an empty list.")))
            .child(el::pre().child(el::code().text("Heading::new(1, \"Good work starts here\").display()")))
            .child(Text::new("These principles survive a change of palette, a smaller screen, or a missing font. The structure does the work; the details give it character."))
            .child(Link::new("Read the composition recipe →", recipe_url))))
}

/// A project overview with semantic progress, trend data and an ordered history.
pub fn project_preview(recipe_url: &str) -> impl Render + use<> {
    styled(el::div().class("showcase-workspace showcase-project")
        .child(Text::new("PROJECT / AUTUMN RELEASE").class("showcase-eyebrow"))
        .child(el::div().class("showcase-page-heading")
            .child(el::div().child(Heading::new(2, "A place to begin.").display())
                .child(Text::new("A thoughtful refresh of the studio’s customer workspace.")))
            .child(Badge::new("On track").tone(Tone::Success)))
        .child(el::div().class("showcase-project-grid")
            .child(Card::new().class("showcase-project-focus")
                .child(Text::new("NEXT MILESTONE").class("showcase-eyebrow"))
                .child(Heading::new(3, "Ready for the first team"))
                .child(Text::new("The order flow is complete. This week we’re checking the details: keyboard navigation, long names, and the moments between actions."))
                .child(Progress::new("Release checklist", 18.0, 24.0).show_value())
                .child(identity("Katherine Johnson", "Project owner · Product design"))
                .child(Link::new("Build this overview →", recipe_url)))
            .child(el::div().class("showcase-project-stats")
                .child(Stat::new("Completed tasks", "18 / 24"))
                .child(Sparkline::new("Tasks completed per week: 2 → 7 across six weeks", [2.0, 3.0, 3.0, 5.0, 4.0, 7.0]))
                .child(Stat::new("Next review", "28 Oct"))))
        .child(el::section().class("showcase-activity")
            .child(Heading::new(3, "Recent activity"))
            .child(Timeline::new("Project activity, newest first")
                .entry(TimelineEntry::new("2026-10-21T14:30:00Z", "21 October · 14:30 UTC", Text::new("Grace Hopper finished the keyboard review. All order actions are reachable without a pointer.")))
                .entry(TimelineEntry::new("2026-10-20T09:15:00Z", "20 October · 09:15 UTC", Text::new("Alexandra Montgomery-Sutherland shared the first customer feedback from Montgomery & Partners.")))
                .entry(TimelineEntry::new("2026-10-18T16:00:00Z", "18 October · 16:00 UTC", Text::new("Renée Dubois approved the reading and empty-state layouts.")))))
        .child(Text::new("Illustrative project · All figures and activity are sample content").class("showcase-caption").size(Size::Sm).tone(Tone::Muted)))
}

/// All complete screen recipes; unknown names select orders.
pub fn screen(name: &str, recipe_url: &str, caption: &str) -> Box<dyn Render> {
    match name {
        "editorial" => Box::new(editorial_preview(recipe_url)),
        "project" => Box::new(project_preview(recipe_url)),
        _ => Box::new(orders_preview(recipe_url, caption)),
    }
}

/// Optional local Newsreader/DM Sans WOFF2 faces, enabled by `atelier-fonts`.
/// Add `showcase-fonts` to an Atelier composition to use the pairing.
#[cfg(feature = "atelier-fonts")]
pub static ATELIER_FONTS: Asset = Asset {
    name: "atelier-fonts",
    css: Some(include_str!("../css/fonts.css")),
    behavior: None,
    deps: &[],
};
#[cfg(feature = "atelier-fonts")]
register_asset!(ATELIER_FONTS);

fn styled(view: impl Render) -> impl Render {
    stucco::render_fn(move |cx| {
        cx.require(&SHOWCASE);
        view.render(cx);
    })
}

/// Applies the optional local font pairing and declares its asset for inline delivery.
#[cfg(feature = "atelier-fonts")]
pub fn with_atelier_fonts(view: impl Render + 'static) -> impl Render {
    let content = el::div().class("showcase-fonts").child(view);
    stucco::render_fn(move |cx| {
        cx.require(&ATELIER_FONTS);
        content.render(cx);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compositions_declare_their_styles_for_inline_delivery() {
        let bundle = Bundle::new(atelier());
        for name in ["orders", "editorial", "project"] {
            let html = Page::new(&bundle, name)
                .delivery(stucco::Delivery::Inline)
                .body(screen(name, "#recipe", "Sample orders"))
                .render();
            assert!(html.contains(".showcase-workspace {"), "{name}");
            assert!(html.contains(".st-heading-display {"), "{name}");
        }
    }
}
