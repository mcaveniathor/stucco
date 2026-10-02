//! A minimal stucco app on axum: a themed page with a GET form, and the same
//! endpoint answering fragment requests for the greeting.
//!
//! Run: `cargo run -p hello` (listens on `127.0.0.1:${PORT:-4180}`).

use std::collections::HashMap;

use axum::Router;
use axum::extract::Query;
use axum::response::Response;
use axum::routing::get;
use stucco::prelude::*;

/// The greeting region: replaced in place by fragment requests.
fn greeting(name: Option<&str>) -> impl Render + '_ {
    el::div()
        .id("greeting")
        .child(name.map(|n| Text::new(format!("Hello, {n}!")).size(Size::Lg)))
}

fn content(name: Option<&str>) -> impl Render + '_ {
    let form = Form::get("/").child(
        Stack::of((
            Field::new("Name", Input::text("q").value(name.unwrap_or_default())),
            Button::new("Greet").submit(),
        ))
        .space(Space::S4),
    );
    Container::of(
        Stack::of((
            Heading::new(1, "Hello"),
            Text::new("A stucco page served by axum.").tone(Tone::Muted),
            form,
            greeting(name),
        ))
        .space(Space::S6),
    )
}

async fn index(page: PageCx, Query(params): Query<HashMap<String, String>>) -> Response {
    let name = params
        .get("q")
        .map(String::as_str)
        .filter(|q| !q.is_empty());
    // Enhanced requests for the greeting get just that region; everything
    // else gets the full page.
    page.respond()
        .fragment("greeting", || greeting(name))
        .page(|| page.title("Hello — stucco").main(content(name)))
}

/// The application: routes, assets and the standard layers.
fn app() -> Router {
    Router::new().route("/", get(index)).stucco(Preset::Slate)
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "4180".to_owned());
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    println!("hello listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app()).await
}
