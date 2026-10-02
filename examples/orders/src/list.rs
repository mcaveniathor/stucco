//! The order list, and actions on several orders at once.

use axum::extract::{RawQuery, State};
use axum::response::{IntoResponse, Response};
use stucco::Slot;
use stucco::prelude::*;
use stucco::server::RequestContext;

use crate::AppState;
use crate::layout::{failure, flash_notice, local_return, redirect, shell, with_back};
use crate::model::{Order, format_cents};

pub(crate) async fn list(
    State(state): State<AppState>,
    RawQuery(raw): RawQuery,
    context: RequestContext,
    flash: IncomingFlash,
    cx: PageCx,
) -> Response {
    let raw = raw.unwrap_or_default();
    let numbered = form_urlencoded::parse(raw.as_bytes()).any(|(k, v)| k == "mode" && v == "pages");
    let (source, action) = if numbered {
        (&state.pages, "/orders?mode=pages")
    } else {
        (&state.cursor, "/orders")
    };
    let orders = match source.load(&raw, &Order::column_specs(), &context).await {
        Ok(orders) => orders,
        Err(error) => return failure(&cx, &context, &error),
    };
    // This exact list, to come back to from an order and after a change.
    let back = orders.query.link(&Href::new(action)).as_str().to_owned();
    let link_back = back.clone();
    let columns = Order::columns().into_iter().map(move |col| {
        if col.spec().key == "id" {
            let back = link_back.clone();
            col.href(move |o: &Order| with_back(&format!("/orders/{}", o.id), &back))
        } else {
            col
        }
    });
    let edit_back = back.clone();
    let table = DataTable::from_collection(&orders, "Orders")
        .action(action)
        .row_id(|o: &Order| o.id.to_string())
        .selectable("bulk", "id", |o| o.label().to_lowercase())
        .columns(columns)
        .column(Col::actions("Actions", move |o: &Order| {
            // Archived orders are read-only, so they offer no edit link. The
            // edit handler refuses them too: hiding a link is not a check.
            if o.archived() {
                return Slot::new("");
            }
            Slot::new(
                el::a()
                    .href(with_back(&format!("/orders/{}/edit", o.id), &edit_back))
                    .text("Edit")
                    .child(
                        el::span()
                            .class("st-sr-only")
                            .text(format!(" order {}", o.id)),
                    ),
            )
        }))
        .empty(
            EmptyState::new("No orders yet")
                .description(el::p().text("Orders you create appear here."))
                .actions(ButtonLink::new("New order", "/orders/new").variant(Variant::Primary)),
        );
    // The row checkboxes join this form through their `form` attribute. It
    // only asks which action to confirm, so it is a GET; the confirmation
    // page posts.
    let bulk = Form::get("/orders/bulk")
        .id("bulk")
        .class("orders-bulk")
        .child(HiddenInput::new("back", back.clone()))
        .child(
            Cluster::new()
                .child(el::span().text("With the selected orders:"))
                .child(
                    Button::new("Archive")
                        .submit()
                        .name("action")
                        .value("archive"),
                )
                .child(
                    Button::new("Delete")
                        .submit()
                        .name("action")
                        .value("delete")
                        .variant(Variant::Danger),
                ),
        );
    let header = PageHeader::new("Orders")
        .description("A persistent collection: search, filter, sort and page with ordinary links.")
        .actions(
            ButtonLink::new("New order", with_back("/orders/new", &back)).variant(Variant::Primary),
        );
    let document = cx.title("Orders").app(shell(
        numbered,
        Stack::new()
            .child(header)
            .child(flash_notice(&flash))
            .child((!orders.page.rows.is_empty()).then_some(bulk))
            .child(table),
    ));
    (flash, document).into_response()
}

/// "1 order", "3 orders".
pub(crate) fn count(n: usize) -> String {
    if n == 1 {
        "1 order".to_owned()
    } else {
        format!("{n} orders")
    }
}

/// The action and order ids a bulk request names: at most 100 ids, each
/// once.
fn bulk_request(state: &FormState) -> (Option<&'static str>, Vec<u64>) {
    let action = match state.value("action") {
        Some("archive") => Some("archive"),
        Some("delete") => Some("delete"),
        _ => None,
    };
    let mut ids: Vec<u64> = Vec::new();
    for id in state.values("id").iter().filter_map(|v| v.parse().ok()) {
        if !ids.contains(&id) && ids.len() < 100 {
            ids.push(id);
        }
    }
    (action, ids)
}

/// `GET /orders/bulk`: confirm an action on the selected orders.
pub(crate) async fn confirm_bulk(
    State(state): State<AppState>,
    RawQuery(raw): RawQuery,
    context: RequestContext,
    cx: PageCx,
) -> Response {
    let form = FormState::from_urlencoded(raw.unwrap_or_default().as_bytes());
    let back = local_return(form.value("back"), "/orders");
    let (action, ids) = bulk_request(&form);
    let Some(action) = action else {
        return redirect(
            back,
            Flash::error("Choose an action for the selected orders."),
        );
    };
    if ids.is_empty() {
        return redirect(
            back,
            Flash::warning(format!("Select the orders to {action} first.")),
        );
    }
    let orders = match state.orders.get_many(ids.clone()).await {
        Ok(orders) => orders,
        Err(error) => return failure(&cx, &context, &error),
    };
    if orders.is_empty() {
        return redirect(back, Flash::warning("The selected orders no longer exist."));
    }
    let missing = ids.len() - orders.len();
    let n = count(orders.len());
    let mut page = match action {
        "archive" => Confirmation::new(format!("Archive {n}?"), "/orders/bulk", format!("Archive {n}"))
            .consequence("They leave the order list. Filter by the Archived status to find them.")
            .consequence("They become read-only until restored. Nothing is deleted."),
        _ => Confirmation::new(
            format!("Delete {n} permanently?"),
            "/orders/bulk",
            format!("Delete {n}"),
        )
        .consequence("The orders and their history are deleted.")
        .consequence("This can't be undone. To keep them out of the list but recoverable, archive them instead.")
        .danger(),
    };
    page = page
        .target(el::ul().children(orders.iter().map(|o| {
            el::li().text(format!(
                "{} — {}, {}, {}",
                o.label(),
                o.customer,
                format_cents(o.total_cents),
                o.status_label()
            ))
        })))
        .hidden("action", action)
        .hidden("back", back.clone())
        .cancel(back.clone());
    for order in &orders {
        page = page.hidden("id", order.id.to_string());
    }
    let skipped = (missing > 0).then(|| {
        Notice::warning(format!(
            "{missing} of the selected orders no longer {}, so {} not included.",
            if missing == 1 { "exists" } else { "exist" },
            if missing == 1 { "it is" } else { "they are" },
        ))
        .quiet()
    });
    cx.title(format!(
        "{} {n}? — Orders",
        if action == "archive" {
            "Archive"
        } else {
            "Delete"
        }
    ))
    .app(shell(
        false,
        Stack::new()
            .child(Breadcrumbs::new().link("Orders", back).current("Confirm"))
            .child(skipped)
            .child(page),
    ))
    .into_response()
}

/// `POST /orders/bulk`: apply a confirmed bulk action.
pub(crate) async fn apply_bulk(
    State(state): State<AppState>,
    context: RequestContext,
    cx: PageCx,
    Submission(form): Submission,
) -> Response {
    let back = local_return(form.value("back"), "/orders");
    let (action, ids) = bulk_request(&form);
    let (Some(action), false) = (action, ids.is_empty()) else {
        return redirect(
            back,
            Flash::warning("Nothing was changed: no orders or action were selected."),
        );
    };
    let asked = ids.len();
    // Each id is checked again here: the orders may have changed since the
    // confirmation page was shown. A real app also checks, per order, that
    // the user may do this.
    let done = match action {
        "archive" => state.orders.set_archived(ids, true).await.map(|c| c.len()),
        _ => state.orders.delete(ids).await,
    };
    let done = match done {
        Ok(done) => done,
        Err(error) => return failure(&cx, &context, &error),
    };
    let verb = if action == "archive" {
        "Archived"
    } else {
        "Deleted"
    };
    let flash = if done == asked {
        Flash::success(format!("{verb} {}.", count(done)))
    } else {
        Flash::warning(format!(
            "{verb} {}. {} had already been changed or deleted.",
            count(done),
            count(asked - done)
        ))
    };
    redirect(back, flash)
}
