//! Tower integration for stucco: serve `Bundle` assets, return pages and
//! fragments, negotiate between them, and apply the standard middleware.
//!
//! The component crates stay free of HTTP; this crate is where stucco meets
//! a server. The `axum` and `tower-http` features (on by default) add axum
//! integration and the standard layer stack.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(feature = "axum")]
mod app;
mod assets;
#[cfg(feature = "axum")]
mod axum_support;
mod collection;
#[cfg(all(feature = "axum", feature = "tower-http"))]
mod layers;
mod negotiate;
mod response;
#[cfg(feature = "axum")]
mod submission;

#[cfg(all(feature = "axum", feature = "tower-http"))]
pub use app::StuccoRouter;
#[cfg(feature = "axum")]
pub use app::{Document, IntoBundle, MissingBundle, PageCx, Respond};
pub use assets::{AssetService, FallbackFuture, WithFallback};
#[cfg(feature = "axum")]
pub use axum_support::assets_router;
pub use collection::{CollectionSource, RequestContext, SourceError};
#[cfg(all(feature = "axum", feature = "tower-http"))]
pub use layers::{LayerConfig, with_standard_layers};
pub use negotiate::{RequestKind, respond};
pub use response::{FragmentResponse, PageResponse, SeeOther};
#[cfg(feature = "axum")]
pub use submission::{Submission, SubmissionRejection};
