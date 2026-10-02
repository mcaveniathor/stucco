# Components

Every stucco component is a Rust value that renders semantic HTML. Builders set options, `Render` produces markup, and the page collects the CSS each component needs.

## Families and features

Components are grouped into families, each behind a cargo feature. The facade enables the common ones by default; turn off `default-features` to pick exactly what you compile.

| Feature | Components |
| --- | --- |
| `layout` | `Stack`, `Cluster`, `Grid`, `Center`, `Container`, `Sidebar`, `Switcher`, `Surface`, `ThemeScope`, `SkipLink` |
| `typography` | `Heading`, `Text`, `Link`, `Code`, `Kbd` |
| `actions` | `Button`, `ButtonLink`, `IconButton` |
| `forms` | `Form`, `Field`, `Input`, `Select`, `Textarea`, `Checkbox`, `RadioGroup`, `Fieldset`, `ErrorSummary`, `FormActions`, `HiddenInput` |
| `feedback` | `Notice`, `EmptyState`, `LiveRegion` |
| `navigation` | `Breadcrumbs`, `Pagination`, `NavLink`, `Tabs` |
| `overlay` | `Dialog`, `Menu`, `Toast`, `Tooltip` |
| `data` | `Card`, `Panel`, `Table`, `ResultCount`, `DescriptionList`, `StatusBadge`, `Timestamp`, `ActivityList` |
| `app` | `AppShell`, `PageHeader`, `SectionHeader`, `Footer`, `Confirmation` (with `forms`) |
| `collections` | `DataTable`, `Col`, `FilterBar`, `ActiveFilters`, `SearchForm`, `SortControl` |
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

## Records and outcomes

A record's page and the feedback around a change use a few small pieces:

```rust
use stucco::prelude::*;

let header = PageHeader::new("Order 1042")
    .breadcrumbs(Breadcrumbs::new().link("Orders", "/orders").current("Order 1042"))
    .description(StatusBadge::new("Paid").tone(Tone::Success))
    .actions(ButtonLink::new("Edit", "/orders/1042/edit").variant(Variant::Primary));
let details = DescriptionList::new()
    .item("Customer", "Ada Lovelace")
    .item("Created", Timestamp::new("2026-09-04T10:00:00Z", "4 Sep 2026, 10:00").zone("UTC"));
let history = ActivityList::new("History of order 1042").item(Activity::new(
    "Grace Hopper",
    "marked the order paid",
    Timestamp::new("2026-09-12T08:30:00Z", "12 Sep 2026, 08:30").zone("UTC"),
));
```

- **`Breadcrumbs`** is a `<nav>` with the trail to the current page, which is marked `aria-current="page"`. Label steps with page names ("Order 1042"), not "Details".
- **`StatusBadge`** always shows its text; the tone only tints it, so status reads the same without colour.
- **`DescriptionList`** lays a record's properties out in columns that stack on narrow screens. Pass "Not set" rather than leaving a value blank.
- **`Timestamp`** is a `<time>` with a machine-readable value. Stucco doesn't format dates or convert time zones: pass both forms, and name the zone whenever a time of day is shown.
- **`ActivityList`** shows who did what, to what, and when, with optional details in a native disclosure. Keep secrets out of activity.
- **`Notice`** reports an outcome ("Order 1042 was saved.") or a state ("This order is archived."). The tone is spelled out for screen readers, outcomes are announced, and `quiet()` notices are not.
- **`Confirmation`** is a full page that asks before acting: what is affected, what will happen, and a confirm button posting a form beside a cancel link. It works without JavaScript; a `Dialog` can offer the same form in place.

`PageHeader` actions wrap onto new lines on narrow screens.

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

The browser test suite runs axe against every gallery page in light and dark schemes, and against the orders example at a 320px width.

Automated checks and contrast validation find some problems, not all: they can't tell whether a label makes sense or a focus order is logical. Check your pages with a keyboard and a screen reader too, and keep the obligations that belong to your application: meaningful labels and messages, headings in order, and announcements that aren't noisy.
