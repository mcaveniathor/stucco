use stucco_core::{BASE_CSS, LAYERS_CSS, RESET_CSS, check_component_css};

#[test]
fn base_layers_obey_the_rules() {
    check_component_css("reset", RESET_CSS).unwrap();
    check_component_css("base", BASE_CSS).unwrap();
}

#[test]
fn layer_order_is_declared_once() {
    assert_eq!(
        LAYERS_CSS.trim(),
        "@layer stucco.reset, stucco.tokens, stucco.base, stucco.layout, stucco.components, stucco.utilities;"
    );
}

#[test]
fn the_checker_rejects_violations() {
    assert!(check_component_css("x", "@layer stucco.components { .a { color: #fff; } }").is_err());
    assert!(
        check_component_css(
            "x",
            "@layer stucco.components { .a { color: rgb(0 0 0); } }"
        )
        .is_err()
    );
    assert!(
        check_component_css(
            "x",
            "@layer stucco.components { .a { color: var(--st-accent-9); } }"
        )
        .is_err()
    );
    assert!(check_component_css("x", ".a { color: var(--st-text); }").is_err());
    assert!(check_component_css("x", "@layer other { .a { color: var(--st-text); } }").is_err());
    assert!(
        check_component_css(
            "x",
            "@layer stucco.components { .a { color: var(--st-text); } }"
        )
        .is_ok()
    );
}

#[test]
fn ids_and_non_colour_hashes_are_not_colours() {
    assert!(
        check_component_css(
            "x",
            "@layer stucco.components { #main { color: var(--st-text); } .a[href='#top'] { x: 1 } }"
        )
        .is_ok()
    );
}
