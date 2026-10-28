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

/// The trail from the top of the site to this page.
///
/// Fill it with [`BreadcrumbLink`] items for the pages above this one and a final
/// [`BreadcrumbCurrent`]. The separators are drawn by CSS, so screen readers do
/// not read them. Place it above `PageHeader`; the header has no slot for it.
#[component]
pub fn Breadcrumbs(
    /// Accessible name of the landmark; name it when a page has more than one trail.
    #[prop(default = "Breadcrumb".to_owned(), into)]
    label: String,
    /// The trail's items.
    children: Children,
) -> impl IntoView {
    view! { <nav class="st-breadcrumbs" aria-label=label><ol>{children()}</ol></nav> }
}

/// A page above the current one in a [`Breadcrumbs`] trail.
#[component]
pub fn BreadcrumbLink(
    /// Destination URL; unsafe schemes such as `javascript:` render as `#`.
    #[prop(into)]
    href: String,
    /// Link text.
    children: Children,
) -> impl IntoView {
    let href = safe_href(&href);
    view! { <li><a href=href>{children()}</a></li> }
}

/// The current page, last in a [`Breadcrumbs`] trail.
#[component]
pub fn BreadcrumbCurrent(
    /// The page's name.
    children: Children,
) -> impl IntoView {
    view! { <li><span aria-current="page">{children()}</span></li> }
}
