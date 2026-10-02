//! Actions.

use stucco::actions::{Button, ButtonLink, IconButton};
use stucco::layout::Cluster;
use stucco::{Bundle, Render, Size, Variant, icon};

use crate::shell::{component_page, section};

/// The actions page.
pub fn page(bundle: &Bundle) -> String {
    let variants = [
        (Variant::Primary, "Primary"),
        (Variant::Secondary, "Secondary"),
        (Variant::Ghost, "Ghost"),
        (Variant::Danger, "Danger"),
    ];
    let sections: Vec<Box<dyn Render>> = vec![
        section(
            "Variants",
            Cluster::new().children(variants.map(|(v, label)| Button::new(label).variant(v))),
        ),
        section(
            "Sizes",
            Cluster::new()
                .child(Button::new("Small").size(Size::Sm))
                .child(Button::new("Medium"))
                .child(Button::new("Large").size(Size::Lg)),
        ),
        section(
            "Icons and states",
            Cluster::new()
                .child(
                    Button::new("Add item")
                        .icon(icon::PLUS)
                        .variant(Variant::Primary),
                )
                .child(Button::new("Download").icon(icon::DOWNLOAD))
                .child(Button::new("Saving").loading())
                .child(Button::new("Disabled").disabled()),
        ),
        section(
            "ButtonLink",
            Cluster::new()
                .child(ButtonLink::new("Get started", "index.html").variant(Variant::Primary))
                .child(ButtonLink::new("Palettes", "palette.html").icon(icon::ARROW_RIGHT)),
        ),
        section(
            "IconButton",
            Cluster::new()
                .child(IconButton::new(icon::SEARCH, "Search"))
                .child(IconButton::new(icon::SETTINGS, "Settings").variant(Variant::Secondary))
                .child(IconButton::new(icon::TRASH_2, "Delete").variant(Variant::Danger)),
        ),
    ];
    component_page(
        bundle,
        "Actions",
        "Buttons default to type=\"button\"; loading buttons stay focusable and announce busy.",
        sections,
    )
}
