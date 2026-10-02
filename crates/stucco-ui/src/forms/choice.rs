//! Checkbox and RadioGroup.

use stucco_core::{Attrs, Cx, FormState, Render, Slot, el};

use super::{FORMS, FieldError, FieldHint, Legend};
use crate::passthrough::apply;

/// A labelled checkbox, optionally required and with a hint and errors.
///
/// Passthrough attributes (`.id()`, `.data()`, …) go on the `<input>`, like
/// the other controls; the label is styled with the `st-checkbox` class.
///
/// ```
/// use stucco_ui::forms::Checkbox;
/// let html = stucco_core::to_html(&Checkbox::new("terms", "I agree").checked(true));
/// assert!(html.contains(r#"type="checkbox" name="terms" value="on" checked"#));
/// ```
#[derive(Debug)]
pub struct Checkbox<'a> {
    attrs: Attrs,
    name: String,
    label: Slot<'a>,
    value: String,
    checked: bool,
    disabled: bool,
    required: bool,
    hint: Option<String>,
    errors: Vec<String>,
}

impl<'a> Checkbox<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &[
        "name",
        "type",
        "value",
        "checked",
        "required",
        "aria-describedby",
        "aria-invalid",
    ];

    /// A checkbox named `name` with value `"on"`.
    pub fn new(name: &str, label: impl Render + 'a) -> Self {
        Checkbox {
            attrs: Attrs::default(),
            name: name.to_owned(),
            label: Slot::new(label),
            value: "on".to_owned(),
            checked: false,
            disabled: false,
            required: false,
            hint: None,
            errors: Vec::new(),
        }
    }

    /// Requires the box to be checked (e.g. accepting terms).
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Help text below the checkbox.
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.hint = Some(text.into());
        self
    }

    /// Adds a validation error.
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.errors.push(message.into());
        self
    }

    /// The submitted value.
    pub fn value(mut self, value: &str) -> Self {
        self.value = value.to_owned();
        self
    }

    /// Whether it starts checked.
    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    /// Disables the checkbox.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Checked when `state` has this value among the submitted values for the
    /// name; also takes the name's errors.
    pub fn bind(mut self, state: &FormState) -> Self {
        self.checked = state.values(&self.name).contains(&self.value);
        self.errors.extend(state.errors(&self.name).iter().cloned());
        self
    }
}

passthrough!(Checkbox<'_>);

impl Render for Checkbox<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FORMS);
        let described = self.hint.is_some() || !self.errors.is_empty();
        let id = match self.attrs.get_id() {
            Some(id) => Some(id.to_owned()),
            None if described => Some(cx.id("checkbox")),
            None => None,
        };
        let ids = id
            .as_ref()
            .map(|id| (format!("{id}-hint"), format!("{id}-error")));
        let mut input = el::input();
        if let Some(id) = &id {
            input = input.id(id.clone());
        }
        let mut input = input
            .attr("type", "checkbox")
            .attr("name", self.name.clone())
            .attr("value", self.value.clone())
            .bool_attr("checked", self.checked)
            .bool_attr("disabled", self.disabled)
            .bool_attr("required", self.required);
        if let Some((hint_id, error_id)) = &ids {
            let d: Vec<&str> = [
                self.hint.as_ref().map(|_| hint_id.as_str()),
                (!self.errors.is_empty()).then_some(error_id.as_str()),
            ]
            .into_iter()
            .flatten()
            .collect();
            if !d.is_empty() {
                input = input.aria("describedby", d.join(" "));
            }
        }
        if !self.errors.is_empty() {
            input = input.aria("invalid", "true");
        }
        let conflicts = self.attrs.reserved_conflicts(Self::RESERVED);
        debug_assert!(
            conflicts.is_empty(),
            "reserved attribute set through passthrough: {conflicts:?}"
        );
        let input = input.attrs(&self.attrs.without(Self::RESERVED).without(&["id"]));
        let label = el::label()
            .class("st-checkbox")
            .child(input)
            .child(&self.label);
        match (described, ids) {
            (true, Some((hint_id, error_id))) => el::div()
                .class("st-checkbox-field")
                .child(label)
                .child(
                    self.hint
                        .as_ref()
                        .map(|h| FieldHint::new(&hint_id, h.clone())),
                )
                .child(FieldError::new(&error_id, &self.errors))
                .render(cx),
            _ => label.render(cx),
        }
    }
}

/// A set of mutually exclusive options in a fieldset, with a hint and errors
/// described on the group.
///
/// ```
/// use stucco_ui::forms::RadioGroup;
/// let html = stucco_core::to_html(&RadioGroup::new("plan", "Plan").option("free", "Free").selected("free"));
/// assert!(html.contains(r#"value="free" checked"#));
/// ```
#[derive(Debug)]
pub struct RadioGroup<'a> {
    attrs: Attrs,
    name: String,
    legend: Legend<'a>,
    options: Vec<(String, Slot<'a>)>,
    selected: Option<String>,
    hint: Option<String>,
    errors: Vec<String>,
    required: bool,
}

impl<'a> RadioGroup<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["role", "aria-describedby", "aria-invalid"];

    /// A group named `name`, captioned `legend`.
    pub fn new(name: &str, legend: impl Render + 'a) -> Self {
        RadioGroup {
            attrs: Attrs::default(),
            name: name.to_owned(),
            legend: Legend::new(legend),
            options: Vec::new(),
            selected: None,
            hint: None,
            errors: Vec::new(),
            required: false,
        }
    }

    /// Adds an option.
    pub fn option(mut self, value: &str, label: impl Render + 'a) -> Self {
        self.options.push((value.to_owned(), Slot::new(label)));
        self
    }

    /// The selected value.
    pub fn selected(mut self, value: &str) -> Self {
        self.selected = Some(value.to_owned());
        self
    }

    /// Help text for the group.
    pub fn hint(mut self, text: impl Into<String>) -> Self {
        self.hint = Some(text.into());
        self
    }

    /// Adds a validation error.
    pub fn error(mut self, message: impl Into<String>) -> Self {
        self.errors.push(message.into());
        self
    }

    /// Takes the selected value and errors from `state`.
    pub fn bind(mut self, state: &FormState) -> Self {
        if let Some(v) = state.value(&self.name) {
            self.selected = Some(v.to_owned());
        }
        self.errors.extend(state.errors(&self.name).iter().cloned());
        self
    }

    /// Requires a choice.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

passthrough!(RadioGroup<'_>);

impl Render for RadioGroup<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FORMS);
        let id = cx.id("radios");
        let hint_id = format!("{id}-hint");
        let error_id = format!("{id}-error");
        let described: Vec<&str> = [
            self.hint.as_ref().map(|_| hint_id.as_str()),
            (!self.errors.is_empty()).then_some(error_id.as_str()),
        ]
        .into_iter()
        .flatten()
        .collect();
        // role="radiogroup" (named by the legend) supports aria-invalid,
        // which the default fieldset role ("group") does not.
        let mut el = el::fieldset()
            .class("st-radio-group")
            .attr("role", "radiogroup");
        if !described.is_empty() {
            el = el.aria("describedby", described.join(" "));
        }
        if !self.errors.is_empty() {
            el = el.aria("invalid", "true");
        }
        let mut el = el.child(&self.legend);
        for (value, label) in &self.options {
            let input = el::input()
                .attr("type", "radio")
                .attr("name", self.name.clone())
                .attr("value", value.clone())
                .bool_attr("checked", self.selected.as_ref() == Some(value))
                .bool_attr("required", self.required);
            el = el.child(el::label().class("st-radio").child(input).child(label));
        }
        let el = el
            .child(
                self.hint
                    .as_ref()
                    .map(|h| FieldHint::new(&hint_id, h.clone())),
            )
            .child(FieldError::new(&error_id, &self.errors));
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}
