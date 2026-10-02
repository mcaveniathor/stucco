#[test]
fn palette_page_shows_every_preset_in_both_schemes() {
    let html = gallery::palette::palette_page(&gallery::bundle());
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
