#[test]
fn every_ui_stylesheet_obeys_the_component_rules() {
    for asset in stucco_ui::ui_assets() {
        stucco_core::check_component_css(asset.name, asset.css.unwrap_or("")).unwrap();
    }
}
