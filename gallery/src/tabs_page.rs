//! Tabs: one page per tab, as a server renders them. Each tab is a link to
//! its own URL, and the page marks the current one and shows its content.

use stucco::data::{Card, Row, Table};
use stucco::layout::Stack;
use stucco::navigation::{NavLink, Tabs};
use stucco::typography::Text;
use stucco::{Bundle, Space, Tone};

use crate::shell::{component_page, section};

/// The tabs, as `(file name, label)`; the first is the default.
pub const TABS: [(&str, &str); 3] = [
    ("tabs.html", "Details"),
    ("tabs-items.html", "Items"),
    ("tabs-history.html", "History"),
];

/// Every tab's page as `(file name, HTML)`.
pub fn pages(bundle: &Bundle) -> Vec<(String, String)> {
    TABS.iter()
        .map(|(file, label)| ((*file).to_owned(), page(bundle, label)))
        .collect()
}

/// The page showing the tab labelled `current`.
fn page(bundle: &Bundle, current: &str) -> String {
    let tabs = TABS
        .iter()
        .fold(Tabs::new("Order sections"), |tabs, (file, label)| {
            tabs.tab(NavLink::new(*label, *file).current(*label == current))
        });
    let panel = match current {
        "Items" => Stack::new().space(Space::S3).child(
            Table::new("Items in order 1042")
                .header(Row::new().header("Item").header("Quantity"))
                .row(Row::new().cell("Linen shirt").cell("2"))
                .row(Row::new().cell("Canvas tote").cell("1")),
        ),
        "History" => Stack::new()
            .space(Space::S2)
            .child(Text::new("Placed on 2 October at 09:14."))
            .child(Text::new("Paid on 2 October at 09:15."))
            .child(Text::new("Shipped on 3 October at 16:40.")),
        _ => Stack::new()
            .space(Space::S2)
            .child(Text::new("Order 1042 for Ada Lovelace."))
            .child(Text::new("Total $48.00, paid by card.").tone(Tone::Muted)),
    };
    component_page(
        bundle,
        "Tabs",
        "Each tab is a link to its own page, so tabs work without script, can be bookmarked and keep the back button. The server marks the current tab and renders only its content.",
        [section(
            "Order 1042",
            Card::new().child(Stack::new().space(Space::S4).child(tabs).child(panel)),
        )],
    )
}
