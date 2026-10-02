//! Passthrough attributes and reserved-attribute enforcement (spec §4.5).

use stucco_core::Attrs;
use stucco_core::el::Element;

/// Generates `.class() .id() .attr() .data() .aria()` for a component struct
/// with an `attrs: Attrs` field.
#[allow(unused_macros)] // unused when no component family is enabled
macro_rules! passthrough {
    (<$generic:ident> $ty:ty) => { passthrough!(@impl [<$generic>] $ty); };
    ($ty:ty) => { passthrough!(@impl [] $ty); };
    (@impl [$($generics:tt)*] $ty:ty) => {
        impl $($generics)* $ty {
            /// Adds space-separated classes to the root element.
            pub fn class(mut self, classes: impl AsRef<str>) -> Self {
                self.attrs = self.attrs.class(classes);
                self
            }
            /// Sets the root element's id.
            pub fn id(mut self, id: impl Into<String>) -> Self {
                self.attrs = self.attrs.id(id);
                self
            }
            /// Sets an attribute on the root element (safety policy applies;
            /// reserved attributes are refused).
            pub fn attr(mut self, name: &str, value: impl Into<String>) -> Self {
                self.attrs = self.attrs.attr(name, value);
                self
            }
            /// Sets `data-{name}` on the root element.
            pub fn data(mut self, name: &str, value: impl Into<String>) -> Self {
                self.attrs = self.attrs.data(name, value);
                self
            }
            /// Sets `aria-{name}` on the root element.
            pub fn aria(mut self, name: &str, value: impl Into<String>) -> Self {
                self.attrs = self.attrs.aria(name, value);
                self
            }
        }
    };
}

/// Applies passthrough `attrs` to a component's root element, refusing the
/// component's `reserved` attributes: debug builds panic ("reserved
/// attribute"), release builds ignore them.
#[cfg_attr(
    not(any(
        feature = "layout",
        feature = "typography",
        feature = "actions",
        feature = "forms"
    )),
    allow(dead_code)
)]
pub(crate) fn apply<'a>(el: Element<'a>, attrs: &Attrs, reserved: &[&str]) -> Element<'a> {
    let conflicts = attrs.reserved_conflicts(reserved);
    debug_assert!(
        conflicts.is_empty(),
        "reserved attribute set through passthrough: {conflicts:?}"
    );
    el.attrs(&attrs.without(reserved))
}
#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::{el, to_html};

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "reserved attribute")]
    fn reserved_passthrough_panics_in_debug() {
        let _ = apply(el::div(), &Attrs::default().attr("role", "x"), &["role"]);
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn reserved_passthrough_is_ignored_in_release() {
        let html = to_html(&apply(
            el::div().attr("role", "list"),
            &Attrs::default().attr("role", "x").class("c"),
            &["role"],
        ));
        assert_eq!(html, r#"<div class="c" role="list"></div>"#);
    }

    #[test]
    fn unreserved_passthrough_merges() {
        let html = to_html(&apply(
            el::div().class("st-x"),
            &Attrs::default().class("y").data("k", "v"),
            &["role"],
        ));
        assert_eq!(html, r#"<div class="st-x y" data-k="v"></div>"#);
    }
}
