use crate::Variant;
use crate::actions::{Button, ButtonLink};
use crate::forms::{Input, Select};
use crate::{data::ResultCount, passthrough::apply};
use stucco_core::{
    Attrs, Capabilities, CollectionQuery, ColumnKind, ColumnSpec, Cx, Filter, Href, Render, Slot,
    Window, el,
};
/// Shared state for GET collection controls.
#[derive(Clone, Debug)]
pub struct CollectionView<'a> {
    /// Endpoint, including unrelated query state.
    pub action: Href,
    /// Validated current query.
    pub query: &'a CollectionQuery,
    /// Effective backend/column operations.
    pub capabilities: &'a Capabilities,
    /// Filter declarations.
    pub columns: &'a [ColumnSpec],
    /// Collection label, used in search labels.
    pub label: String,
}
impl<'a> CollectionView<'a> {
    /// Creates controls labelled as records.
    pub fn new(
        action: impl Into<Href>,
        query: &'a CollectionQuery,
        capabilities: &'a Capabilities,
        columns: &'a [ColumnSpec],
    ) -> Self {
        Self {
            action: action.into(),
            query,
            capabilities,
            columns,
            label: "records".into(),
        }
    }
    /// Sets the collection label.
    pub fn label(mut self, label: &str) -> Self {
        self.label = label.into();
        self
    }
}
fn hidden(name: &str, value: &str) -> stucco_core::el::VoidElement {
    el::input()
        .attr("type", "hidden")
        .attr("name", name)
        .attr("value", value)
}
fn search(view: &CollectionView<'_>, cx: &mut Cx) -> stucco_core::el::Element<'static> {
    let id = cx.id("search");
    el::div()
        .class("st-collection-control")
        .data("control", "search")
        .child(
            el::label()
                .class("st-collection-label")
                .attr("for", &id)
                .text(format!("Search {}", view.label.to_lowercase())),
        )
        .child(
            Input::search("q")
                .id(id)
                .value(view.query.search.clone())
                .autocomplete("off"),
        )
}
fn label(key: &str) -> String {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str().replace('_', " ")),
        None => String::new(),
    }
}
/// The display label for column `key`: an explicit one, else derived from
/// the key (`created_at` → "Created at").
pub(super) fn column_label(labels: &[(String, String)], key: &str) -> String {
    labels
        .iter()
        .find(|(k, _)| k == key)
        .map_or_else(|| label(key), |(_, l)| l.clone())
}
/// Standalone GET search, preserving other collection state.
#[derive(Debug)]
pub struct SearchForm<'a> {
    attrs: Attrs,
    view: &'a CollectionView<'a>,
}
impl<'a> SearchForm<'a> {
    /// Creates a search form.
    pub fn new(view: &'a CollectionView<'a>) -> Self {
        Self {
            attrs: Attrs::default(),
            view,
        }
    }
}
passthrough!(SearchForm<'_>);
impl Render for SearchForm<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::COLLECTIONS);
        let mut form = el::form()
            .class("st-filter-bar")
            .attr("method", "get")
            .action(self.view.action.clone());
        let reset = self
            .view
            .query
            .clone()
            .with_window(Window::default())
            .to_query_string();
        for (k, v) in form_urlencoded::parse(reset.as_bytes()).filter(|(k, _)| k != "q") {
            form = form.child(hidden(&k, &v));
        }
        for (k, v) in CollectionQuery::action_parameters(&self.view.action) {
            form = form.child(hidden(&k, &v));
        }
        if self.view.capabilities.searchable {
            form = form.child(search(self.view, cx));
        }
        apply(
            form.child(
                el::div()
                    .class("st-collection-actions")
                    .child(Button::new("Search").submit().variant(Variant::Primary)),
            ),
            &self.attrs,
            &["method", "action"],
        )
        .render(cx);
    }
}
/// Shared search, typed filters, sort and page-size controls in one GET form.
#[derive(Debug)]
pub struct FilterBar<'a> {
    attrs: Attrs,
    view: &'a CollectionView<'a>,
    labels: Vec<(String, String)>,
}
impl<'a> FilterBar<'a> {
    /// Creates a filter form.
    pub fn new(view: &'a CollectionView<'a>) -> Self {
        Self {
            attrs: Attrs::default(),
            view,
            labels: Vec::new(),
        }
    }
    /// Labels the filter and sort option for column `key`. Without one, the
    /// label is derived from the key (`created_at` → "Created at").
    pub fn column_label(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.labels.push((key.into(), label.into()));
        self
    }
}
passthrough!(FilterBar<'_>);
impl Render for FilterBar<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::COLLECTIONS);
        let view = self.view;
        let mut form = el::form()
            .class("st-filter-bar")
            .attr("method", "get")
            .action(view.action.clone());
        for (k, v) in CollectionQuery::action_parameters(&view.action) {
            form = form.child(hidden(&k, &v));
        }
        if view.capabilities.searchable {
            form = form.child(search(view, cx));
        }
        for col in view
            .columns
            .iter()
            .filter(|c| view.capabilities.filterable.contains(&c.key))
        {
            let id = cx.id("filter");
            let key = format!("f.{}", col.key);
            let current = view.query.filters.get(&col.key);
            let text = column_label(&self.labels, &col.key);
            let control = match &col.kind {
                ColumnKind::Enumeration(options) => {
                    let selected = match current {
                        Some(Filter::Enumeration(s)) => s.as_str(),
                        _ => "",
                    };
                    let select = Select::new(&key)
                        .id(&id)
                        .option("", "All")
                        .options(options.iter().map(|o| (o.clone(), o.clone())))
                        .selected(selected);
                    el::div()
                        .class("st-collection-control")
                        .child(collection_label(&id, text))
                        .child(select)
                }
                ColumnKind::Text => {
                    let value = match current {
                        Some(Filter::Text(s)) => s.clone(),
                        _ => String::new(),
                    };
                    el::div()
                        .class("st-collection-control")
                        .child(collection_label(&id, text))
                        .child(Input::text(&key).id(&id).value(value))
                }
                ColumnKind::Number | ColumnKind::Date => {
                    let number = matches!(col.kind, ColumnKind::Number);
                    let (min, max) = match current {
                        Some(Filter::Number { min, max }) => (
                            min.map(|v| v.to_string()).unwrap_or_default(),
                            max.map(|v| v.to_string()).unwrap_or_default(),
                        ),
                        Some(Filter::Date { min, max }) => (
                            min.clone().unwrap_or_default(),
                            max.clone().unwrap_or_default(),
                        ),
                        _ => (String::new(), String::new()),
                    };
                    let bound = |bound: &str, name: &'static str, value: String| {
                        let field_id = format!("{id}-{bound}");
                        let input = if number {
                            Input::number(&format!("{key}.{bound}"))
                                .step("any")
                                .placeholder(name)
                        } else {
                            Input::date(&format!("{key}.{bound}"))
                        };
                        [
                            Slot::new(
                                el::label()
                                    .class("st-collection-bound")
                                    .attr("for", &field_id)
                                    .text(name),
                            ),
                            Slot::new(input.id(field_id).value(value)),
                        ]
                    };
                    el::fieldset()
                        .class("st-collection-control st-collection-range")
                        .child(el::legend().class("st-collection-label").text(text))
                        .child(
                            el::div()
                                .class("st-collection-range-inputs")
                                .children(bound("min", "From", min))
                                .child(
                                    el::span()
                                        .class("st-collection-range-sep")
                                        .aria("hidden", "true")
                                        .text("–"),
                                )
                                .children(bound("max", "To", max)),
                        )
                }
                ColumnKind::Custom => continue,
            };
            form = form.child(control);
        }
        let mut sort = SortControl::new(view);
        sort.labels.clone_from(&self.labels);
        form = form.child(sort).child(PageSizeSelect::new(view)).child(
            el::div()
                .class("st-collection-actions")
                .child(
                    ButtonLink::new("Clear", view.query.clone().reset().link(&view.action))
                        .variant(Variant::Ghost),
                )
                .child(
                    Button::new("Apply filters")
                        .submit()
                        .variant(Variant::Primary),
                ),
        );
        apply(form, &self.attrs, &["method", "action"]).render(cx);
    }
}
fn collection_label(id: &str, text: String) -> stucco_core::el::Element<'static> {
    el::label()
        .class("st-collection-label")
        .attr("for", id)
        .text(text)
}
/// Sort selection inside a parent GET form.
#[derive(Debug)]
pub struct SortControl<'a> {
    attrs: Attrs,
    view: &'a CollectionView<'a>,
    labels: Vec<(String, String)>,
}
impl<'a> SortControl<'a> {
    /// Creates sort controls.
    pub fn new(view: &'a CollectionView<'a>) -> Self {
        Self {
            attrs: Attrs::default(),
            view,
            labels: Vec::new(),
        }
    }
    /// Labels the sort option for column `key` (see [`FilterBar::column_label`]).
    pub fn column_label(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.labels.push((key.into(), label.into()));
        self
    }
}
passthrough!(SortControl<'_>);
impl Render for SortControl<'_> {
    fn render(&self, cx: &mut Cx) {
        if self.view.capabilities.sortable.is_empty() {
            return;
        }
        cx.require(&super::COLLECTIONS);
        let id = cx.id("sort");
        let dir = cx.id("direction");
        let query = self.view.query;
        let sort = Select::new("sort")
            .id(&id)
            .option("", "Default")
            .options(
                self.view
                    .capabilities
                    .sortable
                    .iter()
                    .map(|k| (k.clone(), column_label(&self.labels, k))),
            )
            .selected(query.sort.clone().unwrap_or_default());
        let direction = Select::new("dir")
            .id(&dir)
            .options([("asc", "Ascending"), ("desc", "Descending")])
            .selected(query.direction.as_str());
        apply(
            el::div()
                .class("st-collection-sort")
                .child(
                    el::div()
                        .class("st-collection-control")
                        .child(collection_label(&id, "Sort by".into()))
                        .child(sort),
                )
                .child(
                    el::div()
                        .class("st-collection-control")
                        .child(collection_label(&dir, "Direction".into()))
                        .child(direction),
                ),
            &self.attrs,
            &[],
        )
        .render(cx);
    }
}
/// Page size select inside a GET form.
#[derive(Debug)]
pub struct PageSizeSelect<'a> {
    attrs: Attrs,
    view: &'a CollectionView<'a>,
}
impl<'a> PageSizeSelect<'a> {
    /// Creates page-size control.
    pub fn new(view: &'a CollectionView<'a>) -> Self {
        Self {
            attrs: Attrs::default(),
            view,
        }
    }
}
passthrough!(PageSizeSelect<'_>);
impl Render for PageSizeSelect<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::COLLECTIONS);
        let id = cx.id("per");
        let per = self.view.query.per_page;
        let sizes = std::collections::BTreeSet::from([per, 10, 25, 50, 100]);
        apply(
            el::div()
                .class("st-collection-control")
                .data("control", "per-page")
                .child(collection_label(&id, "Rows per page".into()))
                .child(
                    Select::new("per")
                        .id(id)
                        .options(sizes.into_iter().map(|n| (n.to_string(), n.to_string())))
                        .selected(per.to_string()),
                ),
            &self.attrs,
            &[],
        )
        .render(cx);
    }
}
/// Collection filters and a result count.
#[derive(Debug)]
pub struct CollectionToolbar<'a> {
    attrs: Attrs,
    view: &'a CollectionView<'a>,
    shown: usize,
    total: Option<u64>,
}
impl<'a> CollectionToolbar<'a> {
    /// Creates a toolbar.
    pub fn new(view: &'a CollectionView<'a>, shown: usize, total: Option<u64>) -> Self {
        Self {
            attrs: Attrs::default(),
            view,
            shown,
            total,
        }
    }
}
passthrough!(CollectionToolbar<'_>);
impl Render for CollectionToolbar<'_> {
    fn render(&self, cx: &mut Cx) {
        let offset = if self.view.capabilities.offset {
            match self.view.query.window {
                Window::Offset { page } => Some(
                    page.saturating_sub(1)
                        .saturating_mul(u64::from(self.view.query.per_page)),
                ),
                _ => None,
            }
        } else {
            None
        };
        apply(
            el::div()
                .class("st-collection-toolbar")
                .child(FilterBar::new(self.view))
                .child(
                    crate::feedback::LiveRegion::new()
                        .child(ResultCount::new(self.shown, self.total, offset)),
                ),
            &self.attrs,
            &[],
        )
        .render(cx);
    }
}
