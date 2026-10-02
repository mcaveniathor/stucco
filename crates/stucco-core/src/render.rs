//! The [`Render`] trait, the render context [`Cx`], slots and composition.

use std::borrow::Cow;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use crate::escape::escape_text;
use crate::identity::Identity;
use crate::{Asset, AssetRequirements};

/// Anything that can write itself as HTML into a [`Cx`].
///
/// Strings and numbers render as escaped text; `Option`, `Vec`, slices and
/// tuples render their contents in order.
pub trait Render {
    /// Writes this value into `cx`.
    fn render(&self, cx: &mut Cx);
}

/// The render context: output buffer, identity state and (later) asset
/// requirements for one page or fragment.
#[derive(Debug, Default)]
pub struct Cx {
    out: String,
    ids: Identity,
    required: Vec<&'static Asset>,
}

impl Cx {
    /// A context with the empty id namespace, as used by full pages.
    pub fn new() -> Cx {
        Cx::default()
    }

    /// A context whose generated ids are prefixed with `namespace`.
    pub(crate) fn with_namespace(namespace: &str) -> Cx {
        let mut cx = Cx::default();
        cx.ids.namespace = namespace.to_owned();
        cx
    }

    /// Returns a fresh id `"{prefix}-{n}"` (or `"{namespace}-{prefix}-{n}"`),
    /// counting per prefix from 1. It is recorded when emitted as an element id.
    pub fn id(&mut self, prefix: &str) -> String {
        self.ids.generate(prefix)
    }

    /// Records an explicit id. A duplicate within one context panics in debug
    /// builds ("duplicate id") and is kept in release builds.
    pub fn claim_id(&mut self, id: &str) {
        self.ids.claim(id);
    }

    /// Writes escaped text.
    pub fn text(&mut self, s: &str) {
        escape_text(s, &mut self.out);
    }

    /// Writes markup verbatim. Only the crate's own safe constructs use this.
    pub(crate) fn raw(&mut self, s: &str) {
        self.out.push_str(s);
    }

    /// Records that the output needs `asset` (and its dependencies).
    pub fn require(&mut self, asset: &'static Asset) {
        if !self.required.iter().any(|a| std::ptr::eq(*a, asset)) {
            self.required.push(asset);
        }
    }

    /// Finishes rendering: the HTML and the resolved asset requirements.
    pub fn finish(self) -> (String, AssetRequirements) {
        let assets = AssetRequirements::resolve(&self.required);
        (self.out, assets)
    }
}

/// Renders `r` on its own and returns only the HTML.
///
/// Asset requirements are discarded; use `render_fragment` for partial
/// responses, or `Page` for documents.
pub fn to_html(r: &(impl Render + ?Sized)) -> String {
    let mut cx = Cx::new();
    r.render(&mut cx);
    cx.finish().0
}

/// Wraps a closure as a [`Render`] value.
pub fn render_fn<F: Fn(&mut Cx)>(f: F) -> RenderFn<F> {
    RenderFn(f)
}

/// A closure that renders; see [`render_fn`].
pub struct RenderFn<F>(F);

impl<F: Fn(&mut Cx)> Render for RenderFn<F> {
    fn render(&self, cx: &mut Cx) {
        (self.0)(cx)
    }
}

impl<F> fmt::Debug for RenderFn<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("RenderFn")
    }
}

/// Arbitrary renderable content held by a component.
///
/// The lifetime lets slots borrow request data. `Debug` prints the type the
/// slot was built from, so components holding slots can derive `Debug`.
pub struct Slot<'a> {
    inner: Box<dyn Render + 'a>,
    type_name: &'static str,
}

impl<'a> Slot<'a> {
    /// Boxes `r` as a slot.
    pub fn new<R: Render + 'a>(r: R) -> Slot<'a> {
        Slot {
            inner: Box::new(r),
            type_name: std::any::type_name::<R>(),
        }
    }
}

impl Render for Slot<'_> {
    fn render(&self, cx: &mut Cx) {
        self.inner.render(cx)
    }
}

impl fmt::Debug for Slot<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Slot({})", self.type_name)
    }
}

/// Trusted markup rendered verbatim. This is the escape hatch for HTML; every
/// use is a trust decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Raw(String);

impl Raw {
    /// Marks `html` as trusted markup.
    pub fn trusted(html: impl Into<String>) -> Raw {
        Raw(html.into())
    }
}

impl Render for Raw {
    fn render(&self, cx: &mut Cx) {
        cx.raw(&self.0);
    }
}

impl Render for str {
    fn render(&self, cx: &mut Cx) {
        cx.text(self);
    }
}

impl Render for String {
    fn render(&self, cx: &mut Cx) {
        cx.text(self);
    }
}

impl Render for Cow<'_, str> {
    fn render(&self, cx: &mut Cx) {
        cx.text(self);
    }
}

impl Render for char {
    fn render(&self, cx: &mut Cx) {
        cx.text(self.encode_utf8(&mut [0; 4]));
    }
}

macro_rules! render_display {
    ($($t:ty),*) => {$(
        impl Render for $t {
            fn render(&self, cx: &mut Cx) {
                cx.raw(&self.to_string());
            }
        }
    )*};
}
render_display!(
    i8, i16, i32, i64, i128, isize, u8, u16, u32, u64, u128, usize
);

macro_rules! render_float {
    ($($t:ty),*) => {$(
        /// Non-finite values render nothing.
        impl Render for $t {
            fn render(&self, cx: &mut Cx) {
                if self.is_finite() {
                    cx.raw(&self.to_string());
                }
            }
        }
    )*};
}
render_float!(f32, f64);

impl<T: Render> Render for Option<T> {
    fn render(&self, cx: &mut Cx) {
        if let Some(v) = self {
            v.render(cx);
        }
    }
}

impl<T: Render> Render for [T] {
    fn render(&self, cx: &mut Cx) {
        for v in self {
            v.render(cx);
        }
    }
}

impl<T: Render> Render for Vec<T> {
    fn render(&self, cx: &mut Cx) {
        self.as_slice().render(cx);
    }
}

impl<T: Render + ?Sized> Render for &T {
    fn render(&self, cx: &mut Cx) {
        (**self).render(cx);
    }
}

impl<T: Render + ?Sized> Render for Box<T> {
    fn render(&self, cx: &mut Cx) {
        (**self).render(cx);
    }
}

impl<T: Render + ?Sized> Render for Rc<T> {
    fn render(&self, cx: &mut Cx) {
        (**self).render(cx);
    }
}

impl<T: Render + ?Sized> Render for Arc<T> {
    fn render(&self, cx: &mut Cx) {
        (**self).render(cx);
    }
}

macro_rules! render_tuple {
    ($($name:ident)+) => {
        impl<$($name: Render),+> Render for ($($name,)+) {
            #[allow(non_snake_case)]
            fn render(&self, cx: &mut Cx) {
                let ($($name,)+) = self;
                $($name.render(cx);)+
            }
        }
    };
}
render_tuple!(A);
render_tuple!(A B);
render_tuple!(A B C);
render_tuple!(A B C D);
render_tuple!(A B C D E);
render_tuple!(A B C D E F);
render_tuple!(A B C D E F G);
render_tuple!(A B C D E F G H);
render_tuple!(A B C D E F G H I);
render_tuple!(A B C D E F G H I J);
render_tuple!(A B C D E F G H I J K);
render_tuple!(A B C D E F G H I J K L);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_render_escaped_and_compose() {
        let none: Option<&str> = None;
        assert_eq!(
            to_html(&("a<b", none, Some(3u8), vec!['&', 'x'], 1.5f64)),
            "a&lt;b3&amp;x1.5"
        );
    }

    #[test]
    fn raw_is_the_only_unescaped_path() {
        assert_eq!(to_html(&Raw::trusted("<b>x</b>")), "<b>x</b>");
        assert_eq!(to_html(&"<b>x</b>"), "&lt;b&gt;x&lt;/b&gt;");
    }

    #[test]
    fn generated_ids_are_per_prefix_and_namespaced() {
        let mut cx = Cx::new();
        assert_eq!(
            (cx.id("tabs"), cx.id("tabs"), cx.id("menu")),
            ("tabs-1".into(), "tabs-2".into(), "menu-1".into())
        );
        let mut ns = Cx::with_namespace("order-42");
        assert_eq!(ns.id("tabs"), "order-42-tabs-1");
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "duplicate id")]
    fn duplicate_explicit_ids_panic_in_debug() {
        let mut cx = Cx::new();
        cx.claim_id("orders");
        cx.claim_id("orders");
    }

    #[test]
    fn a_generated_id_can_be_emitted_once() {
        let html = to_html(&render_fn(|cx: &mut Cx| {
            let id = cx.id("probe");
            crate::el::div().id(id).render(cx);
        }));
        assert_eq!(html, r#"<div id="probe-1"></div>"#);
    }

    #[test]
    fn slots_borrow_and_debug_print_their_type() {
        let owned = String::from("<hi>");
        let s = Slot::new(&owned);
        assert_eq!(to_html(&s), "&lt;hi&gt;");
        assert_eq!(format!("{s:?}"), "Slot(&alloc::string::String)");
    }

    #[test]
    fn closures_render() {
        assert_eq!(to_html(&render_fn(|cx: &mut Cx| cx.text("<"))), "&lt;");
    }
}
