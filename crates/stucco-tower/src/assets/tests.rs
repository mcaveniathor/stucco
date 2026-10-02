use super::*;
use http_body_util::BodyExt;
use stucco_theme::Preset;
use tower::ServiceExt;

fn req(method: &str, uri: &str) -> Request<String> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(String::new())
        .unwrap()
}

fn req_with(method: &str, uri: &str, name: &str, value: &str) -> Request<String> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(name, value)
        .body(String::new())
        .unwrap()
}

async fn body<B: http_body::Body>(res: Response<B>) -> Vec<u8>
where
    B::Error: std::fmt::Debug,
{
    res.into_body().collect().await.unwrap().to_bytes().to_vec()
}

#[derive(Clone)]
struct Teapot;

impl<B> Service<Request<B>> for Teapot {
    type Response = Response<Full<Bytes>>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Infallible>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }
    fn call(&mut self, _: Request<B>) -> Self::Future {
        ready(Ok(Response::builder()
            .status(418)
            .body(Full::default())
            .unwrap()))
    }
}

struct NeverReady;

impl<B> Service<Request<B>> for NeverReady {
    type Response = Response<Full<Bytes>>;
    type Error = Infallible;
    type Future = Ready<Result<Self::Response, Infallible>>;
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Pending
    }
    fn call(&mut self, _: Request<B>) -> Self::Future {
        unreachable!("never ready")
    }
}

#[tokio::test]
async fn serves_hashed_assets_with_immutable_caching() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let url = bundle.stylesheet_url().to_owned();
    let res = AssetService::new(bundle.clone())
        .oneshot(req("GET", &format!("{url}?v=1")))
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    assert_eq!(res.headers()["content-type"], "text/css; charset=utf-8");
    assert_eq!(
        res.headers()["cache-control"],
        "public, max-age=31536000, immutable"
    );
    assert_eq!(res.headers()["x-content-type-options"], "nosniff");
    assert_eq!(body(res).await, bundle.css().as_bytes());
}

#[tokio::test]
async fn unknown_asset_head_preserves_get_headers_without_a_body() {
    let svc = AssetService::new(Arc::new(Bundle::new(Preset::Slate)));
    let get = svc
        .clone()
        .oneshot(req("GET", "/_stucco/nope.css"))
        .await
        .unwrap();
    let head = svc.oneshot(req("HEAD", "/_stucco/nope.css")).await.unwrap();
    assert_eq!(get.status(), 404);
    assert_eq!(head.status(), get.status());
    assert_eq!(head.headers(), get.headers());
    assert_eq!(body(get).await, b"not found");
    assert!(body(head).await.is_empty());
}

#[tokio::test]
async fn conditional_requests_return_304() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let url = bundle.stylesheet_url().to_owned();
    let etag = bundle.get(&url).unwrap().etag;
    for inm in [
        etag.clone(),
        format!("\"x\", {etag}"),
        format!("W/{etag}"),
        "*".to_owned(),
    ] {
        let res = AssetService::new(bundle.clone())
            .oneshot(req_with("GET", &url, "if-none-match", &inm))
            .await
            .unwrap();
        assert_eq!(res.status(), 304, "{inm}");
        assert_eq!(res.headers()["etag"], etag.as_str());
        assert_eq!(
            res.headers()["cache-control"],
            "public, max-age=31536000, immutable"
        );
        assert!(body(res).await.is_empty());
    }
    let res = AssetService::new(bundle)
        .oneshot(req_with("GET", &url, "if-none-match", "\"nope\""))
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn head_has_headers_but_no_body_and_other_methods_are_405() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let url = bundle.stylesheet_url().to_owned();
    let head = AssetService::new(bundle.clone())
        .oneshot(req("HEAD", &url))
        .await
        .unwrap();
    assert_eq!(head.status(), 200);
    assert_eq!(
        head.headers()["content-length"],
        bundle.css().len().to_string().as_str()
    );
    assert!(body(head).await.is_empty());
    let post = AssetService::new(bundle)
        .oneshot(req("POST", &url))
        .await
        .unwrap();
    assert_eq!(post.status(), 405);
    assert_eq!(post.headers()["allow"], "GET, HEAD");
}

#[tokio::test]
async fn odd_paths_under_the_prefix_are_404_and_never_fall_back() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let name = bundle
        .stylesheet_url()
        .trim_start_matches("/_stucco/")
        .to_owned();
    let encoded = name.replacen('.', "%2E", 1);
    for path in [
        format!("/_stucco/{encoded}"),
        format!("/_stucco/../{name}"),
        "/_stucco/nope.css".to_owned(),
    ] {
        let res = AssetService::new(bundle.clone())
            .fallback(Teapot)
            .oneshot(req("GET", &path))
            .await
            .unwrap();
        assert_eq!(res.status(), 404, "{path}");
    }
    let res = AssetService::new(bundle)
        .fallback(Teapot)
        .oneshot(req("GET", "/orders"))
        .await
        .unwrap();
    assert_eq!(res.status(), 418, "outside the prefix goes to the fallback");
}

#[tokio::test]
async fn fallback_readiness_is_delegated() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let mut svc = AssetService::new(bundle).fallback(NeverReady);
    let mut cx = Context::from_waker(std::task::Waker::noop());
    assert!(Service::<Request<String>>::poll_ready(&mut svc, &mut cx).is_pending());
}

#[tokio::test]
async fn a_root_prefix_still_reaches_the_fallback_for_non_asset_paths() {
    let bundle = Arc::new(Bundle::new(Preset::Slate).prefix("/"));
    let url = bundle.stylesheet_url().to_owned();
    assert!(url.starts_with("/stucco."));
    let css = AssetService::new(bundle.clone())
        .fallback(Teapot)
        .oneshot(req("GET", &url))
        .await
        .unwrap();
    assert_eq!(css.status(), 200);
    let app = AssetService::new(bundle)
        .fallback(Teapot)
        .oneshot(req("GET", "/orders"))
        .await
        .unwrap();
    assert_eq!(app.status(), 418);
}

#[tokio::test]
async fn a_custom_prefix_is_respected() {
    let bundle = Arc::new(Bundle::new(Preset::Slate).prefix("/assets"));
    let url = bundle.stylesheet_url().to_owned();
    let svc = AssetService::new(bundle).fallback(Teapot);
    assert_eq!(
        svc.clone()
            .oneshot(req("GET", &url))
            .await
            .unwrap()
            .status(),
        200
    );
    assert_eq!(
        svc.clone()
            .oneshot(req("GET", "/assets/nope.css"))
            .await
            .unwrap()
            .status(),
        404
    );
    assert_eq!(
        svc.oneshot(req("GET", "/_stucco/x"))
            .await
            .unwrap()
            .status(),
        418
    );
}

#[tokio::test]
async fn repeated_if_none_match_lines_are_one_list() {
    let bundle = Arc::new(Bundle::new(Preset::Slate));
    let url = bundle.stylesheet_url().to_owned();
    let etag = bundle.get(&url).unwrap().etag;
    let request = Request::builder()
        .uri(&url)
        .header("if-none-match", "\"other\"")
        .header("if-none-match", etag.as_str())
        .body(String::new())
        .unwrap();
    let res = AssetService::new(bundle).oneshot(request).await.unwrap();
    assert_eq!(res.status(), 304);
}
