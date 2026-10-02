use stucco_core::{ColumnKind, ColumnSpec, Href, Slot, el};
/// A typed display column; backend capabilities determine which operations appear.
pub struct Col<'a, T> {
    pub(crate) spec: ColumnSpec,
    pub(crate) label: String,
    pub(crate) sortable: bool,
    pub(crate) searchable: bool,
    pub(crate) filterable: bool,
    pub(crate) wrap: bool,
    pub(crate) actions: bool,
    pub(crate) render: Box<dyn Fn(&T) -> Slot<'a> + 'a>,
}
impl<T> std::fmt::Debug for Col<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Col")
            .field("spec", &self.spec)
            .field("label", &self.label)
            .finish_non_exhaustive()
    }
}
impl<'a, T: 'a> Col<'a, T> {
    fn new(key: &str, label: &str, kind: ColumnKind, render: impl Fn(&T) -> Slot<'a> + 'a) -> Self {
        Self {
            spec: ColumnSpec::new(key, kind),
            label: label.into(),
            sortable: false,
            searchable: false,
            filterable: false,
            wrap: false,
            actions: false,
            render: Box::new(render),
        }
    }
    /// The column's key and kind, as a query parser needs them.
    pub fn spec(&self) -> &ColumnSpec {
        &self.spec
    }
    /// Escaped text values.
    pub fn text(key: &str, label: &str, get: impl Fn(&T) -> String + 'a) -> Self {
        Self::new(key, label, ColumnKind::Text, move |r| Slot::new(get(r)))
    }
    /// Finite numbers; non-finite values display as unavailable.
    pub fn number(key: &str, label: &str, get: impl Fn(&T) -> f64 + 'a) -> Self {
        Self::new(key, label, ColumnKind::Number, move |r| {
            let n = get(r);
            Slot::new(if n.is_finite() {
                n.to_string()
            } else {
                "Unavailable".into()
            })
        })
    }
    /// ISO date values.
    pub fn date(key: &str, label: &str, get: impl Fn(&T) -> String + 'a) -> Self {
        Self::new(key, label, ColumnKind::Date, move |r| Slot::new(get(r)))
    }
    /// Stored enum values with human-readable display labels.
    pub fn enumeration(
        key: &str,
        label: &str,
        choices: Vec<(String, String)>,
        get: impl Fn(&T) -> String + 'a,
    ) -> Self {
        let kind = ColumnKind::Enumeration(choices.iter().map(|(k, _)| k.clone()).collect());
        Self::new(key, label, kind, move |r| {
            let value = get(r);
            let text = choices
                .iter()
                .find(|(k, _)| k == &value)
                .map_or_else(|| value.clone(), |(_, v)| v.clone());
            Slot::new(el::span().class("st-tag").data("value", value).text(text))
        })
    }
    /// Per-row actions (links or small forms), right-aligned in the last
    /// column. Name each action for its row ("Edit order 42", with an
    /// `aria-label` or visually hidden text) so a list of links still makes
    /// sense out of context. Leave out actions the user is not allowed to
    /// take; the application decides which those are, and checks again
    /// when the action arrives.
    pub fn actions(label: &str, get: impl Fn(&T) -> Slot<'a> + 'a) -> Self {
        let mut col = Self::new("actions", label, ColumnKind::Custom, move |r| {
            Slot::new(el::div().class("st-row-actions").child(get(r)))
        });
        col.actions = true;
        col
    }
    /// Arbitrary safe Render content; no implicit sorting/filtering.
    pub fn custom(key: &str, label: &str, get: impl Fn(&T) -> Slot<'a> + 'a) -> Self {
        Self::new(key, label, ColumnKind::Custom, get)
    }
    /// Replaces how cells display (formatting a number as money, say). The
    /// column keeps its kind, so sorting and filtering still apply to the
    /// underlying value.
    pub fn display(mut self, show: impl Fn(&T) -> String + 'a) -> Self {
        self.render = Box::new(move |r| Slot::new(show(r)));
        self
    }
    /// Links each cell to `href` for its row, such as a record's name to
    /// its page. Sorting, search and filters are unchanged. Call it after
    /// [`Col::display`], which replaces how cells render.
    ///
    /// ```
    /// use stucco_core::to_html;
    /// use stucco_ui::collections::{Col, DataTable};
    ///
    /// let rows = [(7, "Ada")];
    /// let table = DataTable::new(&rows, "Customers").column(
    ///     Col::text("name", "Name", |r: &(u32, &str)| r.1.to_string())
    ///         .href(|r| format!("/customers/{}", r.0)),
    /// );
    /// assert!(to_html(&table).contains(r#"<a href="/customers/7">Ada</a>"#));
    /// ```
    pub fn href<H: Into<Href>>(mut self, href: impl Fn(&T) -> H + 'a) -> Self {
        let render = std::mem::replace(&mut self.render, Box::new(|_| Slot::new("")));
        self.render = Box::new(move |r| Slot::new(el::a().href(href(r).into()).child(render(r))));
        self
    }
    /// Lets long values wrap onto several lines instead of widening the
    /// table (notes, descriptions, addresses).
    pub fn wrap(mut self) -> Self {
        self.wrap = true;
        self
    }
    /// Requests sorting if the backend supports it.
    pub fn sortable(mut self) -> Self {
        if !matches!(self.spec.kind, ColumnKind::Custom) {
            self.sortable = true;
        }
        self
    }
    /// Requests shared text search.
    pub fn searchable(mut self) -> Self {
        if matches!(self.spec.kind, ColumnKind::Text) {
            self.searchable = true;
        }
        self
    }
    /// Requests a kind-specific filter.
    pub fn filter(mut self) -> Self {
        if !matches!(self.spec.kind, ColumnKind::Custom) {
            self.filterable = true;
        }
        self
    }
}
