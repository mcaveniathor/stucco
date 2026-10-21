//! The theme playground: pick a preset or seed, override options, preview the
//! result in light and dark, and export it as Rust, CSS or JSON.

use stucco::actions::Button;
use stucco::forms::{Field, Fieldset, Form, Input, Select};
use stucco::layout::{Cluster, Stack};
use stucco::theme::spec::{Base, OPTIONS, ThemeSpec};
use stucco::theme::{Direction, Preset, Scope};
use stucco::typography::{Heading, Text};
use stucco::{ColorScheme, layout::ThemeScope};
use stucco::{Size, Space, Tone, Variant, el};

use crate::shell::{Section, Site, title_case};

/// The playground page. It renders the default theme's exports so the page
/// is complete without JavaScript; the site script then drives the controls.
pub fn page(site: &Site) -> String {
    let spec = ThemeSpec::new(Base::GuidedV1(Direction::Atelier, 0));
    let built = spec
        .theme()
        .build()
        .expect("Atelier passes contrast checks");
    let initial_css = built.css(Scope::Named("playground"));
    let presets = Preset::ALL.map(|p| (p.name().to_owned(), title_case(p.name())));

    let base = Fieldset::new("Start from")
        .child(Field::new("Generation", Select::new("mode").id("pg-mode")
            .option("guided", "Guided direction").option("unrestricted", "Presets & unrestricted seeds")))
        .child(Field::new("Art direction", Select::new("direction").id("pg-direction")
            .options(Direction::ALL.map(|d| (d.name(), title_case(d.name()))))))
        .child(Field::new(
            "Preset",
            Select::new("preset")
                .id("pg-preset")
                .options(presets)
                .selected("slate"),
        ))
        .child(
            Field::new(
                "Seed",
                Input::text("seed")
                    .id("pg-seed")
                    .autocomplete("off")
                    .attr("spellcheck", "false"),
            )
            .hint("Guided mode uses a whole number; zero is the signature. Unrestricted mode also accepts a product name, or an empty seed to use a preset."),
        )
        .child(
            Cluster::new()
                .space(Space::S2)
                .child(Button::new("Random seed").data("pg-action", "random"))
                .child(
                    Button::new("Clear seed")
                        .variant(Variant::Ghost)
                        .data("pg-action", "clear"),
                ),
        );
    let locks = Fieldset::new("Keep when generating").id("pg-locks")
        .children([("palette", "Lock palette"), ("type", "Lock typography"), ("structure", "Lock surfaces & motion")].map(|(group, label)|
            el::label().class("site-pg-lock").child(el::input().attr("type", "checkbox").data("pg-lock", group)).text(label)))
        .child(Text::new("Locks retain the current values on the next random seed. Explicit option edits are always allowed.").size(Size::Sm));
    // "Base" shows the base's choice once the script runs: "Base (Soft)".
    let options = Fieldset::new("Options").child(el::div().class("site-pg-options").children(
        OPTIONS.iter().map(|option| {
            Field::new(
                option.label,
                Select::new(option.key)
                    .id(format!("pg-{}", option.key))
                    .option("", "Base")
                    .options(option.choices.iter().map(|c| (c.value, c.label))),
            )
        }),
    ));
    let actions = Stack::new()
        .space(Space::S2)
        .child(
            Cluster::new()
                .space(Space::S2)
                .child(
                    Button::new("Use on this site")
                        .variant(Variant::Primary)
                        .data("pg-action", "apply"),
                )
                .child(Button::new("Copy link").data("pg-action", "link"))
                .child(Button::new("Save favorite").data("pg-action", "favorite")),
        )
        .child(
            el::p()
                .id("pg-status")
                .class("site-pg-status")
                .aria("live", "polite"),
        );
    let controls = Form::get(site.url("playground.html"))
        .id("pg-form")
        .class("site-pg-controls")
        .child(
            Stack::new()
                .space(Space::S5)
                .child(base)
                .child(actions)
                .child(locks)
                .child(options),
        );
    let preview = el::section()
        .class("site-pg-preview")
        .aria("labelledby", "pg-preview-title")
        .child(
            Stack::new()
                .space(Space::S3)
                .child(
                    Heading::new(2, "Preview")
                        .id("pg-preview-title")
                        .size(Size::Lg),
                )
                .child(
                    Text::new("Atelier")
                        .tone(Tone::Muted)
                        .attr("id", "pg-label"),
                )
                .child(
                    el::div()
                        .class("site-pg-view-controls")
                        .child(Field::new(
                            "Preview screen",
                            Select::new("screen")
                                .id("pg-screen")
                                .option("orders", "Orders")
                                .option("editorial", "Editorial")
                                .option("project", "Project"),
                        ))
                        .child(Field::new(
                            "Preview scheme",
                            Select::new("scheme")
                                .id("pg-scheme")
                                .option("both", "Both schemes")
                                .option("light", "Light")
                                .option("dark", "Dark"),
                        ))
                        .child(
                            el::label()
                                .child(el::input().attr("type", "checkbox").id("pg-narrow"))
                                .text(" Narrow preview"),
                        ),
                )
                .child(previews(
                    "playground",
                    &site.url("guide/visual-design.html"),
                ))
                .child(
                    el::div()
                        .class("site-pg-favorites")
                        .child(Field::new(
                            "Compare favorite",
                            Select::new("favorite")
                                .id("pg-favorite")
                                .option("", "Choose a saved theme"),
                        ))
                        .child(Button::new("Load favorite").id("pg-load-favorite"))
                        .child(Button::new("Remove favorite").id("pg-remove-favorite")),
                )
                .child(
                    el::section()
                        .id("pg-comparison")
                        .attr("hidden", "")
                        .child(Heading::new(3, "Saved theme"))
                        .child(Text::new("").id("pg-comparison-label"))
                        .child(previews(
                            "comparison",
                            &site.url("guide/visual-design.html"),
                        )),
                ),
        );
    let exports = [
        ("pg-rust", "Rust", "theme.rs", spec.rust()),
        ("pg-css", "CSS", "theme.css", built.css(Scope::Root)),
        (
            "pg-json",
            "JSON tokens",
            "tokens.json",
            spec.export_json(&built),
        ),
    ];
    let export = el::section()
        .class("site-pg-export")
        .aria("labelledby", "pg-export-title")
        .child(
            Stack::new()
                .space(Space::S5)
                .child(
                    Stack::new()
                        .space(Space::S2)
                        .child(Heading::new(2, "Export").id("pg-export-title"))
                        .child(
                            Text::new(
                                "Rust builds the theme in your app. The CSS works anywhere: \
                                 load it after the stucco stylesheet, or use the tokens in \
                                 your own styles. The JSON includes both colour schemes and the canonical recipe for every choice.",
                            )
                            .tone(Tone::Muted),
                        ),
                )
                .children(exports.map(|(id, label, file, text)| {
                    Stack::new()
                        .space(Space::S2)
                        .child(
                            Cluster::new()
                                .space(Space::S2)
                                .child(Heading::new(3, label).size(Size::Md))
                                .child(
                                    Button::new(format!("Copy {label}"))
                                        .size(stucco::Size::Sm)
                                        .data("copy", id),
                                )
                                .child(
                                    Button::new(format!("Download {file}"))
                                        .size(stucco::Size::Sm)
                                        .variant(Variant::Ghost)
                                        .data("download", id)
                                        .data("filename", file),
                                ),
                        )
                        .child(
                            el::pre()
                                .class("site-code site-code-scroll")
                                .attr("tabindex", "0")
                                .aria("label", format!("{label} export"))
                                .child(el::code().id(id).text(text)),
                        )
                })),
        );
    let main = Stack::new()
        .space(Space::S8)
        .child(
            Stack::new()
                .space(Space::S2)
                .child(Heading::new(1, "Theme playground"))
                .child(
                    Text::new(
                        "Explore a coordinated direction, lock what you like, and compare saved variations. Or start from any preset or unrestricted seed. Every theme here comes from the same Rust code your app runs, \
                         compiled to WebAssembly, so it passes the same contrast checks.",
                    )
                    .tone(Tone::Muted)
                    .size(Size::Lg),
                )
                .child(el::noscript().child(el::p().class("site-notice").text(
                    "Discovery controls need JavaScript. The complete Atelier previews and exports below work without it.",
                ))),
        )
        .child(
            el::div()
                .class("site-pg")
                .data("playground", "")
                .child(controls)
                .child(preview),
        )
        .child(export);
    site.page_with_head(
        "Theme playground",
        "Build a stucco theme from a preset or seed, preview it, and export Rust, CSS or JSON.",
        Section::Playground,
        None,
        format!(
            r#"<style id="pg-theme">{initial_css}</style><style id="pg-comparison-theme"></style>"#
        ),
        main,
    )
}

/// Fully rendered recipes; enhancement only changes visibility and theme tokens.
fn previews(scope: &'static str, recipe: &str) -> impl stucco::Render + use<> {
    el::div()
        .class("site-pg-screens")
        .children(["orders", "editorial", "project"].map(|screen| {
            el::div().data("pg-screen", screen).children(
                [(ColorScheme::Light, "light"), (ColorScheme::Dark, "dark")].map(
                    |(scheme, name)| {
                        el::div()
                            .data("pg-scheme", name)
                            .child(el::p().class("showcase-eyebrow").text(name))
                            .child(ThemeScope::new().named(scope).scheme(scheme).child(
                                stucco_showcase::screen(
                                    screen,
                                    recipe,
                                    &format!("{scope} orders in {name}"),
                                ),
                            ))
                    },
                ),
            )
        }))
}
