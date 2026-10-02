use stucco_core::{Attrs, Cx, FormState, Render, Slot, el};

use super::control::{CONTROL_RESERVED, Control, Wiring};
use super::{FORMS, FieldError, FieldHint};
use crate::passthrough::apply;

/// A labelled control with an optional hint and validation errors, wired for
/// assistive technology: `label for`, `aria-describedby`, `aria-invalid`.
///
/// ```
/// use stucco_core::FormState;
/// use stucco_ui::forms::{Field, Input};
///
/// let state = FormState::new().with_value("email", "a@b").with_error("email", "Not an address");
/// let html = stucco_core::to_html(&Field::new("Email", Input::email("email")).bind(&state));
/// assert!(html.contains(r#"aria-invalid="true""#) && html.contains("Not an address"));
/// ```
#[derive(Debug)]
pub struct Field<'a, C: Control + 'a> {
    attrs: Attrs,
    label: Slot<'a>,
    control: C,
    hint: Option<String>,
    errors: Vec<String>,
    value: Option<String>,
    required: bool,
    optional: bool,
}

impl<'a, C: Control + 'a> Field<'a, C> {
    /// `control` labelled `label`.
    pub fn new(label: impl Render + 'a, control: C) -> Self {
        Field {
            attrs: Attrs::default(),
            label: Slot::new(label),
            control,
            hint: None,
            errors: Vec::new(),
            value: None,
            required: false,
            optional: false,
        }
    }

    /// Help text below the control.
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.hint = Some(text.into());
        self
    }

    /// Adds a validation error.
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.errors.push(message.into());
        self
    }

    /// Takes this control's errors and submitted value from `state`; the
    /// value is skipped for sensitive controls.
    pub fn bind(mut self, state: &FormState) -> Self {
        let name = self.control.name();
        self.errors.extend(state.errors(name).iter().cloned());
        if !self.control.is_sensitive() {
            self.value = state.value(name).map(str::to_owned);
        }
        self
    }

    /// Marks the field required (a visual marker plus `required`).
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Adds "(optional)" to the label. In a form where most fields are
    /// required, mark the few optional ones instead of every required one.
    pub fn optional(mut self) -> Self {
        self.optional = true;
        self
    }
}

impl<C: Control> Field<'_, C> {
    /// Adds space-separated classes to the field wrapper.
    pub fn class(mut self, classes: impl AsRef<str>) -> Self {
        self.attrs = self.attrs.class(classes);
        self
    }

    /// Sets an attribute on the field wrapper (reserved names are refused).
    pub fn attr(mut self, name: &str, value: impl Into<String>) -> Self {
        self.attrs = self.attrs.attr(name, value);
        self
    }

    /// Sets `data-{name}` on the field wrapper.
    pub fn data(mut self, name: &str, value: impl Into<String>) -> Self {
        self.attrs = self.attrs.data(name, value);
        self
    }
}

/// A control rendered with a field's wiring.
struct Wired<'c, C: Control>(&'c C, Wiring);

impl<C: Control> Render for Wired<'_, C> {
    fn render(&self, cx: &mut Cx) {
        self.0.render_wired(cx, &self.1);
    }
}

impl<C: Control> Render for Field<'_, C> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FORMS);
        let id = match self.control.explicit_id() {
            Some(id) => id.to_owned(),
            None => cx.id("field"),
        };
        let hint_id = format!("{id}-hint");
        let error_id = format!("{id}-error");
        let described: Vec<&str> = [
            self.hint.as_ref().map(|_| hint_id.as_str()),
            (!self.errors.is_empty()).then_some(error_id.as_str()),
        ]
        .into_iter()
        .flatten()
        .collect();
        let wiring = Wiring {
            id: Some(id.clone()),
            described_by: (!described.is_empty()).then(|| described.join(" ")),
            invalid: !self.errors.is_empty(),
            value: self.value.clone(),
            required: self.required,
        };
        let label = el::label()
            .class("st-label")
            .attr("for", id.clone())
            .child(&self.label)
            .child(self.required.then(|| {
                el::span()
                    .class("st-required")
                    .aria("hidden", "true")
                    .text("*")
            }))
            .child(
                (self.optional && !self.required)
                    .then(|| el::span().class("st-optional").text(" (optional)")),
            );
        let el = el::div()
            .class("st-field")
            .child(label)
            .child(Wired(&self.control, wiring))
            .child(
                self.hint
                    .as_ref()
                    .map(|h| FieldHint::new(&hint_id, h.clone())),
            )
            .child(FieldError::new(&error_id, &self.errors));
        apply(el, &self.attrs, CONTROL_RESERVED).render(cx);
    }
}
