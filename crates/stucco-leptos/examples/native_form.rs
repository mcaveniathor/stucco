//! A native GET form using the public component API and shared stylesheet.
//!
//! Run without arguments to print the HTML document, or with `--css` to print
//! its stylesheet. The application owns the `/search` route; this example does
//! not start a server or install a WASM hydration entry point.

use leptos::prelude::*;
use stucco_leptos::{PageHeader, SubmitButton, TextField, stylesheet};
use stucco_theme::Preset;

#[component]
fn SearchForm() -> impl IntoView {
    let query = RwSignal::new(String::new());

    view! {
        <form class="st-form" method="get" action="/search">
            <TextField
                id="search-query"
                name="q"
                label="Search terms"
                value=query
                on_input=Callback::new(move |value| query.set(value))
                hint="Search by title or description."
                required=true
            />
            // Native controls can use the same CSS when no wrapper is available.
            <div class="st-field">
                <label class="st-label" for="search-kind">"Content type"</label>
                <select class="st-select" id="search-kind" name="kind">
                    <option value="all">"All content"</option>
                    <option value="articles">"Articles"</option>
                </select>
            </div>
            <SubmitButton label="Search"/>
        </form>
    }
}

fn main() {
    if std::env::args().nth(1).as_deref() == Some("--css") {
        print!("{}", stylesheet(Preset::Slate));
        return;
    }

    let owner = Owner::new();
    let document = owner.with(|| {
        view! {
            <html lang="en">
                <head>
                    <meta charset="utf-8"/>
                    <meta name="viewport" content="width=device-width, initial-scale=1"/>
                    <title>"Stucco search form"</title>
                    <link rel="stylesheet" href="/stucco.css"/>
                </head>
                <body>
                    <main class="st-center st-stack" data-max="prose" data-gutters="4" data-space="6">
                        <PageHeader heading="Search" description="A native form with shared Stucco styles."/>
                        <SearchForm/>
                    </main>
                </body>
            </html>
        }
        .to_html()
    });
    println!("<!doctype html>{document}");
}
