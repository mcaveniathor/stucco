/// An opaque URL-safe cursor, bounded to 4096 ASCII characters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cursor(String);
impl Cursor {
    /// Validates the outer encoding; the source validates the payload.
    ///
    /// ```
    /// assert!(stucco_core::Cursor::new("YWJj").is_some());
    /// assert!(stucco_core::Cursor::new("a/b").is_none());
    /// ```
    pub fn new(value: &str) -> Option<Self> {
        (!value.is_empty()
            && value.len() <= 4096
            && value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
        .then(|| Self(value.to_owned()))
    }
    /// Encoded cursor.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
