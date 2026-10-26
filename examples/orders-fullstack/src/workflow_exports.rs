//! Native export pages enhanced with Rust actions and bounded status polling.
use crate::{
    auth_ui::{AccessRequired, AuthScope},
    export_api::{self, ExportProblem},
    exports::{ExportState, ExportView},
};
use leptos::prelude::*;
use stucco_leptos::{PageHeader, StatusMessage, SubmitButton};

fn state_label(state: &ExportState) -> String {
    match state {
        ExportState::Queued => "Queued".into(),
        ExportState::Running { processed, total } => {
            format!("Exporting {processed} of {total} orders")
        }
        ExportState::Complete { rows } => format!("Ready — {rows} orders"),
        ExportState::Failed(reason) => format!("Failed: {reason}"),
        ExportState::Cancelled => "Cancelled".into(),
    }
}
#[component]
fn IdentityFields() -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    view! {
        <input type="hidden" name="_csrf" value=move || scope.auth.session.get().csrf/>
        <input type="hidden" name="expected_user_id" value=scope.key.user_id/>
        <input type="hidden" name="expected_tenant_id" value=scope.key.tenant_id/>
        <input type="hidden" name="expected_generation" value=scope.key.generation/>
    }
}
#[component]
fn Problem(problem: ExportProblem) -> impl IntoView {
    match problem {
        ExportProblem::Access(reason) => view! {<AccessRequired reason/>}.into_any(),
        ExportProblem::Service(crate::exports::ExportError::NotFound) => {
            crate::not_found_status();
            view!{<p role="alert">"This export is unavailable or has expired."</p><a href="/exports">"Back to exports"</a>}.into_any()
        }
        other => view! {<p role="alert">{other.to_string()}</p>}.into_any(),
    }
}
#[component]
pub(crate) fn ExportList() -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let key = scope.key;
    let loaded = Resource::new_blocking(
        || (),
        move |_| async move {
            export_api::list_exports(key.user_id, key.tenant_id, key.generation).await
        },
    );
    view! {
        <leptos_meta::Title text="Exports · Stucco"/>
        <PageHeader heading="Order exports" description="Download a CSV snapshot of your team’s orders. Exports remain available for 24 hours."/>
        <p>"Each export includes up to 1,000 orders. Downloads are private to the account that requested them."</p>
        <Suspense fallback=||view!{<p role="status">"Loading exports…"</p>}>
            {move || if scope.current() {loaded.get().map(|result|match result {
                Ok(Ok(initial))=>view!{<ExportListContent initial/>}.into_any(),
                Ok(Err(problem))=>view!{<Problem problem/>}.into_any(),
                Err(_)=>view!{<p role="alert">"Could not load exports."</p><button type="button" on:click=move |_|loaded.refetch()>"Try again"</button>}.into_any(),
            })} else {None}}
        </Suspense>
    }
}
#[component]
fn ExportListContent(initial: export_api::ExportListState) -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let key = scope.key;
    let token = StoredValue::new(initial.new_token);
    let notice = RwSignal::new(String::new());
    let navigate = leptos_router::hooks::use_navigate();
    let hydrated = RwSignal::new(false);
    let start = Action::new(move |_: &()| {
        let token = token.get_value();
        let csrf = scope.auth.session.get_untracked().csrf;
        async move {
            export_api::start_export(token, csrf, key.user_id, key.tenant_id, key.generation).await
        }
    });
    let pending = start.pending();
    Effect::new(move |_| hydrated.set(true));
    Effect::new(move |_| {
        if let Some(result) = start.value().get() {
            if !scope.current() {
                return;
            }
            match result {
                Ok(Ok(job)) => navigate(&format!("/exports/{}", job.token), Default::default()),
                Ok(Err(ExportProblem::Access(_))) => scope.reject(),
                Ok(Err(error)) => notice.set(error.to_string()),
                Err(_) => {
                    notice.set("The request could not be confirmed. Check this export’s status or submit again; the same request will not create a second export.".into());
                    leptos::task::spawn_local(async move {
                        scope.check_session().await;
                    });
                }
            }
        }
    });
    view! {
        <section class="order-card" data-hydrated=move ||hydrated.get().to_string()>
            <form action="/exports" method="post" on:submit=move |event| {
                event.prevent_default(); if scope.current() && !pending.get_untracked() {notice.set(String::new());start.dispatch(());}
            }>
                <IdentityFields/><input type="hidden" name="token" value=token.get_value()/>
                <SubmitButton label="Export orders" pending pending_label="Requesting export…"/>
                <StatusMessage message=notice/>
                <Show when=move ||!notice.get().is_empty()><a href=move ||format!("/exports/{}",token.get_value())>"Check request status"</a></Show>
            </form>
            <h2>"Recent exports"</h2>
            {if initial.jobs.is_empty(){view!{<p>"No exports yet."</p>}.into_any()}else{
                view!{<ul>{initial.jobs.into_iter().map(|job|view!{<li><a href=format!("/exports/{}",job.token)>{format!("Export {}", &job.token[..8])}</a>" · "{state_label(&job.state)}</li>}).collect_view()}</ul>}.into_any()
            }}
        </section>
    }
}
#[component]
pub(crate) fn ExportDetail() -> impl IntoView {
    let params = leptos_router::hooks::use_params_map();
    // Each token owns its resource, controls and polling lifetime. History can
    // change route parameters while a previous export read is still pending.
    view! {
        <For each=move ||vec![params.get().get("token").unwrap_or_default()]
            key=|token|token.clone()
            children=|token|view!{<ExportStatus token/>}/>
    }
}
#[component]
fn ExportStatus(token: String) -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let key = scope.key;
    let token = StoredValue::new(token);
    let fetching = RwSignal::new(false);
    let loaded = Resource::new_blocking(
        || (),
        move |_| async move {
            let token = token.get_value();
            fetching.set(true);
            let result =
                export_api::export_status(token, key.user_id, key.tenant_id, key.generation).await;
            fetching.try_set(false);
            result
        },
    );
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        let pending = loaded
            .get()
            .is_some_and(|result| matches!(result,Ok(Ok(job)) if job.state.is_pending()));
        if pending
            && scope.current()
            && let Ok(handle) = leptos::leptos_dom::helpers::set_interval_with_handle(
                move || {
                    if scope.current() && !fetching.get_untracked() {
                        loaded.refetch();
                    }
                },
                std::time::Duration::from_secs(2),
            )
        {
            on_cleanup(move || handle.clear());
        }
    });
    view! {
        <leptos_meta::Title text="Export status · Stucco"/>
        <PageHeader heading="Export status" description="You can leave this page and return to your export later."/>
        <a href="/exports">"Back to exports"</a>
        <a class="export-refresh" href=move ||format!("/exports/{}",token.get_value()) on:click=move |event| {event.prevent_default();loaded.refetch();}>"Refresh status"</a>
        <Transition fallback=||view!{<p role="status">"Loading export…"</p>}>
            {move || if scope.current() {loaded.get().map(|result|match result {
                Ok(Ok(job))=>view!{<ExportProgress job refresh=Callback::new(move |()|loaded.refetch())/>}.into_any(),
                Ok(Err(problem))=>view!{<Problem problem/>}.into_any(),
                Err(_)=>view!{<p role="alert">"Could not check this export. Refresh to try again."</p>}.into_any(),
            })} else {None}}
        </Transition>
    }
}
#[component]
fn ExportProgress(job: ExportView, refresh: Callback<()>) -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let key = scope.key;
    let data = StoredValue::new(job.clone());
    let notice = RwSignal::new(String::new());
    let hydrated = RwSignal::new(false);
    let action = Action::new(move |retry: &bool| {
        let retry = *retry;
        let job = data.get_value();
        let csrf = scope.auth.session.get_untracked().csrf;
        async move {
            if retry {
                export_api::retry_export(
                    job.token,
                    job.attempt,
                    csrf,
                    key.user_id,
                    key.tenant_id,
                    key.generation,
                )
                .await
            } else {
                export_api::cancel_export(
                    job.token,
                    csrf,
                    key.user_id,
                    key.tenant_id,
                    key.generation,
                )
                .await
            }
        }
    });
    let pending = action.pending();
    Effect::new(move |_| hydrated.set(true));
    Effect::new(move |_| {
        if let Some(result) = action.value().get() {
            if !scope.current() {
                return;
            }
            match result {
                Ok(Ok(_)) => refresh.run(()),
                Ok(Err(ExportProblem::Access(_))) => scope.reject(),
                Ok(Err(error)) => notice.set(error.to_string()),
                Err(_) => {
                    notice.set("The change could not be confirmed. Refresh status before choosing another action.".into());
                    leptos::task::spawn_local(async move {
                        scope.check_session().await;
                    });
                }
            }
        }
    });
    let retry = matches!(job.state, ExportState::Failed(_))
        && job.attempt < crate::exports::MAX_EXPORT_ATTEMPTS;
    let cancellable = !matches!(job.state, ExportState::Cancelled);
    view! {
        <section class="order-card export-progress" data-hydrated=move ||hydrated.get().to_string()>
            <h2>"Your export"</h2><p role="status">{state_label(&job.state)}</p>
            <p>{format!("Attempt {} of {}",job.attempt,crate::exports::MAX_EXPORT_ATTEMPTS)}</p>
            {matches!(job.state,ExportState::Complete{..}).then(||view!{<a href=format!("/exports/{}/download",job.token) download>"Download CSV"</a>})}
            {retry.then(||view!{
                <form action=format!("/exports/{}/retry",job.token) method="post" on:submit=move |event|{event.prevent_default();if scope.current() && !pending.get_untracked(){notice.set(String::new());action.dispatch(true);}}>
                    <IdentityFields/><input type="hidden" name="expected_attempt" value=job.attempt/>
                    <SubmitButton label="Retry export" pending pending_label="Submitting…"/>
                </form>
            })}
            {cancellable.then(||view!{
                <form action=format!("/exports/{}/cancel",job.token) method="post" on:submit=move |event|{event.prevent_default();if scope.current() && !pending.get_untracked(){notice.set(String::new());action.dispatch(false);}}>
                    <IdentityFields/><SubmitButton label="Cancel and remove export" pending pending_label="Submitting…"/>
                </form>
            })}
            <StatusMessage message=notice/>
        </section>
    }
}
