//! The client contract shared by Rust and the JavaScript behaviours.
//!
//! JavaScript sources refer to these values as `__STUCCO_<NAME>__`
//! placeholders, substituted when the bundle is built; a test fails if a
//! script uses a placeholder that is not defined here.

/// `localStorage` key holding the user's theme choice.
pub const THEME_STORAGE_KEY: &str = "stucco-theme";
/// Attribute forcing a colour scheme on a subtree.
pub const THEME_ATTR: &str = "data-theme";
/// Attribute applying a named theme to a subtree.
pub const NAMED_THEME_ATTR: &str = "data-st-theme";
/// Element announcing the modules a fragment needs.
pub const REQUIRE_TAG: &str = "st-require";
/// Attribute marking an enhanced region (the unit that shows load failures).
pub const REGION_ATTR: &str = "data-st-region";
/// Attribute carrying behaviour state.
pub const STATE_ATTR: &str = "data-state";
/// Event dispatched when a required module fails to load.
pub const ASSET_ERROR_EVENT: &str = "stucco:asset-error";
/// Request header marking an enhanced (fragment) request.
pub const HEADER_REQUEST: &str = "Stucco-Request";
/// Request header naming the fragment's target element id.
pub const HEADER_TARGET: &str = "Stucco-Target";
/// Response header telling the runtime where to navigate.
pub const HEADER_LOCATION: &str = "Stucco-Location";
/// Request header carrying the CSRF token.
pub const HEADER_CSRF: &str = "Stucco-Csrf";

/// `(placeholder, value)` for every constant in this module.
pub fn js_placeholders() -> &'static [(&'static str, &'static str)] {
    &[
        ("__STUCCO_THEME_STORAGE_KEY__", THEME_STORAGE_KEY),
        ("__STUCCO_THEME_ATTR__", THEME_ATTR),
        ("__STUCCO_NAMED_THEME_ATTR__", NAMED_THEME_ATTR),
        ("__STUCCO_REQUIRE_TAG__", REQUIRE_TAG),
        ("__STUCCO_REGION_ATTR__", REGION_ATTR),
        ("__STUCCO_STATE_ATTR__", STATE_ATTR),
        ("__STUCCO_ASSET_ERROR_EVENT__", ASSET_ERROR_EVENT),
        ("__STUCCO_HEADER_REQUEST__", HEADER_REQUEST),
        ("__STUCCO_HEADER_TARGET__", HEADER_TARGET),
        ("__STUCCO_HEADER_LOCATION__", HEADER_LOCATION),
        ("__STUCCO_HEADER_CSRF__", HEADER_CSRF),
    ]
}
