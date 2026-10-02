//! Submitted form values and validation errors, for redisplaying a form
//! (spec §6).

use std::collections::BTreeMap;

use crate::behavior;

/// Values and errors from a form submission, keyed by field name.
///
/// ```
/// use stucco_core::FormState;
///
/// let state = FormState::new()
///     .with_value("email", "a@b.c")
///     .with_error("email", "Unknown address");
/// assert_eq!(state.value("email"), Some("a@b.c"));
/// assert!(state.has_errors());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FormState {
    values: BTreeMap<String, Vec<String>>,
    errors: BTreeMap<String, Vec<String>>,
    form_errors: Vec<String>,
}

impl FormState {
    /// An empty state.
    pub fn new() -> FormState {
        FormState::default()
    }

    /// Appends a submitted value for `name`.
    pub fn with_value(mut self, name: impl Into<String>, value: impl Into<String>) -> FormState {
        self.values
            .entry(name.into())
            .or_default()
            .push(value.into());
        self
    }

    /// Appends an error message for field `name`.
    pub fn with_error(mut self, name: impl Into<String>, message: impl Into<String>) -> FormState {
        self.errors
            .entry(name.into())
            .or_default()
            .push(message.into());
        self
    }

    /// Appends an error about the form as a whole.
    pub fn with_form_error(mut self, message: impl Into<String>) -> FormState {
        self.form_errors.push(message.into());
        self
    }

    /// Parses an `application/x-www-form-urlencoded` body, keeping repeated
    /// names in order. The CSRF field (`_csrf`) is dropped so it is never
    /// echoed back into a re-rendered form.
    ///
    /// ```
    /// use stucco_core::FormState;
    ///
    /// let state = FormState::from_urlencoded(b"tags=a&tags=b&note=caf%C3%A9+au+lait&_csrf=t");
    /// assert_eq!(state.values("tags"), ["a", "b"]);
    /// assert_eq!(state.value("note"), Some("café au lait"));
    /// assert_eq!(state.value("_csrf"), None);
    /// ```
    pub fn from_urlencoded(body: &[u8]) -> FormState {
        form_urlencoded::parse(body)
            .filter(|(name, _)| name != behavior::CSRF_FIELD)
            .fold(FormState::new(), |state, (name, value)| {
                state.with_value(name, value)
            })
    }

    /// The first submitted value for `name`.
    pub fn value(&self, name: &str) -> Option<&str> {
        self.values(name).first().map(String::as_str)
    }

    /// Every submitted value for `name` (checkbox groups, multi-selects).
    pub fn values(&self, name: &str) -> &[String] {
        self.values.get(name).map_or(&[], Vec::as_slice)
    }

    /// Error messages for field `name`.
    pub fn errors(&self, name: &str) -> &[String] {
        self.errors.get(name).map_or(&[], Vec::as_slice)
    }

    /// Errors about the form as a whole.
    pub fn form_errors(&self) -> &[String] {
        &self.form_errors
    }

    /// Fields with errors and their messages, ordered by field name.
    pub fn field_errors(&self) -> impl Iterator<Item = (&str, &[String])> {
        self.errors
            .iter()
            .filter(|(_, messages)| !messages.is_empty())
            .map(|(name, messages)| (name.as_str(), messages.as_slice()))
    }

    /// Whether any field or form error is present.
    pub fn has_errors(&self) -> bool {
        !self.form_errors.is_empty() || self.errors.values().any(|e| !e.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn form_state_collects_values_and_errors() {
        let s = FormState::new()
            .with_value("tags", "a")
            .with_value("tags", "b")
            .with_error("email", "Required")
            .with_form_error("Try again");
        assert_eq!(s.value("tags"), Some("a"));
        assert_eq!(s.values("tags"), ["a", "b"]);
        assert_eq!(s.errors("email"), ["Required"]);
        assert!(s.errors("name").is_empty() && s.has_errors());
        assert_eq!(s.form_errors(), ["Try again"]);
        assert!(!FormState::new().has_errors());
        let names: Vec<_> = s.field_errors().map(|(name, _)| name).collect();
        assert_eq!(names, ["email"]);
    }

    #[test]
    fn urlencoded_bodies_decode_and_drop_the_csrf_field() {
        let s = FormState::from_urlencoded(b"a=1&b=x%26y&a=2&empty=&_csrf=secret&flag");
        assert_eq!(s.values("a"), ["1", "2"]);
        assert_eq!(s.value("b"), Some("x&y"));
        assert_eq!(s.value("empty"), Some(""));
        assert_eq!(s.value("flag"), Some(""));
        assert!(s.values("_csrf").is_empty() && !s.has_errors());
        assert_eq!(FormState::from_urlencoded(b""), FormState::new());
    }
}
