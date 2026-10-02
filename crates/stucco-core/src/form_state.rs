//! Submitted form values and validation errors, for redisplaying a form
//! (spec §6).

use std::collections::BTreeMap;

use crate::behavior;

/// A form's values and errors, keyed by field name.
///
/// The same type holds a form's *initial* values (an edit form filled from a
/// stored record, built with [`FormState::with_value`]) and its *submitted*
/// values (parsed with [`FormState::from_urlencoded`]); [`is_submitted`]
/// tells them apart. Submitted values are kept exactly as sent, including
/// input that failed to parse, so a re-rendered form shows what the user
/// typed rather than a stored value or a blank.
///
/// [`is_submitted`]: FormState::is_submitted
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
    submitted: bool,
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
    /// echoed back into a re-rendered form. The state is marked submitted.
    ///
    /// Browsers send nothing for an unchecked checkbox, so a missing name
    /// means "unchecked" (see `Validator::flag`), and a name sent several
    /// times (checkbox groups, multi-selects) keeps every value.
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
            .mark_submitted()
    }

    /// Marks the state as a submission (for values read some other way,
    /// such as a multipart or JSON body).
    pub fn mark_submitted(mut self) -> FormState {
        self.submitted = true;
        self
    }

    /// Whether the values came from a submission rather than being the
    /// form's initial values.
    pub fn is_submitted(&self) -> bool {
        self.submitted
    }

    /// Drops the values of `names` and keeps their errors: for passwords,
    /// tokens and other secrets, before a state is re-rendered, stored or
    /// logged. (`Field` never redisplays a password or sensitive control's
    /// value, but the state still holds it, and its `Debug` output shows
    /// every value.)
    ///
    /// ```
    /// use stucco_core::FormState;
    ///
    /// let state = FormState::from_urlencoded(b"email=a%40b.c&password=hunter2")
    ///     .with_error("password", "Wrong password")
    ///     .without_values(&["password"]);
    /// assert_eq!(state.value("password"), None);
    /// assert_eq!(state.errors("password"), ["Wrong password"]);
    /// assert!(!format!("{state:?}").contains("hunter2"));
    /// ```
    pub fn without_values(mut self, names: &[&str]) -> FormState {
        for name in names {
            self.values.remove(*name);
        }
        self
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
        assert_eq!(
            FormState::from_urlencoded(b""),
            FormState::new().mark_submitted()
        );
    }

    #[test]
    fn initial_values_are_not_a_submission() {
        let initial = FormState::new().with_value("name", "Ada");
        assert!(!initial.is_submitted());
        assert!(FormState::from_urlencoded(b"name=Ada").is_submitted());
        let kept = FormState::from_urlencoded(b"a=1&b=2").without_values(&["a", "missing"]);
        assert_eq!(kept.value("a"), None);
        assert_eq!(kept.value("b"), Some("2"));
        assert!(kept.is_submitted());
    }
}
