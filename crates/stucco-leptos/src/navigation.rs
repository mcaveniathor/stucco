//! Links that show where the user is: [`NavLink`] and [`Tabs`].

use leptos::prelude::*;

use crate::href::safe_href;

/// A navigation link that marks the current page with `aria-current="page"`.
///
/// This is an ordinary link before and after hydration; the application decides
/// which link is current, usually from its router or request path. Unsafe URL
/// schemes such as `javascript:` render as `#`.
#[component]
pub fn NavLink(
    /// Visible link text.
    #[prop(into)]
    label: String,
    /// Destination URL.
    #[prop(into)]
    href: String,
    /// Whether this link is the current page.
    #[prop(default = Signal::derive(|| false), into)]
    current: Signal<bool>,
) -> impl IntoView {
    let href = safe_href(&href);
    view! {
        <a href=href aria-current=move || current.get().then_some("page")>{label}</a>
    }
}

/// Tabs across sections of one page, each a [`NavLink`] to its own URL.
///
/// The server renders the current tab, so tabs work without WASM, can be
/// bookmarked and keep the back button. `label` names the landmark.
#[component]
pub fn Tabs(
    /// Accessible name of the navigation landmark.
    #[prop(into)]
    label: String,
    /// The tabs, as [`NavLink`] children.
    children: Children,
) -> impl IntoView {
    view! { <nav class="st-tabs" aria-label=label>{children()}</nav> }
}
