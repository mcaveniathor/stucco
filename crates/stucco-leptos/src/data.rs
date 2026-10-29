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

/// Dated activity in order, such as an order's history.
///
/// Fill it with [`TimelineItem`]s. Each item's time is a semantic `<time>`, and
/// its content can be any view. With no children, or with `empty` set, it shows
/// `empty_label` instead. When the entries come from a list that may be empty, pass
/// `empty=entries.is_empty()`; for a list that changes after rendering, render
/// `<Timeline label=…/>` without children when it is empty.
#[component]
pub fn Timeline(
    /// Accessible name of the list.
    #[prop(into)]
    label: String,
    /// Text shown when there are no items.
    #[prop(default = "No activity yet".to_owned(), into)]
    empty_label: String,
    /// Show the empty item and ignore `children`, for a list known to be empty.
    #[prop(optional)]
    empty: bool,
    /// The entries, as [`TimelineItem`]s.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    let body = match children {
        Some(items) if !empty => items().into_any(),
        _ => view! { <li>{empty_label}</li> }.into_any(),
    };
    view! { <ol class="st-timeline" aria-label=label>{body}</ol> }
}

/// One entry of a [`Timeline`].
#[component]
pub fn TimelineItem(
    /// Machine-readable time, such as `2026-10-01T09:00`.
    #[prop(into)]
    datetime: String,
    /// The time as shown to people.
    #[prop(into)]
    time: String,
    /// What happened.
    children: Children,
) -> impl IntoView {
    view! {
        <li>
            <span class="st-timeline-mark" aria-hidden="true"></span>
            <div class="st-timeline-content">
                <time datetime=datetime>{time}</time>
                {children()}
            </div>
        </li>
    }
}

/// A small trend line over supplied samples, with the values also written as text.
///
/// Non-finite samples are ignored. No samples show `no_data_label`; one sample is
/// a dot; a constant series is a flat line. The line is decorative: `summary`
/// names the figure and the visually hidden list gives the values.
#[component]
pub fn Sparkline(
    /// What the line shows, used as the figure's caption.
    #[prop(into)]
    summary: Signal<String>,
    /// The samples, oldest first.
    #[prop(into)]
    values: Signal<Vec<f64>>,
    /// Text shown when there are no usable samples.
    #[prop(default = "No data".to_owned(), into)]
    no_data_label: String,
    /// Text introducing the hidden list of values.
    #[prop(default = "Values in order: ".to_owned(), into)]
    values_label: String,
) -> impl IntoView {
    let body = move || {
        let samples: Vec<f64> = values
            .get()
            .into_iter()
            .filter(|value| value.is_finite())
            .collect();
        if samples.is_empty() {
            return view! { <p>{no_data_label.clone()}</p> }.into_any();
        }
        let shape = if samples.len() == 1 {
            view! { <circle cx="100" cy="30" r="3"/> }.into_any()
        } else {
            view! {
                <polyline points=sparkline_points(&samples) fill="none" stroke="currentColor"
                    stroke-width="2" vector-effect="non-scaling-stroke"/>
            }
            .into_any()
        };
        let listed = samples
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        view! {
            <svg viewBox="0 0 200 60" aria-hidden="true" focusable="false">{shape}</svg>
            <p class="st-sr-only">{format!("{}{listed}", values_label)}</p>
        }
        .into_any()
    };
    view! {
        <figure class="st-sparkline">
            <figcaption>{move || summary.get()}</figcaption>
            {body}
        </figure>
    }
}

/// Polyline points for at least two finite samples in a 200 by 60 box.
///
/// Samples are divided by the largest magnitude before subtracting, so opposite
/// extrema of `f64` cannot overflow.
fn sparkline_points(samples: &[f64]) -> String {
    let scale = samples.iter().map(|v| v.abs()).fold(1.0, f64::max);
    let lo = samples
        .iter()
        .map(|v| v / scale)
        .fold(f64::INFINITY, f64::min);
    let hi = samples
        .iter()
        .map(|v| v / scale)
        .fold(f64::NEG_INFINITY, f64::max);
    samples
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let x = 4.0 + 192.0 * i as f64 / (samples.len() - 1) as f64;
            let y = if hi == lo {
                30.0
            } else {
                56.0 - 52.0 * (v / scale - lo) / (hi - lo)
            };
            format!("{x:.2},{y:.2}")
        })
        .collect::<Vec<_>>()
        .join(" ")
}
