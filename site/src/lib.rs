//! Generates the stucco documentation site: the landing page, the guide, the
//! theme playground, the component gallery and a 404 page. Built with stucco
//! itself.

mod guide;
mod landing;
mod playground;
mod shell;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use stucco::{Asset, el, register_asset};

pub use shell::{Section, Site};

/// The site's own layout and prose styles.
pub static SITE: Asset = Asset {
    name: "site",
    css: Some(include_str!("../css/site.css")),
    behavior: None,
    deps: &[],
};
register_asset!(SITE);

/// What to build.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// The URL path the site is served under, such as `/stucco/`.
    pub base: String,
    /// The compiled theme engine to copy to `assets/site_wasm.wasm`. Without
    /// it, the playground and random themes report that the engine is
    /// missing.
    pub wasm: Option<PathBuf>,
}

/// Builds the site into `out` and returns the paths written, relative to
/// `out`.
pub fn build(out: &Path, options: &Options) -> io::Result<Vec<String>> {
    let site = Site::new(&options.base);
    let mut files: Vec<(String, Vec<u8>)> = vec![
        ("index.html".into(), landing::page(&site).into_bytes()),
        (
            "playground.html".into(),
            playground::page(&site).into_bytes(),
        ),
        ("404.html".into(), not_found(&site).into_bytes()),
        (
            "assets/site.js".into(),
            include_bytes!("../assets/site.js").to_vec(),
        ),
        (
            "assets/favicon.svg".into(),
            include_bytes!("../assets/favicon.svg").to_vec(),
        ),
    ];
    for (path, html) in guide::pages(&site) {
        files.push((path, html.into_bytes()));
    }
    // The fixtures exist for the browser tests, not for readers.
    for (name, html) in gallery::pages(site.bundle())
        .into_iter()
        .filter(|(name, _)| !name.starts_with("fixtures/"))
    {
        files.push((
            format!("gallery/{name}"),
            site.with_chrome(&html).into_bytes(),
        ));
    }
    for path in site.bundle().paths() {
        let file = site.bundle().get(&path).expect("listed paths are served");
        let relative = path
            .strip_prefix(site.base())
            .expect("bundle paths sit under the site base");
        files.push((relative.to_owned(), file.bytes.to_vec()));
    }
    if let Some(wasm) = &options.wasm {
        files.push(("assets/site_wasm.wasm".into(), fs::read(wasm)?));
    }
    // GitHub Pages must not run Jekyll over the output.
    files.push((".nojekyll".into(), Vec::new()));
    let mut written = Vec::with_capacity(files.len());
    for (path, bytes) in files {
        let target = out.join(&path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target, bytes)?;
        written.push(path);
    }
    Ok(written)
}

fn not_found(site: &Site) -> String {
    site.page(
        "Page not found",
        "This page does not exist.",
        Section::Other,
        None,
        el::div()
            .class("site-prose")
            .child(el::h1().text("Page not found"))
            .child(el::p().text("The page you asked for does not exist. It may have moved."))
            .child(
                el::p()
                    .child(el::a().href(site.url("")).text("Go to the home page"))
                    .text(" or ")
                    .child(el::a().href(site.url("guide/")).text("read the guide"))
                    .text("."),
            ),
    )
}
