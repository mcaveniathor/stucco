//! Actions (feature `actions`): buttons, button-styled links and icon buttons.

use stucco_core::el::Element;
use stucco_core::{Asset, Attrs, Cx, Href, Render, Slot, el, register_asset};

use crate::passthrough::apply;
use crate::{Icon, Size, Variant};

#[cfg(test)]
mod tests;

/// The actions family's stylesheet.
pub static ACTIONS: Asset = Asset {
    name: "st-actions",
    css: Some(include_str!("../../css/actions.css")),
    behavior: None,
    deps: &[],
};
register_asset!(ACTIONS);

fn size_value(size: Size) -> &'static str {
    match size {
        Size::Xs | Size::Sm => "sm",
        Size::Md => "md",
        _ => "lg",
    }
}

fn styled<'a>(el: Element<'a>, variant: Variant, size: Size) -> Element<'a> {
    el.class("st-button")
        .data("variant", variant.as_str())
        .data("size", size_value(size))
}

/// What a [`Button`] does in a form.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ButtonType {
    Button,
    Submit,
    Reset,
}

/// A button. Defaults to `type="button"`, so it never submits a form by
/// accident; use [`Button::submit`] for submit buttons.
///
/// ```
/// use stucco_ui::{Variant, actions::Button};
/// let html = stucco_core::to_html(&Button::new("Save").submit().variant(Variant::Primary));
/// assert!(html.contains(r#"type="submit" data-variant="primary""#));
/// ```
#[derive(Debug)]
pub struct Button<'a> {
    attrs: Attrs,
    label: Slot<'a>,
    kind: ButtonType,
    variant: Variant,
    size: Size,
    name: Option<String>,
    value: Option<String>,
    icon: Option<Icon>,
    disabled: bool,
    loading: bool,
}

impl<'a> Button<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &[
        "type",
        "data-variant",
        "data-size",
        "disabled",
        "aria-busy",
        "aria-disabled",
    ];

    /// A secondary, medium `type="button"` button.
    pub fn new(label: impl Render + 'a) -> Self {
        Button {
            attrs: Attrs::default(),
            label: Slot::new(label),
            kind: ButtonType::Button,
            variant: Variant::Secondary,
            size: Size::Md,
            name: None,
            value: None,
            icon: None,
            disabled: false,
            loading: false,
        }
    }

    /// Submits its form.
    pub fn submit(mut self) -> Self {
        self.kind = ButtonType::Submit;
        self
    }

    /// Resets its form.
    pub fn reset(mut self) -> Self {
        self.kind = ButtonType::Reset;
        self
    }

    /// Emphasis.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Size (`Sm`, `Md` or `Lg`; other sizes map to the nearest).
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// The name submitted with the form.
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_owned());
        self
    }

    /// The value submitted with the form.
    pub fn value(mut self, value: &str) -> Self {
        self.value = Some(value.to_owned());
        self
    }

    /// A decorative icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Disables the button.
    pub fn disabled(mut self) -> Self {
        self.disabled = true;
        self
    }

    /// Shows a busy state; the button stays focusable and announces it.
    pub fn loading(mut self) -> Self {
        self.loading = true;
        self
    }
}

passthrough!(Button<'_>);

impl Render for Button<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&ACTIONS);
        let kind = match self.kind {
            ButtonType::Button => "button",
            ButtonType::Submit => "submit",
            ButtonType::Reset => "reset",
        };
        let mut el = styled(el::button().attr("type", kind), self.variant, self.size);
        if self.loading {
            el = el.aria("busy", "true").aria("disabled", "true");
        }
        if let Some(name) = &self.name {
            el = el.attr("name", name.clone());
        }
        if let Some(value) = &self.value {
            el = el.attr("value", value.clone());
        }
        let el = el
            .bool_attr("disabled", self.disabled)
            .child(self.icon)
            .child(&self.label);
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}

/// A link styled as a button (navigation, not an action).
///
/// ```
/// use stucco_ui::actions::ButtonLink;
/// let html = stucco_core::to_html(&ButtonLink::new("Sign up", "/signup"));
/// assert!(html.starts_with(r#"<a class="st-button" href="/signup""#));
/// ```
#[derive(Debug)]
pub struct ButtonLink<'a> {
    attrs: Attrs,
    label: Slot<'a>,
    href: Href,
    variant: Variant,
    size: Size,
    icon: Option<Icon>,
}

impl<'a> ButtonLink<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["href", "data-variant", "data-size"];

    /// A secondary, medium button-styled link.
    pub fn new(label: impl Render + 'a, href: impl Into<Href>) -> Self {
        ButtonLink {
            attrs: Attrs::default(),
            label: Slot::new(label),
            href: href.into(),
            variant: Variant::Secondary,
            size: Size::Md,
            icon: None,
        }
    }

    /// Emphasis.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// A decorative icon before the label.
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }
}

passthrough!(ButtonLink<'_>);

impl Render for ButtonLink<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&ACTIONS);
        let el = styled(el::a().href(self.href.clone()), self.variant, self.size)
            .child(self.icon)
            .child(&self.label);
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}

/// A button showing only an icon; the label becomes its accessible name.
///
/// ```
/// use stucco_ui::{Icon, actions::IconButton};
/// let close = Icon::custom("x", r#"<path d="M18 6 6 18M6 6l12 12"/>"#);
/// let html = stucco_core::to_html(&IconButton::new(close, "Close"));
/// assert!(html.contains(r#"aria-label="Close""#));
/// ```
#[derive(Debug, Clone)]
pub struct IconButton {
    attrs: Attrs,
    icon: Icon,
    label: String,
    variant: Variant,
    size: Size,
    submit: bool,
}

impl IconButton {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &[
        "type",
        "aria-label",
        "data-variant",
        "data-size",
        "data-icon-only",
    ];

    /// A ghost icon button named `label` (debug panic "icon button label" if
    /// it is blank).
    pub fn new(icon: Icon, label: &str) -> Self {
        debug_assert!(
            !label.trim().is_empty(),
            "icon button label must not be empty"
        );
        IconButton {
            attrs: Attrs::default(),
            icon,
            label: label.to_owned(),
            variant: Variant::Ghost,
            size: Size::Md,
            submit: false,
        }
    }

    /// Emphasis.
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// Size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Submits its form.
    pub fn submit(mut self) -> Self {
        self.submit = true;
        self
    }
}

passthrough!(IconButton);

impl Render for IconButton {
    fn render(&self, cx: &mut Cx) {
        cx.require(&ACTIONS);
        let kind = if self.submit { "submit" } else { "button" };
        let el = styled(el::button().attr("type", kind), self.variant, self.size)
            .bool_attr("data-icon-only", true)
            .aria("label", self.label.clone())
            .child(self.icon);
        apply(el, &self.attrs, Self::RESERVED).render(cx);
    }
}
