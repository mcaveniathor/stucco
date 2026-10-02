//! Overlays: dialogs, menus, tooltips and toasts.

use stucco::actions::{Button, IconButton};
use stucco::layout::{Cluster, Stack};
use stucco::overlay::{Dialog, Menu, Toast, Tooltip};
use stucco::typography::Text;
use stucco::{Bundle, Render, Tone, Variant, el, icon};

use crate::shell::{component_page, section};

/// The overlays page.
pub fn page(bundle: &Bundle) -> String {
    let dialog = Dialog::new("delete-order", "Delete order 1042?")
        .child(Text::new(
            "The order and its history are removed. This can't be undone.",
        ))
        .actions(Dialog::close_button("delete-order", "Cancel"))
        .actions(
            el::form().attr("method", "dialog").child(
                Button::new("Delete order")
                    .submit()
                    .variant(Variant::Danger),
            ),
        );
    let sections: Vec<Box<dyn Render>> = vec![
        section(
            "Dialog",
            Stack::new()
                .child(Text::new(
                    "A native modal dialog: focus moves in and back, Escape and a click outside close it.",
                ))
                .child(
                    Cluster::new()
                        .child(dialog.opener("Delete order").variant(Variant::Danger))
                        .child(dialog.link_opener("Delete (link opener)", "#delete-order")),
                )
                .child(dialog),
        ),
        section(
            "Menu",
            Cluster::new()
                .child(
                    Menu::new("Actions")
                        .popover_id("order-actions")
                        .link("Edit", "#edit")
                        .link("Duplicate", "#duplicate")
                        .separator()
                        .item(
                            el::form()
                                .attr("method", "get")
                                .attr("action", "overlays.html")
                                .child(el::button().attr("type", "submit").text("Archive")),
                        ),
                )
                .child(
                    Menu::new("More")
                        .variant(Variant::Ghost)
                        .link("Export as CSV", "#csv")
                        .link("Print", "#print"),
                ),
        ),
        section(
            "Tooltip",
            Cluster::new()
                .child(Tooltip::new(IconButton::new(icon::PENCIL, "Edit"), "Edit"))
                .child(Tooltip::new(
                    IconButton::new(icon::COPY, "Duplicate").variant(Variant::Secondary),
                    "Duplicate",
                ))
                .child(Tooltip::new(
                    IconButton::new(icon::TRASH_2, "Delete").variant(Variant::Danger),
                    "Delete",
                )),
        ),
        section(
            "Toast",
            Stack::new()
                .child(Text::new(
                    "A status message at the edge of the screen, such as the one in the corner. With script it can be dismissed.",
                ))
                .child(Toast::new("Order 1042 saved").tone(Tone::Success)),
        ),
    ];
    component_page(
        bundle,
        "Overlays",
        "Dialogs, menus, tooltips and toasts, built on native HTML and enhanced by one small script.",
        sections,
    )
}
