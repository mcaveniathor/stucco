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
use std::path::Path;
use stucco::prelude::*;
use stucco::server::{CollectionSource, RequestContext};
use stucco_redb::{IndexTable, PostcardCodec, RedbCollection, Store, StoreError, U64Key};

type OrdersTable = IndexTable<u64, Order>;
type OrdersSource = RedbCollection<u64, Order, U64Key, PostcardCodec<Order>, OrdersMapping>;
#[derive(Clone)]
struct AppState {
    cursor: OrdersSource,
    pages: OrdersSource,
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
    let cursor = RedbCollection::new(open(db_path)?, OrdersMapping { pages: false });
    let pages = cursor.with_mapping(OrdersMapping { pages: true });
    let state = AppState { cursor, pages };
    Ok(Router::new()
        .route("/", get(|| async { Redirect::to("/orders") }))
        .route("/orders", get(list))
        .with_state(state)
        .stucco(Preset::Slate))
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
                    Col::number("total", "Total", |row: &Order| {
                        row.total_cents as f64 / 100.0
                    })
                    .display(|row: &Order| {
                        format!("${}.{:02}", row.total_cents / 100, row.total_cents % 100)
                    })
                    .filter(),
                )
                .column(
                    Col::date("created", "Created", |row: &Order| row.created.clone()).filter(),
                );
            cx.title("Orders — stucco").app(
                AppShell::new()
                    .header(
                        PageHeader::new("Orders")
                            .description("A persistent collection with ordinary GET navigation."),
                    )
                    .link(NavLink::new("Orders", "/orders").current(!pages))
                    .link(NavLink::new("Numbered pages", "/orders?mode=pages").current(pages))
                    .main(table)
                    .footer(Footer::new().child(el::p().text("Read-only demo data"))),
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
