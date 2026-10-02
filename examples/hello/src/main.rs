//! A minimal stucco app on axum: a themed page with a GET form, and the same
//! endpoint answering fragment requests for the greeting.
//!
//! Run: `cargo run -p hello` (listens on `127.0.0.1:${PORT:-4180}`).

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::routing::get;
use stucco::actions::Button;
use stucco::forms::{Field, Form, Input};
use stucco::layout::{Container, SkipLink, Stack};
use stucco::theme::Preset;
use stucco::typography::{Heading, Text};
use stucco::{Bundle, Page, Render, Size, Space, Tone, el, render_fragment};
use stucco_tower::{
    FragmentResponse, LayerConfig, PageResponse, RequestKind, assets_router, respond,
    with_standard_layers,
};

/// The greeting region: replaced in place by fragment requests.
fn greeting(name: Option<&str>) -> impl Render + '_ {
    el::div()
        .id("greeting")
        .child(name.map(|n| Text::new(format!("Hello, {n}!")).size(Size::Lg)))
}

fn page(bundle: &Bundle, name: Option<&str>) -> String {
    let form = Form::get("/").child(
        Stack::new()
            .space(Space::S4)
            .child(Field::new(
                "Name",
                Input::text("q").value(name.unwrap_or_default()),
            ))
            .child(Button::new("Greet").submit()),
    );
    Page::new(bundle, "Hello — stucco")
        .body((
            SkipLink::new(),
            el::main().id("main").child(
                Container::new().child(
                    Stack::new()
                        .space(Space::S6)
                        .child(Heading::new(1, "Hello"))
                        .child(
                            Text::new("A stucco page served by axum through stucco-tower.")
                                .tone(Tone::Muted),
                        )
                        .child(form)
                        .child(greeting(name)),
                ),
            ),
        ))
        .render()
}

async fn index(
    State(bundle): State<Arc<Bundle>>,
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
        || PageResponse::new(page(&bundle, name)),
        |_| FragmentResponse::new(&render_fragment("greeting", &greeting(name)), &bundle),
    )
}

/// The application: routes, assets and the standard layers.
fn app(bundle: Arc<Bundle>) -> Router {
    let routes = Router::new()
        .route("/", get(index))
        .with_state(bundle.clone());
    with_standard_layers(routes.merge(assets_router(bundle)), &LayerConfig::default())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let port = std::env::var("PORT").unwrap_or_else(|_| "4180".to_owned());
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    println!("hello listening on http://{}", listener.local_addr()?);
    axum::serve(listener, app(bundle)).await
}
