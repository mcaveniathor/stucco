//! Collection, detail and deletion screens built from ordinary Leptos components.

use crate::{
    Drafts, NotFound, PendingDelete, PendingDeletes, api,
    auth_ui::{AccessRequired, AuthScope},
    domain::{DeleteError, Order, OrderPage, money},
};
use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::{components::Form, hooks::use_navigate, hooks::use_query_map};
use stucco_leptos::{
    ErrorSummary, PageHeader, StatusMessage, SubmitButton, SubmitVariant, TextField,
};

fn page_href(query: &str, page: u32) -> String {
    let query = form_urlencoded::Serializer::new(String::new())
        .append_pair("q", query)
        .append_pair("page", &page.to_string())
        .finish();
    format!("/orders?{query}")
}

#[component]
pub(crate) fn OrderList() -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let params = use_query_map();
    let query = Memo::new(move |_| params.get().get("q").unwrap_or_default());
    let page = Memo::new(move |_| {
        params
            .get()
            .get("page")
            .and_then(|page| page.parse::<u32>().ok())
            .unwrap_or(1)
            .max(1)
    });
    let search = RwSignal::new(query.get_untracked());
    let submitted = RwSignal::new(false);
    let hydrated = RwSignal::new(false);
    let heading = NodeRef::<leptos::html::Div>::new();
    let results_heading = NodeRef::<leptos::html::H2>::new();
    // This Leptos release runs one resource's fetches serially. Give each URL
    // its own resource so a slow earlier search cannot delay the latest one.
    // The memo's owner disposes the previous resource when the request changes.
    let orders = Memo::new_with_compare(
        move |_| {
            let request = (query.get(), page.get());
            Resource::new(
                || (),
                move |_| {
                    let (query, page) = request.clone();
                    async move {
                        let result = api::list_orders(
                            query.clone(),
                            page,
                            scope.key.user_id,
                            scope.key.tenant_id,
                            scope.key.generation,
                        )
                        .await;
                        ((query, page), result)
                    }
                },
            )
        },
        |_, _| true,
    );
    let status = Signal::derive(move || match orders.get().get() {
        Some((key, result)) if key == (query.get(), page.get()) => match result {
            Ok(Ok(result)) => format!(
                "{} orders · Page {} of {}",
                result.total_count, result.page, result.total_pages
            ),
            _ => String::new(),
        },
        _ => "Loading orders…".to_owned(),
    });

    // Synchronize browser Back/Forward and pagination without replacing text
    // newly entered into the server-rendered field before hydration.
    let previous_query = std::cell::RefCell::new(query.get_untracked());
    Effect::new(move |_| {
        let current = query.get();
        if *previous_query.borrow() != current {
            search.set(current.clone());
            *previous_query.borrow_mut() = current;
        }
    });
    Effect::new(move |_| hydrated.set(true));
    #[cfg(feature = "hydrate")]
    {
        let focused_query = std::cell::RefCell::new(None);
        Effect::new(move |_| {
            let requested = (query.get(), page.get());
            let was_submitted = submitted.get();
            let Some((loaded, _)) = orders.get().get() else {
                return;
            };
            if loaded != requested {
                return;
            }
            if focused_query.borrow().is_none() {
                if let Some(element) = heading.get() {
                    crate::focus_if_unclaimed(&element);
                    *focused_query.borrow_mut() = Some(loaded);
                }
            } else if (focused_query.borrow().as_ref() != Some(&loaded) || was_submitted)
                && let Some(element) = results_heading.get()
            {
                let _ = element.focus();
                *focused_query.borrow_mut() = Some(loaded);
                submitted.set(false);
            }
        });
    }

    view! {
        <Title text="Orders · Stucco"/>
        <Transition fallback=|| view! { <p role="status">"Loading orders…"</p> }>
        <section data-hydrated=move || hydrated.get().to_string()>
            <div node_ref=heading tabindex="-1" class="page-heading">
                <PageHeader heading="Orders" description="Find an order, review its details, or start a new one.">
                    <a href="/orders/new" class="st-button" data-variant="primary" data-size="md">"New order"</a>
                </PageHeader>
            </div>
            <Form action="/orders" method="get" attr:autocomplete="off" on_form_data=std::sync::Arc::new(move |_| submitted.set(true))>
                <div class="search-form">
                    <TextField id="order-search" name="q" label="Search customers" value=search
                        on_input=Callback::new(move |value| search.set(value))/>
                    <SubmitButton label="Search"/>
                </div>
            </Form>
            <h2 node_ref=results_heading tabindex="-1" class="results-heading">"Order results"</h2>
            <StatusMessage message=status/>
            // Each result boundary must also be replaced. A shared Suspense
            // still waits for the old request's task after its resource changes.
            <For
                each=move || vec![((query.get(), page.get()), orders.get())]
                key=|(request, _)| request.clone()
                children=move |(_, current)| view! {
                    <Suspense fallback=|| view! { <p>"Loading orders…"</p> }>
                        {move || current.get().map(|(_, result)| match result {
                            Ok(Ok(result)) => view! { <OrderResults result=result/> }.into_any(),
                            Ok(Err(reason)) => view! { <AccessRequired reason/> }.into_any(),
                Err(_) => view! {
                                <div class="order-card">
                                    <p role="alert">"The orders could not be loaded. Please try again."</p>
                                    <button class="st-button" type="button" on:click=move |_| current.refetch()>"Try again"</button>
                                </div>
                            }.into_any(),
                        })}
                    </Suspense>
                }
            />
        </section>
        </Transition>
    }
}

#[component]
fn OrderResults(result: OrderPage) -> impl IntoView {
    let previous = (result.page > 1).then(|| page_href(&result.query, result.page - 1));
    let next =
        (result.page < result.total_pages).then(|| page_href(&result.query, result.page + 1));
    let empty = result.orders.is_empty();
    let has_pages = previous.is_some() || next.is_some();
    view! {
        <div class="order-card">
            {if empty {
                view! { <p>"No orders match this search. Try another customer name or create an order."</p> }.into_any()
            } else {
                view! {
                    <div class="table-scroll" tabindex="0" role="region" aria-labelledby="order-results-caption">
                        <table class="st-table">
                            <caption id="order-results-caption" class="st-sr-only">"Orders matching your search"</caption>
                            <thead><tr><th scope="col">"Order"</th><th scope="col">"Customer"</th><th scope="col">"Total"</th><th scope="col">"Revision"</th></tr></thead>
                            <tbody>{result.orders.into_iter().map(|order| {
                                let total = order.total_cents().map(money).unwrap_or_else(|error| error.to_string());
                                view! {
                                    <tr>
                                        <th scope="row"><a href=format!("/orders/{}", order.id)>{format!("Order #{}", order.id)}</a></th>
                                        <td>{order.customer}</td><td>{total}</td><td>{order.revision}</td>
                                    </tr>
                                }
                            }).collect_view()}</tbody>
                        </table>
                    </div>
                }.into_any()
            }}
            {has_pages.then(|| view! { <nav class="pagination" aria-label="Order pages">
                {previous.map(|href| view! { <a href=href rel="prev">"Previous page"</a> })}
                {next.map(|href| view! { <a href=href rel="next">"Next page"</a> })}
            </nav> })}
        </div>
    }
}

#[component]
pub(crate) fn OrderDetail() -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let id = crate::route_id();
    let order = Resource::new_blocking(
        move || id.get(),
        move |id| async move {
            match id {
                Some(id) => {
                    api::load_order(
                        id,
                        scope.key.user_id,
                        scope.key.tenant_id,
                        scope.key.generation,
                    )
                    .await
                }
                None => Ok(Ok(None)),
            }
        },
    );
    view! {
        <Title text=move || id.try_get().flatten().map(|id| format!("Order #{id} · Stucco")).unwrap_or_else(|| "Order not found · Stucco".into())/>
        <Suspense fallback=|| view! { <p role="status">"Loading order…"</p> }>
            {move || order.get().map(|result| match result {
                Ok(Ok(Some(state))) => view! { <OrderDetails order=state.order/> }.into_any(),
                Ok(Ok(None)) => view! { <NotFound/> }.into_any(),
                Ok(Err(reason)) => view! { <AccessRequired reason/> }.into_any(),
                Err(_) => view! {
                    <PageHeader heading="Order unavailable"/>
                    <p role="alert">"The order could not be loaded. Please try again."</p>
                    <button class="st-button" type="button" on:click=move |_| order.refetch()>"Try again"</button>
                    <a href="/orders">"Back to orders"</a>
                }.into_any(),
            })}
        </Suspense>
    }
}

#[component]
fn OrderDetails(order: Order) -> impl IntoView {
    let hydrated = RwSignal::new(false);
    let heading = NodeRef::<leptos::html::Div>::new();
    Effect::new(move |_| hydrated.set(true));
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if let Some(element) = heading.get() {
            crate::focus_if_unclaimed(&element);
        }
    });
    let total = order
        .total_cents()
        .map(money)
        .unwrap_or_else(|error| error.to_string());
    view! {
        <section data-hydrated=move || hydrated.get().to_string()>
            <div node_ref=heading tabindex="-1" class="page-heading">
                <PageHeader heading=format!("Order #{}", order.id) description=order.customer>
                    <div class="form-actions">
                        <a href=format!("/orders/{}/edit", order.id) class="st-button" data-variant="primary" data-size="md">"Edit order"</a>
                        <a href=format!("/orders/{}/delete", order.id)>"Delete order"</a>
                        <a href="/orders">"Back to orders"</a>
                    </div>
                </PageHeader>
            </div>
            <div class="order-card">
                <p>{format!("Saved revision {}", order.revision)}</p>
                <div class="table-scroll" tabindex="0" role="region" aria-labelledby="order-items-caption">
                    <table class="st-table">
                        <caption id="order-items-caption">"Line items"</caption>
                        <thead><tr><th scope="col">"Item"</th><th scope="col">"Unit price"</th><th scope="col">"Quantity"</th></tr></thead>
                        <tbody>{order.items.into_iter().map(|item| view! {
                            <tr><th scope="row">{item.name}</th><td>{money(item.unit_price_cents)}</td><td>{item.quantity}</td></tr>
                        }).collect_view()}</tbody>
                    </table>
                </div>
                <div class="total"><span>"Order total"</span><output>{total}</output></div>
            </div>
            <crate::workflow_attachments::AttachmentPanel order_id=order.id/>
        </section>
    }
}

#[component]
pub(crate) fn DeletePage() -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let id = crate::route_id();
    let order = Resource::new_blocking(
        move || id.get(),
        move |id| async move {
            match id {
                Some(id) => {
                    api::load_delete(
                        id,
                        scope.key.user_id,
                        scope.key.tenant_id,
                        scope.key.generation,
                    )
                    .await
                }
                None => Ok(Ok(None)),
            }
        },
    );
    view! {
        <Title text=move || id.try_get().flatten().map(|id| format!("Delete order #{id} · Stucco")).unwrap_or_else(|| "Order not found · Stucco".into())/>
        <Suspense fallback=|| view! { <p role="status">"Loading order…"</p> }>
            {move || order.get().map(|result| match result {
                Ok(Ok(Some(state))) => view! { <DeleteConfirmation state=state reload=Callback::new(move |_| order.refetch())/> }.into_any(),
                Ok(Ok(None)) => view! { <NotFound/> }.into_any(),
                Ok(Err(reason)) => view! { <AccessRequired reason/> }.into_any(),
                Err(_) => view! {
                    <PageHeader heading="Order unavailable"/>
                    <p role="alert">"The order could not be loaded. Please try again."</p>
                    <button class="st-button" type="button" on:click=move |_| order.refetch()>"Try again"</button>
                    <a href="/orders">"Back to orders"</a>
                }.into_any(),
            })}
        </Suspense>
    }
}

fn deletion_message(error: &DeleteError) -> String {
    match error {
        DeleteError::Conflict { current, .. } => format!(
            "This order changed to revision {} for {}. It was not deleted. Reload the saved order to review the changes before deleting.",
            current.revision, current.customer,
        ),
        _ => error.to_string(),
    }
}

#[component]
fn DeleteConfirmation(state: api::DeleteState, reload: Callback<()>) -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let id = state.order.id;
    let deletes = expect_context::<PendingDeletes>();
    let retained = deletes
        .0
        .with_untracked(|pending| pending.get(&(scope.key, id)).cloned());
    let uncertain_initial = retained.as_ref().is_some_and(|pending| pending.uncertain);
    let mut state = retained
        .map(|pending| pending.state)
        .unwrap_or_else(|| state.clone());
    // Recover the original revision, with the freshly authenticated CSRF token.
    state.csrf = scope.auth.session.get_untracked().csrf;
    if matches!(state.error, Some(DeleteError::Access { .. })) {
        state.error = None;
    }
    let snapshot = StoredValue::new(state.clone());
    let total = state
        .order
        .total_cents()
        .map(money)
        .unwrap_or_else(|error| error.to_string());
    let drafts = expect_context::<Drafts>();
    let revision = StoredValue::new(state.expected_revision);
    let csrf = Signal::derive(move || scope.auth.session.get().csrf);
    let has_initial_error = state.error.is_some();
    let error = RwSignal::new(state.error);
    let focus_trigger = RwSignal::new(u32::from(has_initial_error));
    let message = Signal::derive(move || {
        error
            .get()
            .as_ref()
            .map(deletion_message)
            .unwrap_or_default()
    });
    let notice = RwSignal::new(if uncertain_initial {
        "The deletion outcome is unknown. Check the orders list before trying again.".into()
    } else {
        String::new()
    });
    let uncertain = RwSignal::new(uncertain_initial);
    let hydrated = RwSignal::new(false);
    let heading = NodeRef::<leptos::html::Div>::new();
    let navigate = use_navigate();
    let delete = Action::new(move |_: &()| {
        let revision = revision.get_value();
        let token = csrf.get_untracked();
        async move {
            let result = api::delete_order(
                id,
                revision,
                token,
                scope.key.user_id,
                scope.key.tenant_id,
                scope.key.generation,
            )
            .await;
            if !scope.current() {
                return result;
            }
            // The root draft map survives route teardown. Reconcile a definite
            // deletion even when this confirmation screen is no longer mounted.
            if matches!(&result, Ok(Ok(()))) {
                let _ = drafts.0.try_update(|drafts| {
                    drafts.remove(&(scope.key, id));
                });
            }
            let _ = deletes.0.try_update(|pending| match &result {
                Ok(Ok(())) => {
                    pending.remove(&(scope.key, id));
                }
                Ok(Err(error)) => {
                    if let Some(state) = pending.get_mut(&(scope.key, id)) {
                        state.state.error = Some(error.clone());
                        state.uncertain = false;
                    }
                }
                Err(error) if crate::auth_ui::csrf_rejected(error) => {
                    if let Some(state) = pending.get_mut(&(scope.key, id)) {
                        state.state.error = None;
                        state.uncertain = false;
                    }
                }
                Err(_) => {}
            });
            match &result {
                Ok(Err(DeleteError::Access { .. })) => scope.reject(),
                Err(_) => scope.check_session().await,
                _ => {}
            }
            result
        }
    });
    let pending = delete.pending();

    Effect::new(move |_| hydrated.set(true));
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if !has_initial_error && let Some(element) = heading.get() {
            crate::focus_if_unclaimed(&element);
        }
    });
    Effect::new(move |_| {
        if let Some(result) = delete.value().get() {
            if !scope.current() {
                return;
            }
            match result {
                Ok(Ok(())) => {
                    navigate("/orders", Default::default());
                }
                Ok(Err(failure)) => {
                    error.set(Some(failure));
                    focus_trigger.update(|version| *version = version.saturating_add(1));
                }
                Err(failure) if crate::auth_ui::csrf_rejected(&failure) => {
                    error.set(None);
                    uncertain.set(false);
                    notice.set("Form protection rejected the request before deleting. The order was not deleted. Review and submit again.".into());
                }
                Err(_) => {
                    error.set(None);
                    uncertain.set(true);
                    notice.set("The deletion outcome is unknown. Check the orders list before trying again.".into());
                }
            }
        }
    });

    view! {
        <section data-hydrated=move || hydrated.get().to_string()>
            <div node_ref=heading tabindex="-1" class="page-heading">
                <PageHeader heading=format!("Delete order #{id}?") description=state.order.customer/>
            </div>
            <div class="order-card">
                <p>"This permanently removes the order from this demo. Review the saved order before confirming."</p>
                <p>{format!("Revision {} · Total {}", state.order.revision, total)}</p>
                <form action=format!("/orders/{id}/delete") method="post" autocomplete="off"
                    aria-busy=move || pending.get().to_string()
                    on:submit=move |event| {
                        event.prevent_default();
                        if scope.current() && !pending.get_untracked() && !uncertain.get_untracked() {
                            notice.set(String::new());
                            deletes.0.update(|pending| { pending.insert((scope.key, id), PendingDelete {
                                state: snapshot.get_value(), uncertain: true,
                            }); });
                            delete.dispatch(());
                        }
                    }
                >
                    <input type="hidden" name="expected_user_id" value=scope.key.user_id/>
                    <input type="hidden" name="expected_tenant_id" value=scope.key.tenant_id/>
                    <input type="hidden" name="expected_generation" value=scope.key.generation/>
                    <input type="hidden" name="_csrf" value=move || csrf.get()/>
                    <input type="hidden" name="expected_revision" value=revision.get_value()/>
                    <ErrorSummary id="delete-errors" message=message focus_trigger=focus_trigger/>
                    <div class="form-actions">
                        <SubmitButton label="Delete order" pending=pending blocked=uncertain pending_label="Deleting…" variant=SubmitVariant::Danger/>
                        <a href=format!("/orders/{id}")>"Cancel"</a>
                        <a href=format!("/orders/{id}/delete") rel="external" on:click=move |event| {
                            event.prevent_default();
                            if !pending.get_untracked() {
                                deletes.0.update(|pending| { pending.remove(&(scope.key, id)); });
                                reload.run(());
                            }
                        }>"Reload saved order"</a>
                    </div>
                    <StatusMessage message=notice/>
                    <Show when=move || uncertain.get()>
                        <a href="/orders">"Check orders list"</a>
                    </Show>
                </form>
            </div>
        </section>
    }
}
