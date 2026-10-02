use super::*;
use crate::Tone;
use stucco_core::to_html;

#[test]
fn a_dialog_is_a_labelled_native_dialog_with_command_buttons() {
    let dialog = Dialog::new("confirm", "Delete order?")
        .child("This can't be undone.")
        .actions(Dialog::close_button("confirm", "Cancel"));
    let html = to_html(&dialog);
    assert!(html.starts_with(
        r#"<dialog id="confirm" class="st-dialog" aria-labelledby="confirm-title" closedby="any">"#
    ));
    assert!(html.contains(r#"<h2 id="confirm-title""#));
    assert!(html.contains(r#"commandfor="confirm" command="close" aria-label="Close""#));
    assert!(html.contains(r#"<div class="st-dialog-actions"><button"#));
    let opener = to_html(&dialog.opener("Delete"));
    assert!(opener.contains(r#"type="button""#));
    assert!(opener.contains(r#"commandfor="confirm" command="show-modal""#));
    let link = to_html(&dialog.link_opener("Delete", "/orders/7/delete"));
    assert!(
        link.contains(r#"href="/orders/7/delete""#) && link.contains(r#"data-st-opens="confirm""#)
    );
    assert_eq!(dialog.element_id(), "confirm");
}

#[test]
fn a_menu_is_a_popover_list_behind_its_button() {
    let html = to_html(
        &Menu::new("Actions")
            .popover_id("row-menu")
            .link("Edit", "/edit")
            .separator()
            .item(stucco_core::el::form().attr("method", "post").text("x")),
    );
    assert!(html.contains(r#"popovertarget="row-menu""#));
    assert!(html.contains(r#"<div id="row-menu" class="st-menu-popover" popover="auto">"#));
    assert!(html.contains(r#"<a class="st-menu-item" href="/edit">Edit</a>"#));
    assert!(html.contains(r#"<li class="st-menu-separator" aria-hidden="true"></li>"#));
    assert!(!html.contains(r#"role="menu""#));
    // Generated ids differ between menus on one page.
    let two = to_html(&(
        Menu::new("A").link("a", "/a"),
        Menu::new("B").link("b", "/b"),
    ));
    let ids: Vec<&str> = two.matches("popovertarget=\"").collect();
    assert_eq!(ids.len(), 2);
    assert!(two.contains(r#"popovertarget="menu-1""#) && two.contains(r#"popovertarget="menu-2""#));
}

#[test]
fn toasts_and_tooltips_carry_their_roles() {
    let toast = to_html(&Toast::new("Saved <ok>").tone(Tone::Success));
    assert!(toast.contains(r#"role="status" data-tone="success""#));
    assert!(toast.contains("Saved &lt;ok&gt;"));
    assert!(toast.contains("data-st-dismiss"));
    let tip = to_html(&Tooltip::new(stucco_core::el::button().text("x"), "Delete"));
    assert!(tip.contains(r#"<span class="st-tooltip" aria-hidden="true">Delete</span>"#));
}
