use stucco_core::{Attrs, Cx, Render, el};

use super::FORMS;
use super::control::{Control, Wiring, passthrough_without_id, unwired, wire};

/// A drop-down list of options.
///
/// ```
/// use stucco_ui::forms::Select;
/// let html = stucco_core::to_html(&Select::new("plan").option("free", "Free").selected("free"));
/// assert!(html.contains(r#"<option value="free" selected>Free</option>"#));
/// ```
#[derive(Debug, Clone)]
pub struct Select {
    attrs: Attrs,
    name: String,
    options: Vec<(String, String)>,
    placeholder: Option<String>,
    selected: Option<String>,
    required: bool,
}

impl Select {
    /// A select named `name`.
    pub fn new(name: &str) -> Self {
        Select {
            attrs: Attrs::default(),
            name: name.to_owned(),
            options: Vec::new(),
            placeholder: None,
            selected: None,
            required: false,
        }
    }

    /// Adds an option.
    pub fn option(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push((value.into(), label.into()));
        self
    }

    /// Adds options from `(value, label)` pairs.
    pub fn options<V: Into<String>, L: Into<String>>(
        mut self,
        options: impl IntoIterator<Item = (V, L)>,
    ) -> Self {
        self.options
            .extend(options.into_iter().map(|(v, l)| (v.into(), l.into())));
        self
    }

    /// A first, unselectable "choose…" option, selected while nothing else is.
    pub fn placeholder(mut self, label: impl Into<String>) -> Self {
        self.placeholder = Some(label.into());
        self
    }

    /// The selected value.
    pub fn selected(mut self, value: impl Into<String>) -> Self {
        self.selected = Some(value.into());
        self
    }

    /// Requires a choice.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }
}

passthrough!(Select);

impl Control for Select {
    fn name(&self) -> &str {
        &self.name
    }

    fn explicit_id(&self) -> Option<&str> {
        self.attrs.get_id()
    }

    fn is_sensitive(&self) -> bool {
        false
    }

    fn render_wired(&self, cx: &mut Cx, wiring: &Wiring) {
        cx.require(&FORMS);
        let selected = wiring.value.as_ref().or(self.selected.as_ref());
        let mut el = wire(
            el::select()
                .class("st-select")
                .attr("name", self.name.clone()),
            wiring,
            self.required,
        )
        .attrs(&passthrough_without_id(&self.attrs));
        if let Some(p) = &self.placeholder {
            el = el.child(
                el::option()
                    .attr("value", "")
                    .bool_attr("disabled", true)
                    .bool_attr("selected", selected.is_none())
                    .text(p.clone()),
            );
        }
        for (value, label) in &self.options {
            el = el.child(
                el::option()
                    .attr("value", value.clone())
                    .bool_attr("selected", selected == Some(value))
                    .text(label.clone()),
            );
        }
        el.render(cx);
    }
}

impl Render for Select {
    fn render(&self, cx: &mut Cx) {
        self.render_wired(cx, &unwired(self.explicit_id()));
    }
}
