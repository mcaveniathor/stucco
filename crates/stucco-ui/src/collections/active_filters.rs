use super::CollectionView;
use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Filter, Render, el};

/// The search and filters narrowing a collection, each with a link that
/// removes just that one, and a link that clears them all. Renders nothing
/// when the collection is unfiltered.
///
/// Every link keeps the sort and page size and returns to the first page,
/// since the old position means nothing once the results change.
///
/// ```
/// use stucco_core::{Capabilities, CollectionQuery, ColumnKind, ColumnSpec, Filter, to_html};
/// use stucco_ui::collections::{ActiveFilters, CollectionView};
///
/// let caps = Capabilities { filterable: vec!["status".into()], searchable: true, ..Default::default() };
/// let columns = [ColumnSpec::new("status", ColumnKind::Enumeration(vec!["paid".into()]))];
/// let query = CollectionQuery::default()
///     .with_search("ada")
///     .with_filter("status", Some(Filter::Enumeration("paid".into())));
/// let view = CollectionView::new("/orders", &query, &caps, &columns).label("Orders");
/// let html = to_html(&ActiveFilters::new(&view));
/// assert!(html.contains(r#"aria-label="Remove filter: Status is paid""#));
/// assert!(html.contains(r#"href="/orders?q=ada&amp;per=25""#), "removing status keeps the search");
/// assert!(html.contains(">Clear all<"));
/// ```
#[derive(Debug)]
pub struct ActiveFilters<'a> {
    attrs: Attrs,
    view: &'a CollectionView<'a>,
    labels: Vec<(String, String)>,
}

impl<'a> ActiveFilters<'a> {
    /// The active filters of `view`.
    pub fn new(view: &'a CollectionView<'a>) -> Self {
        ActiveFilters {
            attrs: Attrs::default(),
            view,
            labels: Vec::new(),
        }
    }

    /// Labels column `key` (see [`super::FilterBar::column_label`]).
    pub fn column_label(mut self, key: impl Into<String>, label: impl Into<String>) -> Self {
        self.labels.push((key.into(), label.into()));
        self
    }
}

passthrough!(ActiveFilters<'_>);

/// How a filter reads: "is paid", "from 10 to 20", "from 2026-01-01".
fn describe(filter: &Filter) -> String {
    let range = |min: Option<String>, max: Option<String>| match (min, max) {
        (Some(a), Some(b)) => format!("from {a} to {b}"),
        (Some(a), None) => format!("from {a}"),
        (None, Some(b)) => format!("up to {b}"),
        (None, None) => "any".to_owned(),
    };
    match filter {
        Filter::Text(v) => format!("contains “{v}”"),
        Filter::Enumeration(v) => format!("is {v}"),
        Filter::Number { min, max } => {
            range(min.map(|n| n.to_string()), max.map(|n| n.to_string()))
        }
        Filter::Date { min, max } => range(min.clone(), max.clone()),
    }
}

impl Render for ActiveFilters<'_> {
    fn render(&self, cx: &mut Cx) {
        let view = self.view;
        let query = view.query;
        if !query.is_filtered() {
            return;
        }
        cx.require(&super::COLLECTIONS);
        let title_id = cx.id("filters");
        let chip = |text: String, href| {
            el::li().child(
                el::a()
                    .class("st-filter-chip")
                    .href(href)
                    .aria("label", format!("Remove filter: {text}"))
                    .child(el::span().text(text))
                    .child(el::span().aria("hidden", "true").text("×")),
            )
        };
        let mut list = el::ul()
            .class("st-active-filters-list")
            .aria("labelledby", title_id.clone());
        if !query.search.is_empty() {
            list = list.child(chip(
                format!("Search “{}”", query.search),
                query.clone().with_search("").link(&view.action),
            ));
        }
        for (key, filter) in &query.filters {
            let label = super::toolbar::column_label(&self.labels, key);
            list = list.child(chip(
                format!("{label} {}", describe(filter)),
                query.clone().with_filter(key, None).link(&view.action),
            ));
        }
        let root = el::div()
            .class("st-active-filters")
            .child(
                el::p()
                    .class("st-active-filters-title")
                    .id(title_id)
                    .text("Active filters"),
            )
            .child(list)
            .child(
                el::a()
                    .class("st-active-filters-clear")
                    .href(query.clone().reset().link(&view.action))
                    .text("Clear all"),
            );
        apply(root, &self.attrs, &[]).render(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::{Capabilities, CollectionQuery, ColumnKind, ColumnSpec, Window, to_html};

    #[test]
    fn ranges_read_naturally_and_unfiltered_renders_nothing() {
        assert_eq!(
            describe(&Filter::Number {
                min: Some(10.0),
                max: None
            }),
            "from 10"
        );
        assert_eq!(
            describe(&Filter::Date {
                min: None,
                max: Some("2026-01-31".into())
            }),
            "up to 2026-01-31"
        );
        let caps = Capabilities::default();
        let query = CollectionQuery::default();
        let view = CollectionView::new("/x", &query, &caps, &[]);
        assert_eq!(to_html(&ActiveFilters::new(&view)), "");
    }

    #[test]
    fn removal_links_reset_the_position_and_keep_the_rest() {
        let caps = Capabilities {
            filterable: vec!["total".into()],
            ..Capabilities::default()
        };
        let columns = [ColumnSpec::new("total", ColumnKind::Number)];
        let query = CollectionQuery::default()
            .with_sort("total", stucco_core::Direction::Desc)
            .with_filter(
                "total",
                Some(Filter::Number {
                    min: Some(1.5),
                    max: Some(9.0),
                }),
            )
            .with_window(Window::Offset { page: 3 });
        let view = CollectionView::new("/orders?mode=pages", &query, &caps, &columns);
        let html = to_html(&ActiveFilters::new(&view).column_label("total", "Total <$>"));
        assert!(html.contains("Total &lt;$&gt; from 1.5 to 9"));
        assert!(
            html.contains(r#"href="/orders?mode=pages&amp;sort=total&amp;dir=desc&amp;per=25""#)
        );
        assert!(!html.contains("page=3"));
    }
}
