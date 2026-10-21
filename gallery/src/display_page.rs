//! Status, details, progress and disclosure components.

use stucco::actions::ButtonLink;
use stucco::data::{Avatar, Badge, DescriptionList, Stat};
use stucco::feedback::{Callout, Meter, Progress};
use stucco::layout::{Accordion, Cluster, Disclosure, Grid, Stack};
use stucco::navigation::{Breadcrumbs, Steps};
use stucco::typography::Text;
use stucco::{Bundle, Measure, Size, Space, Tone, Trend, icon};

use crate::shell::{component_page, section};

/// A label long enough to test wrapping at narrow widths.
const LONG: &str = "Bartholomew Montgomery-Fitzwilliam Hargreaves the Third, Esq.";

const TONES: [(Tone, &str); 7] = [
    (Tone::Default, "Draft"),
    (Tone::Muted, "Archived"),
    (Tone::Accent, "New"),
    (Tone::Info, "Paid"),
    (Tone::Success, "Shipped"),
    (Tone::Warning, "Pending"),
    (Tone::Danger, "Overdue"),
];

/// The display page.
pub fn page(bundle: &Bundle) -> String {
    let badges = Cluster::new()
        .children(TONES.map(|(tone, label)| Badge::new(label).tone(tone)))
        .child(
            Badge::new("Verified")
                .tone(Tone::Success)
                .icon(icon::CIRCLE_CHECK),
        )
        .child(Badge::new(LONG));
    let avatars = Cluster::new()
        .child(Avatar::new("Ada Lovelace").size(Size::Sm))
        .child(Avatar::new("Grace Hopper"))
        .child(Avatar::new("Alan Turing").size(Size::Lg))
        .child(Avatar::new("Katherine Johnson").image("avatar.svg"))
        .child(
            Cluster::new()
                .child(Avatar::new("Ada Lovelace").decorative())
                .child(Text::new("Ada Lovelace").inline()),
        );
    let details = DescriptionList::new()
        .item("Customer", "Ada Lovelace")
        .item("Status", Badge::new("Paid").tone(Tone::Info))
        .item("Total", "$12.50")
        .item("Notes", LONG);
    let stats = Grid::new()
        .min(Measure::Xs)
        .space(Space::S6)
        .child(
            Stat::new("Revenue", "$12,400")
                .change("12%", Trend::Up)
                .change_tone(Tone::Success)
                .context("vs last week"),
        )
        .child(
            Stat::new("Refunds", "18")
                .change("4", Trend::Up)
                .change_tone(Tone::Danger)
                .context("vs last week"),
        )
        .child(Stat::new("Open orders", "42").change("0%", Trend::Flat))
        .child(Stat::new("Average basket", "$31.20").change("2%", Trend::Down));
    let breadcrumbs = Stack::new()
        .child(
            Breadcrumbs::new()
                .link("Orders", "#orders")
                .link("Order 42", "#order")
                .current("Edit"),
        )
        .child(
            Breadcrumbs::new()
                .label("Customer location")
                .link("Customers", "#customers")
                .current(LONG),
        );
    let steps = Steps::new("Checkout")
        .step("Cart", "#cart")
        .step_disabled("Account")
        .step("Shipping", "#shipping")
        .step("Payment", "#payment")
        .current(2);
    let progress = Stack::new()
        .child(Progress::new("Import", 42.0, 100.0).show_value())
        .child(Progress::indeterminate("Uploading"))
        .child(
            Meter::new("Storage", 7.2, 0.0, 10.0)
                .low(3.0)
                .high(8.0)
                .optimum(1.0)
                .value_text("7.2 of 10 GB"),
        )
        .child(
            Meter::new("Battery", 0.9, 0.0, 1.0)
                .low(0.2)
                .high(0.8)
                .optimum(1.0)
                .value_text("90%"),
        );
    let callouts = Stack::new()
        .child(
            Callout::new(Tone::Warning)
                .title("Payment overdue")
                .child(Text::new("Invoice 1042 was due on 1 October."))
                .actions(ButtonLink::new("Pay now", "#pay")),
        )
        .child(Callout::new(Tone::Info).title("Prices change on 1 November"))
        .child(Callout::new(Tone::Success).title("Your export is ready"))
        .child(
            Callout::new(Tone::Danger)
                .title("This order can't be shipped")
                .child(Text::new(LONG)),
        )
        .child(Callout::new(Tone::Accent).child(Text::new("A note in the accent colour.")))
        .child(Callout::new(Tone::Default).title("A neutral note"));
    let disclosure = Stack::new()
        .child(
            Disclosure::new("Shipping details")
                .heading(3)
                .child(Text::new("Courier, two working days.")),
        )
        .child(Disclosure::new(LONG).child(Text::new("Long summaries wrap.")));
    let accordion = Accordion::new()
        .exclusive()
        .heading(3)
        .item_open(
            "How long does delivery take?",
            Text::new("Two working days."),
        )
        .item("Can I return an order?", Text::new("Within 30 days."))
        .item("Do you ship abroad?", Text::new("To most of Europe."));
    component_page(
        bundle,
        "Display & status",
        "Badges, avatars, details, stats, where-you-are navigation, progress, callouts and disclosure, all without script.",
        [
            section("Badge", badges),
            section("Avatar", avatars),
            section("Description list", details),
            section("Stat", stats),
            section("Breadcrumbs", breadcrumbs),
            section("Steps", steps),
            section("Progress and meter", progress),
            section("Callout", callouts),
            section("Disclosure", disclosure),
            section("Accordion", accordion),
        ],
    )
}
