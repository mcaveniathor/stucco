//! A minimal stucco app on axum: a themed page with a GET form, and the same
//! endpoint answering fragment requests for the greeting.
//!
//! Run: `cargo run -p hello` (listens on `127.0.0.1:${PORT:-4180}`).

use std::collections::HashMap;

use axum::Router;
use axum::extract::Query;
use axum::response::IntoResponse;
use axum::routing::get;
use stucco::prelude::*;
use stucco::server::{RequestKind, respond};

/// The greeting region: replaced in place by fragment requests.
fn greeting(name: Option<&str>) -> impl Render + '_ {
    el::div()
        .id("greeting")
        .child(name.map(|n| Text::new(format!("Hello, {n}!")).size(Size::Lg)))
}

fn content(name: Option<&str>) -> impl Render + '_ {
    let form = Form::get("/").child(
        Stack::new()
            .space(Space::S4)
            .child(Field::new(
                "Name",
                Input::text("q").value(name.unwrap_or_default()),
            ))
            .child(Button::new("Greet").submit()),
    );
    Container::new().child(
        Stack::new()
            .space(Space::S6)
            .child(Heading::new(1, "Hello"))
            .child(Text::new("A stucco page served by axum.").tone(Tone::Muted))
            .child(form)
            .child(greeting(name)),
    )
}

async fn index(
    page: PageCx,
    kind: RequestKind,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let name = params
        .get("q")
        .map(String::as_str)
        .filter(|q| !q.is_empty());
    // Only the greeting can be requested as a fragment; anything else gets
    // the full page.
    let kind = match kind {
        RequestKind::Fragment { ref target } if target == "greeting" => kind,
        _ => RequestKind::Full,
    };
    respond(
        &kind,
        || page.title("Hello — stucco").main(content(name)).into(),
        |_| page.fragment("greeting", &greeting(name)),
    )
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
