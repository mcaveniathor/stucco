# Collections

Turn a list of records into a searchable, filterable, sortable, paginated table that works through ordinary GET requests.

Enable the `collections` feature:

```toml
stucco = { version = "0.2", features = ["collections"] }
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
stucco = { version = "0.2", features = ["derive"] }
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
| `link = path` | A function giving each cell's link (such as the record's page) |
| `sortable`, `searchable`, `filter` | Enable that control |
| `skip` | No column for this field |

| Column | Filter control | Display |
| --- | --- | --- |
| `Col::text` | Text input; can join shared search | As text |
| `Col::number` | From–to range | End-aligned, tabular figures |
| `Col::date` | From–to date range (ISO dates) | Tabular figures |
| `Col::enumeration` | Select of the allowed values | A tag with `data-value` for styling |
| `Col::custom` | None | Any `Render` content |
| `Col::actions` | None | Per-row links or small forms, end-aligned |

`display` changes how cells look without changing the value that sorts and filters, and `href` links each cell (call it after `display`). `wrap` lets long text wrap instead of widening the table. The table renders a filter bar, the active filters, the table itself in a scrolling card, a result count in a live region, and pagination.

## Rows, actions and selection

- `row_id(|o| o.id.to_string())` puts the record's key on each `<tr>` as `data-row-id`. Use the key, not the position, so it survives sorting and paging.
- `Col::actions` holds per-row actions. Name each for its row ("Edit order 42", with visually hidden text or `aria-label`), and leave out actions the user may not take; your handlers still check, since hiding a link is not a permission check.
- `selectable("bulk", "id", |o| format!("order {}", o.id))` adds a labelled checkbox to each row, submitted as `id=<row id>` with the form whose id is `bulk`. The checkboxes join that form through their `form` attribute, so the form can sit anywhere on the page and selection needs no JavaScript. Only the visible page can be selected; nothing implies "every matching record".

Make the bulk form a GET that leads to a confirmation page naming the action and how many records it affects ("Archive 3 orders?"). The confirmation posts the ids; the handler checks each one again, since records may have changed or vanished, and says what it did ("Archived 2 orders. 1 had already been deleted.").

## Empty, filtered and failed

A table with no rows tells three situations apart:

| Situation | Shows |
| --- | --- |
| The collection is empty | "No orders yet", or your own `empty(EmptyState …)` with the action that adds the first record |
| A search or filter matches nothing | "No matching orders", with a link that clears the search and filters |
| A page past the end (a stale link, or the last row was deleted) | "Nothing on this page", with a link to the first page and the pagination |

When loading fails, there is no table to show: render a `Notice::danger` that says to try again and gives the request id, with status 500.

The active filters appear above the table, each with a link that removes just that one, and a "Clear all" link. Every link keeps the sort and page size and returns to the first page.

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

### The query contract

| Parameter | Meaning | Accepted |
| --- | --- | --- |
| `sort` | Sort key | A column key the source lists in `sortable` and the columns declare |
| `dir` | Direction | `asc` (default) or `desc` |
| `q` | Shared search | Up to 256 characters, when the source is `searchable` |
| `f.<key>` | Text or enumeration filter | Text up to 256 characters; one of the declared values |
| `f.<key>.min`, `f.<key>.max` | Inclusive number or date range | Finite numbers; real `YYYY-MM-DD` dates; min ≤ max |
| `per` | Page size | 1–100 (default 25; larger values are capped) |
| `page` | Numbered page | 1 or more, when the source supports `offset` |
| `after`, `before` | Cursor position | An opaque cursor; both at once means the first page |

Anything else is ignored: unknown parameters, keys the source doesn't support, malformed values and query strings over 16 KiB. A control only appears for an operation the source declares, so the page never offers what the backend can't do.

`to_query_string` writes a query in one canonical order (`sort`, `dir`, `q`, filters by key, `per`, then the position), so equal queries give equal URLs. `link(&action)` keeps the action's unrelated parameters (such as `mode=pages`) and its fragment.

Changing the sort, search, a filter or the page size resets the position to the first page (`with_sort`, `with_search`, `with_filter`, `with_per_page`); changing only the position keeps the rest (`with_window`). `is_filtered` says whether a search or filter narrows the results.

To come back to the same list from a record, link to the record with the list's canonical URL (`query.link(&action)`) as a parameter, and check that parameter is one of your own paths before using it.

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

The result count never invents a total: with an unknown total it says "Showing 25 results" (the rows on this page), and only a source with `total_count` gets "Showing 26–50 of 67". Pagination never needs the whole dataset in memory, but what a page costs depends on the source: `stucco-redb`'s cursor mode may scan many records to fill a filtered page, and its numbered mode reads every matching record to count them.

Cursors are locators, not authorisation tokens: always scope queries to what the requester may see.

## Embedded storage

`stucco-redb` stores typed records in [redb](https://github.com/cberner/redb) with transactional secondary indexes and a `CollectionSource` implementation whose blocking scans run within a bounded budget. The [orders example](https://github.com/mcaveniathor/stucco/tree/main/examples/orders) shows the complete flow with 67 seeded records.
