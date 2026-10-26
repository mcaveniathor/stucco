use crate::{
    attachment_api::{self, AttachmentFailure},
    attachments::{Attachment, AttachmentError, MAX_ATTACHMENT_BYTES, MAX_ATTACHMENTS},
    auth_ui::{AccessRequired, AuthScope},
};
use leptos::prelude::*;
use stucco_leptos::{ErrorSummary, FileInput, SelectedFile, StatusMessage, SubmitButton};

#[component]
pub(crate) fn AttachmentPanel(order_id: u64) -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let initial = Resource::new_blocking(
        || (),
        move |_| async move {
            attachment_api::load_attachments(
                order_id,
                scope.key.user_id,
                scope.key.tenant_id,
                scope.key.generation,
            )
            .await
        },
    );
    view! {
        <section class="order-card" aria-labelledby="attachments-title">
            <h2 id="attachments-title">"Attachments"</h2>
            <p>"Attach up to three files, at most 1 MiB each. Downloads are available to your team."</p>
            <Suspense fallback=|| view! { <p role="status">"Loading attachments…"</p> }>
                {move || initial.get().map(|result| if !scope.current() { ().into_any() } else { match result {
                    Ok(Ok(files)) => view! { <AttachmentReady order_id initial=files/> }.into_any(),
                    Ok(Err(AttachmentFailure::Access(reason))) => view! { <AccessRequired reason/> }.into_any(),
                    _ => view! { <p role="alert">"Attachments could not be loaded."</p><a href=format!("/orders/{order_id}")>"Reload the order"</a> }.into_any(),
                } })}
            </Suspense>
        </section>
    }
}

#[component]
fn AttachmentReady(order_id: u64, initial: Vec<Attachment>) -> impl IntoView {
    let scope = expect_context::<AuthScope>();
    let files = RwSignal::new(initial);
    let busy = RwSignal::new(false);
    let uncertain = RwSignal::new(false);
    let selected = RwSignal::new(Vec::<SelectedFile>::new());
    let problem = RwSignal::new(String::new());
    let status = RwSignal::new(String::new());
    let focus = RwSignal::new(0_u32);
    let hydrated = RwSignal::new(false);
    Effect::new(move |_| hydrated.set(true));
    let form = NodeRef::<leptos::html::Form>::new();
    let refresh_button = NodeRef::<leptos::html::Button>::new();
    let csrf = Signal::derive(move || scope.auth.session.get().csrf);
    let blocked = Signal::derive(move || uncertain.get() || files.get().len() >= MAX_ATTACHMENTS);
    let report = Callback::new(move |message: String| {
        problem.set(message);
        status.set(String::new());
        focus.update(|value| *value = value.wrapping_add(1));
    });

    let reconcile = Callback::new(move |()| {
        if busy.get_untracked() || !scope.current() {
            return;
        }
        busy.set(true);
        status.set("Checking saved attachments…".into());
        leptos::task::spawn_local(async move {
            let result = attachment_api::load_attachments(
                order_id,
                scope.key.user_id,
                scope.key.tenant_id,
                scope.key.generation,
            )
            .await;
            if !scope.current() || busy.try_get_untracked().is_none() {
                return;
            }
            busy.set(false);
            match result {
                Ok(Ok(saved)) => {
                    files.set(saved);
                    uncertain.set(false);
                    problem.set(String::new());
                    status.set(
                        "Attachments refreshed. Check the list before uploading again.".into(),
                    );
                }
                Ok(Err(AttachmentFailure::Access(_))) => scope.reject(),
                _ => report.run(
                    "Saved attachments could not be checked. Try refreshing the list again.".into(),
                ),
            }
        });
    });

    let remove = Callback::new(move |id: u64| {
        if busy.get_untracked() || uncertain.get_untracked() || !scope.current() {
            return;
        }
        busy.set(true);
        problem.set(String::new());
        status.set("Removing attachment…".into());
        let csrf = csrf.get_untracked();
        leptos::task::spawn_local(async move {
            let result = attachment_api::remove_attachment(
                order_id,
                id,
                csrf,
                scope.key.user_id,
                scope.key.tenant_id,
                scope.key.generation,
            )
            .await;
            if !scope.current() || busy.try_get_untracked().is_none() {
                return;
            }
            busy.set(false);
            match result {
                Ok(Ok(())) => {
                    files.update(|files| files.retain(|file| file.id != id));
                    status.set("Attachment removed.".into());
                    #[cfg(feature = "hydrate")]
                    if let Some(button) = refresh_button.get_untracked() {
                        let _ = button.focus();
                    }
                }
                Ok(Err(AttachmentFailure::Access(_))) => scope.reject(),
                Ok(Err(AttachmentFailure::Storage(error)))
                    if error != AttachmentError::Unavailable =>
                {
                    report.run(error.to_string())
                }
                Err(error) if crate::auth_ui::csrf_rejected(&error) => {
                    report.run("Form protection changed. Nothing was removed. Try again after your session is checked.".into());
                    scope.check_session().await;
                }
                _ => {
                    uncertain.set(true);
                    report.run(
                        "The removal outcome is unknown. Refresh attachments before trying again."
                            .into(),
                    );
                    scope.check_session().await;
                }
            }
        });
    });

    let submit = move |event: leptos::ev::SubmitEvent| {
        #[cfg(feature = "hydrate")]
        {
            event.prevent_default();
            if busy.get_untracked() || blocked.get_untracked() || !scope.current() {
                return;
            }
            let selection = selected.get_untracked();
            if selection.len() != 1 || selection[0].size == 0 {
                report.run(AttachmentError::Invalid.to_string());
                return;
            }
            if selection[0].size > MAX_ATTACHMENT_BYTES as u64 {
                report.run(AttachmentError::TooLarge.to_string());
                return;
            }
            let Some(element) = form.get_untracked() else {
                return;
            };
            let data = match web_sys::FormData::new_with_form(&element) {
                Ok(data) => data,
                Err(_) => {
                    report.run("The file could not be read. Select it again.".into());
                    return;
                }
            };
            let token = csrf.get_untracked();
            // The live session can rotate after this form was initially rendered.
            let _ = data.set_with_str("_csrf", &token);
            busy.set(true);
            problem.set(String::new());
            status.set("Uploading attachment…".into());
            leptos::task::spawn_local(async move {
                let result = upload(order_id, data, token).await;
                if !scope.current() || busy.try_get_untracked().is_none() {
                    return;
                }
                busy.set(false);
                match result {
                    UploadResult::Saved(saved) => {
                        files.update(|files| {
                            files.push(saved);
                            files.sort_by_key(|file| file.id);
                        });
                        element.reset();
                        selected.set(Vec::new());
                        status.set("Attachment uploaded.".into());
                    }
                    UploadResult::Access => scope.reject(),
                    UploadResult::Rejected(error) => report.run(error.to_string()),
                    UploadResult::Unknown => {
                        uncertain.set(true);
                        report.run("The upload outcome is unknown. Refresh attachments before trying again. Uploads are never retried automatically.".into());
                        scope.check_session().await;
                    }
                }
            });
        }
        #[cfg(not(feature = "hydrate"))]
        let _ = event;
    };
    #[cfg(not(feature = "hydrate"))]
    let _ = (remove, MAX_ATTACHMENT_BYTES);

    view! {
        <ErrorSummary id="attachment-errors" message=problem focus_trigger=focus/>
        <StatusMessage message=status/>
        <p>{move || format!("{} of {MAX_ATTACHMENTS} attachment slots used.", files.get().len())}</p>
        <ul class="attachment-list">
            <For each=move || files.get() key=|file| file.id children=move |file| {
                let id = file.id;
                view! {
                    <li>
                        <a href=format!("/orders/{order_id}/attachments/{id}") download=file.file_name.clone()>{file.file_name.clone()}</a>
                        <span>{format!(" ({} bytes)", file.size)}</span>
                        <form method="post" action=format!("/orders/{order_id}/attachments/{id}/delete") on:submit=move |event| {
                            #[cfg(feature = "hydrate")]
                            { event.prevent_default(); remove.run(id); }
                            #[cfg(not(feature = "hydrate"))]
                            let _ = event;
                        }>
                            <input type="hidden" name="_csrf" value=move || csrf.get()/>
                            <input type="hidden" name="expected_user_id" value=scope.key.user_id/>
                            <input type="hidden" name="expected_tenant_id" value=scope.key.tenant_id/>
                            <input type="hidden" name="expected_generation" value=scope.key.generation/>
                            <SubmitButton label=format!("Remove {}", file.file_name) blocked=Signal::derive(move || busy.get() || uncertain.get())/>
                        </form>
                    </li>
                }
            }/>
        </ul>
        <form node_ref=form method="post" action=format!("/orders/{order_id}/attachments") enctype="multipart/form-data" on:submit=submit>
            <input type="hidden" name="_csrf" value=move || csrf.get()/>
            <input type="hidden" name="expected_user_id" value=scope.key.user_id/>
            <input type="hidden" name="expected_tenant_id" value=scope.key.tenant_id/>
            <input type="hidden" name="expected_generation" value=scope.key.generation/>
            <FileInput id="order-attachment" name="attachment" label="Choose an attachment" required=true
                hint="One nonempty file, at most 1 MiB. File selections are cleared when you leave this page or sign in again."
                disabled=Signal::derive(move || busy.get() || blocked.get())
                on_change=Callback::new(move |value| selected.set(value))/>
            <SubmitButton label="Upload attachment" pending=busy blocked=blocked pending_label="Working…"/>
        </form>
        <Show when=move || hydrated.get()>
            <button node_ref=refresh_button type="button" class="st-button" data-variant="secondary" data-size="md" aria-disabled=move || busy.get().to_string() aria-busy=move || busy.get().to_string() on:click=move |_| reconcile.run(())>"Refresh attachments"</button>
        </Show>
        <a href=format!("/orders/{order_id}") hidden=move || hydrated.get()>"Reload saved attachments"</a>
        <noscript><p>"Upload and removal use native forms. Reload this page to check saved attachments."</p></noscript>
    }
}

#[cfg(feature = "hydrate")]
enum UploadResult {
    Saved(Attachment),
    Access,
    Rejected(AttachmentError),
    Unknown,
}

#[cfg(feature = "hydrate")]
async fn upload(order_id: u64, data: web_sys::FormData, csrf: String) -> UploadResult {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let attempt = async {
        let options = web_sys::RequestInit::new();
        options.set_method("POST");
        options.set_credentials(web_sys::RequestCredentials::SameOrigin);
        options.set_body(&data);
        let request = web_sys::Request::new_with_str_and_init(
            &format!("/orders/{order_id}/attachments"),
            &options,
        )?;
        request.headers().set("Accept", "application/json")?;
        request.headers().set("Stucco-Csrf", &csrf)?;
        let response = JsFuture::from(window().fetch_with_request(&request))
            .await?
            .dyn_into::<web_sys::Response>()?;
        if response.status() == 401 {
            return Ok(UploadResult::Access);
        }
        let status = response.status();
        let text = JsFuture::from(response.text()?)
            .await?
            .as_string()
            .unwrap_or_default();
        if status == 201 {
            return Ok(serde_json::from_str::<Attachment>(&text)
                .map(UploadResult::Saved)
                .unwrap_or(UploadResult::Unknown));
        }
        #[derive(serde::Deserialize)]
        struct Failure {
            error: AttachmentError,
        }
        Ok(
            match serde_json::from_str::<Failure>(&text).map(|failure| failure.error) {
                Ok(AttachmentError::IdentityChanged) => UploadResult::Access,
                Ok(AttachmentError::Unavailable) | Err(_) => UploadResult::Unknown,
                Ok(error) => UploadResult::Rejected(error),
            },
        )
    };
    let result: Result<UploadResult, wasm_bindgen::JsValue> = attempt.await;
    result.unwrap_or(UploadResult::Unknown)
}
