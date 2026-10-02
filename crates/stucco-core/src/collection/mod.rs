//! Collection presentation state, independent of storage and HTTP.
mod cursor;
mod filter;
mod query;
pub use cursor::Cursor;
pub use filter::{ColumnKind, ColumnSpec, Filter};
pub use query::CollectionQuery;

/// Sort direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Direction {
    /// Ascending order.
    #[default]
    Asc,
    /// Descending order.
    Desc,
}
impl Direction {
    /// Query representation.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
    /// The opposite direction.
    pub fn reversed(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}
/// Position within a collection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Window {
    /// One-based numbered page.
    Offset {
        /// Page number.
        page: u64,
    },
    /// Records after this exclusive anchor.
    After(Cursor),
    /// Records before this exclusive anchor.
    Before(Cursor),
}
impl Default for Window {
    fn default() -> Self {
        Self::Offset { page: 1 }
    }
}
/// Operations supported by a data source.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capabilities {
    /// Sortable column keys.
    pub sortable: Vec<String>,
    /// Filterable column keys.
    pub filterable: Vec<String>,
    /// Whether shared text search is supported.
    pub searchable: bool,
    /// Whether filtered totals can be computed.
    pub total_count: bool,
    /// Whether numbered offsets are supported.
    pub offset: bool,
}
/// Owned rows and navigation metadata from one query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CollectionPage<T> {
    /// Rows in display order.
    pub rows: Vec<T>,
    /// Next-page cursor.
    pub next: Option<Cursor>,
    /// Previous-page cursor.
    pub prev: Option<Cursor>,
    /// Filtered count, if supported.
    pub total: Option<u64>,
}
