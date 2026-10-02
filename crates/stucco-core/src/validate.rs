//! Checking a submitted [`FormState`] and turning it into typed values.

use std::str::FromStr;

use crate::FormState;

/// Checks submitted values, recording one error message per field on the
/// state so the form can be re-rendered.
///
/// Each check returns the value only if it passed. [`Validator::finish`]
/// then either builds the typed result or hands back the state with its
/// errors.
///
/// ```
/// use stucco_core::{FormState, Validator};
///
/// struct Order { customer: String, quantity: u32, note: Option<String> }
///
/// let submitted = FormState::from_urlencoded(b"customer=+Ada+&quantity=0&note=");
/// let mut v = Validator::new(submitted);
/// let customer = v.text("customer").required("Enter a customer").max_chars(80, "Too long").get();
/// let quantity = v
///     .text("quantity")
///     .required("Enter a quantity")
///     .parse::<u32>("Enter a whole number")
///     .check(|q| *q > 0, "Enter at least 1")
///     .get();
/// let note = v.text("note").get();
/// let result = v.finish(|| Some(Order { customer: customer?, quantity: quantity?, note }));
///
/// let state = result.err().expect("quantity is invalid");
/// assert_eq!(state.errors("quantity"), ["Enter at least 1"]);
/// assert!(state.errors("customer").is_empty());
/// assert_eq!(state.value("quantity"), Some("0"), "submitted values are kept");
/// ```
#[derive(Debug)]
pub struct Validator {
    state: FormState,
}

impl Validator {
    /// Validates `state`.
    pub fn new(state: FormState) -> Validator {
        Validator { state }
    }

    /// The first submitted value for `name`, with surrounding whitespace
    /// removed. An empty or missing value is absent: it fails
    /// [`Check::required`] and passes every other check.
    pub fn text(&mut self, name: &str) -> Check<'_, String> {
        let value = self
            .state
            .value(name)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned);
        Check {
            validator: self,
            name: name.to_owned(),
            value,
            failed: false,
        }
    }

    /// Every non-empty submitted value for `name` (checkbox groups,
    /// multi-selects), trimmed. Absent when there are none.
    pub fn list(&mut self, name: &str) -> Check<'_, Vec<String>> {
        let values: Vec<String> = self
            .state
            .values(name)
            .iter()
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
            .map(str::to_owned)
            .collect();
        Check {
            validator: self,
            name: name.to_owned(),
            value: (!values.is_empty()).then_some(values),
            failed: false,
        }
    }

    /// Records an error that no single field check expresses (for example,
    /// a uniqueness conflict found in storage).
    pub fn error(&mut self, name: &str, message: impl Into<String>) {
        self.add(name, message.into());
    }

    /// Records an error about the form as a whole.
    pub fn form_error(&mut self, message: impl Into<String>) {
        let state = std::mem::take(&mut self.state);
        self.state = state.with_form_error(message);
    }

    /// Whether any error has been recorded so far.
    pub fn has_errors(&self) -> bool {
        self.state.has_errors()
    }

    /// Builds the result if no errors were recorded; otherwise returns the
    /// state for re-rendering, without calling `build`.
    ///
    /// `build` returning `None` without any recorded error means an optional
    /// value was used as if it were required. That is a bug in the caller:
    /// debug builds panic; release builds return the state with a generic
    /// form error.
    pub fn finish<T>(self, build: impl FnOnce() -> Option<T>) -> Result<T, FormState> {
        if self.state.has_errors() {
            return Err(self.state);
        }
        match build() {
            Some(value) => Ok(value),
            None => {
                debug_assert!(
                    false,
                    "Validator::finish: a value was missing but no error was recorded; \
                     mark the field required or treat it as optional"
                );
                Err(self
                    .state
                    .with_form_error("The form could not be processed. Please try again."))
            }
        }
    }

    fn add(&mut self, name: &str, message: String) {
        let state = std::mem::take(&mut self.state);
        self.state = state.with_error(name, message);
    }
}

/// A value being checked. After the first failed check, later checks are
/// skipped, so each field records at most one message from its chain.
#[derive(Debug)]
#[must_use = "call .get() to take the checked value"]
pub struct Check<'v, T> {
    validator: &'v mut Validator,
    name: String,
    value: Option<T>,
    failed: bool,
}

impl<'v, T> Check<'v, T> {
    /// Fails with `message` when the value is absent.
    pub fn required(mut self, message: impl Into<String>) -> Self {
        if !self.failed && self.value.is_none() {
            self.fail(message.into());
        }
        self
    }

    /// Fails with `message` unless `predicate` accepts the value. Absent
    /// values are not checked.
    pub fn check(mut self, predicate: impl FnOnce(&T) -> bool, message: impl Into<String>) -> Self {
        if let Some(value) = &self.value {
            if !predicate(value) {
                self.fail(message.into());
            }
        }
        self
    }

    /// Converts the value, failing with the error `convert` returns.
    /// Absent values stay absent.
    pub fn and_then<U>(self, convert: impl FnOnce(T) -> Result<U, String>) -> Check<'v, U> {
        let Check {
            validator,
            name,
            value,
            failed,
        } = self;
        let mut next = Check {
            validator,
            name,
            value: None,
            failed,
        };
        match value.map(convert) {
            Some(Ok(converted)) => next.value = Some(converted),
            Some(Err(message)) => next.fail(message),
            None => {}
        }
        next
    }

    /// The value if it is present and passed every check.
    pub fn get(self) -> Option<T> {
        self.value
    }

    fn fail(&mut self, message: String) {
        self.failed = true;
        self.value = None;
        self.validator.add(&self.name, message);
    }
}

impl<'v> Check<'v, String> {
    /// Fails with `message` when the value has more than `max` characters.
    pub fn max_chars(self, max: usize, message: impl Into<String>) -> Self {
        self.check(|v| v.chars().count() <= max, message)
    }

    /// Fails with `message` when the value has fewer than `min` characters.
    pub fn min_chars(self, min: usize, message: impl Into<String>) -> Self {
        self.check(|v| v.chars().count() >= min, message)
    }

    /// Fails with `message` unless the value is one of `allowed` (for
    /// selects and radio groups, whose options a client can forge).
    pub fn one_of(self, allowed: &[&str], message: impl Into<String>) -> Self {
        self.check(|v| allowed.contains(&v.as_str()), message)
    }

    /// Parses the value with [`FromStr`], failing with `message`.
    pub fn parse<U: FromStr>(self, message: impl Into<String>) -> Check<'v, U> {
        let message = message.into();
        self.and_then(|v| v.parse().map_err(|_| message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(body: &str) -> FormState {
        FormState::from_urlencoded(body.as_bytes())
    }

    #[test]
    fn required_trims_and_treats_blank_as_missing() {
        let mut v = Validator::new(state("a=++&b=+x+"));
        assert_eq!(v.text("a").required("A").get(), None);
        assert_eq!(v.text("b").required("B").get().as_deref(), Some("x"));
        assert_eq!(v.text("c").required("C").get(), None);
        let s = v.finish(|| Some(())).unwrap_err();
        assert_eq!(s.errors("a"), ["A"]);
        assert_eq!(s.errors("c"), ["C"]);
        assert!(s.errors("b").is_empty());
    }

    #[test]
    fn the_first_failure_wins_and_later_checks_are_skipped() {
        let mut v = Validator::new(state("n=abc"));
        let n = v
            .text("n")
            .required("missing")
            .parse::<i32>("not a number")
            .check(|_| false, "never reached")
            .get();
        assert_eq!(n, None);
        assert_eq!(
            v.finish(|| Some(())).unwrap_err().errors("n"),
            ["not a number"]
        );
    }

    #[test]
    fn optional_values_pass_checks_when_absent() {
        let mut v = Validator::new(state("note="));
        let note = v
            .text("note")
            .max_chars(3, "long")
            .min_chars(1, "short")
            .get();
        let n = v.text("n").parse::<u8>("bad").get();
        assert_eq!(v.finish(|| Some((note, n))).unwrap(), (None, None));
    }

    #[test]
    fn length_and_choice_checks_count_characters() {
        let mut v = Validator::new(state("a=%C3%A9%C3%A9&b=open&c=evil"));
        assert!(v.text("a").max_chars(2, "long").get().is_some());
        assert!(
            v.text("b")
                .one_of(&["open", "closed"], "pick")
                .get()
                .is_some()
        );
        assert!(
            v.text("c")
                .one_of(&["open", "closed"], "pick")
                .get()
                .is_none()
        );
        let s = v.finish(|| Some(())).unwrap_err();
        assert_eq!(s.errors("c"), ["pick"]);
        assert_eq!(s.field_errors().count(), 1);
    }

    #[test]
    fn lists_collect_every_non_blank_value() {
        let mut v = Validator::new(state("t=a&t=+&t=b"));
        assert_eq!(v.list("t").required("none").get().unwrap(), ["a", "b"]);
        assert_eq!(v.list("u").get(), None);
        assert!(v.list("u").required("none").get().is_none());
        assert_eq!(v.finish(|| Some(())).unwrap_err().errors("u"), ["none"]);
    }

    #[test]
    fn manual_and_form_errors_block_the_result() {
        let mut v = Validator::new(state("email=a%40b.c"));
        let email = v.text("email").required("Enter an email").get();
        assert!(!v.has_errors());
        v.error("email", "Already registered");
        v.form_error("Try again");
        let s = v.finish(|| email.map(|_| ())).unwrap_err();
        assert_eq!(s.errors("email"), ["Already registered"]);
        assert_eq!(s.form_errors(), ["Try again"]);
        assert_eq!(s.value("email"), Some("a@b.c"));
    }

    #[test]
    fn finish_builds_without_errors() {
        let mut v = Validator::new(state("q=3"));
        let q = v.text("q").required("q").parse::<u32>("q").get();
        assert_eq!(v.finish(|| q), Ok(3));
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "no error was recorded")]
    fn a_missing_value_without_an_error_is_a_bug() {
        let mut v = Validator::new(state(""));
        let q = v.text("q").get();
        let _ = v.finish(|| q);
    }
}
