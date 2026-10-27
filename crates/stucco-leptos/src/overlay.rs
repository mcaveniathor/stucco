use leptos::prelude::*;

/// A native modal dialog with a link to an application-owned fallback page.
///
/// Before hydration, or when the browser cannot open the dialog, the trigger
/// follows `fallback_href`. Serve equivalent content there. After hydration,
/// Rust opens the native dialog; the browser owns focus containment, Escape,
/// and focus restoration. The close button is always present. Modified clicks
/// retain normal link behavior. Supply a unique `id`; `{id}-title` is reserved.
/// Do not nest dialogs, or place this component inside another form when its
/// children contain a form.
#[component]
pub fn Dialog(
    /// Stable, page-unique dialog ID.
    #[prop(into)]
    id: String,
    /// Visible heading and accessible name of the dialog.
    #[prop(into)]
    title: String,
    /// Visible text of the opening link.
    #[prop(into)]
    trigger_label: String,
    /// Native destination providing the same content without hydration.
    #[prop(into)]
    fallback_href: String,
    /// Dialog content, including any application-owned actions.
    children: Children,
    /// Localizable accessible label for the close button.
    #[prop(default = "Close".to_owned(), into)]
    close_label: String,
    /// Optional BCP 47 language tag; otherwise inherits from the page.
    #[prop(optional, into)]
    lang: Option<String>,
    /// Optional native direction (`"ltr"`, `"rtl"` or `"auto"`).
    #[prop(optional, into)]
    dir: Option<String>,
) -> impl IntoView {
    let dialog = NodeRef::<leptos::html::Dialog>::new();
    let title_id = format!("{id}-title");
    view! {
        <div lang=lang dir=dir>
            <a
                class="st-button"
                data-variant="secondary"
                data-size="md"
                href=fallback_href
                aria-haspopup="dialog"
                aria-controls=id.clone()
                on:click=move |event| {
                    if event.button() == 0
                        && !event.ctrl_key()
                        && !event.meta_key()
                        && !event.shift_key()
                        && !event.alt_key()
                        && let Some(element) = dialog.get_untracked()
                        && element.show_modal().is_ok()
                    {
                        event.prevent_default();
                    }
                }
            >{trigger_label}</a>
            <dialog
                node_ref=dialog
                id=id
                class="st-dialog"
                aria-labelledby=title_id.clone()
            >
                <div class="st-dialog-header">
                    <h2 id=title_id.clone() class="st-heading" data-size="lg">{title}</h2>
                    <button
                        class="st-dialog-close"
                        type="button"
                        aria-label=close_label
                        on:click=move |_| {
                            if let Some(element) = dialog.get_untracked() {
                                element.close();
                            }
                        }
                    ><span aria-hidden="true">"×"</span></button>
                </div>
                <div class="st-dialog-body">{children()}</div>
            </dialog>
        </div>
    }
}
