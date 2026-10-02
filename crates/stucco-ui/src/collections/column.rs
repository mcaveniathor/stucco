use stucco_core::{ColumnKind, ColumnSpec, Slot, el};
/// A typed display column; backend capabilities determine which operations appear.
pub struct Col<'a, T> {
    pub(crate) spec: ColumnSpec,
    pub(crate) label: String,
    pub(crate) sortable: bool,
    pub(crate) searchable: bool,
    pub(crate) filterable: bool,
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
