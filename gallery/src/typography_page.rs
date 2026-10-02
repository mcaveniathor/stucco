//! Typography.

use stucco::layout::{Cluster, Stack};
use stucco::typography::{Code, Heading, Kbd, Link, Text, Weight};
use stucco::{Bundle, Render, Size, Space, Tone};

use crate::shell::{component_page, section};

/// The typography page.
pub fn page(bundle: &Bundle) -> String {
    let sizes = [Size::Xl3, Size::Xl2, Size::Xl, Size::Lg, Size::Md, Size::Sm];
    let headings = Stack::new().space(Space::S2).children(
        sizes
            .iter()
            .map(|s| Heading::new(3, format!("Heading at size {}", s.as_str())).size(*s)),
    );
    let tones = [
        (Tone::Default, "Default text"),
        (Tone::Muted, "Muted text"),
        (Tone::Accent, "Accent text"),
        (Tone::Success, "Success text"),
        (Tone::Warning, "Warning text"),
        (Tone::Danger, "Danger text"),
        (Tone::Info, "Info text"),
    ];
    let sections: Vec<Box<dyn Render>> = vec![
        section("Heading", headings),
        section(
            "Text",
            Stack::new()
                .space(Space::S2)
                .children(tones.map(|(t, label)| Text::new(label).tone(t)))
                .child(Text::new("Bold text").weight(Weight::Bold))
                .child(Text::new("Small text").size(Size::Sm)),
        ),
        section(
            "Link",
            Cluster::new()
                .space(Space::S6)
                .child(Link::new("Internal link", "palette.html"))
                .child(Link::new("External link", "https://lucide.dev").external()),
        ),
        section(
            "Code and Kbd",
            Stack::new()
                .space(Space::S3)
                .child(Text::new((
                    "Run ",
                    Code::new("cargo run -p gallery"),
                    " to rebuild this site.",
                )))
                .child(Text::new((
                    "Open the command palette with ",
                    Kbd::new(&["Ctrl", "K"]),
                    ".",
                ))),
        ),
    ];
    component_page(
        bundle,
        "Typography",
        "Headings with independent level and size, toned text, safe links and inline code.",
        sections,
    )
}
