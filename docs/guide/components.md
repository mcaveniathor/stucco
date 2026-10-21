# Components

Every stucco component is a Rust value that renders semantic HTML. Builders set options, `Render` produces markup, and the page collects the CSS each component needs.

## Families and features

Components are grouped into families, each behind a cargo feature. The facade enables the common ones by default; turn off `default-features` to pick exactly what you compile.

| Feature | Components |
| --- | --- |
| `layout` | `Stack`, `Cluster`, `Grid`, `Center`, `Container`, `Sidebar`, `Switcher`, `Surface`, `ThemeScope`, `SkipLink`, `Disclosure`, `Accordion` |
| `typography` | `Heading`, `Text`, `Link`, `Code`, `Kbd` |
| `actions` | `Button`, `ButtonLink`, `IconButton` |
| `forms` | `Form`, `Field`, `Input`, `Select`, `Textarea`, `Checkbox`, `RadioGroup`, `Fieldset`, `ErrorSummary`, `ConflictNotice` |
| `feedback` | `EmptyState`, `LiveRegion`, `ReadinessPanel`, `Requirement`, `Progress`, `Meter`, `Callout` |
| `navigation` | `Pagination`, `NavLink`, `Tabs`, `Breadcrumbs`, `Steps` |
| `overlay` | `Dialog`, `Menu`, `Toast`, `Tooltip` |
| `data` | `Card`, `Panel`, `Table`, `ResultCount`, `Badge`, `Avatar`, `DescriptionList`, `Stat` |
| `app` | `AppShell`, `PageHeader`, `SectionHeader`, `Footer` |
| `collections` | `DataTable`, `Col`, `FilterBar`, `SearchForm`, `SortControl` |
| `icons` | The Lucide icon catalog |

The [gallery](../gallery/) shows every component in every state.

## Composing

Layout primitives take children and space them with one setting, so pages stay consistent without per-component margins:

```rust
use stucco::layout::{Cluster, Stack};
use stucco::actions::Button;
use stucco::typography::{Heading, Text};
use stucco::{Space, Variant};

let card = Stack::new()
    .space(Space::S4)
    .child(Heading::new(2, "Invite your team"))
    .child(Text::new("Members can view and edit every project."))
    .child(
        Cluster::new()
            .space(Space::S2)
            .child(Button::new("Send invites").variant(Variant::Primary))
            .child(Button::new("Not now")),
    );
```

`el` builds any element directly, for markup no component covers: `el::section().class("hero").child(...)`.

## Attributes and safety

Every component accepts passthrough attributes: `.class()`, `.id()`, `.attr()`, `.data()` and `.aria()`. Attributes a component owns, such as a button's `type`, are reserved: setting one panics in debug builds and is ignored in release, so a component's behaviour can't be broken by accident.

Rendering is safe by default:

- Text is always escaped.
- URLs go through `Href`, which accepts relative URLs and `http`, `https`, `mailto` and `tel`, and renders anything else, such as `javascript:`, as `#`.
- Generated ids are unique within a render, and duplicate ids are caught in debug builds.
- `Raw::trusted` is the one way to insert markup as-is, and its name says what it means.

## Writing your own

Implement `Render` to make a component. Ask the context for unique ids and for any assets the markup needs:

```rust
use stucco::{Cx, Render, el};

struct Disclosure<'a> {
    summary: &'a str,
    body: &'a str,
}

impl Render for Disclosure<'_> {
    fn render(&self, cx: &mut Cx) {
        let id = cx.id("disclosure");
        el::details()
            .id(id)
            .child(el::summary().text(self.summary))
            .child(el::p().text(self.body))
            .render(cx);
    }
}
```

For one-off markup, `render_fn(|cx| ...)` wraps a closure as a component.

## Application shell

`AppShell` lays out a page with a header, navigation, the main content and a footer. It has two kinds of navigation:

- **Primary links**, added with `link`, are the app's top-level pages. They are a flat list of `NavLink`s in a `<nav>` landmark.
- **The sidebar**, set with `sidebar`, holds anything else: section links, filters or a nested outline. It sits in a disclosure so it can collapse.

```rust
use stucco::app::{AppShell, PageHeader};
use stucco::navigation::NavLink;

let shell = AppShell::new()
    .header(PageHeader::new("Orders"))
    .link(NavLink::new("Orders", "/orders").current(true))
    .link(NavLink::new("Customers", "/customers"))
    .main(/* … */);
```

The theme's shell layout decides where the primary links go: at the top of the side column, or in a row under the header. The sidebar is always a column beside the content, and narrow screens stack everything.

## Status and details

The `data` family presents a record's status, people, fields and metrics:

```rust
use stucco::prelude::*;

let status = Badge::new("Paid").tone(Tone::Success);
let customer = Cluster::of((
    Avatar::new("Ada Lovelace").image("/people/ada.jpg").decorative(),
    Text::new("Ada Lovelace"),
));
let details = DescriptionList::new()
    .item("Customer", customer)
    .item("Status", status)
    .item("Total", "$128.00");
let orders = Stat::new("Orders", "128")
    .change("12%", Trend::Up)
    .change_tone(Tone::Success)
    .context("vs last week");
```

- **`Badge`** is a short text label with an optional decorative `icon`. Choose a label that says what the status means; `tone` reinforces it with colour.
- **`Avatar`** shows initials, with an optional image over them, and accepts `size`. The name labels the avatar; use `decorative` when the same name is written beside it. Initials remain underneath if the image fails to load.
- **`DescriptionList`** renders term/detail pairs as a `<dl>`. Details accept any `Render` value, including badges and links. Terms stack above details in narrow containers and sit beside them from 32rem of container width.
- **`Stat`** shows a label and a formatted value. `change` takes text and `Trend::Up`, `Down` or `Flat`, with hidden text conveying the direction. Changes stay neutral until you set `change_tone`: a rising number can be good or bad. `context` explains the comparison.

See [Show status and details](recipes.html#show-status-and-details) for a complete order page.

## Where you are

The `navigation` family offers a trail through the site and a position within a flow:

```rust
use stucco::prelude::*;

let header = PageHeader::new("Order 1042").breadcrumbs(
    Breadcrumbs::new()
        .link("Orders", "/orders")
        .current("Order 1042"),
);
let checkout = Steps::new("Checkout")
    .step("Cart", "/checkout/cart")
    .step("Delivery", "/checkout/delivery")
    .step_disabled("Payment")
    .current(1);
```

`Breadcrumbs` is a navigation landmark labelled "Breadcrumb" by default; `label` changes that name. Ancestors are links and the current page is text marked `aria-current="page"`. CSS draws the separators, so assistive technology doesn't read them.

`Steps::current` takes a zero-based index and clamps past the end to the last step. Completed steps link back, the current step is text marked `aria-current="step"`, and later steps are text. `step_disabled` never links, even when completed. Completed and future states also have hidden text. Applications own the flow's eligibility checks and routes.

## Progress

Use `Progress` for work advancing toward completion and `Meter` for a measurement within a known range. Both belong to `feedback` and render native HTML controls with visible labels:

```rust
use stucco::feedback::{Meter, Progress};

let import = Progress::new("Importing orders", 42.0, 100.0).show_value();
let waiting = Progress::indeterminate("Preparing import");
let storage = Meter::new("Storage used", 7.2, 0.0, 10.0)
    .low(2.0)
    .high(8.0)
    .optimum(0.0)
    .value_text("7.2 of 10 GB");
```

`Progress::show_value` displays the percentage beside a determinate bar. Use `indeterminate` while the total amount of work is unknown. `Meter` uses `low`, `high` and `optimum` to describe its bands; `value_text` adds visible text linked with `aria-describedby`. Keep updates and any live announcements under application control.

Values clamp to the range, and non-finite values become its minimum. An invalid progress maximum becomes 1; a non-finite meter minimum becomes 0, and a meter maximum that isn't finite and above its minimum falls back to `min + 1`.

## Disclosure and callouts

`Disclosure` and `Accordion`, in `layout`, use native `<details>` and `<summary>`, so opening and closing works without JavaScript:

```rust
use stucco::prelude::*;

let shipping = Disclosure::new("Shipping details")
    .heading(2)
    .open(true)
    .child(Text::new("Arrives by Friday."));
let help = Accordion::new()
    .heading(2)
    .item_open("Shipping", Text::new("Tracked delivery is included."))
    .item("Returns", Text::new("Return within 30 days."))
    .exclusive();
let note = Callout::new(Tone::Warning)
    .title("Payment overdue")
    .child(Text::new("Pay the invoice before we can ship your order."))
    .actions(Link::new("View invoice", "/orders/1042/invoice"));
```

`Disclosure::open(true)` and `Accordion::item_open` choose the initial open state. `heading` puts each summary in a heading, clamped to levels 2–6. `Accordion::exclusive` gives the details a shared name so supporting browsers keep at most one open; other browsers still offer independent disclosures.

`Callout`, in `feedback`, is a static note with a tone, an optional title, body children and actions. Its built-in icon is decorative, and hidden text names non-neutral tones. It has no live role: use `FlashMessages` after a redirect or `LiveRegion` for changes that need announcing.

## Overlays

The `overlay` family, on by default, puts content above the page. Each component is native HTML first, and one small behaviour script, loaded only on pages that use them, fills the gaps.

```rust
use stucco::prelude::*;

let confirm = Dialog::new("delete-order", "Delete order 1042?")
    .child(Text::new("The order and its history are removed."))
    .actions(Dialog::close_button("delete-order", "Cancel"))
    .actions(
        Form::post("/orders/1042/delete")
            .child(Button::new("Delete order").submit().variant(Variant::Danger)),
    );
let row_actions = Menu::new("Actions")
    .link("Edit", "/orders/1042/edit")
    .separator()
    .item(confirm.opener("Delete").variant(Variant::Ghost));
```

- **`Dialog`** is a native modal `<dialog>`: it moves focus in, keeps it there, closes on Escape or a click outside, and returns focus to its opener. `opener` gives a button that opens it with the HTML invoker commands, which the script adds to browsers without them. Where a page must work without script, `link_opener` is a link to a page with the same content, which the script turns into opening the dialog in place. A form inside submits as usual.
- **`Menu`** is a popover list behind a button: links, or buttons in their own forms. It opens and closes without script and closes on Escape or a click elsewhere; the script anchors it under its button. The items stay an ordinary list of links and buttons rather than an ARIA menu, so they work the usual way with every keyboard and screen reader.
- **`Toast`** is a status message at the edge of the screen, such as "Order saved" after a form redirects. It never disappears on its own; with script it has a Dismiss button. Show one by putting it on the page the redirect lands on.
- **`Tooltip`** shows a control's name beside it on hover and keyboard focus, and Escape hides it. It is for controls whose accessible name already says the same thing, such as an icon button, so it is hidden from assistive technology.

`Tabs`, in the `navigation` family, are links to sections of a page, each with its own URL; the server renders the current one, so they need no script and keep the back button working.

## Accessibility

Components carry the accessibility work so pages get it without extra effort:

- Fields wire `label for`, `aria-describedby` for hints and errors, and `aria-invalid`.
- Icon buttons require a label, and decorative icons are hidden from assistive technology.
- `AppShell` adds a skip link and the main landmark; tables live in a focusable, labelled scroll region.
- Focus rings use `:focus-visible`, and motion respects `prefers-reduced-motion`.
- Every theme passes WCAG contrast checks for its text and interface colours before it builds.

The browser test suite runs axe against every gallery page in light and dark schemes.

## Action readiness

`ReadinessPanel` presents the application's prerequisite checks with textual
statuses, concrete reasons, optional role guidance and ordinary remedy links:

```rust
use stucco::feedback::{ReadinessPanel, Requirement};

let panel = ReadinessPanel::new("Ready to publish")
    .requirement(Requirement::ready("Customer details"))
    .requirement(Requirement::blocked("Approval", "A review is still needed.")
        .guidance("An editor can approve this record.")
        .remedy("Open review", "/orders/42/review"));
```

The panel labels its region, counts completed checks and lists prerequisites
in application order. Empty panels say that no checks are available. Status
text conveys readiness without relying on color. A remedy can use a URL from
`ReturnContext::carry` to keep the user's queue context.

This is a snapshot. Applications own eligibility, role rules and the actual
submission controls, and must recheck every prerequisite when executing an
action. Stucco does not infer permission from the displayed checks.
