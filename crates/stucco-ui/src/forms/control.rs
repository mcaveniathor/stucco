//! The [`Control`] trait: how a [`super::Field`] connects a label, hint and
//! error to a form control.

use stucco_core::el::{Element, VoidElement};
use stucco_core::{Cx, Render};

/// What a `Field` passes to its control at render time.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Wiring {
    /// The control's id (the label's `for`).
    pub id: Option<String>,
    /// Space-separated ids of the hint and error, in that order.
    pub described_by: Option<String>,
    /// Whether the field has errors.
    pub invalid: bool,
    /// A submitted value to redisplay (never set for sensitive controls).
    pub value: Option<String>,
    /// Whether the field is required.
    pub required: bool,
}

/// A form control a [`super::Field`] can label and describe.
pub trait Control: Render {
    /// The submitted field name.
    fn name(&self) -> &str;
    /// The id set with `.id()`, if any.
    fn explicit_id(&self) -> Option<&str>;
    /// Whether submitted values must never be redisplayed (passwords).
    fn is_sensitive(&self) -> bool;
    /// Renders the control with the field's wiring applied.
    fn render_wired(&self, cx: &mut Cx, wiring: &Wiring);
}

/// Adds id, `aria-describedby`, `aria-invalid` and `required` to a control
/// element (`Element` or `VoidElement`).
macro_rules! wire_element {
    ($el:expr, $wiring:expr, $required:expr) => {{
        let mut el = $el;
        if let Some(id) = &$wiring.id {
            el = el.id(id.clone());
        }
        if let Some(d) = &$wiring.described_by {
            el = el.aria("describedby", d.clone());
        }
        if $wiring.invalid {
            el = el.aria("invalid", "true");
        }
        el.bool_attr("required", $required || $wiring.required)
    }};
}

pub(crate) fn wire_void(el: VoidElement, wiring: &Wiring, required: bool) -> VoidElement {
    wire_element!(el, wiring, required)
}

pub(crate) fn wire<'a>(el: Element<'a>, wiring: &Wiring, required: bool) -> Element<'a> {
    wire_element!(el, wiring, required)
}

/// The default wiring for a control rendered on its own.
pub(crate) fn unwired(explicit_id: Option<&str>) -> Wiring {
    Wiring {
        id: explicit_id.map(str::to_owned),
        ..Wiring::default()
    }
}

/// A control's passthrough attributes minus the reserved ones and the id
/// (which arrives through the wiring). Reserved names panic in debug builds.
pub(crate) fn passthrough_without_id(attrs: &stucco_core::Attrs) -> stucco_core::Attrs {
    let conflicts = attrs.reserved_conflicts(CONTROL_RESERVED);
    debug_assert!(
        conflicts.is_empty(),
        "reserved attribute set through passthrough: {conflicts:?}"
    );
    attrs.without(CONTROL_RESERVED).without(&["id"])
}

/// Reserved attributes shared by every control.
pub(crate) const CONTROL_RESERVED: &[&str] = &[
    "name",
    "type",
    "value",
    "required",
    "aria-describedby",
    "aria-invalid",
];
