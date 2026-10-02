# Recipes

Short answers to "how do I…?". Each one is a complete handler or a few lines you can drop into an app built with the [getting started](getting-started.html) setup.

## Start a new app

The CLI writes a working app: axum, a theme, a shared layout, a home page and a table.

```sh
cargo install stucco-cli
stucco new my-app --name my-app   # a theme derived from the app's name
cd my-app && cargo run
```

`--preset`, `--seed` and `--random` pick the theme instead. `--git` takes stucco from its repository, for features not yet released.

## Share a layout between pages

Write the shell once, and mark the current page from each handler:

```rust
use stucco::prelude::*;

fn shell<'a>(current: &str) -> AppShell<'a> {
    AppShell::new()
        .header(PageHeader::new("Shop"))
        .link(NavLink::new("Orders", "/orders").current(current == "/orders"))
        .link(NavLink::new("Customers", "/customers").current(current == "/customers"))
}

async fn customers(page: PageCx) -> Document {
    page.title("Customers").app(shell("/customers").main(Heading::new(2, "Customers")))
}
```

## Handle a form with errors

Read the submission, validate it, and either redirect or show the form again with its values and errors, answering 422:

```rust
use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use stucco::prelude::*;
use stucco::FormState;
use stucco::Validator;
use stucco::server::{SeeOther, Submission};

fn order_form(state: &FormState) -> impl Render + '_ {
    Form::post("/orders")
        .child(ErrorSummary::new(state).field("customer", "order-customer"))
        .child(Field::new("Customer", Input::text("customer").id("order-customer")).bind(state))
        .child(Button::new("Create order").submit().variant(Variant::Primary))
}

async fn create(page: PageCx, Submission(form): Submission) -> Response {
    let mut v = Validator::new(form);
    let customer = v.text("customer").required("Enter a customer name").get();
    match v.finish(|| customer) {
        Ok(_customer) => IntoResponse::into_response(SeeOther::new("/orders")),
        Err(state) => page
            .title("New order")
            .main(order_form(&state))
            .status(StatusCode::UNPROCESSABLE_ENTITY)
            .into_response(),
    }
}
```

See [Forms](forms.html) for every check and control.

## Show a searchable, filterable table

Derive the columns from your row type, implement `CollectionSource` for your storage, and load a page per request. The query string carries the search, filters, sort and page, so every view has a URL:

```rust
use axum::extract::{RawQuery, State};
use stucco::prelude::*;
use stucco::server::RequestContext;

#[derive(Columns)]
struct Customer {
    #[col(sortable, searchable)]
    name: String,
    #[col(enumeration("active", "paused"), filter)]
    status: String,
}

async fn customers(
    State(source): State<CustomerSource>, // your `CollectionSource`
    RawQuery(raw): RawQuery,
    context: RequestContext,
    page: PageCx,
) -> Document {
    let raw = raw.unwrap_or_default();
    match source.load(&raw, &Customer::column_specs(), &context).await {
        Ok(customers) => page.title("Customers").main(
            DataTable::from_collection(&customers, "Customers")
                .action("/customers")
                .columns(Customer::columns()),
        ),
        Err(_) => page.title("Customers unavailable").status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

[Collections](collections.html) covers `CollectionSource`; `stucco-redb` implements it for embedded storage.

## Update part of a page without reloading

Give the region an id, and answer enhanced requests for it with just that region. Everything else, including browsers without JavaScript, gets the full page:

```rust
use stucco::prelude::*;

fn cart(items: usize) -> impl Render {
    el::div().id("cart").child(Text::new(format!("{items} items")))
}

async fn shop(page: PageCx) -> axum::response::Response {
    let items = 3;
    page.respond()
        .fragment("cart", || cart(items))
        .page(|| page.title("Shop").main(Stack::of((Heading::new(1, "Shop"), cart(items)))))
}
```

See [Progressive enhancement](enhancement.html) for how the browser side asks for a fragment.

## Use a second theme on part of a page

Add a named theme to the bundle, then wrap a subtree in a `ThemeScope` with that name:

```rust
use stucco::prelude::*;
use stucco::layout::ThemeScope;

let bundle = Bundle::new(Preset::Slate).with_theme("promo", Theme::seeded_str("summer sale"));
let banner = ThemeScope::new()
    .named("promo")
    .child(Card::new().child(Heading::new(2, "Summer sale")));
let app = axum::Router::new().stucco(bundle);
```

## Write static pages without a server

Render pages to strings and write the bundle's files next to them:

```rust
use std::fs;
use stucco::prelude::*;

let bundle = Bundle::new(Preset::Slate);
fs::create_dir_all("public/_stucco")?;
for path in bundle.paths() {
    let file = bundle.get(&path).expect("listed paths exist");
    fs::write(format!("public{path}"), &*file.bytes)?;
}
let html = Page::new(&bundle, "Hello").main(Heading::new(1, "Hello")).render();
fs::write("public/index.html", html)?;
```

## Change the request limits

`stucco` uses a 1 MiB body limit and a 30 second timeout. `stucco_with` takes your own:

```rust
use std::time::Duration;
use stucco::prelude::*;
use stucco::server::LayerConfig;

let config = LayerConfig { body_limit: 10 * 1024 * 1024, timeout: Duration::from_secs(60) };
let app = axum::Router::new().stucco_with(Preset::Slate, &config);
```

## Confirm before deleting

Put the delete form in a dialog, and open it from the row's button. The form posts as usual; Cancel, Escape or a click outside closes it:

```rust
use stucco::prelude::*;

fn delete_order(id: u64) -> impl Render {
    let dialog_id = format!("delete-order-{id}");
    let confirm = Dialog::new(&dialog_id, format!("Delete order {id}?"))
        .child(Text::new("The order and its history are removed."))
        .actions(Dialog::close_button(&dialog_id, "Cancel"))
        .actions(
            Form::post(format!("/orders/{id}/delete"))
                .child(Button::new("Delete order").submit().variant(Variant::Danger)),
        );
    (confirm.opener("Delete").variant(Variant::Ghost), confirm)
}
```

After the redirect, show a `Toast::new("Order deleted").tone(Tone::Success)` on the page it lands on.
