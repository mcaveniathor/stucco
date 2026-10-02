# Collections

Turn a list of records into a searchable, filterable, sortable, paginated table that works through ordinary GET requests.

Enable the `collections` feature:

```toml
stucco = { version = "0.1", features = ["collections"] }
```

## A table from columns

Declare columns once. Each has a key, a label, a kind and a way to read the value from a row:

```rust
use stucco::collections::{Col, DataTable};
use stucco::CollectionPage;

let page = CollectionPage { rows: orders, next: None, prev: None, total: None };
let table = DataTable::from_page(&page, "Orders")
    .column(Col::number("id", "ID", |o: &Order| o.id as f64).sortable())
    .column(Col::text("customer", "Customer", |o: &Order| o.customer.clone()).sortable().searchable())
    .column(Col::enumeration(
        "status",
        "Status",
        vec![("pending".into(), "Pending".into()), ("paid".into(), "Paid".into())],
        |o: &Order| o.status.clone(),
    ).filter())
    .column(
        Col::number("total", "Total", |o: &Order| o.total_cents as f64 / 100.0)
            .display(|o: &Order| format!("${}.{:02}", o.total_cents / 100, o.total_cents % 100))
            .filter(),
    )
    .column(Col::date("created", "Created", |o: &Order| o.created.clone()).filter());
```

## Columns from a struct

With the `derive` feature, `#[derive(Columns)]` builds the columns from a row type's fields instead, one per field in order. The same columns then give the query parser its column list, so the table and the parser can't disagree:

```toml
stucco = { version = "0.1", features = ["derive"] }
```

```rust
use stucco::prelude::*;

#[derive(Columns)]
struct Order {
    #[col(label = "ID", sortable)]
    id: u64,
    #[col(sortable, searchable)]
    customer: String,
    #[col(enumeration("pending", "paid", "shipped"), filter)]
    status: String,
    #[col(key = "total", label = "Total", value = dollars, display = money, filter)]
    total_cents: u64,
    #[col(date, filter)]
    created: String,
}

fn dollars(o: &Order) -> f64 { o.total_cents as f64 / 100.0 }
fn money(o: &Order) -> String { format!("${}.{:02}", o.total_cents / 100, o.total_cents % 100) }

let table = DataTable::from_collection(&orders, "Orders").columns(Order::columns());
let specs = Order::column_specs(); // for CollectionQuery::parse or CollectionSource::load
```

A field's kind comes from its type, numbers for numeric primitives and text for strings, or from `#[col(text)]`, `#[col(number)]`, `#[col(date)]` or `#[col(enumeration(...))]`. A field whose kind can't be told from its type is a compile error that says which options to add.

| Option | Effect |
| --- | --- |
| `key = "k"` | The query key (default: the field name) |
| `label = "L"` | The heading (default: the field name as words, so `total_cents` is "Total cents") |
| `value = path` | A function giving the value that sorts and filters, in place of the field |
| `display = path` | A function giving how cells read |
| `sortable`, `searchable`, `filter` | Enable that control |
| `skip` | No column for this field |

| Column | Filter control | Display |
| --- | --- | --- |
| `Col::text` | Text input; can join shared search | As text |
| `Col::number` | From–to range | End-aligned, tabular figures |
| `Col::date` | From–to date range (ISO dates) | Tabular figures |
| `Col::enumeration` | Select of the allowed values | A tag with `data-value` for styling |
| `Col::custom` | None | Any `Render` content |

`display` changes how cells look without changing the value that sorts and filters. The table renders a filter bar, the table itself in a scrolling card, a result count in a live region, and pagination.

## Capabilities

A column marked `sortable`, `searchable` or `filter` only gets a control when the data source supports that operation. Sources report what they can do with `Capabilities`:

```rust
use stucco::Capabilities;

let caps = Capabilities {
    sortable: vec!["id".into(), "customer".into()],
    filterable: vec!["status".into(), "total".into(), "created".into()],
    searchable: true,
    total_count: false, // cursor pagination: no "Showing 1–25 of 67"
    offset: false,      // no numbered pages
};
```

Pass them to the table with `.capabilities(&caps)`, along with `.query(&query)` and `.action("/orders")`.

## Queries

`CollectionQuery::parse` turns the request's query string into validated state: the sort key and direction, the search text, typed filters, the page size (1 to 100) and the page position. It never rejects a request: unknown sort keys, malformed filters and garbage cursors are dropped, so a stale or hand-edited URL still shows a sensible page.

Every control is a link or a GET form, so the URL always describes what's on screen. Bookmark it, share it, or press back.

## Loading pages

`stucco-tower` defines `CollectionSource`, a storage-independent trait that turns a query into a `CollectionPage`:

```rust
use stucco_tower::{CollectionSource, RequestContext, SourceError};

impl CollectionSource for OrdersSource {
    type Row = Order;

    fn capabilities(&self) -> Capabilities { /* … */ }

    async fn query(
        &self,
        query: &CollectionQuery,
        cx: &RequestContext,
    ) -> Result<CollectionPage<Order>, SourceError> {
        // run the query against your database
    }
}
```

`RequestContext` carries the request id and any request extensions, such as the signed-in user, so the source can scope results.

In a handler, `load` parses the request's query string and reads the page in one call. It returns a `Collection`: the query, the capabilities and the page, which `DataTable::from_collection` takes whole.

```rust
use stucco::prelude::*;
use stucco::server::RequestContext;

async fn orders(
    State(source): State<OrdersSource>,
    RawQuery(raw): RawQuery,
    context: RequestContext,
    page: PageCx,
) -> Document {
    let raw = raw.unwrap_or_default();
    match source.load(&raw, &columns(), &context).await {
        Ok(orders) => page.title("Orders").main(
            DataTable::from_collection(&orders, "Orders")
                .action("/orders")
                .column(Col::text("customer", "Customer", |o: &Order| o.customer.clone()).searchable()),
        ),
        Err(_) => page.title("Orders unavailable").status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

`columns()` lists each column's key and kind as `ColumnSpec`s, so the parser knows which filters are valid.

## Pagination

Two modes:

- **Cursor** (the default): opaque, query-scoped cursors for Next and Previous. Scales to large tables; the total is unknown.
- **Numbered**: pages 1, 2, 3 with a total. The source must count matching rows, so use it for small collections.

Cursors are locators, not authorisation tokens: always scope queries to what the requester may see.

## Embedded storage

`stucco-redb` stores typed records in [redb](https://github.com/cberner/redb) with transactional secondary indexes and a `CollectionSource` implementation whose blocking scans run within a bounded budget. The [orders example](https://github.com/mcaveniathor/stucco/tree/main/examples/orders) shows the complete flow with 67 seeded records.
