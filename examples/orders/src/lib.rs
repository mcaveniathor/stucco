//! Persistent, server-rendered orders: list, create, edit and delete. See
//! the example README for scan costs.
mod form;
mod handlers;
mod model;
mod source;
use axum::{
    Router,
    extract::{RawQuery, State},
    response::Redirect,
    routing::get,
};
use model::Order;
use source::OrdersMapping;
use std::error::Error;
use std::path::Path;
use stucco::Slot;
use stucco::prelude::*;
use stucco::server::{CsrfLayer, FlashLayer, RequestContext, SecretKey};
use stucco_redb::{
    AsyncTable, IndexTable, PostcardCodec, RedbCollection, Store, StoreError, U64Key,
};

type OrdersTable = IndexTable<u64, Order>;
type OrdersSource = RedbCollection<u64, Order, U64Key, PostcardCodec<Order>, OrdersMapping>;
#[derive(Clone)]
struct AppState {
    cursor: OrdersSource,
    pages: OrdersSource,
}

impl AppState {
    fn orders(&self) -> &AsyncTable<u64, Order> {
        self.cursor.table()
    }
}

fn open(path: &Path) -> Result<OrdersTable, StoreError> {
    // Seed a new database only: one emptied by deleting every order stays empty.
    let fresh = !path.exists();
    let store = Store::open(path)?;
    let table = store
        .table("orders", U64Key, PostcardCodec::<Order>::default())?
        .index("customer", |row| row.customer.as_bytes().to_vec())?;
    if fresh {
        table.put_many(
            &model::seed()
                .into_iter()
                .map(|row| (row.id, row))
                .collect::<Vec<_>>(),
        )?;
    }
    table.rebuild()?;
    Ok(table)
}

/// Opens persistent storage and constructs the router. `origin` is the
/// public origin browsers use, such as `https://orders.example.com`; cookies
/// are marked `Secure` when it is HTTPS.
pub fn app(db_path: &Path, key: SecretKey, origin: &str) -> Result<Router, Box<dyn Error>> {
    let cursor = RedbCollection::new(
        AsyncTable::new(open(db_path)?),
        OrdersMapping { pages: false },
    );
    let pages = cursor.with_mapping(OrdersMapping { pages: true });
    let state = AppState { cursor, pages };
    let secure = origin.starts_with("https://");
    Ok(Router::new()
        .route("/", get(|| async { Redirect::to("/orders") }))
        .route("/orders", get(list).post(handlers::create))
        .route("/orders/new", get(handlers::new_order))
        .route("/orders/{id}", get(handlers::show).post(handlers::update))
        .route("/orders/{id}/edit", get(handlers::edit))
        .route(
            "/orders/{id}/delete",
            get(handlers::confirm_delete).post(handlers::delete),
        )
        .fallback(handlers::fallback)
        .with_state(state)
        .layer(FlashLayer::new(key.clone()).secure(secure))
        .layer(CsrfLayer::new(key, origin)?.secure(secure))
        .stucco(Preset::Slate))
}

/// The application shell shared by every page.
fn shell<'a>(main: impl Render + 'a) -> AppShell<'a> {
    AppShell::new()
        .link(NavLink::new("Orders", "/orders"))
        .link(NavLink::new("New order", "/orders/new"))
        .main(main)
}

fn tone_of(status: &str) -> Tone {
    match status {
        "pending" => Tone::Warning,
        "paid" => Tone::Info,
        "shipped" => Tone::Success,
        _ => Tone::Default,
    }
}

async fn list(
    State(state): State<AppState>,
    RawQuery(raw): RawQuery,
    context: RequestContext,
    cx: PageCx,
) -> Document {
    let raw = raw.unwrap_or_default();
    let pages = form_urlencoded::parse(raw.as_bytes()).any(|(k, v)| k == "mode" && v == "pages");
    let source = if pages { &state.pages } else { &state.cursor };
    match source.load(&raw, &Order::column_specs(), &context).await {
        Ok(orders) => {
            let csrf = handlers::csrf(&cx);
            let table = state.orders();
            let actions = Col::custom("actions", "Actions", |order: &Order| {
                match table.revision_of(order) {
                    Ok(revision) => Slot::new(handlers::row_actions(order, revision, &csrf)),
                    Err(_) => Slot::new(""),
                }
            });
            let table = DataTable::from_collection(&orders, "Orders")
                .action(if pages {
                    "/orders?mode=pages"
                } else {
                    "/orders"
                })
                .columns(Order::columns().into_iter().map(|column| {
                    if column.spec().key == "status" {
                        column.cell(|o| {
                            Slot::new(
                                Badge::new(o.status.clone())
                                    .tone(tone_of(&o.status))
                                    .data("value", o.status.clone()),
                            )
                        })
                    } else {
                        column
                    }
                }))
                .column(actions);
            cx.title("Orders — stucco").app(
                AppShell::new()
                    .header(
                        PageHeader::new("Orders")
                            .description("A persistent collection with ordinary GET navigation.")
                            .actions(
                                ButtonLink::new("New order", "/orders/new")
                                    .variant(Variant::Primary),
                            ),
                    )
                    .link(NavLink::new("Orders", "/orders").current(!pages))
                    .link(NavLink::new("Numbered pages", "/orders?mode=pages").current(pages))
                    .main(
                        Stack::new()
                            .child(FlashMessages::new(context.get::<Flash>().cloned()))
                            .child(table),
                    ),
            )
        }
        Err(error) => {
            tracing::error!(%error, request_id = %context.request_id, "orders query failed");
            cx.title("Orders unavailable")
                .app(
                    AppShell::new().main(PageHeader::new("Orders unavailable").description(
                        format!("Please try again. Request ID: {}", context.request_id),
                    )),
                )
                .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;
    use tower::ServiceExt;
    #[tokio::test]
    async fn stored_hostile_customer_is_escaped_and_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("orders.redb");
        {
            let table = open(&path).unwrap();
            let mut row = table.get(&1).unwrap().unwrap();
            row.customer = "<script>alert(1)</script>".into();
            table.put(&1, &row).unwrap();
        }
        let response = app(&path, SecretKey::new([7; 32]), "http://localhost")
            .unwrap()
            .oneshot(
                axum::http::Request::builder()
                    .uri("/orders")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let body = String::from_utf8(
            response
                .into_body()
                .collect()
                .await
                .unwrap()
                .to_bytes()
                .to_vec(),
        )
        .unwrap();
        assert!(body.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
        assert!(!body.contains("<script>alert(1)</script>"));
    }
}
