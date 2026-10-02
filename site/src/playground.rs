//! The theme playground: pick a preset or seed, override options, preview the
//! result in light and dark, and export it as Rust, CSS or JSON.

use stucco::actions::Button;
use stucco::forms::{Field, Fieldset, Form, Input, Select};
use stucco::layout::{Cluster, Stack};
use stucco::theme::spec::{OPTIONS, ThemeSpec, tokens_json};
use stucco::theme::{Preset, Scope, Theme};
use stucco::typography::{Heading, Text};
use stucco::{Size, Space, Tone, Variant, el};

use crate::shell::{Section, Site, title_case};

/// The playground page. It renders the default theme's exports so the page
/// is complete without JavaScript; the site script then drives the controls.
pub fn page(site: &Site) -> String {
    let spec = ThemeSpec::default();
    let built = Theme::preset(Preset::Slate)
        .build()
        .expect("Slate passes its contrast checks");
    let initial_css = built.css(Scope::Named("playground"));
    let presets = Preset::ALL.map(|p| (p.name().to_owned(), title_case(p.name())));

    let base = Fieldset::new("Start from")
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
            .hint("A whole number, or any text such as your product's name. Leave it empty to start from the preset."),
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
                .child(Button::new("Copy link").data("pg-action", "link")),
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
                .child(Text::new("Slate").tone(Tone::Muted).attr("id", "pg-label"))
                .child(gallery::themes_page::panels("playground")),
        );
    let exports = [
        ("pg-rust", "Rust", "theme.rs", spec.rust()),
        ("pg-css", "CSS", "theme.css", built.css(Scope::Root)),
        ("pg-json", "JSON tokens", "tokens.json", tokens_json(&built)),
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
                                 your own styles. The JSON lists every colour role in both \
                                 schemes.",
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
                        "Start from a preset or a seed, override any option, and export the \
                         result. Every theme here comes from the same Rust code your app runs, \
                         compiled to WebAssembly, so it passes the same contrast checks.",
                    )
                    .tone(Tone::Muted)
                    .size(Size::Lg),
                )
                .child(el::noscript().child(el::p().class("site-notice").text(
                    "The playground needs JavaScript. The exports below show the default Slate theme.",
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
        format!(r#"<style id="pg-theme">{initial_css}</style>"#),
        main,
    )
}
