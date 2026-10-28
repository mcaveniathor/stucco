//! Status and detail displays: [`Badge`] and [`Avatar`].

use leptos::prelude::*;

use crate::href::safe_href;
use crate::vocab::{Size, Tone};

/// A short status label with a tone.
///
/// Choose a label that says what the status means; the tone only reinforces it.
#[component]
pub fn Badge(
    /// The status colour.
    #[prop(optional)]
    tone: Tone,
    /// The label.
    children: Children,
) -> impl IntoView {
    view! { <span class="st-badge" data-tone=tone.as_str()>{children()}</span> }
}

/// A person's image, or their initials.
///
/// The initials always render underneath, so a failed image still identifies the
/// person. The name labels the avatar unless it is `decorative`, which hides it
/// from assistive technology; use that when the same name is written beside it.
/// `Xs` and `Sm` render small, `Md` medium, and `Lg` and `Xl` large. The image
/// URL is checked like any link.
#[component]
pub fn Avatar(
    /// The person's name, used for initials and the accessible label.
    #[prop(into)]
    name: String,
    /// Image URL shown over the initials.
    #[prop(optional, into)]
    image: Option<String>,
    /// The avatar's size.
    #[prop(optional)]
    size: Size,
    /// Hide the avatar from assistive technology.
    #[prop(optional)]
    decorative: bool,
) -> impl IntoView {
    let size = match size {
        Size::Xs | Size::Sm => "sm",
        Size::Md => "md",
        Size::Lg | Size::Xl => "lg",
    };
    let initials = initials(&name);
    let labelled = !decorative && image.is_none();
    let alt = if decorative {
        String::new()
    } else {
        name.clone()
    };
    let picture = image.map(|src| view! { <img src=safe_href(&src) alt=alt/> });
    view! {
        <span class="st-avatar" data-size=size
            aria-hidden=decorative.then_some("true")
            role=labelled.then_some("img")
            aria-label=labelled.then_some(name)>
            <span class="st-avatar-initials" aria-hidden="true">{initials}</span>
            {picture}
        </span>
    }
}

/// The first letter of the first and last words of `name`, uppercased.
fn initials(name: &str) -> String {
    let mut words = name.split_whitespace();
    let first = words.next().and_then(|word| word.chars().next());
    let last = words.last().and_then(|word| word.chars().next());
    first
        .into_iter()
        .chain(last)
        .filter_map(|initial| initial.to_uppercase().next())
        .collect()
}
