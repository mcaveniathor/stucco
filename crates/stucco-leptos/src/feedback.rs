//! Status and empty-state feedback: [`Progress`], [`Meter`], [`Callout`] and
//! [`EmptyState`].

use leptos::prelude::*;

use crate::vocab::Tone;

fn clamp_finite(value: f64, lo: f64, hi: f64) -> f64 {
    if value.is_finite() {
        value.clamp(lo, hi)
    } else {
        lo
    }
}

/// A number as an HTML attribute value, without a negative zero.
fn num(value: f64) -> String {
    format!("{}", value + 0.0)
}

/// Labelled progress of a task, determinate or indeterminate.
///
/// Give it a page-unique `id`, which its label uses. A `value` shows determinate
/// progress, clamped to `0..=max`; omit it for an indeterminate task. A `max` that
/// is not a positive finite number counts as 1, and a non-finite `value` as 0.
/// With `show_value`, the percentage is also shown as text.
#[component]
pub fn Progress(
    /// Page-unique id of the `<progress>` element.
    #[prop(into)]
    id: String,
    /// What is progressing; this is the control's visible label.
    #[prop(into)]
    label: String,
    /// Completed amount; `None` means indeterminate.
    #[prop(optional, into)]
    value: MaybeProp<f64>,
    /// The amount that counts as complete.
    #[prop(default = 100.0)]
    max: f64,
    /// Also show the percentage as visible text.
    #[prop(optional)]
    show_value: bool,
) -> impl IntoView {
    let max = if max.is_finite() && max > 0.0 {
        max
    } else {
        1.0
    };
    let current = move || value.get().map(|value| clamp_finite(value, 0.0, max));
    let percent =
        move || current().map(|value| format!("{}%", (value / max * 100.0).round() as u32));
    let shown = move || {
        percent()
            .filter(|_| show_value)
            .map(|text| view! { <span class="st-progress-value">{text}</span> })
    };
    view! {
        <div class="st-progress">
            <label for=id.clone()>{label}</label>
            <progress id=id value=move || current().map(num)
                max=move || current().map(|_| num(max))>
                {move || percent().unwrap_or_default()}
            </progress>
            {shown}
        </div>
    }
}

/// Reconciles a meter's bounds: `min` finite (else 0) and `max` above it.
fn meter_bounds(min: f64, max: f64) -> (f64, f64) {
    let mut min = if min.is_finite() { min } else { 0.0 };
    let max = if max.is_finite() && max > min {
        max
    } else if (min + 1.0).is_finite() && min + 1.0 > min {
        min + 1.0
    } else if min == f64::MAX {
        min = f64::from_bits(f64::MAX.to_bits() - 1);
        f64::MAX
    } else {
        // `min` is finite, nonzero and too large for +1 to change it: step one
        // bit toward positive infinity.
        let bits = min.to_bits();
        f64::from_bits(if min > 0.0 { bits + 1 } else { bits - 1 })
    };
    (min, max)
}

/// A labelled gauge for a value within a known range, such as disk usage.
///
/// Give it a page-unique `id`; `{id}-text` is reserved for `value_text`. Bounds are
/// reconciled like the static meter: a `min` that is not finite counts as 0, and a
/// `max` that is not above `min` is raised just above it. The value and thresholds
/// are clamped into range. Use `value_text` for a human description such as
/// "70% used", which is also exposed through `aria-describedby`.
#[component]
pub fn Meter(
    /// Page-unique id of the `<meter>` element.
    #[prop(into)]
    id: String,
    /// What is measured; this is the control's visible label.
    #[prop(into)]
    label: String,
    /// The measured value.
    #[prop(into)]
    value: Signal<f64>,
    /// Lowest value of the range.
    min: f64,
    /// Highest value of the range.
    max: f64,
    /// Upper edge of the low band.
    #[prop(optional, into)]
    low: Option<f64>,
    /// Lower edge of the high band.
    #[prop(optional, into)]
    high: Option<f64>,
    /// The best value.
    #[prop(optional, into)]
    optimum: Option<f64>,
    /// A description of the value, such as "70% used". Pass a signal when the text
    /// depends on `value`, so the description does not go stale.
    #[prop(optional, into)]
    value_text: MaybeProp<String>,
) -> impl IntoView {
    let (min, max) = meter_bounds(min, max);
    let low = low.map(|low| clamp_finite(low, min, max));
    let high = high
        .map(|high| clamp_finite(high, min, max))
        .map(|high| high.max(low.unwrap_or(min)));
    let optimum = optimum.map(|optimum| clamp_finite(optimum, min, max));
    let current = move || clamp_finite(value.get(), min, max);
    let text_id = format!("{id}-text");
    let described_by = {
        let text_id = text_id.clone();
        move || value_text.get().map(|_| text_id.clone())
    };
    let description = move || {
        value_text.get().map(|text| {
            view! { <span id=text_id.clone() class="st-meter-text">{text}</span> }
        })
    };
    view! {
        <div class="st-meter">
            <label for=id.clone()>{label}</label>
            <meter id=id value=move || num(current()) min=num(min) max=num(max)
                low=low.map(num) high=high.map(num) optimum=optimum.map(num)
                aria-describedby=described_by>
                {move || num(current())}
            </meter>
            {description}
        </div>
    }
}

/// Actions shown at the end of a [`Callout`]; use as `<CalloutActions slot>…`.
#[slot]
pub struct CalloutActions {
    /// The action links or buttons.
    children: ChildrenFn,
}

/// A static note with a tone, an icon, an optional title, content and actions.
///
/// Because the tone is conveyed by colour and icon, a visually hidden prefix says
/// it too: "Note: ", "Success: ", "Warning: " or "Important: ". Translate them
/// with `note_label`, `success_label`, `warning_label` and `important_label`.
/// `Default` and `Muted` have no prefix.
#[component]
pub fn Callout(
    /// The kind of note.
    tone: Tone,
    /// Heading line of the callout.
    #[prop(optional, into)]
    title: Option<String>,
    /// Hidden prefix for `Info` and `Accent`.
    #[prop(default = "Note: ".to_owned(), into)]
    note_label: String,
    /// Hidden prefix for `Success`.
    #[prop(default = "Success: ".to_owned(), into)]
    success_label: String,
    /// Hidden prefix for `Warning`.
    #[prop(default = "Warning: ".to_owned(), into)]
    warning_label: String,
    /// Hidden prefix for `Danger`.
    #[prop(default = "Important: ".to_owned(), into)]
    important_label: String,
    /// Optional actions at the end.
    #[prop(optional)]
    callout_actions: Option<CalloutActions>,
    /// The callout's content.
    children: Children,
) -> impl IntoView {
    let (icon, prefix) = match tone {
        Tone::Info | Tone::Accent => (CalloutIcon::Info, Some(note_label)),
        Tone::Success => (CalloutIcon::CircleCheck, Some(success_label)),
        Tone::Warning => (CalloutIcon::TriangleAlert, Some(warning_label)),
        Tone::Danger => (CalloutIcon::CircleAlert, Some(important_label)),
        Tone::Default | Tone::Muted => (CalloutIcon::Info, None),
    };
    let prefix = prefix.map(|text| view! { <span class="st-sr-only">{text}</span> });
    let heading = match title {
        Some(title) => view! { <p class="st-callout-title">{prefix}{title}</p> }.into_any(),
        None => prefix.into_any(),
    };
    let actions = callout_actions.map(|actions| {
        view! { <div class="st-callout-actions">{(actions.children)()}</div> }
    });
    view! {
        <div class="st-callout" data-tone=tone.as_str()>
            <CalloutIcon kind=icon/>
            <div class="st-callout-body">{heading}{children()}{actions}</div>
        </div>
    }
}

#[derive(Clone, Copy)]
enum CalloutIcon {
    Info,
    CircleCheck,
    TriangleAlert,
    CircleAlert,
}

/// Lucide `info`, `circle-check`, `triangle-alert` and `circle-alert` (ISC; see
/// LICENSE-LUCIDE), as the static callout embeds them.
#[component]
fn CalloutIcon(kind: CalloutIcon) -> impl IntoView {
    let shapes = match kind {
        CalloutIcon::Info => view! {
            <circle cx="12" cy="12" r="10"/><path d="M12 16v-4"/><path d="M12 8h.01"/>
        }
        .into_any(),
        CalloutIcon::CircleCheck => view! {
            <circle cx="12" cy="12" r="10"/><path d="m16 9-5.5 5.5L8 12"/>
        }
        .into_any(),
        CalloutIcon::TriangleAlert => view! {
            <path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/>
            <path d="M12 9v4"/><path d="M12 17h.01"/>
        }
        .into_any(),
        CalloutIcon::CircleAlert => view! {
            <circle cx="12" cy="12" r="10"/>
            <line x1="12" x2="12" y1="8" y2="12"/><line x1="12" x2="12.01" y1="16" y2="16"/>
        }
        .into_any(),
    };
    view! {
        <svg class="st-icon" viewBox="0 0 24 24" width="1em" height="1em" fill="none"
            stroke="currentColor" stroke-width="2" stroke-linecap="round"
            stroke-linejoin="round" aria-hidden="true" focusable="false">{shapes}</svg>
    }
}

/// A prompt for when there is nothing to show, with an optional description and
/// actions.
///
/// `description` and `actions` take closures returning any view, for example
/// `actions=|| view! { <a href="/orders">"Reset filters"</a> }`.
#[component]
pub fn EmptyState(
    /// What is missing, as a level-two heading.
    #[prop(into)]
    title: String,
    /// Explanation or next step.
    #[prop(optional, into)]
    description: ViewFn,
    /// Links or buttons that help.
    #[prop(optional, into)]
    actions: ViewFn,
) -> impl IntoView {
    view! {
        <div class="st-empty-state">
            <h2 class="st-heading" data-size="2xl">{title}</h2>
            {description.run()}
            {actions.run()}
        </div>
    }
}
