//! The bundle: every stylesheet and script, content-hashed and servable
//! (spec §4.7).

use std::collections::HashMap;
use std::sync::Arc;

use stucco_theme::BuiltTheme;

use stucco_theme::Scope;

use crate::behavior::js_placeholders;
use crate::hash::short_hash;
use crate::{
    Asset, AssetRequirements, BASE_CSS, Behavior, LAYERS_CSS, RESET_CSS, registered_assets,
};

/// A file served by the bundle.
#[derive(Clone, Debug)]
pub struct AssetFile {
    /// File contents.
    pub bytes: Arc<[u8]>,
    /// `Content-Type` value.
    pub mime: &'static str,
    /// Quoted `ETag` value.
    pub etag: String,
    /// Whether the URL is content-addressed (cache forever).
    pub immutable: bool,
}

/// Every stylesheet and script a site needs, built once at startup.
#[derive(Clone, Debug)]
pub struct Bundle {
    pub(crate) root: BuiltTheme,
    named: Vec<(String, BuiltTheme)>,
    prefix: String,
    css: String,
    stylesheet_url: String,
    runtime_url: String,
    /// `(asset address, url)` for registered behaviour modules.
    scripts: Vec<(usize, String)>,
    /// File name (without prefix) → file.
    files: HashMap<String, AssetFile>,
}

const CSS_MIME: &str = "text/css; charset=utf-8";
const JS_MIME: &str = "text/javascript; charset=utf-8";
const RUNTIME_JS: &str = include_str!("../js/runtime.js");

impl Bundle {
    /// A bundle whose root theme is `theme`: a preset, a built theme, or a
    /// [`Theme`](stucco_theme::Theme), which is built here.
    ///
    /// # Panics
    ///
    /// If `theme` is a `Theme` that fails its contrast checks; use
    /// [`Bundle::try_new`] to handle that.
    pub fn new(theme: impl Into<BuiltTheme>) -> Bundle {
        let mut b = Bundle {
            root: theme.into(),
            named: Vec::new(),
            prefix: "/_stucco/".to_owned(),
            css: String::new(),
            stylesheet_url: String::new(),
            runtime_url: String::new(),
            scripts: Vec::new(),
            files: HashMap::new(),
        };
        b.build();
        b
    }

    /// A bundle for `theme`, or the report of the contrast checks it fails.
    ///
    /// ```
    /// use stucco_core::Bundle;
    /// use stucco_theme::Theme;
    ///
    /// assert!(Bundle::try_new(Theme::seeded(7)).is_ok());
    /// assert!(Bundle::try_new(Theme::seeded(7).min_contrast(30.0)).is_err());
    /// ```
    pub fn try_new(theme: stucco_theme::Theme) -> Result<Bundle, stucco_theme::ContrastReport> {
        theme.build().map(Bundle::new)
    }

    /// Adds a named theme, applied with `data-st-theme="<name>"`.
    ///
    /// `name` must match `[a-z0-9-]+`; an invalid name panics in debug builds
    /// and is skipped in release builds.
    pub fn with_theme(mut self, name: &str, theme: impl Into<BuiltTheme>) -> Bundle {
        let valid = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        debug_assert!(valid, "invalid theme name: {name:?} (use [a-z0-9-]+)");
        if !valid {
            return self;
        }
        self.named.push((name.to_owned(), theme.into()));
        self.build();
        self
    }

    /// Sets the URL prefix (default `/_stucco/`); normalised to leading and
    /// trailing `/`. Must be same-origin for fragments.
    pub fn prefix(mut self, prefix: &str) -> Bundle {
        let trimmed = prefix.trim_matches('/');
        self.prefix = if trimmed.is_empty() {
            "/".to_owned()
        } else {
            format!("/{trimmed}/")
        };
        self.build();
        self
    }

    /// The normalised URL prefix assets are served under (default `/_stucco/`).
    pub fn url_prefix(&self) -> &str {
        &self.prefix
    }

    /// URL of the site-wide stylesheet.
    pub fn stylesheet_url(&self) -> &str {
        &self.stylesheet_url
    }

    /// URL of the client runtime module.
    pub fn runtime_url(&self) -> &str {
        &self.runtime_url
    }

    /// URL of `asset`'s behaviour module; `None` if it has no script or was
    /// not registered.
    pub fn script_url(&self, asset: &Asset) -> Option<&str> {
        let addr = asset as *const Asset as usize;
        self.scripts
            .iter()
            .find(|(a, _)| *a == addr)
            .map(|(_, url)| url.as_str())
    }

    /// The site-wide stylesheet.
    pub fn css(&self) -> &str {
        &self.css
    }

    /// A stylesheet with the base layers, all theme tokens and only the CSS
    /// of `required` assets (for inline delivery).
    pub fn css_for(&self, required: &AssetRequirements) -> String {
        self.stylesheet(required.iter())
    }

    /// Every served URL path, sorted.
    pub fn paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self
            .files
            .keys()
            .map(|name| format!("{}{name}", self.prefix))
            .collect();
        paths.sort();
        paths
    }

    /// The file at request path `path` (a `?query` is ignored).
    pub fn get(&self, path: &str) -> Option<AssetFile> {
        let path = path.split('?').next().unwrap_or(path);
        let name = path.strip_prefix(self.prefix.as_str())?;
        self.files.get(name).cloned()
    }

    fn stylesheet<'a>(&self, assets: impl Iterator<Item = &'a Asset>) -> String {
        let mut css = String::new();
        css.push_str(LAYERS_CSS);
        css.push_str(RESET_CSS);
        css.push_str(&self.root.css(Scope::Root));
        for (name, theme) in &self.named {
            css.push_str(&theme.css(Scope::Named(name)));
        }
        css.push_str(BASE_CSS);
        for asset in assets {
            if let Some(chunk) = asset.css {
                css.push_str(chunk);
                css.push('\n');
            }
        }
        css
    }

    fn build(&mut self) {
        self.files.clear();
        self.scripts.clear();
        let registered = registered_assets();
        self.css = self.stylesheet(registered.iter().copied());
        self.stylesheet_url = self.add("stucco", "css", self.css.clone(), CSS_MIME);
        self.runtime_url = self.add(
            "stucco-runtime",
            "js",
            substitute_placeholders(RUNTIME_JS),
            JS_MIME,
        );
        for asset in registered {
            if let Some(Behavior::Js(js)) = asset.behavior {
                let url = self.add(asset.name, "js", substitute_placeholders(js), JS_MIME);
                self.scripts.push((asset as *const Asset as usize, url));
            }
        }
    }

    /// Stores `content` as `<stem>.<hash>.<ext>` and returns its URL.
    fn add(&mut self, stem: &str, ext: &str, content: String, mime: &'static str) -> String {
        let hash = short_hash(content.as_bytes());
        let name = format!("{stem}.{hash}.{ext}");
        self.files.insert(
            name.clone(),
            AssetFile {
                bytes: Arc::from(content.into_bytes()),
                mime,
                etag: format!("\"{hash}\""),
                immutable: true,
            },
        );
        format!("{}{name}", self.prefix)
    }
}

/// Replaces every `__STUCCO_*__` placeholder with its contract value.
pub(crate) fn substitute_placeholders(js: &str) -> String {
    let mut out = js.to_owned();
    for (placeholder, value) in js_placeholders() {
        out = out.replace(placeholder, value);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_theme::Preset;

    #[test]
    fn stylesheet_order() {
        let b = Bundle::new(Preset::Slate).with_theme("brand", Preset::Iris);
        let at = |s: &str| b.css().find(s).unwrap_or_else(|| panic!("missing {s}"));
        assert!(at("@layer stucco.reset, stucco.tokens") < at(":root { color-scheme"));
        assert!(at(":root { color-scheme") < at("[data-st-theme=\"brand\"]"));
        assert!(at("[data-st-theme=\"brand\"]") < at("@layer stucco.base"));
    }

    #[test]
    fn serves_hashed_files_ignoring_queries() {
        let b = Bundle::new(Preset::Slate);
        let url = b.stylesheet_url().to_owned();
        assert!(url.starts_with("/_stucco/stucco.") && url.ends_with(".css"));
        let f = b.get(&url).unwrap();
        assert_eq!((f.mime, f.immutable), ("text/css; charset=utf-8", true));
        assert_eq!(&*f.bytes, b.css().as_bytes());
        assert!(b.get(&format!("{url}?v=1")).is_some());
        assert_eq!(
            b.get(b.runtime_url()).unwrap().mime,
            "text/javascript; charset=utf-8"
        );
    }

    #[test]
    fn rejects_unknown_paths() {
        let b = Bundle::new(Preset::Slate).prefix("/assets");
        let name = b.stylesheet_url().rsplit('/').next().unwrap().to_owned();
        assert!(b.stylesheet_url().starts_with("/assets/"));
        for bad in [
            name.clone(),
            format!("/_stucco/{name}"),
            format!("/assets/../{name}"),
            "/assets/stucco.0000000000.css".to_owned(),
            "/assets/".to_owned(),
        ] {
            assert!(b.get(&bad).is_none(), "{bad}");
        }
    }

    #[test]
    #[cfg(debug_assertions)]
    #[should_panic(expected = "invalid theme name")]
    fn theme_names_are_validated() {
        let _ = Bundle::new(Preset::Slate).with_theme("x\"] { } body", Preset::Iris);
    }

    #[test]
    fn url_prefix_reports_the_normalised_prefix() {
        assert_eq!(Bundle::new(Preset::Slate).url_prefix(), "/_stucco/");
        assert_eq!(
            Bundle::new(Preset::Slate).prefix("/assets").url_prefix(),
            "/assets/"
        );
    }

    #[test]
    fn paths_list_every_served_file() {
        let b = Bundle::new(Preset::Slate);
        let paths = b.paths();
        assert!(paths.iter().any(|p| p == b.stylesheet_url()));
        assert!(paths.iter().any(|p| p == b.runtime_url()));
        assert!(paths.iter().all(|p| b.get(p).is_some()));
    }

    #[test]
    fn theme_changes_change_the_url() {
        assert_ne!(
            Bundle::new(Preset::Slate).stylesheet_url(),
            Bundle::new(Preset::Iris).stylesheet_url()
        );
    }

    static STYLED: Asset = Asset {
        name: "styled",
        css: Some("@layer stucco.components { .st-styled { color: var(--st-text); } }"),
        behavior: Some(crate::Behavior::Js(
            "export const tag = '__STUCCO_REQUIRE_TAG__';",
        )),
        deps: &[],
    };
    crate::register_asset!(STYLED);
    static UNREGISTERED: Asset = Asset {
        name: "unregistered",
        css: Some("@layer stucco.components { .st-unreg { color: var(--st-text); } }"),
        behavior: None,
        deps: &[],
    };

    #[test]
    fn registered_assets_are_bundled_and_substituted() {
        let b = Bundle::new(Preset::Slate);
        assert!(b.css().contains(".st-styled"));
        let url = b.script_url(&STYLED).unwrap();
        assert!(url.starts_with("/_stucco/styled.") && url.ends_with(".js"));
        let js = b.get(url).unwrap();
        assert_eq!(&*js.bytes, b"export const tag = 'st-require';");
        assert!(b.script_url(&UNREGISTERED).is_none());
    }

    #[test]
    fn css_for_includes_only_required_assets() {
        let b = Bundle::new(Preset::Slate);
        let mut cx = crate::Cx::new();
        cx.require(&UNREGISTERED);
        let css = b.css_for(&cx.finish().1);
        assert!(css.contains(".st-unreg") && !css.contains(".st-styled"));
        assert!(css.contains("@layer stucco.base"));
    }

    #[test]
    fn a_theme_is_built_into_a_bundle() {
        let seeded = stucco_theme::Theme::seeded(3);
        let built = Bundle::new(seeded.clone().build().unwrap());
        assert_eq!(Bundle::new(seeded.clone()).css(), built.css());
        assert_eq!(Bundle::try_new(seeded).unwrap().css(), built.css());
        assert!(Bundle::try_new(stucco_theme::Theme::seeded(3).min_contrast(30.0)).is_err());
    }

    #[test]
    #[should_panic(expected = "fails its contrast checks")]
    fn a_failing_theme_panics_with_its_report() {
        let _ = Bundle::new(stucco_theme::Theme::seeded(3).min_contrast(30.0));
    }
}
