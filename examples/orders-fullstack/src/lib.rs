//! A full-stack Orders reference using Leptos directly with Stucco components.
#![recursion_limit = "256"]
pub mod api;
pub mod attachment_api;
pub mod attachments;
pub mod auth;
mod auth_ui;
pub mod domain;
mod editor;
pub mod export_api;
#[cfg(feature = "ssr")]
pub mod export_routes;
pub mod exports;
mod gallery;
mod pages;
#[cfg(feature = "ssr")]
mod redb_store;
#[cfg(feature = "ssr")]
pub mod server;
#[cfg(feature = "ssr")]
pub mod sessions;
#[cfg(feature = "sqlite")]
pub mod sqlite_store;
#[cfg(feature = "ssr")]
pub mod store;
mod workflow_attachments;
mod workflow_exports;

use api::EditorState;
use leptos::prelude::*;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(crate) struct Drafts(pub RwSignal<BTreeMap<(auth::PrincipalKey, u64), EditorState>>);

#[derive(Clone)]
pub(crate) struct PendingDelete {
    state: api::DeleteState,
    uncertain: bool,
}
#[derive(Clone, Copy)]
pub(crate) struct PendingDeletes(pub RwSignal<BTreeMap<(auth::PrincipalKey, u64), PendingDelete>>);

pub fn shell(options: leptos::config::LeptosOptions) -> impl IntoView {
    view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <leptos_meta::MetaTags/>
                <link rel="stylesheet" href="/theme.css"/>
                <HydrationScripts options=options/>
            </head>
            <body><App/></body>
        </html>
    }
}

#[component]
pub fn App() -> impl IntoView {
    leptos_meta::provide_meta_context();
    let drafts = Drafts(RwSignal::new(BTreeMap::new()));
    provide_context(drafts);
    provide_context(PendingDeletes(RwSignal::new(BTreeMap::new())));
    view! { <Router><SessionBoundary drafts/></Router> }
}

#[component]
fn SessionBoundary(drafts: Drafts) -> impl IntoView {
    let login = use_context::<auth::LoginState>().unwrap_or_default();
    let session = Resource::new_blocking(
        || (),
        move |_| {
            let login = login.clone();
            async move { api::session().await.map(|session| (session, login)) }
        },
    );
    view! {
        // Bootstrap waits once. Later routed reads must not detach the account
        // controls, live regions, or a focused form while another request waits.
        <Transition fallback=|| view! { <p role="status">"Checking your session…"</p> }>
            {move || session.get().map(|result| match result {
                Ok((initial, login)) => view! { <AuthenticatedApp initial login drafts/> }.into_any(),
                Err(_) => view! { <h1>"Could not check your session"</h1><button type="button" on:click=move |_| session.refetch()>"Try again"</button> }.into_any(),
            })}
        </Transition>
    }
}

#[component]
fn AuthenticatedApp(
    initial: auth::SessionView,
    login: auth::LoginState,
    drafts: Drafts,
) -> impl IntoView {
    let auth = auth_ui::Auth::new(initial, drafts);
    #[cfg(feature = "hydrate")]
    let deletes = expect_context::<PendingDeletes>();
    provide_context(auth);
    provide_context(login);
    #[cfg(feature = "hydrate")]
    {
        let handle = leptos::leptos_dom::helpers::window_event_listener(
            leptos::ev::beforeunload,
            move |event| {
                if !drafts.0.get_untracked().is_empty() || !deletes.0.get_untracked().is_empty() {
                    event.prevent_default();
                    event.set_return_value("Unsaved changes");
                }
            },
        );
        let focus =
            leptos::leptos_dom::helpers::window_event_listener(leptos::ev::focus, move |_| {
                let epoch = auth.epoch.get_untracked();
                leptos::task::spawn_local(async move {
                    auth.refresh(epoch).await;
                });
            });
        on_cleanup(move || {
            handle.remove();
            focus.remove();
        });
    }
    view! {
        <main class="orders-shell">
            <nav aria-label="Main navigation"><a href="/orders" class="eyebrow">"STUCCO / ORDERS"</a><a href="/exports">"Exports"</a><a href="/components">"Components"</a><auth_ui::AccountMenu/></nav>
            <For
                each=move || vec![(auth.epoch.get(), auth.session.get().principal.map(|principal| principal.key))]
                key=|(epoch, _)| *epoch
                children=move |(_, key)| match key {
                    Some(key) => view! { <ProtectedApp scope=auth_ui::AuthScope::new(auth, key)/> }.into_any(),
                    None => view! { <auth_ui::LoginPage/> }.into_any(),
                }
            />
            <footer>"A Rust application with Stucco’s design system."</footer>
        </main>
    }
}

#[component]
fn ProtectedApp(scope: auth_ui::AuthScope) -> impl IntoView {
    provide_context(scope);
    view! {
        <Routes fallback=|| view! { <NotFound page=true/> }>
            <Route path=path!("/login") view=|| view! { <auth_ui::LoginPage/> }.into_any()/>
            <Route path=path!("/components") view=gallery::ComponentGallery/>
            <Route path=path!("/exports") view=workflow_exports::ExportList/>
            <Route path=path!("/exports/:token") view=workflow_exports::ExportDetail/>
            <Route path=path!("/orders") view=|| view! { <pages::OrderList/> }.into_any()/>
            <Route path=path!("/orders/new") view=|| view! { <editor::EditorPage create=true/> }.into_any()/>
            <Route path=path!("/orders/:id") view=|| view! { <pages::OrderDetail/> }.into_any()/>
            <Route path=path!("/orders/:id/edit") view=|| view! { <editor::EditorPage/> }.into_any()/>
            <Route path=path!("/orders/:id/delete") view=|| view! { <pages::DeletePage/> }.into_any()/>
        </Routes>
    }
}

pub(crate) fn not_found_status() {
    #[cfg(feature = "ssr")]
    if let Some(response) = use_context::<leptos_axum::ResponseOptions>() {
        response.set_status(axum::http::StatusCode::NOT_FOUND);
    }
}

#[component]
pub(crate) fn NotFound(#[prop(default = false)] page: bool) -> impl IntoView {
    not_found_status();
    let heading = NodeRef::<leptos::html::H1>::new();
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if let Some(element) = heading.get() {
            focus_if_unclaimed(&element);
        }
    });
    let title = if page {
        "Page not found"
    } else {
        "Order not found"
    };
    view! {
        <leptos_meta::Title text=format!("{title} · Stucco")/>
        <h1 node_ref=heading tabindex="-1">{title}</h1><p>{if page {"This page does not exist."} else {"This order does not exist."}}</p><a href="/orders">"Back to orders"</a>
    }
}

// Hydration must preserve focus acquired before WASM loaded. Route mounts may
// claim focus after the old focused node has been removed, or after activation
// of the persistent app navigation, whose focused link survives a route change.
#[cfg(feature = "hydrate")]
pub(crate) fn focus_if_unclaimed(element: &leptos::web_sys::HtmlElement) {
    fn focus(element: &leptos::web_sys::HtmlElement) {
        if element.is_connected()
            && document().active_element().is_none_or(|active| {
                active.tag_name() == "BODY"
                    || active
                        .closest(".orders-shell > nav[aria-label='Main navigation']")
                        .ok()
                        .flatten()
                        .is_some()
            })
        {
            let _ = element.focus();
        }
    }
    if element.is_connected() {
        focus(element);
    } else {
        // A router transition can resolve the resource before attaching its
        // staged view. Focus after that commit, never a detached node.
        let element = element.clone();
        request_animation_frame(move || focus(&element));
    }
}

pub(crate) fn route_id() -> Memo<Option<u64>> {
    let params = leptos_router::hooks::use_params_map();
    Memo::new(move |_| {
        params
            .get()
            .get("id")
            .and_then(|id| id.parse().ok())
            .filter(|id| *id > 0)
    })
}

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}
