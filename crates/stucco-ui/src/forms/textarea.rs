use stucco_core::{Attrs, Cx, Render, el};

use super::FORMS;
use super::control::{Control, Wiring, passthrough_without_id, unwired, wire};

/// A multi-line text input.
///
/// ```
/// use stucco_ui::forms::Textarea;
/// let html = stucco_core::to_html(&Textarea::new("bio").rows(4));
/// assert!(html.contains(r#"name="bio" rows="4""#));
/// ```
#[derive(Debug, Clone)]
pub struct Textarea {
    attrs: Attrs,
    name: String,
    rows: Option<u8>,
    value: Option<String>,
    placeholder: Option<String>,
    required: bool,
    disabled: bool,
    readonly: bool,
    sensitive: bool,
}

impl Textarea {
    /// A textarea named `name`.
    pub fn new(name: &str) -> Self {
        Textarea {
            attrs: Attrs::default(),
            name: name.to_owned(),
            rows: None,
            value: None,
            placeholder: None,
            required: false,
            disabled: false,
            readonly: false,
            sensitive: false,
        }
    }

    /// Visible rows.
    pub fn rows(mut self, rows: u8) -> Self {
        self.rows = Some(rows);
        self
    }

    /// The initial value.
    pub fn value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    /// Placeholder text.
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = Some(text.into());
        self
    }

    /// Requires a value.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Disables the textarea (its value is not submitted).
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Makes the textarea read-only (its value is still submitted).
    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }

    /// Never renders a value.
    pub fn sensitive(mut self) -> Self {
        self.sensitive = true;
        self
    }
}

passthrough!(Textarea);

impl Control for Textarea {
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
        let mut el = el::textarea()
            .class("st-textarea")
            .attr("name", self.name.clone());
        if let Some(rows) = self.rows {
            el = el.attr("rows", rows.to_string());
        }
        if let Some(p) = &self.placeholder {
            el = el.attr("placeholder", p.clone());
        }
        let mut el = wire(el, wiring, self.required)
            .bool_attr("disabled", self.disabled)
            .bool_attr("readonly", self.readonly)
            .attrs(&passthrough_without_id(&self.attrs));
        if !self.sensitive {
            if let Some(v) = wiring.value.as_ref().or(self.value.as_ref()) {
                // The parser drops one newline right after <textarea>; double
                // a leading newline so the value survives redisplay.
                let v = if v.starts_with('\n') {
                    format!("\n{v}")
                } else {
                    v.clone()
                };
                el = el.text(v);
            }
        }
        el.render(cx);
    }
}

impl Render for Textarea {
    fn render(&self, cx: &mut Cx) {
        self.render_wired(cx, &unwired(self.explicit_id()));
    }
}
