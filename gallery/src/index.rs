//! The gallery home page.

use stucco::{Bundle, Page, el};

/// Links to every gallery page.
pub fn index_page(bundle: &Bundle) -> String {
    Page::new(bundle, "stucco gallery")
        .body(
            el::main()
                .class("g-page")
                .child(el::h1().text("stucco gallery"))
                .child(
                    el::p()
                        .class("g-lead")
                        .text("Components, themes and test fixtures for the stucco UI library."),
                )
                .child(el::ul().child(
                    el::li().child(el::a().href("palette.html").text("Palettes and presets")),
                )),
        )
        .render()
}
