//! Semantic tables: [`Table`], [`TableHead`], [`TableBody`], [`Row`], [`Cell`] and
//! [`HeaderCell`].

use leptos::prelude::*;

/// Whether the enclosing [`Table`] presents rows as cards in narrow containers.
#[derive(Clone, Copy)]
struct Cards(bool);

/// Whether a cell sits in the table's head rather than its body.
#[derive(Clone, Copy)]
struct InHead(bool);

fn cards() -> bool {
    use_context::<Cards>().is_some_and(|cards| cards.0)
}

fn in_head() -> bool {
    use_context::<InHead>().is_some_and(|head| head.0)
}

/// How a cell's content is aligned and formatted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellKind {
    /// Right-aligned digits of equal width.
    Number,
    /// Digits of equal width, such as dates and times.
    Date,
}

impl CellKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Number => "number",
            Self::Date => "date",
        }
    }
}

/// A semantic table inside a keyboard-focusable overflow region.
///
/// Build it from [`TableHead`] and [`TableBody`] children holding [`Row`]s of
/// [`Cell`]s and [`HeaderCell`]s. Cells accept any view, and rows can come from
/// `<For>` or an iterator. With `cards` on, rows become labelled cards below 48rem
/// of container width: give each body cell a `label` naming its column. Cells and
/// controls render once, and explicit ARIA roles keep the table semantics when CSS
/// changes the display. Without CSS this is an ordinary table.
#[component]
pub fn Table(
    /// Accessible caption, also the name of the scroll region.
    #[prop(into)]
    caption: String,
    /// Present rows as labelled cards in narrow containers.
    #[prop(optional)]
    cards: bool,
    /// A [`TableHead`] and a [`TableBody`].
    children: Children,
) -> impl IntoView {
    provide_context(Cards(cards));
    let name = caption.clone();
    let wrapper = if cards {
        "st-table-scroll st-table-cards"
    } else {
        "st-table-scroll"
    };
    view! {
        <div class=wrapper role="region" aria-label=name tabindex="0">
            <table class="st-table" role=cards.then_some("table")>
                <caption>{caption}</caption>
                {children()}
            </table>
        </div>
    }
}

/// The table's column headings: [`Row`]s of [`HeaderCell`]s.
#[component]
pub fn TableHead(
    /// The heading rows.
    children: Children,
) -> impl IntoView {
    provide_context(InHead(true));
    view! { <thead role=cards().then_some("rowgroup")>{children()}</thead> }
}

/// The table's data rows.
#[component]
pub fn TableBody(
    /// The rows; omit for an empty table.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    provide_context(InHead(false));
    view! { <tbody role=cards().then_some("rowgroup")>{children.map(|rows| rows())}</tbody> }
}

/// A table row.
#[component]
pub fn Row(
    /// The row's cells.
    children: Children,
) -> impl IntoView {
    view! { <tr role=cards().then_some("row")>{children()}</tr> }
}

/// A data cell.
///
/// In a `cards` table, `label` is the visible caption of this value in card
/// layout (hidden from assistive technology, which hears the column heading);
/// without one the caption is empty. Without `cards`, `label` is ignored.
#[component]
pub fn Cell(
    /// Alignment and number formatting.
    #[prop(optional, into)]
    kind: Option<CellKind>,
    /// Card-layout caption naming this cell's column.
    #[prop(optional, into)]
    label: Option<String>,
    /// The cell's content.
    children: Children,
) -> impl IntoView {
    let cards = cards();
    view! {
        <td role=cards.then_some("cell") data-kind=kind.map(CellKind::as_str)>
            {card_content(cards, label, children)}
        </td>
    }
}

/// A heading cell: a column heading in [`TableHead`], a row heading in
/// [`TableBody`].
///
/// `label` applies to row headings in a `cards` table, as for [`Cell`]. Set
/// `aria_sort` to `"ascending"`, `"descending"` or `"none"` on a sortable column.
#[component]
pub fn HeaderCell(
    /// Alignment and number formatting.
    #[prop(optional, into)]
    kind: Option<CellKind>,
    /// Card-layout caption naming this column; used by row headings only.
    #[prop(optional, into)]
    label: Option<String>,
    /// The column's sort direction, when sortable.
    #[prop(optional, into)]
    aria_sort: MaybeProp<String>,
    /// The heading's content.
    children: Children,
) -> impl IntoView {
    let cards = cards();
    let head = in_head();
    let (scope, role) = if head {
        ("col", "columnheader")
    } else {
        ("row", "rowheader")
    };
    view! {
        <th scope=scope role=cards.then_some(role) data-kind=kind.map(CellKind::as_str)
            aria-sort=move || aria_sort.get()>
            {card_content(cards && !head, label, children)}
        </th>
    }
}

/// Wraps a body cell's content in its card label and value when cards are on.
fn card_content(wrap: bool, label: Option<String>, children: Children) -> AnyView {
    if wrap {
        view! {
            <span class="st-table-card-label" aria-hidden="true">{label.unwrap_or_default()}</span>
            <div class="st-table-card-value">{children()}</div>
        }
        .into_any()
    } else {
        children().into_any()
    }
}
