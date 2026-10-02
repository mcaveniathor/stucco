use stucco_core::{Attrs, Cx, Render, Slot, el};

use super::FORMS;
use super::control::{Control, Wiring, passthrough_without_id, unwired, wire_void};

/// A single-line text input. Use it inside a [`super::Field`] for a label.
///
/// ```
/// use stucco_ui::forms::Input;
/// let html = stucco_core::to_html(&Input::email("email").placeholder("you@example.com"));
/// assert!(html.contains(r#"type="email" name="email""#));
/// ```
#[derive(Debug)]
pub struct Input<'a> {
    attrs: Attrs,
    kind: &'static str,
    name: String,
    value: Option<String>,
    placeholder: Option<String>,
    autocomplete: Option<String>,
    min: Option<String>,
    max: Option<String>,
    step: Option<String>,
    required: bool,
    disabled: bool,
    readonly: bool,
    sensitive: bool,
    prefix: Option<Slot<'a>>,
    suffix: Option<Slot<'a>>,
}

macro_rules! kinds {
    ($($(#[$doc:meta])* $fn:ident => $kind:literal),* $(,)?) => {$(
        $(#[$doc])*
        pub fn $fn(name: &str) -> Self {
            Input::of($kind, name)
        }
    )*};
}

impl<'a> Input<'a> {
    fn of(kind: &'static str, name: &str) -> Self {
        Input {
            attrs: Attrs::default(),
            kind,
            name: name.to_owned(),
            value: None,
            placeholder: None,
            autocomplete: None,
            min: None,
            max: None,
            step: None,
            required: false,
            disabled: false,
            readonly: false,
            sensitive: kind == "password",
            prefix: None,
            suffix: None,
        }
    }

    kinds! {
        /// `type="text"`.
        text => "text",
        /// `type="email"`.
        email => "email",
        /// `type="password"` (sensitive: never redisplayed).
        password => "password",
        /// `type="number"`.
        number => "number",
        /// `type="search"`.
        search => "search",
        /// `type="tel"`.
        tel => "tel",
        /// `type="url"`.
        url => "url",
        /// `type="date"`.
        date => "date",
    }

    /// The initial value (never rendered for sensitive inputs).
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Placeholder text (not a substitute for a label).
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// The `autocomplete` token (e.g. `"email"`, `"new-password"`).
    pub fn autocomplete(mut self, token: &str) -> Self {
        self.autocomplete = Some(token.to_owned());
        self
    }

    /// Minimum (numbers and dates).
    pub fn min(mut self, min: &str) -> Self {
        self.min = Some(min.to_owned());
        self
    }

    /// Maximum (numbers and dates).
    pub fn max(mut self, max: &str) -> Self {
        self.max = Some(max.to_owned());
        self
    }

    /// Step (numbers).
    pub fn step(mut self, step: &str) -> Self {
        self.step = Some(step.to_owned());
        self
    }

    /// Requires a value.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Disables the input.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Makes the input read-only.
    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }

    /// Never renders a value (for secrets other than passwords).
    pub fn sensitive(mut self) -> Self {
        self.sensitive = true;
        self
    }

    /// Content shown before the input (e.g. `https://`, a currency sign).
    pub fn prefix(mut self, content: impl Render + 'a) -> Self {
        self.prefix = Some(Slot::new(content));
        self
    }

    /// Content shown after the input (e.g. a unit).
    pub fn suffix(mut self, content: impl Render + 'a) -> Self {
        self.suffix = Some(Slot::new(content));
        self
    }
}

passthrough!(Input<'_>);

impl Control for Input<'_> {
    fn name(&self) -> &str {
        &self.name
    }

    fn explicit_id(&self) -> Option<&str> {
        self.attrs.get_id()
    }

    fn is_sensitive(&self) -> bool {
        self.sensitive
    }

    fn render_wired(&self, cx: &mut Cx, wiring: &Wiring) {
        cx.require(&FORMS);
        let mut input = el::input()
            .class("st-input")
            .attr("type", self.kind)
            .attr("name", self.name.clone());
        let value = wiring.value.as_ref().or(self.value.as_ref());
        if let (Some(v), false) = (value, self.sensitive) {
            input = input.attr("value", v.clone());
        }
        for (name, v) in [
            ("placeholder", &self.placeholder),
            ("autocomplete", &self.autocomplete),
            ("min", &self.min),
            ("max", &self.max),
            ("step", &self.step),
        ] {
            if let Some(v) = v {
                input = input.attr(name, v.clone());
            }
        }
        // Adornments (units, schemes) carry meaning: with an id available,
        // give them ids and put them first in aria-describedby.
        let adornment_id = |which: &str, slot: &Option<Slot<'_>>| {
            slot.as_ref()
                .and(wiring.id.as_ref())
                .map(|id| format!("{id}-{which}"))
        };
        let prefix_id = adornment_id("prefix", &self.prefix);
        let suffix_id = adornment_id("suffix", &self.suffix);
        let mut wiring = wiring.clone();
        let described: Vec<&str> = [
            prefix_id.as_deref(),
            suffix_id.as_deref(),
            wiring.described_by.as_deref(),
        ]
        .into_iter()
        .flatten()
        .collect();
        wiring.described_by = (!described.is_empty()).then(|| described.join(" "));
        let input = wire_void(input, &wiring, self.required)
            .bool_attr("disabled", self.disabled)
            .bool_attr("readonly", self.readonly)
            .attrs(&passthrough_without_id(&self.attrs));
        if self.prefix.is_none() && self.suffix.is_none() {
            input.render(cx);
            return;
        }
        el::div()
            .class("st-input-group")
            .child(adornment(&self.prefix, prefix_id))
            .child(input)
            .child(adornment(&self.suffix, suffix_id))
            .render(cx);
    }
}

fn adornment<'s>(
    slot: &'s Option<Slot<'_>>,
    id: Option<String>,
) -> Option<stucco_core::el::Element<'s>> {
    slot.as_ref().map(|s| {
        let mut span = el::span();
        if let Some(id) = id {
            span = span.id(id);
        }
        span.class("st-input-adornment").child(s)
    })
}

impl Render for Input<'_> {
    fn render(&self, cx: &mut Cx) {
        self.render_wired(cx, &unwired(self.explicit_id()));
    }
}
