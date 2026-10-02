//! Builds the whole site and checks that every internal link and asset
//! reference resolves to a written file.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn out_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("stucco-site-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// Every `href="…"` and `src="…"` value in `html`.
fn references(html: &str) -> Vec<String> {
    let mut out = Vec::new();
    for attr in ["href=\"", "src=\""] {
        let mut rest = html;
        while let Some(i) = rest.find(attr) {
            rest = &rest[i + attr.len()..];
            let end = rest.find('"').expect("attribute values close");
            out.push(rest[..end].replace("&amp;", "&"));
            rest = &rest[end..];
        }
    }
    out
}

/// The file a site URL path resolves to, as a static host would serve it.
fn resolve(out: &Path, page: &Path, url: &str, base: &str) -> Option<PathBuf> {
    let path = url.split(['#', '?']).next().unwrap_or_default();
    if path.is_empty() {
        return None;
    }
    let file = if let Some(rooted) = path.strip_prefix(base) {
        out.join(rooted)
    } else if path.starts_with('/') {
        panic!("{url} on {} escapes the site base {base}", page.display());
    } else {
        page.parent().expect("pages sit in a directory").join(path)
    };
    Some(if path.ends_with('/') {
        file.join("index.html")
    } else {
        file
    })
}

#[test]
fn every_page_builds_and_every_internal_link_resolves() {
    let out = out_dir();
    let base = "/stucco/";
    let files = site::build(
        &out,
        &site::Options {
            base: base.into(),
            wasm: None,
        },
    )
    .expect("the site builds");
    for page in [
        "index.html",
        "playground.html",
        "404.html",
        "guide/index.html",
        "guide/theming.html",
        "gallery/index.html",
        "gallery/themes.html",
        "assets/site.js",
        ".nojekyll",
    ] {
        assert!(files.iter().any(|f| f == page), "{page} was not written");
    }
    assert!(!files.iter().any(|f| f.starts_with("gallery/fixtures/")));

    let mut missing = BTreeSet::new();
    for file in files.iter().filter(|f| f.ends_with(".html")) {
        let page = out.join(file);
        let html = fs::read_to_string(&page).unwrap();
        assert!(
            html.contains("assets/site.js"),
            "{file} lacks the site script"
        );
        assert!(
            html.contains("data-site-theme=\"menu\""),
            "{file} lacks the theme menu"
        );
        assert!(
            html.contains(r#"aria-label="Site""#),
            "{file} lacks the site navigation"
        );
        if file.starts_with("gallery/") {
            // The site header follows a skip link.
            assert!(
                html.contains(
                    r##"<body><a class="st-skip-link" href="#main">Skip to main content</a><div class="site-bar""##
                ),
                "{file} lacks the site header after its skip link"
            );
        }
        for url in references(&html) {
            if url.starts_with("http") || url.starts_with("mailto:") {
                continue;
            }
            // The API reference and the engine are added by the deploy workflow.
            if url.starts_with(&format!("{base}api/")) || url.ends_with("site_wasm.wasm") {
                continue;
            }
            if let Some(target) = resolve(&out, &page, &url, base) {
                if !target.exists() {
                    missing.insert(format!("{file} -> {url}"));
                }
            }
        }
    }
    assert!(missing.is_empty(), "broken references:\n{missing:#?}");
    let _ = fs::remove_dir_all(&out);
}

#[test]
fn site_css_obeys_the_component_rules() {
    stucco::check_component_css("site", site::SITE.css.unwrap()).unwrap();
}

#[test]
fn the_wasm_engine_is_copied_when_given() {
    let out = out_dir().with_extension("wasm");
    let fake = out.with_extension("bin");
    fs::create_dir_all(fake.parent().unwrap()).unwrap();
    fs::write(&fake, b"\0asm").unwrap();
    site::build(
        &out,
        &site::Options {
            base: "/".into(),
            wasm: Some(fake.clone()),
        },
    )
    .unwrap();
    assert_eq!(
        fs::read(out.join("assets/site_wasm.wasm")).unwrap(),
        b"\0asm"
    );
    let _ = fs::remove_dir_all(&out);
    let _ = fs::remove_file(&fake);
}
