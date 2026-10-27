#![cfg(feature = "ssr")]

use leptos::prelude::*;
use stucco_leptos::{
    Autocomplete, AutocompleteOption, Dialog, ErrorSummary, FileInput, PageHeader, RepeatedField,
    RepeatedValue, StatusMessage, SubmitButton, SubmitVariant, SummaryError, TextField,
};

#[test]
fn submit_button_preserves_native_submission_and_escapes_labels() {
    let owner = Owner::new();
    let html = owner.with(|| view! { <SubmitButton label="Save <order> & continue"/> }.to_html());

    assert!(html.contains("type=\"submit\""), "{html}");
    assert!(html.contains("data-variant=\"primary\""), "{html}");
    assert!(html.contains("aria-disabled=\"false\""), "{html}");
    assert!(html.contains("aria-busy=\"false\""), "{html}");
    assert!(html.contains("Save &lt;order&gt; &amp; continue"), "{html}");
    assert!(!html.contains(" disabled"), "{html}");
}

#[test]
fn pending_destructive_button_exposes_busy_state_without_losing_focusability() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <SubmitButton
                label="Delete"
                pending=true
                pending_label="Deleting <order>…"
                variant=SubmitVariant::Danger
            />
        }
        .to_html()
    });

    assert!(html.contains("data-variant=\"danger\""), "{html}");
    assert!(html.contains("aria-disabled=\"true\""), "{html}");
    assert!(html.contains("aria-busy=\"true\""), "{html}");
    assert!(html.contains("Deleting &lt;order&gt;…"), "{html}");
    assert!(!html.contains(" disabled"), "{html}");
    assert!(!html.contains("tabindex"), "{html}");
}

#[test]
fn blocked_submit_button_stays_focusable_without_claiming_an_active_request() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! { <SubmitButton label="Create order" blocked=true pending=false/> }.to_html()
    });

    assert!(html.contains("type=\"submit\""), "{html}");
    assert!(html.contains("aria-disabled=\"true\""), "{html}");
    assert!(html.contains("aria-busy=\"false\""), "{html}");
    assert!(html.contains("Create order"), "{html}");
    assert!(!html.contains("Saving"), "{html}");
    assert!(!html.contains(" disabled"), "{html}");
    assert!(!html.contains("tabindex"), "{html}");
}

#[test]
fn error_summary_connects_its_heading_and_encodes_field_fragments() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <ErrorSummary
                id="order-errors"
                message=Signal::derive(|| "Could not save <order> & changes.".to_owned())
                errors=Signal::derive(|| vec![
                    SummaryError { target: Some("customer".into()), message: "Enter <name>".into() },
                    SummaryError { target: Some("name\"&é%".into()), message: "Check special ID".into() },
                    SummaryError { target: Some("invalid id".into()), message: "Unlinked error".into() },
                    SummaryError { target: Some(String::new()), message: "No ID".into() },
                    SummaryError { target: None, message: "Try again".into() },
                ])
                focus_trigger=1
            />
        }.to_html()
    });

    assert!(html.contains("id=\"order-errors\""), "{html}");
    assert!(html.contains("role=\"alert\""), "{html}");
    assert!(html.contains("tabindex=\"-1\""), "{html}");
    assert!(
        html.contains("aria-labelledby=\"order-errors-title\""),
        "{html}"
    );
    assert!(html.contains("id=\"order-errors-title\""), "{html}");
    assert!(
        html.contains("Could not save &lt;order&gt; &amp; changes."),
        "{html}"
    );
    assert!(html.contains("href=\"#customer\""), "{html}");
    assert!(html.contains("Enter &lt;name&gt;"), "{html}");
    assert!(html.contains("href=\"#name%22%26%C3%A9%25\""), "{html}");
    assert_eq!(html.matches("<a ").count(), 2, "{html}");
    assert!(html.contains("Unlinked error"), "{html}");
    assert!(!html.contains(" hidden"), "{html}");
    assert!(!html.contains("autofocus"), "{html}");
}

#[test]
fn an_empty_error_summary_is_hidden_and_a_form_message_alone_is_visible() {
    let owner = Owner::new();
    let (empty, form_error) = owner.with(|| {
        (
            view! { <ErrorSummary id="empty-errors"/> }.to_html(),
            view! {
                <ErrorSummary id="form-errors" message=Signal::derive(|| "Try again".to_owned())/>
            }
            .to_html(),
        )
    });

    let empty_root = empty.split('>').next().unwrap();
    assert!(empty_root.contains(" hidden"), "{empty}");
    let visible_root = form_error.split('>').next().unwrap();
    assert!(!visible_root.contains(" hidden"), "{form_error}");
    assert!(form_error.contains("Try again"), "{form_error}");
    let list = form_error
        .split("<ul")
        .nth(1)
        .unwrap()
        .split('>')
        .next()
        .unwrap();
    assert!(
        list.contains("class=\"st-error-summary-list\"") && list.contains(" hidden"),
        "{form_error}"
    );
}

#[test]
fn status_region_remains_present_when_empty_and_escapes_its_text() {
    let owner = Owner::new();
    let (empty, filled) = owner.with(|| {
        (
            view! { <StatusMessage message=Signal::derive(String::new)/> }.to_html(),
            view! { <StatusMessage message=Signal::derive(|| "Saved <order> & item".to_owned())/> }
                .to_html(),
        )
    });

    for html in [&empty, &filled] {
        assert!(html.contains("role=\"status\""), "{html}");
        assert!(html.contains("aria-atomic=\"true\""), "{html}");
        assert!(!html.contains(" hidden"), "{html}");
    }
    assert!(
        filled.contains("Saved &lt;order&gt; &amp; item"),
        "{filled}"
    );
}

#[test]
fn page_header_reuses_heading_styles_and_accepts_native_actions() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <PageHeader heading="Orders <today>" description="Review & edit orders">
                <a href="/orders/new">"New order"</a>
            </PageHeader>
        }
        .to_html()
    });

    assert!(html.contains("class=\"st-page-header\""), "{html}");
    let heading = html.split("<h1").nth(1).unwrap().split('>').next().unwrap();
    assert!(
        heading.contains("class=\"st-heading\"") && heading.contains("data-size=\"3xl\""),
        "{html}"
    );
    assert!(html.contains(">Orders &lt;today&gt;</h1>"), "{html}");
    assert!(
        html.contains("class=\"st-page-header-description\""),
        "{html}"
    );
    assert!(html.contains("Review &amp; edit orders"), "{html}");
    assert!(html.contains("href=\"/orders/new\""), "{html}");
}

#[test]
fn text_field_labels_and_escapes_submitted_values() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <TextField
                id="customer"
                name="customer"
                label="Customer <name>"
                value=Signal::derive(|| "<Ada & \"Grace\">".to_owned())
                on_input=Callback::new(|_: String| {})
                hint="Use <legal> name & surname."
            />
        }
        .to_html()
    });

    assert!(html.contains("for=\"customer\""), "{html}");
    assert!(html.contains("id=\"customer\""), "{html}");
    assert!(html.contains("name=\"customer\""), "{html}");
    assert!(html.contains("Customer &lt;name&gt;"), "{html}");
    assert!(
        html.contains("value=\"&lt;Ada &amp; &quot;Grace&quot;&gt;\""),
        "{html}"
    );
    assert!(
        html.contains("aria-describedby=\"customer-hint\""),
        "{html}"
    );
    assert!(
        html.contains("Use &lt;legal&gt; name &amp; surname."),
        "{html}"
    );
    assert!(!html.contains("aria-invalid"), "{html}");
    assert!(!html.contains("st-field-error"), "{html}");
}

#[test]
fn invalid_numeric_text_keeps_its_value_and_wires_hint_and_error() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <TextField
                id="quantity"
                name="quantity"
                label="Quantity"
                value=Signal::derive(|| "2x".to_owned())
                on_input=Callback::new(|_: String| {})
                hint="Whole items only."
                error=Signal::derive(|| "Enter <100 & whole items.".to_owned())
                inputmode="numeric"
                required=Signal::derive(|| true)
                disabled=Signal::derive(|| true)
            />
        }
        .to_html()
    });

    assert!(html.contains("type=\"text\""), "{html}");
    assert!(html.contains("value=\"2x\""), "{html}");
    assert!(html.contains("inputmode=\"numeric\""), "{html}");
    assert!(
        html.contains("aria-describedby=\"quantity-hint quantity-error\""),
        "{html}"
    );
    assert!(html.contains("aria-invalid=\"true\""), "{html}");
    assert!(html.contains("id=\"quantity-error\""), "{html}");
    assert!(html.contains("Enter &lt;100 &amp; whole items."), "{html}");
    assert!(html.contains(" required"), "{html}");
    assert!(html.contains(" disabled"), "{html}");
}

#[test]
fn an_unannotated_field_has_no_dangling_description() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <TextField
                id="note"
                name="note"
                label="Note"
                value=Signal::derive(String::new)
                on_input=Callback::new(|_: String| {})
            />
        }
        .to_html()
    });

    assert!(!html.contains("aria-describedby"), "{html}");
    assert!(!html.contains("aria-invalid"), "{html}");
    assert!(!html.contains("st-field-hint"), "{html}");
    assert!(!html.contains("st-field-error"), "{html}");
}

#[test]
fn stylesheet_reuses_existing_theme_and_form_styles() {
    let css = stucco_leptos::stylesheet(stucco_theme::Preset::Slate);
    assert!(css.contains("--st-text:"));
    assert!(css.contains(".st-field"));
    assert!(css.contains(".st-input"));
    assert!(css.contains(".st-field-error"));
    assert!(css.contains(".st-dialog::backdrop"));
    assert!(!css.contains("stucco-runtime"));
}

#[test]
fn dialog_retains_native_fallback_and_labels_a_closed_modal() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <Dialog id="help" title="Help <details>" trigger_label="Read & review"
                fallback_href="/help?section=orders&details=open" close_label="Fermer"
                lang="fr" dir="rtl">
                <p>"Read the <instructions>."</p>
                <a href="/contact">"Contact support"</a>
            </Dialog>
        }
        .to_html()
    });

    assert!(
        html.contains("href=\"/help?section=orders&amp;details=open\""),
        "{html}"
    );
    assert!(html.contains("aria-haspopup=\"dialog\""), "{html}");
    assert!(html.contains("aria-controls=\"help\""), "{html}");
    assert!(html.contains("aria-labelledby=\"help-title\""), "{html}");
    assert!(html.contains("id=\"help-title\""), "{html}");
    assert!(html.contains("Help &lt;details&gt;"), "{html}");
    assert!(html.contains("Read the &lt;instructions&gt;."), "{html}");
    assert!(html.contains("aria-label=\"Fermer\""), "{html}");
    assert!(html.contains("type=\"button\""), "{html}");
    assert!(html.contains("lang=\"fr\" dir=\"rtl\""), "{html}");
    let dialog = html
        .split("<dialog")
        .nth(1)
        .unwrap()
        .split('>')
        .next()
        .unwrap();
    assert!(!dialog.contains(" open"), "{html}");
    assert!(!html.contains("<script"), "{html}");
}

#[test]
fn autocomplete_uses_a_native_datalist_without_restricting_free_text() {
    let owner = Owner::new();
    let html = owner.with(|| view! {
        <Autocomplete id="recipient" name="recipient" label="Recipient"
            value=Signal::derive(|| "A new <recipient>".to_owned())
            on_input=Callback::new(|_: String| {})
            options=Signal::derive(|| vec![
                AutocompleteOption {value: "Ada & Grace".into(), label: Some("Research <team>".into())},
                AutocompleteOption {value: "Lin".into(), label: None},
            ])
            hint="Choose a suggestion or enter a new value."
            error="Check the recipient" required=true lang="en" dir="auto"
        />
    }.to_html());

    assert!(html.contains("list=\"recipient-options\""), "{html}");
    assert!(
        html.contains("<datalist id=\"recipient-options\""),
        "{html}"
    );
    assert!(html.contains("value=\"Ada &amp; Grace\""), "{html}");
    assert!(html.contains("label=\"Research &lt;team&gt;\""), "{html}");
    assert!(html.contains("value=\"A new &lt;recipient&gt;\""), "{html}");
    assert!(
        html.contains("aria-describedby=\"recipient-hint recipient-error\""),
        "{html}"
    );
    assert!(html.contains("aria-invalid=\"true\""), "{html}");
    assert!(!html.contains("role=\"combobox\""), "{html}");
    assert!(!html.contains("pattern="), "{html}");
    assert!(!html.contains("<select"), "{html}");
}

#[test]
fn file_input_preserves_native_multipart_semantics_and_localized_errors() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <FileInput id="attachment" name="attachment" label="Pièce jointe"
                accept=".txt,text/plain" multiple=true required=true
                hint="Maximum 2 Mo" error="Fichier <invalide>" error_prefix="Erreur : "
                lang="fr" dir="ltr"/>
        }
        .to_html()
    });

    assert!(html.contains("type=\"file\""), "{html}");
    assert!(html.contains("for=\"attachment\""), "{html}");
    assert!(html.contains("name=\"attachment\""), "{html}");
    assert!(html.contains("accept=\".txt,text/plain\""), "{html}");
    assert!(
        html.contains(" multiple") && html.contains(" required"),
        "{html}"
    );
    assert!(
        html.contains("aria-describedby=\"attachment-hint attachment-error\""),
        "{html}"
    );
    assert!(html.contains("aria-invalid=\"true\""), "{html}");
    assert!(
        html.contains("Erreur : ") && html.contains("Fichier &lt;invalide&gt;"),
        "{html}"
    );
    assert!(!html.contains("value="), "{html}");
    assert!(!html.contains(" disabled"), "{html}");
}

#[test]
fn unannotated_file_input_has_no_dangling_description_or_fake_value() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <FileInput id="document" name="document" label="Document" disabled=true/>
        }
        .to_html()
    });
    assert!(!html.contains("aria-describedby"), "{html}");
    assert!(!html.contains("aria-invalid"), "{html}");
    assert!(!html.contains("value="), "{html}");
    assert!(html.contains(" disabled"), "{html}");
}

#[test]
fn repeated_fields_keep_stable_ids_native_names_and_localized_position_labels() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <RepeatedField id="recipients" name="recipient" label="Destinataires"
                values=Signal::derive(|| vec![
                    RepeatedValue {id: 71, value: "<Ada>".into()},
                    RepeatedValue {id: 19, value: "Grace & Lin".into()},
                ])
                on_input=Callback::new(|_: (u64, String)| {})
                on_add=Callback::new(|()| {}) on_remove=Callback::new(|_: u64| {})
                item_label=Callback::new(|position| format!("Destinataire {position}"))
                remove_label=Callback::new(|position| format!("Retirer {position}"))
                add_label="Ajouter" hint="Vous pouvez modifier ces valeurs."
                min_values=1 max_values=5 lang="fr" dir="rtl"
            />
        }
        .to_html()
    });

    assert_eq!(html.matches("name=\"recipient\"").count(), 2, "{html}");
    assert!(html.contains("id=\"recipients-71\""), "{html}");
    assert!(html.contains("id=\"recipients-19\""), "{html}");
    assert!(
        html.contains("Destinataire 1") && html.contains("Destinataire 2"),
        "{html}"
    );
    assert!(
        html.contains("Retirer 1") && html.contains("Retirer 2"),
        "{html}"
    );
    assert!(html.contains("value=\"&lt;Ada&gt;\""), "{html}");
    assert!(html.contains("value=\"Grace &amp; Lin\""), "{html}");
    assert!(
        html.contains("aria-describedby=\"recipients-hint\""),
        "{html}"
    );
    assert_eq!(html.matches("type=\"button\"").count(), 3, "{html}");
    assert_eq!(html.matches(" disabled").count(), 3, "{html}");
    for input in html.split("<input").skip(1) {
        assert!(
            !input.split('>').next().unwrap().contains(" disabled"),
            "{html}"
        );
    }
}

#[test]
fn summaries_and_text_fields_accept_translated_content_and_direction() {
    let owner = Owner::new();
    let html = owner.with(|| {
        view! {
            <ErrorSummary id="errors" heading="Vérifiez le formulaire" message="Champ invalide"
                lang="fr" dir="rtl"/>
            <TextField id="name" name="name" label=move || "Nom"
                value=Signal::derive(String::new) on_input=Callback::new(|_: String| {})
                error="Entrez un nom" error_prefix="Erreur : " lang="fr" dir="auto"/>
        }
        .to_html()
    });
    assert!(html.contains("Vérifiez le formulaire"), "{html}");
    assert!(!html.contains("There is a problem"), "{html}");
    assert!(html.contains("Erreur : "), "{html}");
    assert!(!html.contains("Error: "), "{html}");
    assert!(
        html.contains("dir=\"rtl\"") && html.contains("dir=\"auto\""),
        "{html}"
    );
}
