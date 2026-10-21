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

## Show status and details

Compose an order detail page from a breadcrumb trail, a status badge, labelled fields and a static callout. This handler uses the default component features and the `axum` feature:

```rust
use stucco::prelude::*;

async fn order_detail(page: PageCx) -> Document {
    let header = PageHeader::new("Order 1042").breadcrumbs(
        Breadcrumbs::new()
            .link("Orders", "/orders")
            .current("Order 1042"),
    );
    let details = DescriptionList::new()
        .item("Customer", "Ada Lovelace")
        .item("Status", Badge::new("Paid").tone(Tone::Success))
        .item("Total", "$128.00")
        .item("Placed", "21 October 2026");
    let shipping = Callout::new(Tone::Info)
        .title("Preparing for dispatch")
        .child(Text::new("Your order will ship within two working days."))
        .actions(Link::new("Delivery information", "/delivery"));

    page.title("Order 1042").main(
        Stack::new()
            .space(Space::S6)
            .child(header)
            .child(details)
            .child(shipping),
    )
}
```

Load the order in your handler and use its values in place of these examples. The badge's text carries the status independently of colour. The callout is part of the page; it doesn't interrupt screen readers with a live announcement.

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
        Ok(_customer) => SeeOther::new("/orders").into_response(),
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

## Build a create/edit/delete flow

Store records in an `AsyncTable`, and carry each record's revision through its
edit form. If someone else saved the record in the meantime, the update is
refused and the user keeps what they typed. This is the edit half of the
[orders example](https://github.com/mcaveniathor/stucco/tree/main/examples/orders),
which also creates, lists and deletes:

```rust
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use stucco::forms::HiddenInput;
use stucco::prelude::*;
use stucco::server::{CsrfToken, SeeOther, Submission};
use stucco::{FormState, Validator};
use stucco_redb::{AsyncTable, Revision};

// Bad ids (`/customers/abc`) get the same page as missing ones, rather than
// axum's plain-text 400.
fn not_found(page: PageCx) -> Response {
    page.title("Not found")
        .main(Heading::new(1, "Customer not found"))
        .status(StatusCode::NOT_FOUND)
        .into_response()
}

fn edit_page(page: PageCx, id: u64, state: &FormState, revision: &str) -> Response {
    let token = page.context().get::<CsrfToken>().map(|t| t.as_str().to_owned());
    let status = if state.conflict().is_some() { StatusCode::CONFLICT }
        else if state.has_errors() { StatusCode::UNPROCESSABLE_ENTITY }
        else { StatusCode::OK };
    page.title("Edit customer")
        .main(
            Form::post(format!("/customers/{id}"))
                .csrf(token.as_deref().unwrap_or_default())
                .child(HiddenInput::new("revision", revision))
                .child(ErrorSummary::new(state).field("name", "customer-name"))
                .child(ConflictNotice::new(state, format!("/customers/{id}/edit")).focus(false))
                .child(Field::new("Name", Input::text("name").id("customer-name")).bind(state))
                .child(Button::new("Save").submit().variant(Variant::Primary)),
        )
        .status(status)
        .into_response()
}

async fn edit(
    State(db): State<AsyncTable<u64, String>>,
    page: PageCx,
    path: Result<Path<u64>, PathRejection>,
) -> Response {
    let Ok(Path(id)) = path else { return not_found(page) };
    match db.get_with_revision(id).await {
        Ok(Some((name, revision))) => {
            let state = FormState::new().with_value("name", name);
            edit_page(page, id, &state, &revision.to_string())
        }
        Ok(None) => not_found(page),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

async fn update(
    State(db): State<AsyncTable<u64, String>>,
    page: PageCx,
    path: Result<Path<u64>, PathRejection>,
    Submission(form): Submission,
) -> Response {
    let Ok(Path(id)) = path else { return not_found(page) };
    let submitted = form.value("revision").unwrap_or_default().to_owned();
    let mut v = Validator::new(form.clone());
    let name = v.text("name").required("Enter a name").get();
    let name = match v.finish(|| name) {
        Ok(name) => name,
        Err(state) => return edit_page(page, id, &state, &submitted),
    };
    // A malformed revision can't match anything: treat it as a conflict.
    let Ok(revision) = submitted.parse::<Revision>() else {
        let state = form.with_conflict(submitted.clone(), String::new());
        return edit_page(page, id, &state, &submitted);
    };
    match db.update_if(id, revision, move |_| name).await {
        Ok(_) => SeeOther::new("/customers")
            .flash(Flash::new(FlashLevel::Success, "Customer saved."))
            .into_response(),
        // Changed meanwhile: show the conflict, keeping the user's entries
        // and the revision they started from.
        Err(e) => match e.conflict_revision() {
            Some(Some(current)) => {
                let state = form.with_conflict(submitted.clone(), current.to_string());
                edit_page(page, id, &state, &submitted)
            }
            Some(None) => not_found(page),
            None => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        },
    }
}
```

The router needs `CsrfLayer` and `FlashLayer` (see [Forms](forms.html)).
Delete the same way: put the revision in the delete form and call
`remove_if`. To confirm first, put that form in a `Dialog` and open it with
`link_opener`, which links to a confirmation page where script doesn't run:

```rust
use stucco::prelude::*;

fn delete_order(id: u64, revision: &str, token: &str) -> impl Render {
    let dialog_id = format!("delete-order-{id}");
    let confirm = Dialog::new(&dialog_id, format!("Delete order {id}?"))
        .child(Text::new("The order and its history are removed."))
        .actions(Dialog::close_button(&dialog_id, "Cancel"))
        .actions(
            Form::post(format!("/orders/{id}/delete"))
                .csrf(token)
                .child(stucco::forms::HiddenInput::new("revision", revision))
                .child(Button::new("Delete order").submit().variant(Variant::Danger)),
        );
    (confirm.link_opener("Delete…", format!("/orders/{id}/delete")), confirm)
}
```

## Use a server-side theme preference without JavaScript

`Page::server_theme` renders an explicit scheme on the document and omits the
localStorage initializer. Pass `None` to follow the OS through CSS; pass a scheme
read from your own validated cookie or account preference to override it.

```rust
use stucco::{Bundle, Page, el};
use stucco::theme::{Preset, Scheme};
let bundle = Bundle::new(Preset::Slate);
let html = Page::new(&bundle, "Account")
    .server_theme(Some(Scheme::Dark))
    .body(el::main().text("Server-rendered content"))
    .render();
assert!(!html.contains("<script"));
```

This does not disable component behavior: enhanced pages and components that need
scripts still load those scripts. `el::svg`, `el::path`, `el::circle`, `el::line`,
`el::polyline`, and `el::text` also support server-rendered charts with escaped
attributes and labels, without a charting runtime.
