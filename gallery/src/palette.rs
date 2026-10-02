//! The palette page: every preset's scales and roles, light and dark.

use stucco::theme::Preset;
use stucco::{Bundle, Page, Render, el};

const SCALES: [&str; 6] = ["neutral", "accent", "success", "warning", "danger", "info"];

/// Every preset, each in a light and a dark panel.
pub fn palette_page(bundle: &Bundle) -> String {
    let presets = Preset::ALL.map(preset_section);
    Page::new(bundle, "Palettes — stucco gallery")
        .body(
            el::main()
                .class("g-page")
                .child(el::h1().text("Palettes"))
                .child(el::p().class("g-lead").text(
                    "Each preset's six 12-step scales and its semantic roles, in light and dark.",
                ))
                .children(presets),
        )
        .render()
}

fn preset_section(preset: Preset) -> impl Render {
    let name = preset.name();
    let heading = format!("{name}-heading");
    el::section()
        .class("g-preset")
        .attr("data-st-theme", name)
        .aria("labelledby", heading.clone())
        .child(el::h2().id(heading).text(title_case(name)))
        .child(
            el::div()
                .class("g-panels")
                .child(panel("light", "Light"))
                .child(panel("dark", "Dark")),
        )
}

fn panel(scheme: &str, label: &str) -> impl Render + 'static {
    el::div()
        .class("g-panel")
        .attr("data-theme", scheme)
        .child(el::h3().text(label.to_owned()))
        .children(SCALES.map(scale_row))
        .child(roles())
}

fn scale_row(scale: &'static str) -> impl Render {
    el::div()
        .class("g-scale")
        .child(el::span().class("g-scale-name").text(scale))
        .children((1..=12).map(move |n| {
            el::span()
                .class("g-swatch")
                .aria("hidden", "true")
                .attr("style", format!("background: var(--st-{scale}-{n})"))
        }))
}

fn roles() -> impl Render {
    el::div()
        .class("g-roles")
        .child(el::span().class("g-surface").text("Text on surface"))
        .child(el::span().class("g-muted").text("Muted text"))
        .child(
            el::button()
                .class("g-accent")
                .attr("type", "button")
                .text("Accent"),
        )
        .child(el::a().class("g-link").href("#").text("Link"))
        .child(el::span().class("g-boxed").text("Strong border"))
        .children(["success", "warning", "danger", "info"].map(|s| {
            el::span()
                .class(format!("g-status g-{s}"))
                .text(title_case(s))
        }))
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    chars
        .next()
        .map(|c| c.to_uppercase().chain(chars).collect())
        .unwrap_or_default()
}
