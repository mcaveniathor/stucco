//! axum integration: a router serving a bundle's files.

use std::sync::Arc;

use stucco_core::Bundle;

use crate::AssetService;

/// A router serving `bundle`'s files under its prefix (by default
/// `/_stucco/{*file}`); merge it into your app's router.
///
/// ```
/// use std::sync::Arc;
/// use stucco_core::Bundle;
/// use stucco_tower::assets_router;
///
/// let bundle = Arc::new(Bundle::new(stucco_theme::Preset::Slate));
/// let app: axum::Router = axum::Router::new().merge(assets_router(bundle));
/// ```
pub fn assets_router(bundle: Arc<Bundle>) -> axum::Router {
    let route = format!("{}{{*file}}", bundle.url_prefix());
    axum::Router::new().route_service(&route, AssetService::new(bundle))
}
