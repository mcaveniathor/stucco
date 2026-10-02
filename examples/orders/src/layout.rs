//! What every page shares: the shell, flash messages, return locations and
//! the result pages for missing orders and failures.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use stucco::prelude::*;
use stucco::server::RequestContext;

/// The application shell around a page's content.
pub(crate) fn shell<'a>(numbered: bool, content: impl Render + 'a) -> AppShell<'a> {
    AppShell::new()
        .link(NavLink::new("Orders", "/orders").current(!numbered))
        .link(NavLink::new("Numbered pages", "/orders?mode=pages").current(numbered))
        .main(content)
        .footer(
            Footer::new().child(
                el::p().text("Demo data. No sign-in: every change is recorded as “Demo user”."),
            ),
        )
}

/// The flash message as a notice.
pub(crate) fn flash_notice(flash: &IncomingFlash) -> Option<Notice<'static>> {
    let flash = flash.get()?;
    let message = flash.message().to_owned();
    Some(match flash.level() {
        FlashLevel::Success => Notice::success(message),
        FlashLevel::Info => Notice::info(message),
        FlashLevel::Warning => Notice::warning(message),
        FlashLevel::Error => Notice::danger(message),
    })
}

/// A return location from the request, if it is one of this app's order
/// pages; otherwise `fallback`. Never redirect to a location from the
/// request without a check like this: it would let a link to this site
/// send people anywhere.
pub(crate) fn local_return(raw: Option<&str>, fallback: &str) -> String {
    raw.filter(|r| {
        let rest = r.strip_prefix("/orders").unwrap_or("x");
        (rest.is_empty() || rest.starts_with(['?', '/']))
            && !r.contains(['\\', '\r', '\n'])
            && !r.contains("//")
    })
    .unwrap_or(fallback)
    .to_owned()
}

/// A 303 redirect to `location` that shows `flash` there: the end of every
/// successful change.
pub(crate) fn redirect(location: impl Into<Href>, flash: Flash) -> Response {
    // `SeeOther` also has an inherent `into_response` (returning a plain
    // `http` response), so name axum's trait to get an axum response.
    IntoResponse::into_response(SeeOther::new(location).flash(flash))
}

/// `path?back=<back>`, the back location encoded.
pub(crate) fn with_back(path: &str, back: &str) -> String {
    if back == "/orders" {
        return path.to_owned();
    }
    let back: String = form_urlencoded::byte_serialize(back.as_bytes()).collect();
    let sep = if path.contains('?') { '&' } else { '?' };
    format!("{path}{sep}back={back}")
}

/// 404 for an order that doesn't exist (or no longer does).
pub(crate) fn not_found(cx: &PageCx, id: u64) -> Response {
    cx.title("Order not found — Orders")
        .app(shell(
            false,
            Stack::new()
                .child(
                    PageHeader::new("Order not found").breadcrumbs(
                        Breadcrumbs::new()
                            .link("Orders", "/orders")
                            .current("Not found"),
                    ),
                )
                .child(
                    EmptyState::new(format!("There is no order {id}"))
                        .description(
                            el::p().text("It may have been deleted, or the link may be wrong."),
                        )
                        .actions(ButtonLink::new("Back to orders", "/orders")),
                ),
        ))
        .status(StatusCode::NOT_FOUND)
        .into_response()
}

/// 500 when storage fails, with the request id to quote to support.
pub(crate) fn failure(
    cx: &PageCx,
    context: &RequestContext,
    error: &dyn std::fmt::Display,
) -> Response {
    tracing::error!(%error, request_id = %context.request_id, "orders storage failed");
    cx.title("Something went wrong — Orders")
        .app(shell(
            false,
            Stack::new()
                .child(PageHeader::new("Something went wrong"))
                .child(
                    Notice::danger(format!(
                        "The orders could not be loaded or saved. Please try again. \
                         If it keeps happening, quote request ID {}.",
                        context.request_id
                    ))
                    .actions(ButtonLink::new("Back to orders", "/orders")),
                ),
        ))
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn return_locations_stay_on_the_order_pages() {
        for good in ["/orders", "/orders?q=Ada&per=10", "/orders/4"] {
            assert_eq!(local_return(Some(good), "/x"), good);
        }
        for bad in [
            "https://evil.example/orders",
            "//evil.example",
            "/orders//evil.example",
            "/ordersevil",
            "/\\evil.example",
            "/orders\\..",
            "javascript:alert(1)",
            "",
        ] {
            assert_eq!(local_return(Some(bad), "/orders"), "/orders", "{bad}");
        }
        assert_eq!(local_return(None, "/orders/1"), "/orders/1");
        assert_eq!(with_back("/orders/1", "/orders"), "/orders/1");
        assert_eq!(
            with_back("/orders/1", "/orders?q=a&b=c"),
            "/orders/1?back=%2Forders%3Fq%3Da%26b%3Dc"
        );
    }
}
