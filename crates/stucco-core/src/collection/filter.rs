/// Semantics of a declared column.
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnKind {
    /// Searchable text.
    Text,
    /// Finite numeric values.
    Number,
    /// Gregorian ISO date (YYYY-MM-DD).
    Date,
    /// Allowed stored values.
    Enumeration(Vec<String>),
    /// Application-defined display.
    Custom,
}
/// Column identity and filter semantics.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnSpec {
    /// Stable query key.
    pub key: String,
    /// Value kind.
    pub kind: ColumnKind,
}
impl ColumnSpec {
    /// Declares a column.
    pub fn new(key: impl Into<String>, kind: ColumnKind) -> Self {
        Self {
            key: key.into(),
            kind,
        }
    }
}
/// Validated filter value.
#[derive(Clone, Debug, PartialEq)]
pub enum Filter {
    /// Text substring.
    Text(String),
    /// One declared enum value.
    Enumeration(String),
    /// Inclusive numeric bounds.
    Number {
        /// Lower bound.
        min: Option<f64>,
        /// Upper bound.
        max: Option<f64>,
    },
    /// Inclusive ISO date bounds.
    Date {
        /// Lower bound.
        min: Option<String>,
        /// Upper bound.
        max: Option<String>,
    },
}
pub(super) fn date_valid(value: &str) -> bool {
    if value.len() != 10
        || value.as_bytes()[4] != b'-'
        || value.as_bytes()[7] != b'-'
        || !value
            .bytes()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
    {
        return false;
    }
    let year = value[..4].parse::<u32>().unwrap_or(0);
    let month = value[5..7].parse::<usize>().unwrap_or(0);
    let day = value[8..].parse::<u32>().unwrap_or(0);
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    year != 0 && (1..=12).contains(&month) && day > 0 && day <= days[month - 1]
}
