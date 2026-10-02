//! Form structure: Form, Fieldset, Legend, FieldHint, FieldError,
//! HiddenInput and CsrfToken.

use stucco_core::{Attrs, Cx, Href, Render, Slot, behavior, el};

use super::FORMS;
use crate::passthrough::apply;

/// A `<form>`. Without JavaScript it submits normally.
///
/// ```
/// use stucco_ui::forms::Form;
/// let html = stucco_core::to_html(&Form::post("/orders").csrf("token"));
/// assert!(html.starts_with(r#"<form class="st-form" method="post" action="/orders">"#));
/// ```
#[derive(Debug)]
pub struct Form<'a> {
    attrs: Attrs,
    post: bool,
    action: Href,
    csrf: Option<String>,
    multipart: bool,
    children: Vec<Slot<'a>>,
}

impl<'a> Form<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["method", "action", "enctype"];

    fn new(post: bool, action: impl Into<Href>) -> Self {
        Form {
            attrs: Attrs::default(),
            post,
            action: action.into(),
            csrf: None,
            multipart: false,
            children: Vec::new(),
        }
    }

    /// A form submitted with POST.
    pub fn post(action: impl Into<Href>) -> Self {
        Form::new(true, action)
    }

    /// A form submitted with GET (search and filters).
    pub fn get(action: impl Into<Href>) -> Self {
        Form::new(false, action)
    }

    /// Adds a CSRF token field (`_csrf`).
    pub fn csrf(mut self, token: &str) -> Self {
        self.csrf = Some(token.to_owned());
        self
    }

    /// Sends files (`multipart/form-data`); POST forms only (debug panic on
    /// GET, ignored in release).
    pub fn multipart(mut self) -> Self {
        debug_assert!(self.post, "multipart encoding requires a POST form");
        self.multipart = self.post;
        self
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Render + 'a) -> Self {
        self.children.push(Slot::new(child));
        self
    }

    /// Appends every item as a child.
    pub fn children<R: Render + 'a>(mut self, items: impl IntoIterator<Item = R>) -> Self {
        self.children.extend(items.into_iter().map(Slot::new));
        self
    }
}

passthrough!(Form<'_>);

impl Render for Form<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FORMS);
        let mut el = el::form()
            .class("st-form")
            .attr("method", if self.post { "post" } else { "get" })
            .action(self.action.clone());
        if self.multipart {
            el = el.attr("enctype", "multipart/form-data");
        }
        let el = el
            .child(self.csrf.as_deref().map(CsrfToken::new))
            .children(self.children.iter());
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}

/// A group of related controls with a caption.
///
/// ```
/// use stucco_ui::forms::Fieldset;
/// let html = stucco_core::to_html(&Fieldset::new("Shipping").child("…"));
/// assert!(html.contains(r#"<legend class="st-legend">Shipping</legend>"#));
/// ```
#[derive(Debug)]
pub struct Fieldset<'a> {
    attrs: Attrs,
    legend: Legend<'a>,
    children: Vec<Slot<'a>>,
}

impl<'a> Fieldset<'a> {
    /// A fieldset captioned `legend`.
    pub fn new(legend: impl Render + 'a) -> Self {
        Fieldset {
            attrs: Attrs::default(),
            legend: Legend::new(legend),
            children: Vec::new(),
        }
    }

    /// Appends a child.
    pub fn child(mut self, child: impl Render + 'a) -> Self {
        self.children.push(Slot::new(child));
        self
    }

    /// Appends every item as a child.
    pub fn children<R: Render + 'a>(mut self, items: impl IntoIterator<Item = R>) -> Self {
        self.children.extend(items.into_iter().map(Slot::new));
        self
    }
}

passthrough!(Fieldset<'_>);

impl Render for Fieldset<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FORMS);
        let el = el::fieldset()
            .class("st-fieldset")
            .child(&self.legend)
            .children(self.children.iter());
        apply(el, &self.attrs, &[]).render(cx);
    }
}

/// A fieldset caption.
#[derive(Debug)]
pub struct Legend<'a> {
    attrs: Attrs,
    content: Slot<'a>,
}

impl<'a> Legend<'a> {
    /// A legend showing `content`.
    pub fn new(content: impl Render + 'a) -> Self {
        Legend {
            attrs: Attrs::default(),
            content: Slot::new(content),
        }
    }
}

passthrough!(Legend<'_>);

impl Render for Legend<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FORMS);
        let el = el::legend().class("st-legend").child(&self.content);
        apply(el, &self.attrs, &[]).render(cx);
    }
}

/// Help text for a control, referenced by its `aria-describedby`.
#[derive(Debug, Clone)]
pub struct FieldHint {
    id: String,
    text: String,
}

impl FieldHint {
    /// Hint `text` with element id `id`.
    pub fn new(id: &str, text: impl Into<String>) -> Self {
        FieldHint {
            id: id.to_owned(),
            text: text.into(),
        }
    }
}

impl Render for FieldHint {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FORMS);
        el::p()
            .class("st-field-hint")
            .id(self.id.clone())
            .text(self.text.clone())
            .render(cx);
    }
}

/// Validation messages for a control; renders nothing when empty.
#[derive(Debug, Clone)]
pub struct FieldError {
    id: String,
    messages: Vec<String>,
}

impl FieldError {
    /// `messages` with element id `id`.
    pub fn new(id: &str, messages: &[String]) -> Self {
        FieldError {
            id: id.to_owned(),
            messages: messages.to_vec(),
        }
    }
}

impl Render for FieldError {
    fn render(&self, cx: &mut Cx) {
        if self.messages.is_empty() {
            return;
        }
        cx.require(&FORMS);
        let mut el = el::p()
            .class("st-field-error")
            .id(self.id.clone())
            .child(el::span().class("st-sr-only").text("Error: "));
        for (i, message) in self.messages.iter().enumerate() {
            if i > 0 {
                el = el.child(el::br());
            }
            el = el.text(message.clone());
        }
        el.render(cx);
    }
}

/// A hidden form value.
#[derive(Debug, Clone)]
pub struct HiddenInput {
    name: String,
    value: String,
}

impl HiddenInput {
    /// `<input type="hidden" name value>`.
    pub fn new(name: &str, value: impl Into<String>) -> Self {
        HiddenInput {
            name: name.to_owned(),
            value: value.into(),
        }
    }
}

impl Render for HiddenInput {
    fn render(&self, cx: &mut Cx) {
        el::input()
            .attr("type", "hidden")
            .attr("name", self.name.clone())
            .attr("value", self.value.clone())
            .render(cx);
    }
}

/// The CSRF token field (`_csrf`).
#[derive(Debug, Clone)]
pub struct CsrfToken(HiddenInput);

impl CsrfToken {
    /// A hidden `_csrf` field carrying `token`.
    pub fn new(token: &str) -> Self {
        CsrfToken(HiddenInput::new(behavior::CSRF_FIELD, token))
    }
}

impl Render for CsrfToken {
    fn render(&self, cx: &mut Cx) {
        self.0.render(cx);
    }
}
