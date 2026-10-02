# Components

Every stucco component is a Rust value that renders semantic HTML. Builders set options, `Render` produces markup, and the page collects the CSS each component needs.

## Families and features

Components are grouped into families, each behind a cargo feature. The facade enables the common ones by default; turn off `default-features` to pick exactly what you compile.

| Feature | Components |
| --- | --- |
| `layout` | `Stack`, `Cluster`, `Grid`, `Center`, `Container`, `Sidebar`, `Switcher`, `Surface`, `ThemeScope`, `SkipLink` |
| `typography` | `Heading`, `Text`, `Link`, `Code`, `Kbd` |
| `actions` | `Button`, `ButtonLink`, `IconButton` |
| `forms` | `Form`, `Field`, `Input`, `Select`, `Textarea`, `Checkbox`, `RadioGroup`, `Fieldset`, `ErrorSummary` |
| `feedback` | `EmptyState`, `LiveRegion` |
| `navigation` | `Pagination`, `NavLink` |
| `data` | `Card`, `Panel`, `Table`, `ResultCount` |
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

## Accessibility

Components carry the accessibility work so pages get it without extra effort:

- Fields wire `label for`, `aria-describedby` for hints and errors, and `aria-invalid`.
- Icon buttons require a label, and decorative icons are hidden from assistive technology.
- `AppShell` adds a skip link and the main landmark; tables live in a focusable, labelled scroll region.
- Focus rings use `:focus-visible`, and motion respects `prefers-reduced-motion`.
- Every theme passes WCAG contrast checks for its text and interface colours before it builds.

The browser test suite runs axe against every gallery page in light and dark schemes.
