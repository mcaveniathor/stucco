# stucco-leptos

A preview of reactive Leptos components using Stucco's existing
styles and accessibility conventions. It provides a small set of form and page
components using Leptos directly; it does not hydrate the static `Render` builders.
This preview does not yet cover the full Stucco component catalogue. Its separate
prerelease version is **0.1.0-alpha.1**, published on crates.io; the existing
Stucco crates are at 0.5.1.

| Area | Available in this preview | Still missing |
| --- | --- | --- |
| Form controls | `TextField`, `Autocomplete`, `RepeatedField`, `FileInput`, `SubmitButton` | Selects, date inputs and other specialized controls |
| Form feedback | `ErrorSummary`, `StatusMessage` | Notification components |
| Status feedback | `Progress`, `Meter`, `Callout`, `EmptyState` | `LiveRegion`, `FlashMessages`, `ReadinessPanel` |
| Page composition | `PageHeader` with description and actions, `Dialog` | The remaining static component catalogue |
| Navigation | `NavLink`, `Tabs`, `Breadcrumbs`, `Steps`, `Pagination`, `CursorPagination` | |
| Tables | `Table`, `TableHead`, `TableBody`, `Row`, `Cell`, `HeaderCell` | `DataTable`, filters, search and sorting controls |
| Data displays | `Badge`, `Avatar`, `DescriptionList`, `Stat`, `Card`, `Panel`, `Timeline`, `Sparkline` | Result counts, icons on badges |

The integration pins Leptos **0.8.22** and requires Rust **1.88**; its native SSR
and WASM dependency graphs are checked on that compiler.
The existing Stucco crates retain their own compiler requirements.

## Dependency and build setup

Add these dependencies and feature forwards to the application's `Cargo.toml`:

```toml
[dependencies]
leptos = { version = "=0.8.22", default-features = false }
stucco-leptos = { version = "=0.1.0-alpha.1", default-features = false }
stucco-theme = { version = "=0.5.1", optional = true }

[features]
default = []
ssr = ["leptos/ssr", "stucco-leptos/ssr", "dep:stucco-theme"]
hydrate = ["leptos/hydrate", "stucco-leptos/hydrate"]
```

To follow unreleased changes from a Stucco checkout instead, add
`path = "../stucco/crates/stucco-leptos"` and
`path = "../stucco/crates/stucco-theme"` to those two dependencies, keeping the
exact versions.

The theme dependency is only needed by server code calling `stylesheet`.
`stucco-leptos` has no default features:

| Feature | Build target | Provides |
| --- | --- | --- |
| `ssr` | Native server | Leptos SSR and the shared `stylesheet` helper |
| `hydrate` | `wasm32-unknown-unknown` | Leptos hydration and browser event behavior |

Enable `ssr` for the native build and `hydrate` for the separate WASM build.
Do not combine those target modes or use `--all-features` for this crate.
This first preview targets SSR with hydration; client-only mounting is not yet a
supported or tested mode. Your application still owns its HTTP server, routing,
hydration entry point and asset delivery. The
[Orders example](https://github.com/mcaveniathor/stucco/tree/main/examples/orders-fullstack)
shows that full setup; its application code is a
checkout example, not a registry-installable application.

## Text fields

```rust
use leptos::prelude::*;
use stucco_leptos::TextField;

#[component]
fn CustomerField() -> impl IntoView {
    let customer = RwSignal::new("Ada Lovelace".to_owned());
    view! {
        <TextField
            id="order-customer"
            name="customer"
            label="Customer"
            value=Signal::derive(move || customer.get())
            on_input=Callback::new(move |text| customer.set(text))
            hint="Name shown on the order."
        />
    }
}
```

Pass a stable, page-unique `id`, and reserve its `-hint` and `-error` suffixes.
`error` accepts a `Signal<String>`; an empty string removes the error and its ARIA
reference. `required`, `disabled` and `read_only` accept `Signal<bool>`. `inputmode="numeric"`
suggests a numeric keyboard while preserving invalid input for server validation.
The caller must update its value from `on_input`, including when hydration adopts
text entered into the server-rendered input before WASM loaded.
`label` accepts Leptos `TextProp`, so a string, signal or closure can supply it.
`lang` and `dir` set native language/direction attributes; omitted values inherit
from the page. `error_prefix` defaults to `"Error: "` and can be translated.
For revision-sensitive forms, decide explicitly how browser-restored text relates
to the loaded revision. The Orders example uses `autocomplete="off"` on its form
so reloads fetch the saved draft; text newly entered before hydration is retained.

## Submit buttons

`SubmitButton` renders a submit button with these props:

| Prop | Type | Default |
| --- | --- | --- |
| `label` | `String` | Required |
| `pending` | `Signal<bool>` | `false` |
| `blocked` | `Signal<bool>` | `false` |
| `pending_label` | `String` | `"Saving…"` |
| `variant` | `SubmitVariant` | `SubmitVariant::Primary` |

`SubmitVariant` provides `Primary` and `Danger` styles. While pending, the button
shows its pending label, exposes `aria-disabled`, and prevents the default action
of clicks. It remains focusable so submitting does not discard keyboard focus.
The containing form must also guard submission while pending, including repeated
Enter presses and programmatic submissions; the button alone is not a duplicate
submission guard.
Use `blocked` when submission is unavailable for another reason, such as an
unknown create outcome. It preserves the normal label and focus without setting
`aria-busy`; explain the reason nearby and also guard the form's submit handler.

## Error summaries and status messages

`ErrorSummary` accepts a stable, page-unique `id: String` and these optional props:

| Prop | Type | Default |
| --- | --- | --- |
| `message` | `Signal<String>` | Empty string |
| `errors` | `Signal<Vec<SummaryError>>` | Empty vector |
| `focus_trigger` | `Signal<u32>` | `0` |

Each `SummaryError` has `message: String` and `target: Option<String>`. Use
`Some(field_id)` to link an error to its field, or `None` for an error without a
field target. Targets must be nonempty element IDs containing no whitespace or
control characters. Pass the ID itself, without `#` or percent encoding; the
component encodes the fragment internally.

The summary has `role="alert"` and the default heading **There is a problem**. Supply
`heading`, `lang` and `dir` to localize it. It is
hidden when both the message and error list are empty. After setting the errors
and message, increment `focus_trigger` to a positive value to request focus. A
positive initial value requests focus during hydration if focus is still on the
document body; subsequent increases
request focus again. This lets the form announce and focus each failed submission
without moving focus every time an individual field changes.

`StatusMessage` accepts `message: Signal<String>` and renders a neutral live region.
The region stays mounted even when its message is empty so later updates can be
announced. Use it for save progress or outcomes; provide field errors through
`TextField` and `ErrorSummary`.

## Page headers

`PageHeader` accepts `heading: String`, an optional `description: Option<String>`,
and optional `children: Option<Children>` for actions. It renders a static `h1`,
the description when supplied, and the action content in its action slot. These
components share Stucco's existing CSS; no separate Leptos styles are needed.

## Dialogs

`Dialog` requires `id`, `title`, `trigger_label`, `fallback_href` and `children`.
Its opener is an ordinary link before hydration. Serve equivalent content at the
fallback destination, including any native form actions. Once WASM loads, an
unmodified click opens a native modal through Rust's `web-sys` binding. The browser
provides focus containment, Escape dismissal and focus restoration. The close
button has a configurable `close_label` (default `"Close"`). Optional `lang` and
`dir` apply to the opener and dialog. The component reserves `{id}-title`.

```rust
use leptos::prelude::*;
use stucco_leptos::Dialog;

#[component]
fn Help() -> impl IntoView {
    view! {
        <Dialog id="shipping-help" title="Shipping information"
            trigger_label="Shipping help" fallback_href="/help/shipping">
            <p>"Orders ship after payment is confirmed."</p>
            <a href="/support">"Contact support"</a>
        </Dialog>
    }
}
```

If opening the modal fails, the link still follows its fallback destination.
Modified clicks retain browser link behavior. Put destructive confirmation logic
and pending/error states in the application; this component does not authorize
actions or prevent dismissing an application-owned pending operation.

## Autocomplete suggestions

`Autocomplete` takes the same core `id`, `name`, `label`, `value` and `on_input`
props as `TextField`, plus `options: Signal<Vec<AutocompleteOption>>`. Each option
has a `value: String` and optional explanatory `label: Option<String>`. `hint`,
`error`, `required`, `disabled`, `read_only`, `error_prefix`, `lang` and `dir` are
also supported. It reserves `{id}-options` as well as the text field suffixes.

The component uses native `<datalist>`, available before hydration. Users can
always type a value that is absent from the suggestions. Browser presentation and
assistive-technology support for suggestions vary; do not rely on the popup as
the only way to complete a form. Use a native select when the user must choose
from a closed set. Applications loading remote suggestions own loading/error
messages and stale response handling; validation remains a server responsibility.

## Repeated values

`RepeatedField` renders a labeled fieldset of controlled text inputs. It requires
`id`, `name`, `label`, `values: Signal<Vec<RepeatedValue>>`,
`on_input: Callback<(u64, String)>`, `on_add: Callback<()>` and
`on_remove: Callback<u64>`. A `RepeatedValue` contains `id: u64` and `value: String`.
IDs must be unique and stable across edits, removals and reorderings; allocate a
fresh ID in the application for each new row.

Callbacks synchronously update the caller's `values`. The component then focuses
the added row, or the following/previous row after removal. With no rows remaining,
focus moves to the add button. The defaults allow zero or more rows; `min_values`
and `max_values` constrain the hydrated buttons. Enforce those limits again in
server validation. `disabled` disables the whole group.

Each input submits the same `name`, so decode repeated native form keys as a list.
Existing rows remain editable and submittable before hydration; add/remove buttons
are disabled until WASM loads. Explain that enhancement with `hint`, or provide
application-owned native add/remove actions when changing the row count must work
without WASM. This component does not implement those application routes.

Translate `label`, `add_label` and `hint`; `item_label: Callback<usize, String>` and
`remove_label: Callback<usize, String>` format labels from each row's current
one-based position. Defaults are `Value 1`, `Remove value 1` and `Add value`.
`lang` and `dir` are inherited by the fields. Reserve `{id}-hint`, `{id}-add`, and
`{id}-{row_id}` with its text-field suffixes.

## File selection

`FileInput` requires `id`, `name` and `label`. It renders a native file picker with
optional `accept`, `multiple`, `required`, `disabled`, `hint`, `error`,
`error_prefix`, `lang` and `dir`. Use a containing form with `method="post"` and
`enctype="multipart/form-data"`. Include CSRF protection and apply authorization,
size/type limits and cleanup in the server's upload handler. `accept` only hints
which files the browser should show.

An optional `on_change: Callback<Vec<SelectedFile>>` receives selected metadata
after hydration and on each change. `SelectedFile` carries `name: String`,
`size: u64` in bytes and `media_type: String`. Format names, byte sizes and status
messages in the application, in its chosen locale. Metadata is untrusted browser
input. The component does not read file bytes, upload, retry or clear the picker;
the native input retains the files for form submission, including a selection
made before hydration. After a server response replaces the form, browsers require
the user to select the file again. A form reset does not emit a native `change`
event; an application resetting its form must also clear its displayed metadata.
Reserve `{id}-hint` and `{id}-error`.

## Navigation

Navigation components render the same markup as their static `stucco-ui`
counterparts, so the shared stylesheet already styles them. They are ordinary
links before and after hydration; hydration only makes their signal props
reactive. The application decides which link is current, usually from its router
or request path, and owns every destination. Unsafe URL schemes such as
`javascript:` render as `#`. `leptos_router` is not required; its link
interception works on these anchors.

```rust
use leptos::prelude::*;
use stucco_leptos::{BreadcrumbCurrent, BreadcrumbLink, Breadcrumbs, NavLink, Tabs};

#[component]
fn OrderNav(tab: Signal<String>) -> impl IntoView {
    view! {
        <Breadcrumbs>
            <BreadcrumbLink href="/orders">"Orders"</BreadcrumbLink>
            <BreadcrumbCurrent>"Order 42"</BreadcrumbCurrent>
        </Breadcrumbs>
        <Tabs label="Order sections">
            <NavLink label="Details" href="?tab=details"
                current=Signal::derive(move || tab.get() == "details")/>
            <NavLink label="History" href="?tab=history"
                current=Signal::derive(move || tab.get() == "history")/>
        </Tabs>
    }
}
```

- `Breadcrumbs` takes `BreadcrumbLink` items and a final `BreadcrumbCurrent`;
  its `label` defaults to "Breadcrumb". Place it above `PageHeader`.
- `Steps` takes `steps: Vec<StepItem>` and a `current` index. Finished steps
  link back when they have an `href`. The hidden "(completed)" and
  "(not started)" text is translatable with `completed_label` and
  `not_started_label`.
- `Pagination` takes `page`, `total_pages` and an `href` callback that builds
  each page's URL, keeping the rest of the query. It shows the first and last
  pages and two either side of the current one, and renders nothing for a single
  page. `page_label` formats each link's accessible name.
- `CursorPagination` takes optional `previous_href` and `next_href` and renders
  nothing when both are absent.

## Tables

`Table` renders the static table's markup: a keyboard-focusable scroll region
around a captioned `<table>`. Compose it from `TableHead` and `TableBody` children
holding `Row`s of `Cell`s and `HeaderCell`s. Cells take any view, and rows can come
from `<For>` or an iterator.

```rust
use leptos::prelude::*;
use stucco_leptos::{Cell, CellKind, HeaderCell, Row, Table, TableBody, TableHead};

#[component]
fn Orders(orders: Vec<(String, String)>) -> impl IntoView {
    view! {
        <Table caption="Orders" cards=true>
            <TableHead><Row>
                <HeaderCell aria_sort="ascending"><a href="?sort=customer">"Customer"</a></HeaderCell>
                <HeaderCell kind=CellKind::Number>"Total"</HeaderCell>
            </Row></TableHead>
            <TableBody>
                {orders.into_iter().map(|(customer, total)| view! {
                    <Row>
                        <HeaderCell label="Customer">{customer}</HeaderCell>
                        <Cell kind=CellKind::Number label="Total">{total}</Cell>
                    </Row>
                }).collect_view()}
            </TableBody>
        </Table>
    }
}
```

- `cards` presents rows as labelled cards below 48rem of container width. Give
  each body cell a `label` naming its column; a cell without one gets an empty
  caption, and labels are ignored when `cards` is off. The label is decoration
  (assistive technology hears the column heading), and explicit ARIA roles keep the
  table semantics when CSS changes the display.
- Sortable columns are ordinary links in a `HeaderCell` with `aria_sort` set to
  `"ascending"`, `"descending"` or `"none"`. The application sorts and builds the URLs.
- `DataTable` and its filter and search components are not included.

## Status and details

The data components present a record's status, people, fields and metrics with
the static components' markup. They render fully on the server, and signal props
update them after hydration.

```rust
use leptos::prelude::*;
use stucco_leptos::{
    Avatar, Badge, DescriptionItem, DescriptionList, Panel, Stat, StatChange, Tone, Trend,
};

#[component]
fn OrderSummary(orders: Signal<String>) -> impl IntoView {
    view! {
        <Panel id="order-1042" title="Order 1042">
            <DescriptionList>
                <DescriptionItem term="Customer">
                    <Avatar name="Ada Lovelace" decorative=true/>" Ada Lovelace"
                </DescriptionItem>
                <DescriptionItem term="Status">
                    <Badge tone=Tone::Success>"Paid"</Badge>
                </DescriptionItem>
            </DescriptionList>
        </Panel>
        <Stat label="Orders" value=orders
            change=Some(StatChange { text: "12%".into(), trend: Trend::Up })
            change_tone=Tone::Success context="vs last week"/>
    }
}
```

- `Badge` takes a `tone`; choose a label that says what the status means.
  Badges have no icon in this preview.
- `Avatar` shows initials with an optional `image` over them, and takes a `size`.
  The name labels it; use `decorative` when the name is written beside it.
- `DescriptionList` and `DescriptionItem` render a `<dl>`; details accept any view.
- `Stat` takes `label`, `value` and `change` as signals. A change stays neutral
  until you set `change_tone`. Translate the hidden direction text with `up_label`,
  `down_label` and `flat_label`.
- `Card` and `Panel` take `header`, `footer` and, for `Panel`, `description` and
  `actions` as closures returning views. `Panel` needs a page-unique `id` and
  reserves `{id}-title` for its heading; `level` sets the heading level.
- `Timeline` holds `TimelineItem`s with a machine-readable `datetime` and display
  `time`; with none it shows `empty_label`.
- `Sparkline` takes a `summary` and `values` signal; non-finite samples are ignored.
  `no_data_label` and `values_label` translate its text.

## Progress, meters and callouts

- `Progress` needs a page-unique `id` (its label uses it) and a `label`. A `value`
  shows determinate progress, clamped between 0 and `max` (default 100); leave it
  out for an indeterminate task. `show_value` also writes the percentage as text.
- `Meter` needs an `id`, `label`, `value`, `min` and `max`, and takes optional
  `low`, `high`, `optimum` and a `value_text` description. `{id}-text` is reserved.
  Out-of-range values and degenerate bounds are corrected the way the static meter
  corrects them.
- `Callout` takes a `tone`, an optional `title` and content. Place actions in a
  `<CalloutActions slot>` child. A visually hidden prefix names the tone ("Note:",
  "Success:", "Warning:", "Important:"); translate them with `note_label`,
  `success_label`, `warning_label` and `important_label`.
- `EmptyState` takes a `title` and optional `description` and `actions` closures.

## Styles and validation

On the server, generate the existing Stucco theme and standard component CSS:

```rust
use stucco_leptos::stylesheet;
use stucco_theme::Preset;

let css = stylesheet(Preset::Slate);
```

Serve that result once as `text/css`, for example at `/stucco.css`, and include
`<link rel="stylesheet" href="/stucco.css">` in the application's Leptos document.
The helper requires the `ssr` feature and emits no runtime script.
The WASM build does not depend on Stucco's static renderer or asset inventory.
Overlay styles come from the existing `stucco-ui` stylesheet. Language, direction
and control labels are application configuration; these components do not add a
translation runtime or a second theme system.

For controls without a Leptos wrapper, compose native elements using the same
styles: `st-form` on a form, `st-field` on a label/control group, `st-label` on its
label, and `st-select`, `st-textarea` or `st-input` on the respective control.
Connect each label with `for` and `id`; connect hints/errors with
`aria-describedby`, and set `aria-invalid="true"` when invalid. A styled native
element still needs its form `name` and application-owned value handling.

The packaged `examples/native_form.rs` renders a complete HTML document with a
native GET form, controlled `TextField`, `SubmitButton` and an ordinary `<select>`.
Its `--css` mode prints the shared stylesheet. From this repository, run:

```sh
cargo run -p stucco-leptos --locked --no-default-features --features ssr --example native_form
cargo run -p stucco-leptos --locked --no-default-features --features ssr --example native_form -- --css
```

The example prints output only; an application must serve the document and CSS
and implement `/search`. Its form works without WASM. To add hydration, render the
same component tree through the application's Leptos integration and hydrate it
with the same initial values. Mutating POST forms also need server-side
authorization, validation and CSRF protection.

Focused SSR checks:

```sh
cargo test -p stucco-leptos --locked --no-default-features --features ssr
cargo doc -p stucco-leptos --locked --no-default-features --features ssr --no-deps
cargo check -p stucco-leptos --locked --no-default-features --features hydrate --target wasm32-unknown-unknown
```

See [Orders full-stack](https://github.com/mcaveniathor/stucco/tree/main/examples/orders-fullstack) for a complete
SSR/hydration setup, native form integration and browser checks. Its `/components`
gallery demonstrates these controls in a separate module without Orders domain or
storage dependencies, including native GET form submission, Arabic labels, RTL
layouts and application-owned value formatting.
