#[test]
fn every_js_placeholder_is_known_and_substituted() {
    let b = stucco_core::Bundle::new(stucco_theme::Preset::Slate);
    let runtime = String::from_utf8(b.get(b.runtime_url()).unwrap().bytes.to_vec()).unwrap();
    assert!(
        !runtime.contains("__STUCCO_"),
        "unsubstituted placeholder in runtime"
    );
    let source = include_str!("../js/runtime.js");
    for word in source.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
        if word.starts_with("__STUCCO_") {
            assert!(
                stucco_core::behavior::js_placeholders()
                    .iter()
                    .any(|(k, _)| *k == word),
                "unknown {word}"
            );
        }
    }
}
