use crate::{data::ResultCount, passthrough::apply};
use stucco_core::{
    Attrs, Capabilities, CollectionQuery, ColumnKind, ColumnSpec, Cx, Filter, Href, Render, Window,
    el,
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
        .child(
            el::label()
                .attr("for", &id)
                .text(format!("Search {}", view.label.to_lowercase())),
        )
        .child(
            el::input()
                .id(id)
                .attr("type", "search")
                .attr("name", "q")
                .attr("value", &view.query.search),
        )
}
fn label(key: &str) -> String {
    let mut chars = key.chars();
    match chars.next() {
        Some(c) => format!("{}{}", c.to_uppercase(), chars.as_str().replace('_', " ")),
        None => String::new(),
    }
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
            form.child(el::button().attr("type", "submit").text("Search")),
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
}
impl<'a> FilterBar<'a> {
    /// Creates a filter form.
    pub fn new(view: &'a CollectionView<'a>) -> Self {
        Self {
            attrs: Attrs::default(),
            view,
        }
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
            let control = match &col.kind {
                ColumnKind::Enumeration(options) => {
                    let selected = match current {
                        Some(Filter::Enumeration(s)) => s.as_str(),
                        _ => "",
                    };
                    let select = el::select()
                        .id(&id)
                        .attr("name", &key)
                        .child(el::option().attr("value", "").text("All"))
                        .children(options.iter().map(|o| {
                            el::option()
                                .attr("value", o)
                                .bool_attr("selected", o == selected)
                                .text(o)
                        }));
                    el::div()
                        .class("st-collection-control")
                        .child(el::label().attr("for", &id).text(label(&col.key)))
                        .child(select)
                }
                ColumnKind::Text => {
                    let value = match current {
                        Some(Filter::Text(s)) => s.as_str(),
                        _ => "",
                    };
                    el::div()
                        .class("st-collection-control")
                        .child(el::label().attr("for", &id).text(label(&col.key)))
                        .child(el::input().id(&id).attr("name", &key).attr("value", value))
                }
                ColumnKind::Number | ColumnKind::Date => {
                    let kind = if matches!(col.kind, ColumnKind::Number) {
                        "number"
                    } else {
                        "date"
                    };
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
                    el::fieldset()
                        .class("st-collection-range")
                        .child(el::legend().text(label(&col.key)))
                        .children([("min", "From", min), ("max", "To", max)].into_iter().map(
                            |(bound, text, value)| {
                                let field_id = format!("{id}-{bound}");
                                let input = el::input()
                                    .id(&field_id)
                                    .attr("type", kind)
                                    .attr("name", format!("{key}.{bound}"))
                                    .attr("value", value);
                                let input = if kind == "number" {
                                    input.attr("step", "any")
                                } else {
                                    input
                                };
                                el::div()
                                    .child(el::label().attr("for", &field_id).text(text))
                                    .child(input)
                            },
                        ))
                }
                ColumnKind::Custom => continue,
            };
            form = form.child(control);
        }
        form = form
            .child(SortControl::new(view))
            .child(PageSizeSelect::new(view));
        form = form
            .child(el::button().attr("type", "submit").text("Apply filters"))
            .child(
                el::a()
                    .href(view.query.clone().reset().link(&view.action))
                    .text("Reset filters"),
            );
        apply(form, &self.attrs, &["method", "action"]).render(cx);
    }
}
/// Sort selection inside a parent GET form.
#[derive(Debug)]
pub struct SortControl<'a> {
    attrs: Attrs,
    view: &'a CollectionView<'a>,
}
impl<'a> SortControl<'a> {
    /// Creates sort controls.
    pub fn new(view: &'a CollectionView<'a>) -> Self {
        Self {
            attrs: Attrs::default(),
            view,
        }
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
        let select = el::select()
            .id(&id)
            .attr("name", "sort")
            .child(el::option().attr("value", "").text("Default"))
            .children(self.view.capabilities.sortable.iter().map(|k| {
                el::option()
                    .attr("value", k)
                    .bool_attr("selected", query.sort.as_ref() == Some(k))
                    .text(label(k))
            }));
        apply(
            el::div()
                .class("st-collection-control")
                .child(el::label().attr("for", &id).text("Sort by"))
                .child(select)
                .child(el::label().attr("for", &dir).text("Direction"))
                .child(
                    el::select().id(dir).attr("name", "dir").children(
                        [("asc", "Ascending"), ("desc", "Descending")]
                            .into_iter()
                            .map(|(k, v)| {
                                el::option()
                                    .attr("value", k)
                                    .bool_attr("selected", query.direction.as_str() == k)
                                    .text(v)
                            }),
                    ),
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
                .child(el::label().attr("for", &id).text("Rows per page"))
                .child(
                    el::select()
                        .id(id)
                        .attr("name", "per")
                        .children(sizes.into_iter().map(|n| {
                            el::option()
                                .attr("value", n.to_string())
                                .bool_attr("selected", n == per)
                                .text(n.to_string())
                        })),
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
