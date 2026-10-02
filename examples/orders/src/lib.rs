//! Persistent, server-rendered orders. See the example README for scan costs.
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
use std::{path::Path, sync::Arc};
use stucco::{
    Bundle, CollectionQuery, Page,
    app::{AppShell, Footer, PageHeader},
    collections::{Col, DataTable},
    el,
    theme::Preset,
};
use stucco_redb::{IndexTable, PostcardCodec, RedbCollection, Store, StoreError, U64Key};
use stucco_tower::{
    CollectionSource, LayerConfig, PageResponse, RequestContext, assets_router,
    with_standard_layers,
};

type OrdersTable = IndexTable<u64, Order>;
#[derive(Clone)]
struct AppState {
    table: OrdersTable,
    bundle: Arc<Bundle>,
}

fn open(path: &Path) -> Result<OrdersTable, StoreError> {
    let store = Store::open(path)?;
    let table = store
        .table("orders", U64Key, PostcardCodec::<Order>::default())?
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
/// Opens persistent storage and constructs the read-only router.
pub fn app(db_path: &Path) -> Result<Router, StoreError> {
    let state = AppState {
        table: open(db_path)?,
        bundle: Arc::new(Bundle::new(Preset::Slate)),
    };
    let routes = Router::new()
        .route("/", get(|| async { Redirect::to("/orders") }))
        .route("/orders", get(list))
        .with_state(state.clone());
    Ok(with_standard_layers(
        routes.merge(assets_router(state.bundle)),
        &LayerConfig::default(),
    ))
}
async fn list(
    State(state): State<AppState>,
    RawQuery(raw): RawQuery,
    context: RequestContext,
) -> PageResponse {
    let raw = raw.unwrap_or_default();
    let pages = form_urlencoded::parse(raw.as_bytes()).any(|(k, v)| k == "mode" && v == "pages");
    let source = RedbCollection::new(state.table, OrdersMapping { pages });
    let caps = source.capabilities();
    let query = CollectionQuery::parse(&raw, &caps, &source::columns());
    match source.query(&query, &context).await {
        Ok(page) => {
            let table = DataTable::from_page(&page, "Orders")
                .action(if pages {
                    "/orders?mode=pages"
                } else {
                    "/orders"
                })
                .query(&query)
                .capabilities(&caps)
                .column(Col::number("id", "ID", |row: &Order| row.id as f64).sortable())
                .column(
                    Col::text("customer", "Customer", |row: &Order| row.customer.clone())
                        .sortable()
                        .searchable(),
                )
                .column(
                    Col::enumeration(
                        "status",
                        "Status",
                        ["pending", "paid", "shipped"]
                            .map(|s| (s.into(), s.into()))
                            .to_vec(),
                        |row: &Order| row.status.clone(),
                    )
                    .filter(),
                )
                .column(
                    Col::number("total_cents", "Total (cents)", |row: &Order| {
                        row.total_cents as f64
                    })
                    .filter(),
                )
                .column(
                    Col::date("created", "Created", |row: &Order| row.created.clone()).filter(),
                );
            PageResponse::new(
                Page::new(&state.bundle, "Orders — stucco")
                    .body(
                        AppShell::new()
                            .header(PageHeader::new("Orders").description(
                                "A persistent collection with ordinary GET navigation.",
                            ))
                            .sidebar(
                                el::nav()
                                    .aria("label", "Application")
                                    .child(el::a().href("/orders").text("Orders"))
                                    .child(
                                        el::a().href("/orders?mode=pages").text("Numbered pages"),
                                    ),
                            )
                            .main(table)
                            .footer(
                                Footer::new().child(el::p().text("Read-only orders demonstration")),
                            ),
                    )
                    .render(),
            )
        }
        Err(error) => {
            tracing::error!(%error, request_id = %context.request_id, "orders query failed");
            PageResponse::new(
                Page::new(&state.bundle, "Orders unavailable")
                    .body(
                        AppShell::new().main(PageHeader::new("Orders unavailable").description(
                            format!("Please try again. Request ID: {}", context.request_id),
                        )),
                    )
                    .render(),
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
        let response = app(&path)
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
