//! Status and detail displays: [`Badge`], [`Avatar`], [`DescriptionList`] and [`Stat`].

use leptos::prelude::*;

use crate::href::safe_href;
use crate::vocab::{Size, Tone, Trend};

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

/// Record fields as responsive term and detail pairs, in a `<dl>`.
///
/// Terms stack above details in narrow containers and sit beside them from 32rem
/// of container width.
#[component]
pub fn DescriptionList(
    /// The fields, as [`DescriptionItem`]s.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! { <dl class="st-dl">{children.map(|fields| fields())}</dl> }
}

/// One field of a [`DescriptionList`]; the details can be any view.
#[component]
pub fn DescriptionItem(
    /// The field's name.
    #[prop(into)]
    term: String,
    /// The field's value.
    children: Children,
) -> impl IntoView {
    view! { <div class="st-dl-item"><dt>{term}</dt><dd>{children()}</dd></div> }
}

/// How a [`Stat`]'s value changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatChange {
    /// The change as formatted text, such as `"12%"`.
    pub text: String,
    /// Which way it moved.
    pub trend: Trend,
}

/// A metric with an optional change and comparison.
///
/// The change stays neutral until you set `change_tone`, because a rising number
/// can be good or bad. The direction is written in visually hidden text as well
/// as the arrow; translate it with `up_label`, `down_label` and `flat_label`.
#[component]
pub fn Stat(
    /// What is measured.
    #[prop(into)]
    label: Signal<String>,
    /// The formatted value.
    #[prop(into)]
    value: Signal<String>,
    /// How the value changed, if known.
    #[prop(default = Signal::derive(|| None), into)]
    change: Signal<Option<StatChange>>,
    /// The tone of the change.
    #[prop(optional)]
    change_tone: Tone,
    /// What the change is compared with, such as `"vs last week"`.
    #[prop(optional, into)]
    context: Option<String>,
    /// Hidden text announcing an increase.
    #[prop(default = "Up ".to_owned(), into)]
    up_label: String,
    /// Hidden text announcing a decrease.
    #[prop(default = "Down ".to_owned(), into)]
    down_label: String,
    /// Hidden text announcing no change.
    #[prop(default = "No change ".to_owned(), into)]
    flat_label: String,
) -> impl IntoView {
    let change = move || {
        change.get().map(|StatChange { text, trend }| {
            let said = match trend {
                Trend::Up => up_label.clone(),
                Trend::Down => down_label.clone(),
                Trend::Flat => flat_label.clone(),
            };
            let context = context.as_ref().map(|context| format!(" {context}"));
            view! {
                <p class="st-stat-change">
                    <span class="st-stat-delta" data-tone=change_tone.as_str()
                        data-trend=trend.as_str()>
                        <TrendArrow trend=trend/>
                        <span class="st-sr-only">{said}</span>
                        {text}
                    </span>
                    {context}
                </p>
            }
        })
    };
    view! {
        <div class="st-stat">
            <p class="st-stat-label">{move || label.get()}</p>
            <p class="st-stat-value">{move || value.get()}</p>
            {change}
        </div>
    }
}

/// The trend arrows of Lucide's `arrow-up`, `arrow-down` and `minus` (ISC; see
/// LICENSE-LUCIDE), as the static `Stat` embeds them.
#[component]
fn TrendArrow(trend: Trend) -> impl IntoView {
    let shapes = match trend {
        Trend::Up => view! { <path d="m5 12 7-7 7 7"/><path d="M12 19V5"/> }.into_any(),
        Trend::Down => view! { <path d="M12 5v14"/><path d="m19 12-7 7-7-7"/> }.into_any(),
        Trend::Flat => view! { <path d="M5 12h14"/> }.into_any(),
    };
    view! {
        <svg class="st-icon" viewBox="0 0 24 24" width="1em" height="1em" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true" focusable="false">{shapes}</svg>
    }
}

/// A raised container with optional header and footer slots.
///
/// `header` and `footer` take a closure returning any view, for example
/// `header=|| view! { <h3>"Orders"</h3> }`.
#[component]
pub fn Card(
    /// Content above the body.
    #[prop(optional, into)]
    header: ViewFn,
    /// Content below the body.
    #[prop(optional, into)]
    footer: ViewFn,
    /// The card's body.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! { <div class="st-card">{header.run()}{children.map(|body| body())}{footer.run()}</div> }
}

/// A titled section with optional description, actions, body and footer.
///
/// The section is labelled by its heading. Give it a page-unique `id`;
/// `{id}-title` is reserved for the heading. `level` is the heading level, 1 to 6
/// (values outside that range are clamped). `description`, `actions` and `footer`
/// take closures returning any view.
#[component]
pub fn Panel(
    /// Page-unique id; `{id}-title` is reserved.
    #[prop(into)]
    id: String,
    /// The panel's heading text.
    #[prop(into)]
    title: String,
    /// Heading level, 1 to 6.
    #[prop(default = 2)]
    level: u8,
    /// Text below the heading.
    #[prop(optional, into)]
    description: ViewFn,
    /// Controls beside the heading.
    #[prop(optional, into)]
    actions: ViewFn,
    /// The panel's body.
    #[prop(optional)]
    children: Option<Children>,
    /// Content below the body.
    #[prop(optional, into)]
    footer: ViewFn,
) -> impl IntoView {
    let title_id = format!("{id}-title");
    let heading = heading(level.clamp(1, 6), title_id.clone(), title);
    view! {
        <section class="st-panel" aria-labelledby=title_id>
            <div class="st-panel-header">
                {heading}
                {description.run()}
                {actions.run()}
            </div>
            {children.map(|body| body())}
            {footer.run()}
        </section>
    }
}

fn heading(level: u8, id: String, title: String) -> AnyView {
    match level {
        1 => view! { <h1 class="st-heading" data-size="lg" id=id>{title}</h1> }.into_any(),
        2 => view! { <h2 class="st-heading" data-size="lg" id=id>{title}</h2> }.into_any(),
        3 => view! { <h3 class="st-heading" data-size="lg" id=id>{title}</h3> }.into_any(),
        4 => view! { <h4 class="st-heading" data-size="lg" id=id>{title}</h4> }.into_any(),
        5 => view! { <h5 class="st-heading" data-size="lg" id=id>{title}</h5> }.into_any(),
        _ => view! { <h6 class="st-heading" data-size="lg" id=id>{title}</h6> }.into_any(),
    }
}
