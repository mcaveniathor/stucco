//! Create, read, update and delete routes for single orders.
use crate::form::{order_form, state_of, validate};
use crate::model::{Order, money};
use crate::{AppState, shell};
use axum::extract::rejection::PathRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use stucco::FormState;
use stucco::forms::HiddenInput;
use stucco::prelude::*;
use stucco::server::{CsrfToken, SeeOther, Submission};
use stucco_redb::{Revision, StoreError};

/// The CSRF token `CsrfLayer` issued for this request.
pub(crate) fn csrf(cx: &PageCx) -> String {
    cx.context()
        .get::<CsrfToken>()
        .map(|t| t.as_str().to_owned())
        .unwrap_or_default()
}

/// The id segment, if it decodes to a positive number. Anything else (text,
/// zero, invalid UTF-8) is answered with the not-found page, not a 400.
type IdPath = Result<Path<String>, PathRejection>;

fn parse_id(path: IdPath) -> Option<u64> {
    path.ok()?.0.parse().ok().filter(|&id| id > 0)
}

fn not_found(cx: &PageCx) -> Response {
    missing(cx, "Order not found", "It may have been deleted.")
}

fn missing(cx: &PageCx, title: &str, detail: &str) -> Response {
    cx.title(format!("{title} — stucco"))
        .app(shell(
            Stack::new()
                .child(PageHeader::new(title.to_owned()))
                .child(Text::new(detail.to_owned()))
                .child(Link::new("Back to orders", "/orders")),
        ))
        .status(StatusCode::NOT_FOUND)
        .into_response()
}

/// Every path no route matches.
pub(crate) async fn fallback(cx: PageCx) -> Response {
    missing(&cx, "Page not found", "There is nothing at this address.")
}

fn failure(cx: &PageCx, error: StoreError) -> Response {
    let id = &cx.context().request_id;
    tracing::error!(%error, request_id = %id, "orders storage failed");
    cx.title("Orders unavailable")
        .app(shell(
            PageHeader::new("Orders unavailable")
                .description(format!("Please try again. Request ID: {id}")),
        ))
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .into_response()
}

/// A confirmation dialog for deleting `order`, opened from `link_opener`.
pub(crate) fn delete_dialog(order: &Order, revision: Revision, csrf: &str) -> Dialog<'static> {
    let id = format!("delete-order-{}", order.id);
    Dialog::new(&id, format!("Delete order {}?", order.id))
        .child(Text::new(format!(
            "The order for {} is removed. This can't be undone.",
            order.customer
        )))
        .actions(Dialog::close_button(&id, "Cancel"))
        .actions(delete_form(order.id, &revision.to_string(), csrf))
}

fn delete_form(id: u64, revision: &str, csrf: &str) -> Form<'static> {
    Form::post(format!("/orders/{id}/delete"))
        .csrf(csrf)
        .child(HiddenInput::new("revision", revision))
        .child(
            Button::new("Delete order")
                .submit()
                .variant(Variant::Danger),
        )
}

/// Edit and Delete for one order, as a menu plus the dialog it opens.
pub(crate) fn row_actions(order: &Order, revision: Revision, csrf: &str) -> impl Render + use<> {
    let dialog = delete_dialog(order, revision, csrf);
    let menu = Menu::new((
        "Actions",
        el::span()
            .class("st-sr-only")
            .text(format!(" for order {}", order.id)),
    ))
    .popover_id(&format!("order-{}-actions", order.id))
    .variant(Variant::Ghost)
    .size(Size::Sm)
    .link("Edit", format!("/orders/{}/edit", order.id))
    .item(dialog.link_opener("Delete…", format!("/orders/{}/delete", order.id)));
    (menu, dialog)
}

pub(crate) async fn new_order(cx: PageCx) -> Document {
    let state = FormState::new();
    cx.title("New order — stucco").app(shell(form_page(
        "New order",
        order_form("/orders".into(), &state, &csrf(&cx), None, "Create order"),
    )))
}

fn form_page<'a>(title: &str, form: impl Render + 'a) -> impl Render + 'a {
    Stack::new()
        .child(PageHeader::new(title.to_owned()))
        .child(form)
}

pub(crate) async fn create(
    State(state): State<AppState>,
    cx: PageCx,
    Submission(form): Submission,
) -> Response {
    let input = match validate(form) {
        Ok(input) => input,
        Err(state) => {
            return cx
                .title("New order — stucco")
                .app(shell(form_page(
                    "New order",
                    order_form("/orders".into(), &state, &csrf(&cx), None, "Create order"),
                )))
                .status(StatusCode::UNPROCESSABLE_ENTITY)
                .into_response();
        }
    };
    match state.orders().create(move |id| input.into_order(id)).await {
        Ok((id, _)) => SeeOther::new("/orders")
            .flash(Flash::new(
                FlashLevel::Success,
                format!("Order {id} created."),
            ))
            .into_response(),
        Err(error) => failure(&cx, error),
    }
}

pub(crate) async fn show(State(state): State<AppState>, cx: PageCx, path: IdPath) -> Response {
    let Some(id) = parse_id(path) else {
        return not_found(&cx);
    };
    let (order, revision) = match state.orders().get_with_revision(id).await {
        Ok(Some(found)) => found,
        Ok(None) => return not_found(&cx),
        Err(error) => return failure(&cx, error),
    };
    let dialog = delete_dialog(&order, revision, &csrf(&cx));
    let actions = Cluster::new()
        .child(ButtonLink::new("Edit", format!("/orders/{id}/edit")))
        .child(
            dialog
                .link_opener("Delete…", format!("/orders/{id}/delete"))
                .variant(Variant::Danger),
        );
    let details = DescriptionList::new()
        .item("Customer", order.customer.clone())
        .item("Status", order.status.clone())
        .item("Total", money(&order))
        .item("Created", order.created.clone());
    cx.title(format!("Order {id} — stucco"))
        .app(shell(
            Stack::new()
                .child(
                    PageHeader::new(format!("Order {id}"))
                        .breadcrumbs(
                            Breadcrumbs::new()
                                .link("Orders", "/orders")
                                .current(format!("Order {id}")),
                        )
                        .actions(actions),
                )
                .child(details)
                .child(dialog),
        ))
        .into_response()
}

pub(crate) async fn edit(State(state): State<AppState>, cx: PageCx, path: IdPath) -> Response {
    let Some(id) = parse_id(path) else {
        return not_found(&cx);
    };
    match state.orders().get_with_revision(id).await {
        Ok(Some((order, revision))) => edit_page(&cx, id, &state_of(&order), &revision.to_string()),
        Ok(None) => not_found(&cx),
        Err(error) => failure(&cx, error),
    }
}

fn edit_page(cx: &PageCx, id: u64, state: &FormState, revision: &str) -> Response {
    let status = if state.conflict().is_some() {
        StatusCode::CONFLICT
    } else if state.has_errors() {
        StatusCode::UNPROCESSABLE_ENTITY
    } else {
        StatusCode::OK
    };
    cx.title(format!("Edit order {id} — stucco"))
        .app(shell(
            Stack::new()
                .child(
                    PageHeader::new(format!("Edit order {id}")).breadcrumbs(
                        Breadcrumbs::new()
                            .link("Orders", "/orders")
                            .link(format!("Order {id}"), format!("/orders/{id}"))
                            .current("Edit"),
                    ),
                )
                // The form's error summary already takes focus on a conflict.
                .child(ConflictNotice::new(state, format!("/orders/{id}/edit")).focus(false))
                .child(order_form(
                    format!("/orders/{id}"),
                    state,
                    &csrf(cx),
                    Some(revision),
                    "Save order",
                )),
        ))
        .status(status)
        .into_response()
}

/// The submitted revision, or `None` when it is missing or malformed.
fn submitted_revision(form: &FormState) -> (String, Option<Revision>) {
    let raw = form.value("revision").unwrap_or_default().to_owned();
    let parsed = raw.parse().ok();
    (raw, parsed)
}

pub(crate) async fn update(
    State(state): State<AppState>,
    cx: PageCx,
    path: IdPath,
    Submission(form): Submission,
) -> Response {
    let Some(id) = parse_id(path) else {
        return not_found(&cx);
    };
    let (submitted, revision) = submitted_revision(&form);
    let Some(revision) = revision else {
        // A malformed revision cannot match: report the current one.
        return match state.orders().get_with_revision(id).await {
            Ok(Some((_, current))) => {
                let form = form.with_conflict(submitted.clone(), current.to_string());
                edit_page(&cx, id, &form, &submitted)
            }
            Ok(None) => not_found(&cx),
            Err(error) => failure(&cx, error),
        };
    };
    let input = match validate(form.clone()) {
        Ok(input) => input,
        Err(invalid) => return edit_page(&cx, id, &invalid, &submitted),
    };
    match state
        .orders()
        .update_if(id, revision, move |old| input.into_order(old.id))
        .await
    {
        Ok(_) => SeeOther::new("/orders")
            .flash(Flash::new(
                FlashLevel::Success,
                format!("Order {id} saved."),
            ))
            .into_response(),
        Err(error) => match error.conflict_revision() {
            Some(Some(current)) => {
                let form = form.with_conflict(submitted.clone(), current.to_string());
                edit_page(&cx, id, &form, &submitted)
            }
            Some(None) => not_found(&cx),
            None => failure(&cx, error),
        },
    }
}

pub(crate) async fn confirm_delete(
    State(state): State<AppState>,
    cx: PageCx,
    path: IdPath,
) -> Response {
    let Some(id) = parse_id(path) else {
        return not_found(&cx);
    };
    match state.orders().get_with_revision(id).await {
        Ok(Some((order, revision))) => {
            confirm_page(&cx, &order, &FormState::new(), &revision.to_string())
        }
        Ok(None) => not_found(&cx),
        Err(error) => failure(&cx, error),
    }
}

fn confirm_page(cx: &PageCx, order: &Order, state: &FormState, revision: &str) -> Response {
    let id = order.id;
    let status = if state.conflict().is_some() {
        StatusCode::CONFLICT
    } else {
        StatusCode::OK
    };
    cx.title(format!("Delete order {id}? — stucco"))
        .app(shell(
            Stack::new()
                .child(
                    PageHeader::new(format!("Delete order {id}?")).breadcrumbs(
                        Breadcrumbs::new()
                            .link("Orders", "/orders")
                            .link(format!("Order {id}"), format!("/orders/{id}"))
                            .current("Delete"),
                    ),
                )
                .child(ConflictNotice::new(state, format!("/orders/{id}/delete")))
                .child(Text::new(format!(
                    "The order for {} is removed. This can't be undone.",
                    order.customer
                )))
                .child(
                    Cluster::new()
                        .child(
                            ButtonLink::new("Cancel", format!("/orders/{id}"))
                                .variant(Variant::Secondary),
                        )
                        .child(delete_form(id, revision, &csrf(cx))),
                ),
        ))
        .status(status)
        .into_response()
}

pub(crate) async fn delete(
    State(state): State<AppState>,
    cx: PageCx,
    path: IdPath,
    Submission(form): Submission,
) -> Response {
    let Some(id) = parse_id(path) else {
        return not_found(&cx);
    };
    let (submitted, revision) = submitted_revision(&form);
    let result = match revision {
        Some(revision) => state.orders().remove_if(id, revision).await,
        None => Err(StoreError::new(std::io::Error::other("malformed revision"))),
    };
    match result {
        Ok(()) => SeeOther::new("/orders")
            .flash(Flash::new(
                FlashLevel::Success,
                format!("Order {id} deleted."),
            ))
            .into_response(),
        Err(error) if error.conflict_revision() == Some(None) => not_found(&cx),
        Err(error) if error.is_conflict() || revision.is_none() => {
            match state.orders().get_with_revision(id).await {
                Ok(Some((order, current))) => {
                    let form = form.with_conflict(submitted.clone(), current.to_string());
                    confirm_page(&cx, &order, &form, &submitted)
                }
                Ok(None) => not_found(&cx),
                Err(error) => failure(&cx, error),
            }
        }
        Err(error) => failure(&cx, error),
    }
}
