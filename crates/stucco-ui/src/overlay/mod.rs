//! Overlays (feature `overlay`): dialogs, menus, toasts and tooltips, built
//! on native HTML (`<dialog>`, the popover API, invoker commands) so they
//! need little or no script. One small behaviour module fills the gaps:
//! invoker commands in browsers without them, menus anchored to their
//! buttons, dismissible toasts and Escape for tooltips.
//!
//! ```
//! use stucco_ui::overlay::{Dialog, Menu, Toast};
//! use stucco_ui::Tone;
//! use stucco_core::to_html;
//!
//! let dialog = Dialog::new("delete-order", "Delete order 1042?")
//!     .child("This can't be undone.")
//!     .actions(Dialog::close_button("delete-order", "Cancel"));
//! let html = to_html(&(dialog.opener("Delete"), dialog));
//! assert!(html.contains(r#"commandfor="delete-order" command="show-modal""#));
//! assert!(html.contains(r#"<dialog id="delete-order""#));
//!
//! let menu = Menu::new("Actions").link("Edit", "/orders/1042/edit");
//! assert!(to_html(&menu).contains("popovertarget="));
//! assert!(to_html(&Toast::new("Order saved").tone(Tone::Success)).contains(r#"role="status""#));
//! ```

use crate::actions::{Button, ButtonLink};
use crate::passthrough::apply;
use crate::typography::Heading;
use crate::{Size, Tone, Variant};
use stucco_core::{Attrs, Cx, Href, Render, Slot, el};

/// Overlay styles and the behaviour module.
pub static OVERLAY: stucco_core::Asset = stucco_core::Asset {
    name: "overlay",
    css: Some(include_str!("../../css/overlay.css")),
    behavior: Some(stucco_core::Behavior::Js(include_str!(
        "../../js/overlay.js"
    ))),
    deps: &[&crate::actions::ACTIONS],
};
stucco_core::register_asset!(OVERLAY);

/// An element id: an ASCII letter, then letters, digits, `-` or `_`.
fn valid_id(id: &str) -> bool {
    let mut chars = id.chars();
    let ok = chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    debug_assert!(ok, "invalid overlay id: {id:?}");
    ok
}

/// A modal dialog: the native `<dialog>`, which traps focus, closes on
/// Escape and returns focus to its opener by itself.
///
/// Open it with a button from [`Dialog::opener`], which uses the HTML
/// invoker commands (`commandfor`, `command="show-modal"`); the overlay
/// behaviour adds them to browsers that lack them. For pages that must work
/// without script, [`Dialog::link_opener`] is a link to a page of its own
/// that the script turns into opening the dialog in place.
///
/// Close it with [`Dialog::close_button`], the dialog's own close button,
/// Escape, or a click outside it. A form inside it submits as usual; a
/// `<form method="dialog">` closes it instead.
#[derive(Debug)]
pub struct Dialog<'a> {
    attrs: Attrs,
    id: String,
    title: String,
    body: Vec<Slot<'a>>,
    actions: Vec<Slot<'a>>,
}

impl<'a> Dialog<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["id", "aria-labelledby", "closedby"];

    /// A dialog with element id `id` (an ASCII letter, then letters, digits,
    /// `-` and `_`) and a heading.
    pub fn new(id: &str, title: impl Into<String>) -> Self {
        Dialog {
            attrs: Attrs::default(),
            id: if valid_id(id) {
                id.to_owned()
            } else {
                "dialog".to_owned()
            },
            title: title.into(),
            body: Vec::new(),
            actions: Vec::new(),
        }
    }

    /// Appends body content.
    pub fn child(mut self, child: impl Render + 'a) -> Self {
        self.body.push(Slot::new(child));
        self
    }

    /// Appends to the row of actions at the bottom, such as a cancel and a
    /// confirm button.
    pub fn actions(mut self, actions: impl Render + 'a) -> Self {
        self.actions.push(Slot::new(actions));
        self
    }

    /// The dialog's element id.
    pub fn element_id(&self) -> &str {
        &self.id
    }

    /// A button that opens this dialog.
    pub fn opener(&self, label: impl Render + 'static) -> Button<'static> {
        Button::new(label)
            .attr("commandfor", self.id.clone())
            .attr("command", "show-modal")
    }

    /// A link to `href` that opens this dialog in place when script runs,
    /// and goes to `href` (a page with the same content) when it doesn't.
    pub fn link_opener(
        &self,
        label: impl Render + 'static,
        href: impl Into<Href>,
    ) -> ButtonLink<'static> {
        ButtonLink::new(label, href).data("st-opens", self.id.clone())
    }

    /// A button that closes the dialog with id `id`, such as Cancel.
    pub fn close_button(id: &str, label: impl Render + 'static) -> Button<'static> {
        Button::new(label)
            .variant(Variant::Secondary)
            .attr("commandfor", id.to_owned())
            .attr("command", "close")
    }
}

passthrough!(Dialog<'_>);

impl Render for Dialog<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&OVERLAY);
        let title_id = format!("{}-title", self.id);
        let close = el::button()
            .attr("type", "button")
            .class("st-dialog-close")
            .attr("commandfor", self.id.clone())
            .attr("command", "close")
            .aria("label", "Close")
            .child(el::span().aria("hidden", "true").text("×"));
        let header = el::div()
            .class("st-dialog-header")
            .child(
                Heading::new(2, &self.title)
                    .size(Size::Lg)
                    .id(title_id.clone()),
            )
            .child(close);
        let actions = (!self.actions.is_empty()).then(|| {
            el::div()
                .class("st-dialog-actions")
                .children(self.actions.iter())
        });
        let dialog = el::dialog()
            .id(self.id.clone())
            .class("st-dialog")
            .aria("labelledby", title_id)
            .attr("closedby", "any")
            .child(header)
            .child(el::div().class("st-dialog-body").children(self.body.iter()))
            .child(actions);
        apply(dialog, &self.attrs, Self::RESERVED).render(cx);
    }
}

/// A menu of actions behind a button, in a popover: links, or buttons in
/// their own forms. It opens and closes without script (the popover API),
/// closes on Escape or a click elsewhere, and keeps the page's tab order;
/// the overlay behaviour anchors it under its button.
///
/// The items are a plain list, not an ARIA `menu`, so every link and button
/// works the usual way with a keyboard and assistive technology.
#[derive(Debug)]
pub struct Menu<'a> {
    attrs: Attrs,
    label: Slot<'a>,
    id: Option<String>,
    variant: Variant,
    size: Size,
    items: Vec<Option<Slot<'a>>>,
}

impl<'a> Menu<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &[];

    /// A menu behind a button labelled `label`.
    pub fn new(label: impl Render + 'a) -> Self {
        Menu {
            attrs: Attrs::default(),
            label: Slot::new(label),
            id: None,
            variant: Variant::Secondary,
            size: Size::Md,
            items: Vec::new(),
        }
    }

    /// The popover's element id (generated when not set).
    pub fn popover_id(mut self, id: &str) -> Self {
        if valid_id(id) {
            self.id = Some(id.to_owned());
        }
        self
    }

    /// The button's variant (default secondary).
    pub fn variant(mut self, variant: Variant) -> Self {
        self.variant = variant;
        self
    }

    /// The button's size.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Adds a link.
    pub fn link(mut self, label: impl Render + 'a, href: impl Into<Href>) -> Self {
        let link = el::a().class("st-menu-item").href(href).child(label);
        self.items.push(Some(Slot::new(link)));
        self
    }

    /// Adds any item, such as a form with one button that posts an action.
    pub fn item(mut self, item: impl Render + 'a) -> Self {
        self.items.push(Some(Slot::new(item)));
        self
    }

    /// Adds a dividing line between groups of items.
    pub fn separator(mut self) -> Self {
        self.items.push(None);
        self
    }
}

passthrough!(Menu<'_>);

impl Render for Menu<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&OVERLAY);
        let id = self.id.clone().unwrap_or_else(|| cx.id("menu"));
        let trigger = Button::new(el::span().child(&self.label))
            .variant(self.variant)
            .size(self.size)
            .attr("popovertarget", id.clone())
            .class("st-menu-button");
        let items = self.items.iter().map(|item| match item {
            Some(slot) => el::li().child(slot),
            // A visual divider only: a list holds list items.
            None => el::li().class("st-menu-separator").aria("hidden", "true"),
        });
        let popover = el::div()
            .id(id)
            .class("st-menu-popover")
            .attr("popover", "auto")
            .child(
                el::ul()
                    .class("st-menu-list")
                    .attr("role", "list")
                    .children(items),
            );
        apply(
            el::div().class("st-menu").child(trigger).child(popover),
            &self.attrs,
            Self::RESERVED,
        )
        .render(cx);
    }
}

/// A short status message, such as "Order saved" after a form redirects,
/// in a polite live region at the edge of the screen. With script it gets a
/// Dismiss button; it never disappears on its own, so nobody misses it.
#[derive(Debug)]
pub struct Toast<'a> {
    attrs: Attrs,
    message: Slot<'a>,
    tone: Tone,
}

impl<'a> Toast<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["role", "data-tone"];

    /// A toast showing `message`.
    pub fn new(message: impl Render + 'a) -> Self {
        Toast {
            attrs: Attrs::default(),
            message: Slot::new(message),
            tone: Tone::Default,
        }
    }

    /// The toast's tone: success, warning, danger or info colour its edge.
    pub fn tone(mut self, tone: Tone) -> Self {
        self.tone = tone;
        self
    }
}

passthrough!(Toast<'_>);

impl Render for Toast<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&OVERLAY);
        let toast = el::div()
            .class("st-toast")
            .attr("role", "status")
            .data("tone", self.tone.as_str())
            .child(el::p().class("st-toast-message").child(&self.message))
            .child(
                el::button()
                    .attr("type", "button")
                    .class("st-toast-dismiss")
                    .data("st-dismiss", "")
                    .text("Dismiss"),
            );
        apply(toast, &self.attrs, Self::RESERVED).render(cx);
    }
}

/// A short label shown beside a control on hover and keyboard focus, such
/// as the name of an icon button. It shows until the pointer or focus
/// leaves, can be hovered itself, and Escape hides it.
///
/// The tooltip is hidden from assistive technology, because the control
/// must already carry the same text as its accessible name (an icon
/// button's label does); it only makes that name visible.
#[derive(Debug)]
pub struct Tooltip<'a> {
    attrs: Attrs,
    trigger: Slot<'a>,
    text: String,
}

impl<'a> Tooltip<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &[];

    /// `text` shown beside `trigger`.
    pub fn new(trigger: impl Render + 'a, text: impl Into<String>) -> Self {
        Tooltip {
            attrs: Attrs::default(),
            trigger: Slot::new(trigger),
            text: text.into(),
        }
    }
}

passthrough!(Tooltip<'_>);

impl Render for Tooltip<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&OVERLAY);
        let wrapper = el::span()
            .class("st-tooltip-anchor")
            .child(&self.trigger)
            .child(
                el::span()
                    .class("st-tooltip")
                    .aria("hidden", "true")
                    .text(&self.text),
            );
        apply(wrapper, &self.attrs, Self::RESERVED).render(cx);
    }
}

#[cfg(test)]
mod tests;
