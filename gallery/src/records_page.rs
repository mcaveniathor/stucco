//! Showing one record and the outcome of changing it.

use stucco::actions::ButtonLink;
use stucco::app::{Confirmation, PageHeader};
use stucco::data::{Activity, ActivityList, DescriptionList, Panel, StatusBadge, Timestamp};
use stucco::feedback::Notice;
use stucco::layout::{Cluster, Stack};
use stucco::navigation::Breadcrumbs;
use stucco::{Bundle, Render, Space, Tone, Variant, el};

use crate::shell::{component_page, section};

/// The records and feedback page.
pub fn page(bundle: &Bundle) -> String {
    let header = PageHeader::new("Order 1042")
        .breadcrumbs(
            Breadcrumbs::new()
                .link("Orders", "records.html")
                .current("Order 1042"),
        )
        .description(
            Cluster::new()
                .child(StatusBadge::new("Paid").tone(Tone::Success))
                .child(el::span().text("Ada Lovelace · $1,234.50")),
        )
        .actions(
            Cluster::new()
                .child(ButtonLink::new("Edit", "#edit").variant(Variant::Primary))
                .child(ButtonLink::new("Archive", "#archive"))
                .child(ButtonLink::new("Delete", "#delete").variant(Variant::Danger)),
        );
    let notices = Stack::new()
        .space(Space::S3)
        .child(Notice::success("Order 1042 was saved.").quiet())
        .child(
            Notice::info("This order is archived, so it is read-only.")
                .title("Archived")
                .actions(ButtonLink::new("Restore", "#restore"))
                .quiet(),
        )
        .child(Notice::warning("Stock for two items is low.").quiet())
        .child(
            Notice::danger(
                "The order couldn’t be saved because of a problem on our side. Your changes are still \
                 below; try again. If it keeps happening, quote request ID \
                 3f6c2a9e-77d1-4f0b-a0c5-0e1d7c1b2a44-and-a-very-long-suffix-that-must-wrap.",
            )
            .title("Something went wrong")
            .quiet(),
        );
    let badges = Cluster::new()
        .child(StatusBadge::new("Draft"))
        .child(StatusBadge::new("Pending").tone(Tone::Warning))
        .child(StatusBadge::new("Paid").tone(Tone::Success))
        .child(StatusBadge::new("Shipped").tone(Tone::Info))
        .child(StatusBadge::new("Refunded").tone(Tone::Danger))
        .child(StatusBadge::new("New").tone(Tone::Accent))
        .child(StatusBadge::new("Archived").tone(Tone::Muted));
    let details = DescriptionList::new()
        .item("Customer", "Ada Lovelace")
        .item("Status", StatusBadge::new("Paid").tone(Tone::Success))
        .item("Total", "$1,234.50")
        .item("Order date", Timestamp::date("2026-09-04"))
        .item(
            "Last changed",
            Timestamp::new("2026-09-30T14:05:00Z", "30 Sep 2026, 14:05").zone("UTC"),
        )
        .item("Reference", el::code().text("ord_3f6c2a9e77d14f0ba0c50e1d7c1b2a44"))
        .item("Coupon", "Not set")
        .item(
            "Note",
            "Deliver to the side entrance; the front door is being replaced. Call ahead if the \
             parcel is larger than a shoebox, and leave it with the neighbours at number 12 otherwise.",
        );
    let activity = ActivityList::new("History of order 1042")
        .item(
            Activity::new(
                "Grace Hopper",
                "edited the order",
                Timestamp::new("2026-09-30T14:05:00Z", "30 Sep 2026, 14:05").zone("UTC"),
            )
            .detail("Changed total, note"),
        )
        .item(
            Activity::new(
                "Ada Lovelace",
                "marked the order paid",
                Timestamp::new("2026-09-12T08:30:00Z", "12 Sep 2026, 08:30").zone("UTC"),
            )
            .target("Invoice 88"),
        )
        .item(Activity::new(
            "Ada Lovelace",
            "created the order",
            Timestamp::new("2026-09-04T10:00:00Z", "4 Sep 2026, 10:00").zone("UTC"),
        ));
    let confirmation = Confirmation::new(
        "Delete order 1042 permanently?",
        "#delete",
        "Delete order 1042",
    )
    .level(3)
    .target("Order 1042 for Ada Lovelace, $1,234.50, Paid")
    .consequence("The order and its history are deleted.")
    .consequence("This can't be undone. To hide it but keep its record, archive it instead.")
    .danger()
    .csrf("demo-token")
    .cancel("#order");
    let sections: Vec<Box<dyn Render>> = vec![
        section("Page header with breadcrumbs and actions", header),
        section("Notices", notices),
        section("Status badges", badges),
        section("Properties", Panel::new("Details").body(details)),
        section(
            "Activity",
            Stack::new()
                .space(Space::S6)
                .child(Panel::new("Activity").body(activity))
                .child(Panel::new("No activity").body(ActivityList::new("History of order 1043"))),
        ),
        section("Confirmation", confirmation),
    ];
    component_page(
        bundle,
        "Records and feedback",
        "A record's page and what follows a change: notices, badges, properties, history and confirmations.",
        sections,
    )
}
