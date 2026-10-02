//! A complete, server-rendered CRUD admin for orders: browse, search, filter,
//! sort and page; view an order and its history; create and edit with
//! validation; archive, restore and delete with confirmation; act on
//! several orders at once. Every workflow works without JavaScript.
//!
//! See the example README for what the example leaves to a real
//! application (sign-in, permissions, CSRF tokens) and for storage costs.
mod layout;
mod list;
mod model;
mod record;
mod source;
mod store;
mod time;

use axum::{
    Router,
    extract::Request,
    http::{Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use model::Order;
use source::OrdersMapping;
use std::path::Path;
use store::{Orders, OrdersTable};
use stucco::prelude::*;
use stucco_redb::{PostcardCodec, RedbCollection, Store, StoreError, U64Key};

type OrdersSource = RedbCollection<u64, Order, U64Key, PostcardCodec<Order>, OrdersMapping>;

#[derive(Clone)]
struct AppState {
    cursor: OrdersSource,
    pages: OrdersSource,
    orders: Orders,
}

/// The table name carries a version: records written by an older version of
/// the example (without history and versions) are left alone, not misread.
const TABLE: &str = "orders-v2";

fn open(path: &Path) -> Result<OrdersTable, StoreError> {
    let store = Store::open(path)?;
    let table = store
        .table(TABLE, U64Key, PostcardCodec::<Order>::default())?
        .index("customer", |row| row.customer.as_bytes().to_vec())?;
    if table.all(false)?.is_empty() {
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

/// Opens persistent storage and builds the router.
pub fn app(db_path: &Path) -> Result<Router, StoreError> {
    let table = open(db_path)?;
    let cursor = RedbCollection::new(table.clone(), OrdersMapping { pages: false });
    let pages = cursor.with_mapping(OrdersMapping { pages: true });
    let state = AppState {
        cursor,
        pages,
        orders: Orders::new(table),
    };
    // Changes are POSTs only: a GET (a crawler, a prefetch, a link in an
    // email) can show a confirmation page but never change anything.
    Ok(Router::new()
        .route("/", get(|| async { Redirect::to("/orders") }))
        .route("/orders", get(list::list).post(record::create))
        .route("/orders/new", get(record::new))
        .route(
            "/orders/bulk",
            get(list::confirm_bulk).post(list::apply_bulk),
        )
        .route("/orders/{id}", get(record::show).post(record::update))
        .route("/orders/{id}/edit", get(record::edit))
        .route(
            "/orders/{id}/archive",
            get(record::confirm_archive).post(record::archive),
        )
        .route("/orders/{id}/restore", post(record::restore))
        .route(
            "/orders/{id}/delete",
            get(record::confirm_delete).post(record::delete),
        )
        .layer(middleware::from_fn(same_origin))
        .with_state(state)
        .stucco(Preset::Slate))
}

/// Refuses cross-site form posts (CSRF) using the headers browsers send:
/// `Sec-Fetch-Site` where supported, else `Origin` compared with `Host`.
/// Requests with neither come from non-browser clients, which can't be
/// tricked into sending a user's cookies, and are let through.
///
/// This is the example's choice, not stucco's: stucco renders the token a
/// signed-token scheme needs (`Form::csrf`), but checking requests belongs
/// to the application. Use an established CSRF library or your framework's
/// defence in a real app.
async fn same_origin(request: Request, next: Next) -> Response {
    let safe = matches!(
        *request.method(),
        Method::GET | Method::HEAD | Method::OPTIONS
    );
    let headers = request.headers();
    let text = |name| {
        headers
            .get(name)
            .and_then(|v: &header::HeaderValue| v.to_str().ok())
    };
    let allowed = safe
        || match (text("sec-fetch-site"), text(header::ORIGIN.as_str())) {
            (Some(site), _) => site == "same-origin" || site == "none",
            (None, Some(origin)) => text(header::HOST.as_str()).is_some_and(|host| {
                origin
                    .split_once("://")
                    .is_some_and(|(_, authority)| authority.eq_ignore_ascii_case(host))
            }),
            (None, None) => true,
        };
    if allowed {
        next.run(request).await
    } else {
        (
            StatusCode::FORBIDDEN,
            "Cross-site form submissions are not accepted.",
        )
            .into_response()
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
        for uri in ["/orders", "/orders/1", "/orders/1/edit"] {
            let response = app(&path)
                .unwrap()
                .oneshot(
                    axum::http::Request::builder()
                        .uri(uri)
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
            assert!(
                body.contains("&lt;script&gt;alert(1)&lt;/script&gt;"),
                "{uri}"
            );
            assert!(!body.contains("<script>alert(1)</script>"), "{uri}");
        }
    }
}
