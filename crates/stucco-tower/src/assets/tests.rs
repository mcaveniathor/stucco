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
