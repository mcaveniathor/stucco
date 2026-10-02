//! One-line setup for axum apps: mount a bundle's assets, add the standard
//! layers, and build pages in handlers from a [`PageCx`].

use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};
use http::StatusCode;
use http::request::Parts;
use stucco_core::{
    AssetRequirements, Attrs, Bundle, Cx, Meta, Page, Raw, Render, WithBody, render_fragment,
};
use stucco_theme::{BuiltTheme, Preset, Theme};

use crate::{FragmentResponse, PageResponse, RequestContext, RequestKind};
#[cfg(feature = "tower-http")]
use crate::{LayerConfig, assets_router, with_standard_layers};

/// Anything that makes the app's [`Bundle`]: a preset, a built theme, a
/// bundle, or a shared bundle.
pub trait IntoBundle {
    /// The shared bundle.
    fn into_bundle(self) -> Arc<Bundle>;
}

impl IntoBundle for Arc<Bundle> {
    fn into_bundle(self) -> Arc<Bundle> {
        self
    }
}

impl IntoBundle for Bundle {
    fn into_bundle(self) -> Arc<Bundle> {
        Arc::new(self)
    }
}

impl IntoBundle for Preset {
    fn into_bundle(self) -> Arc<Bundle> {
        Arc::new(Bundle::new(self))
    }
}

impl IntoBundle for BuiltTheme {
    fn into_bundle(self) -> Arc<Bundle> {
        Arc::new(Bundle::new(self))
    }
}

/// Builds the theme; panics with the contrast report if it fails, like
/// [`Bundle::new`].
impl IntoBundle for Theme {
    fn into_bundle(self) -> Arc<Bundle> {
        Arc::new(Bundle::new(self))
    }
}

/// Sets up an axum router for stucco in one call.
///
/// Call it last, after your routes and state: it mounts the bundle's assets,
/// wraps everything in the [standard layers](with_standard_layers), and
/// makes the bundle available to the [`PageCx`] extractor.
///
/// ```
/// use axum::{Router, routing::get};
/// use stucco_core::el;
/// use stucco_theme::Preset;
/// use stucco_tower::{Document, PageCx, StuccoRouter};
///
/// async fn index(page: PageCx) -> Document {
///     page.title("Hello").body(el::main().id("main").child(el::h1().text("Hello")))
/// }
///
/// let app: Router = Router::new().route("/", get(index)).stucco(Preset::Slate);
/// ```
#[cfg(feature = "tower-http")]
pub trait StuccoRouter {
    /// Mounts `bundle` with the default [`LayerConfig`].
    fn stucco(self, bundle: impl IntoBundle) -> axum::Router;

    /// Mounts `bundle` with the given limits.
    fn stucco_with(self, bundle: impl IntoBundle, config: &LayerConfig) -> axum::Router;
}

#[cfg(feature = "tower-http")]
impl StuccoRouter for axum::Router {
    fn stucco(self, bundle: impl IntoBundle) -> axum::Router {
        self.stucco_with(bundle, &LayerConfig::default())
    }

    fn stucco_with(self, bundle: impl IntoBundle, config: &LayerConfig) -> axum::Router {
        let bundle = bundle.into_bundle();
        with_standard_layers(self.merge(assets_router(bundle.clone())), config)
            .layer(axum::Extension(bundle))
    }
}

/// An extractor for building pages and fragments in a handler: the app's
/// bundle, set up by [`StuccoRouter::stucco`], and the request's context.
#[derive(Clone, Debug)]
pub struct PageCx {
    bundle: Arc<Bundle>,
    context: RequestContext,
    kind: RequestKind,
}

impl PageCx {
    /// A context for `bundle`, outside a request (for tests and static
    /// rendering).
    pub fn new(bundle: impl IntoBundle) -> PageCx {
        PageCx {
            bundle: bundle.into_bundle(),
            context: RequestContext::default(),
            kind: RequestKind::Full,
        }
    }

    /// A page titled `title`.
    pub fn title(&self, title: impl Into<String>) -> Document {
        Document {
            bundle: self.bundle.clone(),
            title: title.into(),
            lang: None,
            meta: None,
            head: None,
            body: None,
            body_attrs: None,
            enhanced: false,
            status: StatusCode::OK,
        }
    }

    /// A fragment response for `content` in namespace `id`, for enhanced
    /// requests that replace one region of a page.
    pub fn fragment(&self, id: &str, content: &impl Render) -> FragmentResponse {
        FragmentResponse::new(&render_fragment(id, content), &self.bundle)
    }

    /// The app's bundle.
    pub fn bundle(&self) -> &Arc<Bundle> {
        &self.bundle
    }

    /// The request's context, such as its request id.
    pub fn context(&self) -> &RequestContext {
        &self.context
    }

    /// Whether the request wants a full page or one fragment.
    pub fn kind(&self) -> &RequestKind {
        &self.kind
    }

    /// Answers the request with a full page or, for an enhanced request,
    /// one of the fragments named with [`Respond::fragment`]. Only the
    /// output that is sent gets rendered; a request for any other fragment
    /// gets the full page.
    ///
    /// ```
    /// use stucco_core::el;
    /// use stucco_tower::PageCx;
    ///
    /// async fn index(page: PageCx) -> axum::response::Response {
    ///     let count = 3;
    ///     page.respond()
    ///         .fragment("count", || el::p().id("count").text(count.to_string()))
    ///         .page(|| page.title("Counter").body(el::main().id("main").text("…")))
    /// }
    /// ```
    pub fn respond(&self) -> Respond<'_> {
        Respond {
            cx: self,
            chosen: None,
        }
    }
}

/// Chooses between a page and its fragments; see [`PageCx::respond`].
#[derive(Debug)]
#[must_use = "finish with `page`, which makes the response"]
pub struct Respond<'p> {
    cx: &'p PageCx,
    chosen: Option<FragmentResponse>,
}

impl Respond<'_> {
    /// Answers a fragment request for element `id` with `content`, rendered
    /// only if this is the fragment requested.
    pub fn fragment<R: Render>(mut self, id: &str, content: impl FnOnce() -> R) -> Self {
        let wanted = matches!(&self.cx.kind, RequestKind::Fragment { target } if target == id);
        if self.chosen.is_none() && wanted {
            self.chosen = Some(self.cx.fragment(id, &content()));
        }
        self
    }

    /// Answers everything else with the page `page` builds.
    pub fn page(self, page: impl FnOnce() -> Document) -> Response {
        match self.chosen {
            Some(fragment) => IntoResponse::into_response(fragment),
            None => page().into_response(),
        }
    }
}

impl<S: Send + Sync> FromRequestParts<S> for PageCx {
    type Rejection = MissingBundle;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let bundle = parts
            .extensions
            .get::<Arc<Bundle>>()
            .cloned()
            .ok_or(MissingBundle)?;
        Ok(PageCx {
            bundle,
            context: RequestContext::from_parts(parts),
            kind: RequestKind::from_parts(parts),
        })
    }
}

/// The rejection when a handler takes a [`PageCx`] but the router has no
/// bundle: call [`StuccoRouter::stucco`] on it.
#[derive(Clone, Copy, Debug)]
pub struct MissingBundle;

impl IntoResponse for MissingBundle {
    fn into_response(self) -> Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "stucco: no bundle on this router; call .stucco(...) on it after adding routes",
        )
            .into_response()
    }
}

/// Content rendered as soon as it is set, with the assets it requires, so
/// a [`Document`] can borrow request data and still be returned.
#[derive(Debug)]
struct Rendered {
    html: String,
    required: AssetRequirements,
}

impl Rendered {
    fn new(content: impl Render) -> Rendered {
        let mut cx = Cx::new();
        content.render(&mut cx);
        let (html, required) = cx.finish();
        Rendered { html, required }
    }
}

impl Render for Rendered {
    fn render(&self, cx: &mut Cx) {
        Raw::trusted(self.html.as_str()).render(cx);
        for asset in self.required.iter() {
            cx.require(asset);
        }
    }
}

/// A page that handlers can return: axum renders it as a full HTML
/// response. Build one with [`PageCx::title`].
///
/// Body and head content are rendered as soon as they are set, so they can
/// borrow request data such as a query or a page of rows.
#[derive(Debug)]
pub struct Document {
    bundle: Arc<Bundle>,
    title: String,
    lang: Option<String>,
    meta: Option<Meta>,
    head: Option<Rendered>,
    body: Option<Rendered>,
    body_attrs: Option<Attrs>,
    enhanced: bool,
    status: StatusCode,
}

impl Document {
    /// The document language (default `"en"`).
    pub fn lang(mut self, lang: &str) -> Self {
        self.lang = Some(lang.to_owned());
        self
    }

    /// Document metadata.
    pub fn meta(mut self, meta: Meta) -> Self {
        self.meta = Some(meta);
        self
    }

    /// Extra `<head>` content.
    pub fn head(mut self, head: impl Render) -> Self {
        self.head = Some(Rendered::new(head));
        self
    }

    /// The `<body>` content. `main` and `app` from the UI crate's `PageExt`
    /// fill it with the right landmarks.
    pub fn body(mut self, body: impl Render) -> Self {
        self.body = Some(Rendered::new(body));
        self
    }

    /// Attributes on `<body>`.
    pub fn body_attrs(mut self, attrs: Attrs) -> Self {
        self.body_attrs = Some(attrs);
        self
    }

    /// Includes the client runtime even if no behaviour is used yet.
    pub fn enhanced(mut self) -> Self {
        self.enhanced = true;
        self
    }

    /// The response status (default 200; 422 for a form with errors).
    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    /// Renders the document's HTML.
    pub fn render(self) -> String {
        let mut page = Page::new(&self.bundle, self.title);
        if let Some(lang) = &self.lang {
            page = page.lang(lang);
        }
        if let Some(meta) = self.meta {
            page = page.meta(meta);
        }
        if let Some(head) = self.head {
            page = page.head(head);
        }
        if let Some(body) = self.body {
            page = page.body(body);
        }
        if let Some(attrs) = self.body_attrs {
            page = page.body_attrs(attrs);
        }
        if self.enhanced {
            page = page.enhanced();
        }
        page.render()
    }
}

impl<'a> WithBody<'a> for Document {
    fn with_body(self, body: impl Render + 'a) -> Self {
        self.body(body)
    }
}

/// For [`respond`](crate::respond), whose full-page branch returns a
/// [`PageResponse`].
impl From<Document> for PageResponse {
    fn from(document: Document) -> PageResponse {
        let status = document.status;
        PageResponse::new(document.render()).status(status)
    }
}

impl IntoResponse for Document {
    fn into_response(self) -> Response {
        PageResponse::from(self)
            .into_response()
            .map(axum::body::Body::new)
    }
}

#[cfg(all(test, feature = "tower-http"))]
mod tests {
    use super::*;
    use axum::Router;
    use axum::body::Body;
    use axum::routing::get;
    use http::Request;
    use http_body_util::BodyExt;
    use stucco_core::el;
    use tower::ServiceExt;

    async fn get_text(app: Router, uri: &str) -> (StatusCode, String) {
        let res = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = res.status();
        let body = res.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8(body.to_vec()).unwrap())
    }

    #[tokio::test]
    async fn one_call_serves_pages_and_their_assets() {
        async fn index(page: PageCx) -> Document {
            page.title("Hello <world>")
                .body(el::main().id("main").child(el::h1().text("Hello")))
        }
        let app = Router::new().route("/", get(index)).stucco(Preset::Slate);
        let (status, html) = get_text(app.clone(), "/").await;
        assert_eq!(status, StatusCode::OK);
        assert!(html.contains("<title>Hello &lt;world&gt;</title>"));
        assert!(html.contains("<h1>Hello</h1>"));
        let css = html
            .split("rel=\"stylesheet\" href=\"")
            .nth(1)
            .and_then(|s| s.split('"').next())
            .expect("a stylesheet link");
        let (status, _) = get_text(app, css).await;
        assert_eq!(status, StatusCode::OK);
    }

    #[test]
    fn documents_render_like_pages_and_may_borrow() {
        static NOTE: stucco_core::Asset = stucco_core::Asset {
            name: "test-note",
            css: Some("@layer stucco.components { .note { color: red } }"),
            behavior: None,
            deps: &[],
        };
        let rows = ["first".to_owned(), "<second>".to_owned()];
        let body = || {
            stucco_core::render_fn(|cx: &mut Cx| {
                cx.require(&NOTE);
                el::ul()
                    .children(rows.iter().map(|r| el::li().text(r)))
                    .render(cx);
            })
        };
        let cx = PageCx::new(Preset::Slate);
        let document = cx.title("Rows").body(body()).render();
        let page = Page::new(cx.bundle(), "Rows").body(body()).render();
        assert_eq!(document, page);
        assert!(document.contains(".note { color: red }"));
        assert!(document.contains("<li>&lt;second&gt;</li>"));
    }

    #[tokio::test]
    async fn respond_renders_only_what_is_asked_for() {
        async fn index(page: PageCx) -> Response {
            page.respond()
                .fragment("count", || el::p().id("count").text("3"))
                .fragment("never", || -> el::Element<'static> {
                    panic!("only the requested fragment renders")
                })
                .page(|| {
                    page.title("Counter")
                        .body(el::main().id("main").text("page"))
                })
        }
        let app = Router::new().route("/", get(index)).stucco(Preset::Slate);
        let request = |target: Option<&str>| {
            let mut req = Request::builder().uri("/");
            if let Some(target) = target {
                req = req
                    .header(stucco_core::behavior::HEADER_REQUEST, "fragment")
                    .header(stucco_core::behavior::HEADER_TARGET, target);
            }
            req.body(Body::empty()).unwrap()
        };
        let text = |res: Response| async move {
            String::from_utf8(res.into_body().collect().await.unwrap().to_bytes().to_vec()).unwrap()
        };
        let full = text(app.clone().oneshot(request(None)).await.unwrap()).await;
        assert!(full.contains("<main id=\"main\">page</main>"));
        let part = text(app.clone().oneshot(request(Some("count"))).await.unwrap()).await;
        assert!(part.contains("<p id=\"count\">3</p>"));
        assert!(!part.contains("<main"));
        let other = text(app.oneshot(request(Some("missing"))).await.unwrap()).await;
        assert!(other.contains("<main id=\"main\">page</main>"));
    }

    #[tokio::test]
    async fn documents_carry_a_status_and_a_missing_bundle_is_explained() {
        async fn gone(page: PageCx) -> Document {
            page.title("Gone").status(StatusCode::GONE)
        }
        let app = Router::new().route("/", get(gone));
        let (status, text) = get_text(app.clone(), "/").await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(text.contains(".stucco("));
        let (status, _) = get_text(app.stucco(Preset::Slate), "/").await;
        assert_eq!(status, StatusCode::GONE);
    }
}
