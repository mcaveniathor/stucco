use http::Request;
use std::future::Future;
use stucco_core::{Capabilities, CollectionPage, CollectionQuery};
use stucco_tower::{CollectionSource, RequestContext, SourceError};
struct Source;
impl CollectionSource for Source {
    type Row = u64;
    fn capabilities(&self) -> Capabilities {
        Capabilities::default()
    }
    fn query(
        &self,
        _: &CollectionQuery,
        _: &RequestContext,
    ) -> impl Future<Output = Result<CollectionPage<u64>, SourceError>> + Send {
        async {
            Ok(CollectionPage {
                rows: vec![1, 2],
                next: None,
                prev: None,
                total: Some(2),
            })
        }
    }
}
#[tokio::test]
async fn context_preserves_typed_extensions_and_query_future_is_send() {
    let mut req = Request::builder()
        .header("x-request-id", "test-42")
        .body(())
        .unwrap();
    req.extensions_mut().insert(17_u64);
    let (parts, _) = req.into_parts();
    let cx = RequestContext::from_parts(&parts);
    assert_eq!(cx.request_id, "test-42");
    assert_eq!(cx.get::<u64>(), Some(&17));
    assert_eq!(parts.extensions.get::<u64>(), Some(&17));
    fn assert_send<T: Send>(_: T) {}
    let q = CollectionQuery::default();
    assert_send(Source.query(&q, &cx));
    assert_eq!(Source.query(&q, &cx).await.unwrap().rows, [1, 2]);
}
