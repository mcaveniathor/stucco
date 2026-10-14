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
    conflict: Option<RevisionConflict>,
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

    /// Records a failed atomic revision check without changing submitted values.
    /// Keep the submitted revision in subsequent forms; never replace it with
    /// the current revision until the user deliberately reloads/reconciles.
    /// The application must perform the comparison atomically in storage.
    pub fn with_conflict(
        mut self,
        submitted: impl Into<String>,
        current: impl Into<String>,
    ) -> Self {
        self.conflict = Some(RevisionConflict {
            submitted: submitted.into(),
            current: current.into(),
        });
        self
    }

    /// The conflict reported by the application, if any.
    pub fn conflict(&self) -> Option<&RevisionConflict> {
        self.conflict.as_ref()
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

    /// Whether any field error, form error or revision conflict is present.
    pub fn has_errors(&self) -> bool {
        self.conflict.is_some()
            || !self.form_errors.is_empty()
            || self.errors.values().any(|e| !e.is_empty())
    }
}

/// Opaque revisions from a rejected save. No ordering or comparison is implied.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevisionConflict {
    submitted: String,
    current: String,
}
impl RevisionConflict {
    /// Revision the user's edit was based on.
    pub fn submitted(&self) -> &str {
        &self.submitted
    }
    /// Revision observed by the application's failed atomic save.
    pub fn current(&self) -> &str {
        &self.current
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
    fn conflicts_preserve_values_and_prevent_validation_success() {
        let state = FormState::from_urlencoded(b"revision=v1&name=My+edit&tags=a&tags=b")
            .with_error("name", "Check this name")
            .with_conflict("v1", "v2");
        assert_eq!(state.value("revision"), Some("v1"));
        assert_eq!(state.values("tags"), ["a", "b"]);
        assert_eq!(state.errors("name"), ["Check this name"]);
        assert!(state.has_errors());
        let conflict_only = FormState::new().with_conflict("one", "two");
        assert!(
            crate::Validator::new(conflict_only)
                .finish(|| Some(42))
                .is_err()
        );
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
