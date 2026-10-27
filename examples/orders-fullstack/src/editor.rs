use crate::{
    Drafts, NotFound,
    api::{self, EditorState},
    auth_ui::{AccessRequired, AuthScope},
    domain::{Field, FieldError, OrderDraft, SaveError, money, validate},
};
use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use stucco_leptos::{ErrorSummary, StatusMessage, SubmitButton, SummaryError, TextField};

#[component]
pub(crate) fn EditorPage(#[prop(default = false)] create: bool) -> impl IntoView {
    let drafts = expect_context::<Drafts>();
    let scope = expect_context::<AuthScope>();
    let params = crate::route_id();
    let id = Memo::new(move |_| if create { Some(0) } else { params.get() });
    let initial = Resource::new(
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
        <Suspense fallback=|| view! { <p role="status">"Loading order…"</p> }>
            {move || initial.get().map(|result| match result {
                Ok(Ok(Some(state))) => view! { <OrderEditor initial=state reload=Callback::new(move |()| initial.refetch())/> }.into_any(),
                Ok(Ok(None)) => match id.get().and_then(|id| drafts.0.with_untracked(|drafts| drafts.get(&(scope.key, id)).cloned())) {
                    Some(state) => view! { <MissingDraft state discard=Callback::new(move |()| initial.refetch())/> }.into_any(),
                    None => view! { <NotFound/> }.into_any(),
                },
                Ok(Err(reason)) => view! { <AccessRequired reason/> }.into_any(),
                Err(_) => view! { <h1>"Could not load the order"</h1><p role="alert">"Your order is unavailable. Please try again."</p><button type="button" on:click=move |_| initial.refetch()>"Try again"</button> }.into_any(),
            })}
        </Suspense>
    }
}

#[component]
fn MissingDraft(state: EditorState, discard: Callback<()>) -> impl IntoView {
    let drafts = expect_context::<Drafts>();
    let scope = expect_context::<AuthScope>();
    let id = state.order.id;
    view! {
        <NotFound/>
        <p>"Your unsaved draft is retained below. This order was deleted and cannot be saved."</p>
        <dl>
            <dt>"Customer"</dt><dd>{state.draft.customer}</dd>
            <dt>"Notebook quantity"</dt><dd>{state.draft.quantity_one}</dd>
            <dt>"Desk organizer quantity"</dt><dd>{state.draft.quantity_two}</dd>
        </dl>
        <button type="button" class="st-button" on:click=move |_| {
            drafts.0.update(|drafts| { drafts.remove(&(scope.key, id)); });
            discard.run(());
        }>"Discard deleted order draft"</button>
    }
}

#[component]
fn OrderEditor(initial: EditorState, reload: Callback<()>) -> impl IntoView {
    let drafts = expect_context::<Drafts>();
    let scope = expect_context::<AuthScope>();
    let id = initial.order.id;
    let mut retained = drafts
        .0
        .with_untracked(|drafts| drafts.get(&(scope.key, id)).cloned())
        .unwrap_or(initial.clone());
    // A navigated-back draft keeps its original order/revision, but uses the
    // current browser's CSRF token. Never pair old edits with the new revision.
    retained.csrf = initial.csrf;
    if matches!(retained.error, Some(SaveError::Access { .. })) {
        retained.error = None;
    }
    let initial = retained;
    let order = RwSignal::new(initial.order);
    let baseline = RwSignal::new(order.get_untracked().draft());
    let customer = RwSignal::new(initial.draft.customer);
    let quantity_one = RwSignal::new(initial.draft.quantity_one);
    let quantity_two = RwSignal::new(initial.draft.quantity_two);
    let revision = RwSignal::new(initial.draft.expected_revision);
    let failure = RwSignal::new(initial.error);
    let uncertain = RwSignal::new(initial.uncertain);
    let confirmed_creation = RwSignal::new(false);
    let reloading = RwSignal::new(false);
    let notice = RwSignal::new(if initial.uncertain {
        if id == 0 {
            "The creation outcome is unknown. Check the orders list before creating another order."
        } else {
            "The save outcome is unknown. Reload the saved order before trying again."
        }
        .to_owned()
    } else {
        String::new()
    });
    let focus_trigger = RwSignal::new(u32::from(failure.get_untracked().is_some()));
    let hydrated = RwSignal::new(false);
    let heading = NodeRef::<leptos::html::H1>::new();
    Effect::new(move |_| {
        hydrated.set(true);
        #[cfg(feature = "hydrate")]
        if focus_trigger.get_untracked() == 0
            && let Some(element) = heading.get()
        {
            crate::focus_if_unclaimed(&element);
        }
    });
    let csrf = Signal::derive(move || scope.auth.session.get().csrf);
    let draft = move || OrderDraft {
        customer: customer.get(),
        quantity_one: quantity_one.get(),
        quantity_two: quantity_two.get(),
        expected_revision: revision.get(),
    };
    let dirty = Memo::new(move |_| draft() != baseline.get());
    Effect::new(move |_| {
        if reloading.get() || !scope.current() {
            return;
        }
        let current = EditorState {
            order: order.get(),
            draft: draft(),
            error: failure.get(),
            csrf: csrf.get_untracked(),
            uncertain: uncertain.get(),
        };
        let keep = (dirty.get() || current.uncertain) && !confirmed_creation.get();
        drafts.0.update(|drafts| {
            if keep {
                drafts.insert((scope.key, id), current);
            } else {
                drafts.remove(&(scope.key, id));
            }
        });
    });
    let save = Action::new(move |draft: &OrderDraft| {
        let draft = draft.clone();
        let submitted = draft.clone();
        let token = csrf.get_untracked();
        async move {
            let result = if id == 0 {
                api::create_order(
                    draft.customer,
                    draft.quantity_one,
                    draft.quantity_two,
                    draft.expected_revision,
                    token,
                    scope.key.user_id,
                    scope.key.tenant_id,
                    scope.key.generation,
                )
                .await
            } else {
                api::save_order(
                    id,
                    draft.customer,
                    draft.quantity_one,
                    draft.quantity_two,
                    draft.expected_revision,
                    token,
                    scope.key.user_id,
                    scope.key.tenant_id,
                    scope.key.generation,
                )
                .await
            };
            if !scope.current() {
                return result;
            }
            // This root-owned cache survives a route unmount. A response can
            // arrive after the editor's effect has been disposed.
            let _ = drafts.0.try_update(|drafts| {
                // A fresh draft can have been started while this response was
                // delayed. Only settle the snapshot that was actually sent.
                if !drafts
                    .get(&(scope.key, id))
                    .is_some_and(|state| state.draft == submitted && state.uncertain)
                {
                    return;
                }
                match &result {
                    Ok(Ok(_)) => {
                        drafts.remove(&(scope.key, id));
                    }
                    Ok(Err(error)) => {
                        if let Some(state) = drafts.get_mut(&(scope.key, id)) {
                            state.error = Some(error.clone());
                            state.uncertain = false;
                        }
                    }
                    Err(error) if crate::auth_ui::csrf_rejected(error) => {
                        if let Some(state) = drafts.get_mut(&(scope.key, id)) {
                            state.error = None;
                            state.uncertain = false;
                        }
                    }
                    Err(_) => {}
                }
            });
            match &result {
                Ok(Err(SaveError::Access { .. })) => scope.reject(),
                Err(_) => scope.check_session().await,
                _ => {}
            }
            result
        }
    });
    let pending = save.pending();
    let navigate = use_navigate();
    Effect::new(move |_| {
        if let Some(result) = save.value().get() {
            if !scope.current() {
                return;
            }
            match result {
                Ok(Ok(saved)) => {
                    let values = saved.draft();
                    baseline.set(values.clone());
                    customer.set(values.customer);
                    quantity_one.set(values.quantity_one);
                    quantity_two.set(values.quantity_two);
                    revision.set(values.expected_revision);
                    drafts.0.update(|drafts| {
                        drafts.remove(&(scope.key, id));
                    });
                    failure.set(None);
                    uncertain.set(false);
                    notice.set("Order saved.".into());
                    let saved_id = saved.id;
                    order.set(saved);
                    if id == 0 {
                        navigate(&format!("/orders/{saved_id}"), Default::default());
                    }
                }
                Ok(Err(error)) => {
                    failure.set(Some(error));
                    uncertain.set(false);
                    notice.set(String::new());
                    focus_trigger.update(|n| *n = n.saturating_add(1));
                }
                Err(error) if crate::auth_ui::csrf_rejected(&error) => {
                    failure.set(None);
                    uncertain.set(false);
                    notice.set("Form protection rejected the request before saving. Your values are retained. Review and submit again.".into());
                }
                Err(_) => {
                    failure.set(None);
                    uncertain.set(true);
                    notice.set(if id == 0 {
                        "The creation outcome is unknown. Check the orders list before creating another order."
                    } else {
                        "The save outcome is unknown. Reload the saved order before trying again."
                    }.into());
                }
            }
        }
    });
    // A returned create form has a new Action, while the original request can
    // still settle its root-owned snapshot. Adopt only that earlier result;
    // this mounted Action's results are handled above.
    if id == 0 {
        Effect::new(move |_| {
            let settled = drafts
                .0
                .with(|drafts| drafts.get(&(scope.key, id)).cloned());
            if !scope.current()
                || reloading.get()
                || !uncertain.get()
                || pending.get()
                || save.value().get_untracked().is_some()
            {
                return;
            }
            match settled {
                Some(state) if !state.uncertain && state.draft == untrack(draft) => {
                    failure.set(state.error);
                    uncertain.set(false);
                    notice.set(String::new());
                    focus_trigger.update(|n| *n = n.saturating_add(1));
                }
                None => {
                    confirmed_creation.set(true);
                    uncertain.set(false);
                    failure.set(None);
                    notice.set("Your earlier order was created. Check the orders list, or discard this completed draft to start another order.".into());
                }
                _ => {}
            }
        });
    }
    let blocked = Signal::derive(move || {
        reloading.get() || (id == 0 && (uncertain.get() || confirmed_creation.get()))
    });
    let read_only = Signal::derive(move || pending.get() || blocked.get());
    let errors = Memo::new(move |_| match failure.get() {
        Some(SaveError::Validation { errors, .. }) => errors,
        _ => Vec::<FieldError>::new(),
    });
    let field_error = move |field| {
        Signal::derive(move || {
            errors
                .get()
                .iter()
                .filter(|error| error.field == field)
                .map(|error| error.message.clone())
                .collect::<Vec<_>>()
                .join(". ")
        })
    };
    let summary_errors = Signal::derive(move || {
        errors
            .get()
            .into_iter()
            .map(|error| SummaryError {
                target: match error.field {
                    Field::Customer => Some("customer"),
                    Field::QuantityOne => Some("quantity-one"),
                    Field::QuantityTwo => Some("quantity-two"),
                    _ => None,
                }
                .map(str::to_owned),
                message: error.message,
            })
            .collect::<Vec<_>>()
    });
    let message = Signal::derive(move || {
        failure.get().map(|error| match error {
        SaveError::Validation {..} => "Check the highlighted fields before saving.".into(),
        SaveError::Conflict {current,..} => format!("This order changed to revision {} for {}. Your edits are still here. Reload the saved order to review the changes.",current.revision,current.customer),
        SaveError::NotFound {..} => "This order was deleted. Your edits are still here, but cannot be saved to it.".into(),
        SaveError::Access {reason,..} => reason.to_string(),
        SaveError::Unavailable {..} => "The order could not be saved. Your edits are still here.".into(),
    }).unwrap_or_default()
    });
    let total = move || {
        let mut preview_draft = draft();
        preview_draft.customer = "Preview".into();
        match validate(&preview_draft) {
            Ok(valid) => {
                let mut preview = order.get();
                preview.items[0].quantity = valid.quantity_one;
                preview.items[1].quantity = valid.quantity_two;
                preview
                    .total_cents()
                    .map(money)
                    .unwrap_or_else(|_| "Total too large".into())
            }
            Err(_) => "Enter valid quantities".into(),
        }
    };
    let action = if id == 0 {
        "/orders/new".to_owned()
    } else {
        format!("/orders/{id}/edit")
    };
    let reload_href = action.clone();
    view! {
        <leptos_meta::Title text=if id==0 {"Create order · Stucco".to_owned()} else {format!("Edit order #{id} · Stucco")}/>
        <header><h1 node_ref=heading tabindex="-1">{if id==0 {"Create order".to_owned()} else {format!("Edit order #{id}")}}</h1>
            <p>"Edit the details, check the total, and save when you’re ready."</p>
            <a href="/orders">"Back to orders"</a>
        </header>
        <section class="order-card" aria-label="Order editor" data-hydrated=move || hydrated.get().to_string()>
            <div class="order-heading"><h2>"Order details"</h2><span>"Draft revision "{move || revision.get()}</span></div>
            <form action=action method="post" novalidate autocomplete="off" aria-busy=move || pending.get().to_string() on:submit=move |event| {
                event.prevent_default();
                if scope.current() && !pending.get_untracked() && !blocked.get_untracked() {
                    notice.set(String::new());
                    failure.set(None);
                    let current = draft();
                    uncertain.set(true);
                    // Mark before dispatch, even if navigation disposes the
                    // editor before its reactive cache effect runs.
                    drafts.0.update(|drafts| { drafts.insert((scope.key, id), EditorState {
                        order: order.get_untracked(), draft: current.clone(),
                        error: None, csrf: csrf.get_untracked(), uncertain: true,
                    }); });
                    save.dispatch(current);
                }
            }>
                <input type="hidden" name="expected_user_id" value=scope.key.user_id/>
                <input type="hidden" name="expected_tenant_id" value=scope.key.tenant_id/>
                    <input type="hidden" name="expected_generation" value=scope.key.generation/>
                <input type="hidden" name="_csrf" value=move || csrf.get()/>
                <input type="hidden" name="expected_revision" value=move || revision.get()/>
                <ErrorSummary id="order-errors" message errors=summary_errors focus_trigger/>
                // Bound each field's async SSR type before composing the form;
                // nesting all three field trees overflows small debug stacks.
                {view! { <TextField id="customer" name="customer" label="Customer" value=customer
                    on_input=Callback::new(move |value| customer.set(value)) error=field_error(Field::Customer) required=true read_only/>
                }.into_any()}
                <h3>"Line items"</h3>
                <div class="line-items">
                    <div class="line-item">
                        <p>{move || order.get().items[0].name.clone()}<span>{move || format!("{} each",money(order.get().items[0].unit_price_cents))}</span></p>
                        {view! { <TextField id="quantity-one" name="quantity_one" label="Notebook quantity" value=quantity_one
                            on_input=Callback::new(move |value| quantity_one.set(value)) inputmode="numeric" hint="Whole quantities, 1–999"
                            error=field_error(Field::QuantityOne) required=true read_only/>
                        }.into_any()}
                    </div>
                    <div class="line-item">
                        <p>{move || order.get().items[1].name.clone()}<span>{move || format!("{} each",money(order.get().items[1].unit_price_cents))}</span></p>
                        {view! { <TextField id="quantity-two" name="quantity_two" label="Desk organizer quantity" value=quantity_two
                            on_input=Callback::new(move |value| quantity_two.set(value)) inputmode="numeric" hint="Whole quantities, 1–999"
                            error=field_error(Field::QuantityTwo) required=true read_only/>
                        }.into_any()}
                    </div>
                </div>
                <div class="total"><span>"Order total"</span><output aria-live="polite">{total}</output></div>
                <div class="form-actions">
                    <SubmitButton label=if id==0 {"Create order"} else {"Save order"} pending blocked/>
                    <a href=reload_href rel="external" on:click=move |event| {
                        event.prevent_default();
                        if !pending.get_untracked() && !reloading.get_untracked() {
                            reloading.set(true);
                            drafts.0.update(|drafts| {drafts.remove(&(scope.key, id));});
                            reload.run(());
                        }
                    }>
                        {if id==0 {"Discard draft and start again"} else {"Reload saved order"}}
                    </a>
                </div>
                <StatusMessage message=notice/>
                <p class="draft-notice">{move || (dirty.get() && !confirmed_creation.get()).then_some("Unsaved changes are kept while navigating this app. Reloading discards them.")}</p>
            </form>
        </section>
    }
}
