use crate::passthrough::apply;
use stucco_core::{Attrs, CollectionQuery, Cursor, Cx, Href, Render, Window, el};
/// Cursor links or a bounded set of numbered-page links.
#[derive(Clone, Debug)]
pub struct Pagination {
    attrs: Attrs,
    action: Href,
    query: CollectionQuery,
    next: Option<Cursor>,
    prev: Option<Cursor>,
    total: Option<u64>,
    label: String,
}
impl Pagination {
    /// Navigation for the given query.
    pub fn new(action: impl Into<Href>, query: &CollectionQuery) -> Self {
        Self {
            attrs: Attrs::default(),
            action: action.into(),
            query: query.clone(),
            next: None,
            prev: None,
            total: None,
            label: "Pagination".into(),
        }
    }
    /// Sets cursor navigation metadata.
    pub fn cursors(mut self, next: Option<Cursor>, prev: Option<Cursor>) -> Self {
        self.next = next;
        self.prev = prev;
        self
    }
    /// Enables numbered pagination when the window is Offset.
    pub fn total(mut self, total: Option<u64>) -> Self {
        self.total = total;
        self
    }
    /// Names navigation distinctly when a page contains multiple collections.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}
passthrough!(Pagination);
impl Render for Pagination {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::NAVIGATION);
        let mut nav = el::nav().class("st-pagination").aria("label", &self.label);
        if let (Some(total), Window::Offset { page }) = (self.total, &self.query.window) {
            let per = u64::from(self.query.per_page.max(1));
            let count = total.div_ceil(per);
            if count <= 1 {
                return;
            }
            let current = (*page).max(1);
            let mut pages = std::collections::BTreeSet::from([1, count]);
            for n in current.saturating_sub(2)..=current.saturating_add(2).min(count) {
                if n > 0 && n <= count {
                    pages.insert(n);
                }
            }
            let mut previous = 0;
            for n in pages {
                if previous > 0 && n > previous + 1 {
                    nav = nav.child(el::span().aria("hidden", "true").text("…"));
                }
                let link = el::a()
                    .href(
                        self.query
                            .clone()
                            .with_window(Window::Offset { page: n })
                            .link(&self.action),
                    )
                    .aria("label", format!("Page {n}"))
                    .text(n.to_string());
                nav = nav.child(if n == current {
                    link.aria("current", "page")
                } else {
                    link
                });
                previous = n;
            }
        } else {
            if let Some(c) = &self.prev {
                nav = nav.child(
                    el::a()
                        .href(
                            self.query
                                .clone()
                                .with_window(Window::Before(c.clone()))
                                .link(&self.action),
                        )
                        .text("Previous"),
                );
            }
            if let Some(c) = &self.next {
                nav = nav.child(
                    el::a()
                        .href(
                            self.query
                                .clone()
                                .with_window(Window::After(c.clone()))
                                .link(&self.action),
                        )
                        .text("Next"),
                );
            }
            if self.next.is_none() && self.prev.is_none() {
                return;
            }
        }
        apply(nav, &self.attrs, &["aria-label"]).render(cx);
    }
}
