//! One order: its page, the create and edit forms, archiving and deleting.
//!
//! Every change is a POST that ends in a redirect (post/redirect/get) with a
//! flash message, so reloading the next page never repeats it. An invalid
//! form is shown again with status 422, the submitted values kept and each
//! problem linked from a summary; an edit that lost a race is shown with
//! status 409.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;
use stucco::prelude::*;
use stucco::server::RequestContext;

use crate::AppState;
use crate::layout::{failure, flash_notice, local_return, not_found, redirect, shell, with_back};
use crate::model::{EDITABLE_STATUSES, Order, OrderInput, format_cents, money};
use crate::store::Update;
use crate::time::{readable, today};

#[derive(Deserialize)]
pub(crate) struct Back {
    back: Option<String>,
}

impl Back {
    fn list(&self) -> String {
        local_return(self.back.as_deref(), "/orders")
    }
}

/// An id from the path; anything else is a missing order.
fn order_id(raw: &str) -> Option<u64> {
    raw.parse().ok()
}

/// Loads an order or answers with the right page for its absence or a
/// storage failure.
async fn load(
    state: &AppState,
    raw: &str,
    cx: &PageCx,
    context: &RequestContext,
) -> Result<Order, Response> {
    let Some(id) = order_id(raw) else {
        return Err(not_found(cx, 0));
    };
    match state.orders.get(id).await {
        Ok(Some(order)) => Ok(order),
        Ok(None) => Err(not_found(cx, id)),
        Err(error) => Err(failure(cx, context, &error)),
    }
}

/// `GET /orders/{id}`.
pub(crate) async fn show(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Query(back): Query<Back>,
    context: RequestContext,
    flash: IncomingFlash,
    cx: PageCx,
) -> Response {
    let order = match load(&state, &raw, &cx, &context).await {
        Ok(order) => order,
        Err(response) => return response,
    };
    let back = back.list();
    let id = order.id;
    let mut actions = Cluster::new();
    if order.archived() {
        actions = actions.child(
            Form::post(format!("/orders/{id}/restore"))
                .child(HiddenInput::new("back", back.clone()))
                .child(Button::new("Restore").submit().variant(Variant::Primary)),
        );
    } else {
        actions = actions
            .child(
                ButtonLink::new("Edit", with_back(&format!("/orders/{id}/edit"), &back))
                    .variant(Variant::Primary),
            )
            .child(ButtonLink::new(
                "Archive",
                with_back(&format!("/orders/{id}/archive"), &back),
            ));
    }
    actions = actions.child(
        ButtonLink::new("Delete", with_back(&format!("/orders/{id}/delete"), &back))
            .variant(Variant::Danger),
    );
    let details = DescriptionList::new()
        .item("Customer", order.customer.clone())
        .item(
            "Status",
            StatusBadge::new(order.status_label()).tone(order.status_tone()),
        )
        .item("Total", money(&order))
        .item("Order date", Timestamp::date(order.created.clone()))
        .item("Priority", if order.priority { "Yes" } else { "No" })
        .item(
            "Note",
            if order.note.is_empty() {
                "No note".to_owned()
            } else {
                order.note.clone()
            },
        );
    let activity = ActivityList::new(format!("History of {}", order.label().to_lowercase())).items(
        order.history.iter().rev().map(|event| {
            let time = Timestamp::new(event.at.clone(), readable(&event.at)).zone("UTC");
            let activity = Activity::new(event.actor.clone(), event.action.clone(), time);
            match &event.detail {
                Some(detail) => activity.detail(detail.clone()),
                None => activity,
            }
        }),
    );
    let archived = order.archived().then(|| {
        Notice::info("This order is archived, so it is read-only and hidden from the order list. Restore it to make changes.")
            .quiet()
    });
    let header = PageHeader::new(order.label())
        .breadcrumbs(
            Breadcrumbs::new()
                .link("Orders", back.clone())
                .current(order.label()),
        )
        .description(
            Cluster::new()
                .child(StatusBadge::new(order.status_label()).tone(order.status_tone()))
                .child(el::span().text(format!("{} · {}", order.customer, money(&order)))),
        )
        .actions(actions);
    let document = cx.title(format!("{} — Orders", order.label())).app(shell(
        false,
        Stack::new()
            .child(header)
            .child(flash_notice(&flash))
            .child(archived)
            .child(Panel::new("Details").body(details))
            .child(Panel::new("Activity").body(activity))
            .child(el::p().child(Link::new("Back to orders", back.clone()))),
    ));
    (flash, document).into_response()
}

/// Which form: a new order, or an edit of one at a version.
enum Mode<'o> {
    New,
    Edit(&'o Order),
}

/// The order form, filled from `values`.
fn form_page(
    cx: &PageCx,
    mode: Mode<'_>,
    values: &FormState,
    version: Option<u64>,
    back: &str,
    status: StatusCode,
) -> Response {
    let (title, action, submit, cancel, crumb) = match mode {
        Mode::New => (
            "New order".to_owned(),
            "/orders".to_owned(),
            "Create order",
            back.to_owned(),
            None,
        ),
        Mode::Edit(order) => (
            format!("Edit {}", order.label().to_lowercase()),
            format!("/orders/{}", order.id),
            "Save changes",
            with_back(&format!("/orders/{}", order.id), back),
            Some((
                order.label(),
                with_back(&format!("/orders/{}", order.id), back),
            )),
        ),
    };
    let mut crumbs = Breadcrumbs::new().link("Orders", back.to_owned());
    if let Some((label, href)) = crumb {
        crumbs = crumbs.link(label, href);
    }
    let summary = ErrorSummary::new(values)
        .field("customer", "order-customer")
        .field("status", "order-status")
        .field("total", "order-total")
        .field("created", "order-created")
        .field("note", "order-note");
    let mut form = Form::post(action)
        // The application's CSRF defence would add its token here with
        // `.csrf(token)`; this example checks the request's origin instead
        // (see `same_origin` in lib.rs).
        .child(HiddenInput::new("back", back.to_owned()))
        .child(
            Fieldset::new("Customer and status")
                .child(
                    Field::new(
                        "Customer",
                        Input::text("customer")
                            .id("order-customer")
                            .autocomplete("off"),
                    )
                    .required()
                    .bind(values),
                )
                .child(
                    Field::new(
                        "Status",
                        Select::new("status")
                            .id("order-status")
                            .placeholder("Choose a status")
                            .options(EDITABLE_STATUSES),
                    )
                    .required()
                    .bind(values),
                )
                .child(
                    Checkbox::new("priority", "Priority order")
                        .id("order-priority")
                        .hint("Priority orders are packed first.")
                        .bind(values),
                ),
        )
        .child(
            Fieldset::new("Payment and date")
                .child(
                    // A text input, not a number input: browsers discard what
                    // a number input can't parse, so "12,5" could never be
                    // shown back with its error.
                    Field::new(
                        "Total",
                        Input::text("total")
                            .id("order-total")
                            .prefix("$")
                            .attr("inputmode", "decimal")
                            .autocomplete("off"),
                    )
                    .hint("In US dollars, like 12.50")
                    .required()
                    .bind(values),
                )
                .child(
                    Field::new("Order date", Input::date("created").id("order-created"))
                        .required()
                        .bind(values),
                ),
        )
        .child(
            Field::new("Note", Textarea::new("note").id("order-note").rows(4))
                .optional()
                .hint("Seen by staff only. Never include card numbers or passwords.")
                .bind(values),
        );
    if let Some(version) = version {
        form = form.child(HiddenInput::new("version", version.to_string()));
    }
    let form = form.child(FormActions::submit(submit).cancel(cancel));
    cx.title(format!("{title} — Orders"))
        .app(shell(
            false,
            Stack::new()
                .child(PageHeader::new(title.clone()).breadcrumbs(crumbs.current(title)))
                .child(summary)
                .child(Card::new().child(form)),
        ))
        .status(status)
        .into_response()
}

/// An order's stored values as the form's initial values.
fn initial(order: &Order) -> FormState {
    let state = FormState::new()
        .with_value("customer", order.customer.clone())
        .with_value("status", order.status.clone())
        .with_value(
            "total",
            format_cents(order.total_cents).trim_start_matches('$'),
        )
        .with_value("created", order.created.clone())
        .with_value("note", order.note.clone());
    if order.priority {
        state.with_value("priority", "on")
    } else {
        state
    }
}

/// `GET /orders/new`.
pub(crate) async fn new(Query(back): Query<Back>, cx: PageCx) -> Response {
    let values = FormState::new()
        .with_value("status", "pending")
        .with_value("created", today());
    form_page(&cx, Mode::New, &values, None, &back.list(), StatusCode::OK)
}

/// The message for a save that failed on the server, kept apart from the
/// field errors: nothing the user typed was wrong.
const NOT_SAVED: &str = "The order couldn’t be saved because of a problem on our side. Your changes are still below; try again.";

/// `POST /orders`.
pub(crate) async fn create(
    State(state): State<AppState>,
    context: RequestContext,
    cx: PageCx,
    Submission(form): Submission,
) -> Response {
    let back = local_return(form.value("back"), "/orders");
    let submitted = form.clone();
    let mut v = Validator::new(form);
    let input = OrderInput::validate(&mut v);
    let input = match v.finish(|| input) {
        Ok(input) => input,
        Err(values) => {
            return form_page(
                &cx,
                Mode::New,
                &values,
                None,
                &back,
                StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    match state.orders.create(input).await {
        Ok(order) => redirect(
            with_back(&format!("/orders/{}", order.id), &back),
            Flash::success(format!("{} was created.", order.label())),
        ),
        Err(error) => {
            tracing::error!(%error, request_id = %context.request_id, "creating an order failed");
            let values = submitted.with_form_error(NOT_SAVED);
            form_page(
                &cx,
                Mode::New,
                &values,
                None,
                &back,
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    }
}

/// `GET /orders/{id}/edit`.
pub(crate) async fn edit(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Query(back): Query<Back>,
    context: RequestContext,
    cx: PageCx,
) -> Response {
    let order = match load(&state, &raw, &cx, &context).await {
        Ok(order) => order,
        Err(response) => return response,
    };
    let back = back.list();
    if order.archived() {
        return read_only(&order, &back);
    }
    form_page(
        &cx,
        Mode::Edit(&order),
        &initial(&order),
        Some(order.version),
        &back,
        StatusCode::OK,
    )
}

/// Archived orders can't be edited: back to the order, saying why.
fn read_only(order: &Order, back: &str) -> Response {
    redirect(
        with_back(&format!("/orders/{}", order.id), back),
        Flash::warning(format!(
            "{} is archived. Restore it before editing.",
            order.label()
        )),
    )
}

/// `POST /orders/{id}`: save an edit.
pub(crate) async fn update(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    context: RequestContext,
    cx: PageCx,
    Submission(form): Submission,
) -> Response {
    let order = match load(&state, &raw, &cx, &context).await {
        Ok(order) => order,
        Err(response) => return response,
    };
    let back = local_return(form.value("back"), "/orders");
    // A missing or mangled version can't be matched, so it reads as stale.
    let version = form
        .value("version")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);
    let submitted = form.clone();
    let mut v = Validator::new(form);
    let input = OrderInput::validate(&mut v);
    let input = match v.finish(|| input) {
        Ok(input) => input,
        Err(values) => {
            return form_page(
                &cx,
                Mode::Edit(&order),
                &values,
                Some(version),
                &back,
                StatusCode::UNPROCESSABLE_ENTITY,
            );
        }
    };
    match state.orders.update(order.id, version, input).await {
        Ok(Update::Saved(order)) => redirect(
            with_back(&format!("/orders/{}", order.id), &back),
            Flash::success(format!("{} was saved.", order.label())),
        ),
        Ok(Update::Stale(current)) => {
            // Keep what this user typed, explain, and carry the current
            // version so saving again is a deliberate overwrite.
            let values = submitted.with_form_error(
                "Someone else changed this order while you were editing. Your changes are below \
                 and have not been saved. Open the order in a new tab to compare, then save again \
                 to replace their changes with yours.",
            );
            form_page(
                &cx,
                Mode::Edit(&current),
                &values,
                Some(current.version),
                &back,
                StatusCode::CONFLICT,
            )
        }
        Ok(Update::Archived(order)) => read_only(&order, &back),
        Ok(Update::Missing) => not_found(&cx, order.id),
        Err(error) => {
            tracing::error!(%error, request_id = %context.request_id, "saving an order failed");
            let values = submitted.with_form_error(NOT_SAVED);
            form_page(
                &cx,
                Mode::Edit(&order),
                &values,
                Some(version),
                &back,
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        }
    }
}

/// A one-line description of the order, for confirmations.
fn summary(order: &Order) -> String {
    format!(
        "{} for {}, {}, {}",
        order.label(),
        order.customer,
        money(order),
        order.status_label()
    )
}

fn confirm_page(cx: &PageCx, order: &Order, back: &str, page: Confirmation<'_>) -> Response {
    let detail = with_back(&format!("/orders/{}", order.id), back);
    cx.title(format!("{} — Orders", order.label()))
        .app(shell(
            false,
            Stack::new()
                .child(
                    Breadcrumbs::new()
                        .link("Orders", back.to_owned())
                        .link(order.label(), detail.clone())
                        .current("Confirm"),
                )
                .child(
                    page.target(summary(order))
                        .hidden("back", back.to_owned())
                        .cancel(detail),
                ),
        ))
        .into_response()
}

/// `GET /orders/{id}/archive`.
pub(crate) async fn confirm_archive(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Query(back): Query<Back>,
    context: RequestContext,
    cx: PageCx,
) -> Response {
    let order = match load(&state, &raw, &cx, &context).await {
        Ok(order) => order,
        Err(response) => return response,
    };
    let back = back.list();
    if order.archived() {
        return redirect(
            with_back(&format!("/orders/{}", order.id), &back),
            Flash::info(format!("{} is already archived.", order.label())),
        );
    }
    let id = order.id;
    confirm_page(
        &cx,
        &order,
        &back,
        Confirmation::new(
            format!("Archive order {id}?"),
            format!("/orders/{id}/archive"),
            format!("Archive order {id}"),
        )
        .consequence("It leaves the order list. Filter by the Archived status to find it.")
        .consequence("It becomes read-only until restored. Nothing is deleted."),
    )
}

/// `POST /orders/{id}/archive` and `POST /orders/{id}/restore`.
async fn set_archived(
    state: AppState,
    raw: String,
    context: RequestContext,
    cx: PageCx,
    form: FormState,
    archived: bool,
) -> Response {
    let order = match load(&state, &raw, &cx, &context).await {
        Ok(order) => order,
        Err(response) => return response,
    };
    let back = local_return(form.value("back"), "/orders");
    let changed = match state.orders.set_archived(vec![order.id], archived).await {
        Ok(changed) => !changed.is_empty(),
        Err(error) => return failure(&cx, &context, &error),
    };
    let flash = match (changed, archived) {
        (true, true) => Flash::success(format!("{} was archived.", order.label())),
        (true, false) => Flash::success(format!("{} was restored.", order.label())),
        (false, true) => Flash::info(format!("{} was already archived.", order.label())),
        (false, false) => Flash::info(format!("{} is not archived.", order.label())),
    };
    redirect(with_back(&format!("/orders/{}", order.id), &back), flash)
}

pub(crate) async fn archive(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    context: RequestContext,
    cx: PageCx,
    Submission(form): Submission,
) -> Response {
    set_archived(state, raw, context, cx, form, true).await
}

pub(crate) async fn restore(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    context: RequestContext,
    cx: PageCx,
    Submission(form): Submission,
) -> Response {
    set_archived(state, raw, context, cx, form, false).await
}

/// `GET /orders/{id}/delete`.
pub(crate) async fn confirm_delete(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    Query(back): Query<Back>,
    context: RequestContext,
    cx: PageCx,
) -> Response {
    let order = match load(&state, &raw, &cx, &context).await {
        Ok(order) => order,
        Err(response) => return response,
    };
    let back = back.list();
    let id = order.id;
    let mut page = Confirmation::new(
        format!("Delete order {id} permanently?"),
        format!("/orders/{id}/delete"),
        format!("Delete order {id}"),
    )
    .consequence("The order and its history are deleted.")
    .danger();
    page = if order.archived() {
        page.consequence("This can't be undone.")
    } else {
        page.consequence(
            el::span()
                .text("This can't be undone. To hide the order but keep its record, ")
                .child(Link::new(
                    "archive it instead",
                    with_back(&format!("/orders/{id}/archive"), &back),
                ))
                .text("."),
        )
    };
    confirm_page(&cx, &order, &back, page)
}

/// `POST /orders/{id}/delete`.
pub(crate) async fn delete(
    State(state): State<AppState>,
    Path(raw): Path<String>,
    context: RequestContext,
    cx: PageCx,
    Submission(form): Submission,
) -> Response {
    let Some(id) = order_id(&raw) else {
        return not_found(&cx, 0);
    };
    // Never return to the page of the order just deleted.
    let back = local_return(form.value("back"), "/orders");
    let gone = format!("/orders/{id}");
    let back = if back == gone
        || back.starts_with(&format!("{gone}/"))
        || back.starts_with(&format!("{gone}?"))
    {
        "/orders".to_owned()
    } else {
        back
    };
    let flash = match state.orders.delete(vec![id]).await {
        Ok(1) => Flash::success(format!("Order {id} was deleted.")),
        // Deleted already, perhaps in another tab: the outcome the user
        // wanted, so say so rather than show an error.
        Ok(_) => Flash::info(format!("Order {id} had already been deleted.")),
        Err(error) => return failure(&cx, &context, &error),
    };
    redirect(back, flash)
}
