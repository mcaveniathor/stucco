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

/// One step of a [`Steps`] flow.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepItem {
    /// Visible step name.
    pub label: String,
    /// Where a finished step links back to; `None` keeps it plain text.
    pub href: Option<String>,
}

/// Where the user is in a multi-page flow, such as a checkout.
///
/// Finished steps link back, the current step is marked `aria-current="step"`,
/// and later steps are text. Each step's state is also written in visually hidden
/// text, which `completed_label` and `not_started_label` let you translate.
/// A `current` past the end means the last step; no steps render nothing.
#[component]
pub fn Steps(
    /// Accessible name of the navigation landmark.
    #[prop(into)]
    label: String,
    /// The steps, in order.
    steps: Vec<StepItem>,
    /// Zero-based index of the current step.
    #[prop(default = Signal::derive(|| 0), into)]
    current: Signal<usize>,
    /// Hidden text added after a finished step's name.
    #[prop(default = " (completed)".to_owned(), into)]
    completed_label: String,
    /// Hidden text added after a later step's name.
    #[prop(default = " (not started)".to_owned(), into)]
    not_started_label: String,
) -> impl IntoView {
    if steps.is_empty() {
        return ().into_any();
    }
    let last = steps.len() - 1;
    let items = move || {
        let current = current.get().min(last);
        steps
            .iter()
            .enumerate()
            .map(|(index, step)| {
                let name = step.label.clone();
                let (state, body) = if index < current {
                    let hidden =
                        view! { <span class="st-sr-only">{completed_label.clone()}</span> };
                    let body = match &step.href {
                        Some(href) => {
                            let href = safe_href(href);
                            view! { <a href=href>{name}{hidden}</a> }.into_any()
                        }
                        None => view! { <span>{name}{hidden}</span> }.into_any(),
                    };
                    ("done", body)
                } else if index == current {
                    (
                        "current",
                        view! { <span aria-current="step">{name}</span> }.into_any(),
                    )
                } else {
                    let hidden =
                        view! { <span class="st-sr-only">{not_started_label.clone()}</span> };
                    ("todo", view! { <span>{name}{hidden}</span> }.into_any())
                };
                view! {
                    <li data-state=state>
                        <span class="st-steps-marker" aria-hidden="true">{index + 1}</span>
                        {body}
                    </li>
                }
            })
            .collect_view()
    };
    view! { <nav class="st-steps" aria-label=label><ol>{items}</ol></nav> }.into_any()
}
