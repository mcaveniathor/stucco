#[test]
fn palette_page_shows_every_preset_in_both_schemes() {
    let bundle = gallery::bundle();
    // The stylesheet hash changes whenever any component CSS changes.
    let html = gallery::palette::palette_page(&bundle)
        .replace(bundle.stylesheet_url(), "/_stucco/stucco.HASH.css");
    for p in stucco::theme::Preset::ALL {
        assert!(
            html.contains(&format!("data-st-theme=\"{}\"", p.name())),
            "{}",
            p.name()
        );
    }
    assert_eq!(html.matches("data-theme=\"dark\"").count(), 14);
    insta::assert_snapshot!(html);
}

#[test]
fn enhanced_fixture_lacks_the_probe_module_that_fragments_require() {
    let dir = std::env::temp_dir().join(format!("stucco-gallery-{}", std::process::id()));
    gallery::write_site(&dir).unwrap();
    let page = std::fs::read_to_string(dir.join("fixtures/enhanced.html")).unwrap();
    let frag = std::fs::read_to_string(dir.join("fixtures/probe-a.html")).unwrap();
    assert!(frag.starts_with("<st-require modules=\"/_stucco/probe."));
    let probe_url = frag.split('"').nth(1).unwrap();
    assert!(!page.contains(probe_url));
    assert!(dir.join(probe_url.trim_start_matches('/')).exists());
    assert!(dir.join("index.html").exists() && dir.join("palette.html").exists());
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn gallery_css_obeys_the_component_rules() {
    stucco::check_component_css("gallery", gallery::GALLERY.css.unwrap()).unwrap();
}

fn page_snapshot(name: &str, html: String, bundle: &stucco::Bundle) {
    assert!(
        !html.contains("style="),
        "{name} uses an inline style attribute"
    );
    assert!(
        html.contains(r##"<a class="st-skip-link" href="#main">"##),
        "{name}"
    );
    assert!(html.contains(r#"<main id="main""#), "{name}");
    let html = html.replace(bundle.stylesheet_url(), "/_stucco/stucco.HASH.css");
    insta::assert_snapshot!(name, html);
}

#[test]
fn component_pages_render_every_family() {
    let bundle = gallery::bundle();
    page_snapshot("layout", gallery::layout_page::page(&bundle), &bundle);
    page_snapshot(
        "typography",
        gallery::typography_page::page(&bundle),
        &bundle,
    );
    page_snapshot("actions", gallery::actions_page::page(&bundle), &bundle);
    page_snapshot("forms", gallery::forms_page::page(&bundle), &bundle);
    page_snapshot("records", gallery::records_page::page(&bundle), &bundle);
    page_snapshot("overlays", gallery::overlays_page::page(&bundle), &bundle);
    for (file, html) in gallery::tabs_page::pages(&bundle) {
        page_snapshot(file.trim_end_matches(".html"), html, &bundle);
    }
    page_snapshot("themes", gallery::themes_page::page(&bundle), &bundle);
    page_snapshot(
        "collections",
        gallery::collections_page::page(&bundle),
        &bundle,
    );
    let app = gallery::collections_page::app_page(&bundle);
    assert_eq!(app.matches("<main ").count(), 1);
    assert!(app.contains("id=\"main\""));
    insta::assert_snapshot!(
        "app",
        app.replace(bundle.stylesheet_url(), "/_stucco/stucco.HASH.css")
    );
}

#[test]
fn the_forms_page_never_redisplays_the_submitted_password() {
    let html = gallery::forms_page::page(&gallery::bundle());
    assert!(!html.contains(gallery::forms_page::SUBMITTED_PASSWORD));
}

#[test]
fn index_links_every_page() {
    let html = gallery::index::index_page(&gallery::bundle());
    for page in [
        "palette.html",
        "themes.html",
        "layout.html",
        "typography.html",
        "actions.html",
        "forms.html",
        "overlays.html",
        "tabs.html",
        "collections.html",
        "app.html",
    ] {
        assert!(html.contains(&format!("href=\"{page}\"")), "{page}");
    }
}
