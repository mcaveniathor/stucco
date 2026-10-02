use super::filter::date_valid;
use super::{Capabilities, ColumnKind, ColumnSpec, Cursor, Direction, Filter, Window};
use crate::Href;
use std::collections::BTreeMap;

/// Shareable, bounded collection query state.
#[derive(Clone, Debug, PartialEq)]
pub struct CollectionQuery {
    /// Active sort key.
    pub sort: Option<String>,
    /// Sort direction.
    pub direction: Direction,
    /// Shared text search.
    pub search: String,
    /// Validated filters, in stable key order.
    pub filters: BTreeMap<String, Filter>,
    /// Page size (1–100; default 25).
    pub per_page: u16,
    /// Page position.
    pub window: Window,
}
impl Default for CollectionQuery {
    fn default() -> Self {
        Self {
            sort: None,
            direction: Direction::Asc,
            search: String::new(),
            filters: BTreeMap::new(),
            per_page: 25,
            window: Window::default(),
        }
    }
}
impl CollectionQuery {
    /// Parses a query without rejecting malformed requests.
    ///
    /// ```
    /// use stucco_core::{Capabilities, CollectionQuery};
    /// let q = CollectionQuery::parse("per=10", &Capabilities::default(), &[]);
    /// assert_eq!(q.per_page, 10);
    /// ```
    pub fn parse(raw: &str, caps: &Capabilities, columns: &[ColumnSpec]) -> Self {
        let mut q = Self::default();
        if raw.len() > 16384 {
            return q;
        }
        let mut fields = BTreeMap::new();
        let mut after = None;
        let mut before = None;
        for pair in raw.split('&') {
            if !valid_encoding(pair) {
                continue;
            }
            let Some((key, value)) = form_urlencoded::parse(pair.as_bytes()).next() else {
                continue;
            };
            let value = value.into_owned();
            match key.as_ref() {
                "sort"
                    if caps.sortable.contains(&value) && columns.iter().any(|c| c.key == value) =>
                {
                    q.sort = Some(value)
                }
                "dir" if value == "asc" || value == "desc" => {
                    q.direction = if value == "desc" {
                        Direction::Desc
                    } else {
                        Direction::Asc
                    }
                }
                "q" if caps.searchable && value.chars().count() <= 256 => q.search = value,
                "per" => {
                    if let Ok(n) = value.parse::<u16>() {
                        if n > 0 {
                            q.per_page = n.min(100);
                        }
                    }
                }
                "page" if caps.offset => {
                    if let Ok(page) = value.parse::<u64>() {
                        if page > 0 {
                            q.window = Window::Offset { page };
                        }
                    }
                }
                "after" => {
                    if let Some(c) = Cursor::new(&value) {
                        after = Some(c);
                    }
                }
                "before" => {
                    if let Some(c) = Cursor::new(&value) {
                        before = Some(c);
                    }
                }
                key if key.starts_with("f.") && value.chars().count() <= 256 => {
                    fields.insert(key.to_owned(), value);
                }
                _ => {}
            }
        }
        q.window = match (after, before) {
            (Some(a), None) => Window::After(a),
            (None, Some(b)) => Window::Before(b),
            (Some(_), Some(_)) => Window::default(),
            _ => q.window,
        };
        for column in columns.iter().filter(|c| caps.filterable.contains(&c.key)) {
            let key = format!("f.{}", column.key);
            let scalar = fields.get(&key).filter(|s| !s.is_empty());
            let lower = fields.get(&format!("{key}.min")).filter(|s| !s.is_empty());
            let upper = fields.get(&format!("{key}.max")).filter(|s| !s.is_empty());
            let filter = match &column.kind {
                ColumnKind::Text => scalar.map(|s| Filter::Text(s.clone())),
                ColumnKind::Enumeration(options) => scalar
                    .filter(|s| options.contains(s))
                    .map(|s| Filter::Enumeration(s.clone())),
                ColumnKind::Number => {
                    let parse = |s: Option<&String>| -> Option<Option<f64>> {
                        match s {
                            None => Some(None),
                            Some(v) => v.parse::<f64>().ok().filter(|n| n.is_finite()).map(Some),
                        }
                    };
                    match (parse(lower), parse(upper)) {
                        (Some(min), Some(max))
                            if (min.is_some() || max.is_some())
                                && !matches!((min,max), (Some(a),Some(b)) if a > b) =>
                        {
                            Some(Filter::Number { min, max })
                        }
                        _ => None,
                    }
                }
                ColumnKind::Date => {
                    if (lower.is_some() || upper.is_some())
                        && lower.is_none_or(|s| date_valid(s))
                        && upper.is_none_or(|s| date_valid(s))
                        && !matches!((lower,upper),(Some(a),Some(b)) if a > b)
                    {
                        Some(Filter::Date {
                            min: lower.cloned(),
                            max: upper.cloned(),
                        })
                    } else {
                        None
                    }
                }
                ColumnKind::Custom => None,
            };
            if let Some(filter) = filter {
                q.filters.insert(column.key.clone(), filter);
            }
        }
        q
    }
    /// Encodes the query deterministically.
    pub fn to_query_string(&self) -> String {
        let mut s = form_urlencoded::Serializer::new(String::new());
        if let Some(sort) = &self.sort {
            s.append_pair("sort", sort)
                .append_pair("dir", self.direction.as_str());
        }
        if !self.search.is_empty() {
            s.append_pair("q", &self.search);
        }
        for (key, filter) in &self.filters {
            let key = format!("f.{key}");
            match filter {
                Filter::Text(v) | Filter::Enumeration(v) => {
                    s.append_pair(&key, v);
                }
                Filter::Number { min, max } => {
                    if let Some(v) = min {
                        s.append_pair(&format!("{key}.min"), &v.to_string());
                    }
                    if let Some(v) = max {
                        s.append_pair(&format!("{key}.max"), &v.to_string());
                    }
                }
                Filter::Date { min, max } => {
                    if let Some(v) = min {
                        s.append_pair(&format!("{key}.min"), v);
                    }
                    if let Some(v) = max {
                        s.append_pair(&format!("{key}.max"), v);
                    }
                }
            }
        }
        s.append_pair("per", &self.per_page.to_string());
        match &self.window {
            Window::Offset { page } if *page > 1 => {
                s.append_pair("page", &page.to_string());
            }
            Window::After(c) => {
                s.append_pair("after", c.as_str());
            }
            Window::Before(c) => {
                s.append_pair("before", c.as_str());
            }
            _ => {}
        }
        s.finish()
    }
    /// Changes sort and resets position.
    pub fn with_sort(mut self, key: &str, direction: Direction) -> Self {
        self.sort = Some(key.to_owned());
        self.direction = direction;
        self.window = Window::default();
        self
    }
    /// Changes search and resets position.
    pub fn with_search(mut self, value: &str) -> Self {
        self.search = value.chars().take(256).collect();
        self.window = Window::default();
        self
    }
    /// Changes a filter and resets position.
    pub fn with_filter(mut self, key: &str, value: Option<Filter>) -> Self {
        if let Some(v) = value {
            self.filters.insert(key.to_owned(), v);
        } else {
            self.filters.remove(key);
        }
        self.window = Window::default();
        self
    }
    /// Changes page size and resets position.
    pub fn with_per_page(mut self, per: u16) -> Self {
        self.per_page = if per == 0 { 25 } else { per.min(100) };
        self.window = Window::default();
        self
    }
    /// Changes position without resetting other values.
    pub fn with_window(mut self, window: Window) -> Self {
        self.window = window;
        self
    }
    /// Clears search, filters and position, retaining sorting and page size.
    pub fn reset(mut self) -> Self {
        self.search.clear();
        self.filters.clear();
        self.window = Window::default();
        self
    }
    /// Unrelated action parameters to carry as hidden inputs in GET forms.
    pub fn action_parameters(action: &Href) -> Vec<(String, String)> {
        let base = action.as_str().split('#').next().unwrap_or("");
        base.split_once('?')
            .map(|(_, q)| {
                form_urlencoded::parse(q.as_bytes())
                    .filter(|(k, _)| !owned_parameter(k))
                    .map(|(k, v)| (k.into_owned(), v.into_owned()))
                    .collect()
            })
            .unwrap_or_default()
    }
    /// Builds a safe URL, retaining unrelated action parameters and fragment.
    pub fn link(&self, action: &Href) -> Href {
        if !action.is_valid() {
            return Href::invalid();
        }
        let (base, fragment) = action
            .as_str()
            .split_once('#')
            .map_or((action.as_str(), None), |(a, b)| (a, Some(b)));
        let path = base.split('?').next().unwrap_or(base);
        let mut s = form_urlencoded::Serializer::new(String::new());
        for (k, v) in Self::action_parameters(action) {
            s.append_pair(&k, &v);
        }
        let unrelated = s.finish();
        let query = self.to_query_string();
        Href::new(format!(
            "{path}?{}{query}{}",
            if unrelated.is_empty() {
                String::new()
            } else {
                format!("{unrelated}&")
            },
            fragment.map(|f| format!("#{f}")).unwrap_or_default()
        ))
    }
}
fn owned_parameter(key: &str) -> bool {
    matches!(
        key,
        "sort" | "dir" | "q" | "page" | "per" | "after" | "before"
    ) || key.starts_with("f.")
}
fn valid_encoding(raw: &str) -> bool {
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit()
            {
                return false;
            }
            i += 3;
        } else {
            i += 1;
        }
    }
    true
}
