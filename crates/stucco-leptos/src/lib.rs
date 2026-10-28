//! Reactive Leptos components that share Stucco's visual and accessibility contracts.
//!
//! This opt-in prerelease uses Leptos **0.8.22** and requires Rust **1.88**.
//! It has no default features. Enable `ssr` in the native application and
//! `hydrate` in its separate `wasm32-unknown-unknown` build; do not combine these
//! modes. Client-only mounting is not currently a supported mode.
//! These components use Leptos directly; they do not hydrate HTML produced by
//! Stucco's static [`Render`](https://docs.rs/stucco-core/0.5.0/stucco_core/trait.Render.html)
//! builders.
//!
//! # Native forms and hydration
//!
//! The application owns HTTP routes, initial values, validation, authorization
//! and CSRF protection. Give forms native `action` and `method` attributes so
//! submitting them does not require hydration. Controlled callbacks must update
//! the caller's state, including input adopted from before hydration:
//!
//! ```
//! use leptos::prelude::*;
//! use stucco_leptos::{SubmitButton, TextField};
//!
//! #[component]
//! fn SearchForm() -> impl IntoView {
//!     let query = RwSignal::new(String::new());
//!     view! {
//!         <form class="st-form" method="get" action="/search">
//!             <TextField id="search-query" name="q" label="Search terms"
//!                 value=query on_input=Callback::new(move |text| query.set(text))/>
//!             <SubmitButton label="Search"/>
//!         </form>
//!     }
//! }
//! ```
//!
//! Text fields, native datalist suggestions and file pickers work before WASM
//! loads. [`RepeatedField`] initially exposes editable rows; adding and removing
//! rows requires hydration. [`Dialog`] follows its application-owned fallback
//! link before hydration and opens a browser-native modal afterwards.
//!
//! # Shared styles
//!
//! With `ssr`, `stylesheet` produces Stucco's existing theme and component CSS.
//! Serve it once as `text/css` and link it from the application's document. The
//! WASM feature does not include the static renderer or asset inventory. Native
//! controls without a wrapper can use the same `st-form`, `st-field`, `st-label`,
//! `st-input`, `st-select` and `st-textarea` classes with explicit labels and
//! accessible hint/error references. The package's `native_form` example shows
//! this composition and can print both HTML and CSS.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

use leptos::prelude::*;

mod data;
mod href;
mod inputs;
mod navigation;
mod overlay;
mod table;
mod vocab;

pub use data::{Avatar, Badge, Card, DescriptionItem, DescriptionList, Panel, Stat, StatChange};
pub use inputs::{
    Autocomplete, AutocompleteOption, FileInput, RepeatedField, RepeatedValue, SelectedFile,
};
pub use navigation::{
    BreadcrumbCurrent, BreadcrumbLink, Breadcrumbs, CursorPagination, NavLink, Pagination,
    StepItem, Steps, Tabs,
};
pub use overlay::Dialog;
pub use table::{Cell, CellKind, HeaderCell, Row, Table, TableBody, TableHead};
pub use vocab::{Size, Tone, Trend};

/// The existing Stucco appearance used by a [`SubmitButton`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SubmitVariant {
    /// The main action on the page.
    #[default]
    Primary,
    /// A destructive action, such as deleting a record.
    Danger,
}

impl SubmitVariant {
    fn attribute(self) -> &'static str {
        match self {
            Self::Primary => "primary",
            Self::Danger => "danger",
        }
    }
}

/// A native submit button whose pending state preserves keyboard focus.
///
/// Pending buttons stay focusable and cancel clicks instead of using `disabled`.
/// The parent form must also guard its submit handler against repeated or
/// programmatic submissions while its action is pending.
#[component]
pub fn SubmitButton(
    /// Text shown when the action is ready.
    #[prop(into)]
    label: String,
    /// Whether the parent action is waiting for a response.
    #[prop(default = Signal::derive(|| false), into)]
    pending: Signal<bool>,
    /// Whether another condition prevents submission, without implying activity.
    /// Keep the reason visible near the form and guard the parent submit handler.
    #[prop(default = Signal::derive(|| false), into)]
    blocked: Signal<bool>,
    /// Text shown while the action is pending.
    #[prop(default = "Saving…".to_owned(), into)]
    pending_label: String,
    /// The existing Stucco button appearance.
    #[prop(optional)]
    variant: SubmitVariant,
) -> impl IntoView {
    view! {
        <button
            class="st-button"
            type="submit"
            data-variant=variant.attribute()
            data-size="md"
            aria-disabled=move || (pending.get() || blocked.get()).to_string()
            aria-busy=move || pending.get().to_string()
            on:click=move |event| {
                if pending.get_untracked() || blocked.get_untracked() {
                    event.prevent_default();
                }
            }
        >
            {move || if pending.get() { pending_label.clone() } else { label.clone() }}
        </button>
    }
}

/// An error listed by [`ErrorSummary`], optionally linked to a form control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryError {
    /// The control's ID without a leading `#`.
    ///
    /// Empty IDs and IDs containing whitespace or control characters are shown
    /// without a link. Other IDs are encoded as URL fragments by the component.
    pub target: Option<String>,
    /// A standalone description of the problem, rendered as escaped text.
    pub message: String,
}

fn field_fragment(target: &str) -> Option<String> {
    if target.is_empty() || target.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return None;
    }
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut fragment = String::from("#");
    for byte in target.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            fragment.push(char::from(byte));
        } else {
            fragment.push('%');
            fragment.push(char::from(HEX[(byte >> 4) as usize]));
            fragment.push(char::from(HEX[(byte & 0x0f) as usize]));
        }
    }
    Some(fragment)
}

/// A focused summary of form and field errors using Stucco's form styles.
///
/// Supply a stable, page-unique `id`; the component reserves `{id}-title`.
/// Set the messages first, then increase `focus_trigger` after each unsuccessful
/// submission. A positive initial trigger focuses errors from an SSR response
/// when hydration starts unless the user has already focused another control.
/// Zero never requests focus. Changing error text alone
/// does not move focus. The summary remains mounted and is hidden when empty.
#[component]
pub fn ErrorSummary(
    /// Stable, page-unique ID for the summary.
    #[prop(into)]
    id: String,
    /// A form-level explanation, omitted when empty.
    #[prop(default = Signal::derive(String::new), into)]
    message: Signal<String>,
    /// Field or other errors in their desired display order.
    #[prop(default = Signal::derive(Vec::new), into)]
    errors: Signal<Vec<SummaryError>>,
    /// Increase after setting errors to request focus; zero never focuses.
    #[prop(default = Signal::derive(|| 0), into)]
    focus_trigger: Signal<u32>,
    /// Localizable heading identifying the summary.
    #[prop(default = "There is a problem".to_owned(), into)]
    heading: String,
    /// Optional BCP 47 language tag; otherwise inherits from the page.
    #[prop(optional, into)]
    lang: Option<String>,
    /// Optional native direction (`"ltr"`, `"rtl"` or `"auto"`).
    #[prop(optional, into)]
    dir: Option<String>,
) -> impl IntoView {
    let summary = NodeRef::<leptos::html::Div>::new();
    let title_id = format!("{id}-title");
    let empty = move || message.get().is_empty() && errors.get().is_empty();

    #[cfg(feature = "hydrate")]
    {
        let focused_version = std::cell::Cell::new(0);
        let initialized = std::cell::Cell::new(false);
        Effect::new(move |_| {
            let version = focus_trigger.get();
            let element = summary.get();
            let first = !initialized.replace(true);
            if version != 0
                && version != focused_version.get()
                && (!message.get_untracked().is_empty() || !errors.get_untracked().is_empty())
                && let Some(element) = element
            {
                if !first
                    || document()
                        .active_element()
                        .is_none_or(|active| active.tag_name() == "BODY")
                {
                    let _ = element.focus();
                }
                focused_version.set(version);
            }
        });
    }
    #[cfg(not(feature = "hydrate"))]
    let _ = focus_trigger;

    view! {
        <div
            node_ref=summary
            id=id
            class="st-error-summary"
            role="alert"
            tabindex="-1"
            aria-labelledby=title_id.clone()
            hidden=empty
            lang=lang
            dir=dir
        >
            <h2 class="st-error-summary-title" id=title_id.clone()>{heading}</h2>
            {move || {
                let text = message.get();
                (!text.is_empty()).then(|| view! { <p>{text}</p> })
            }}
            <ul class="st-error-summary-list" hidden=move || errors.get().is_empty()>
                {move || errors.get().into_iter().map(|error| {
                    let fragment = error.target.as_deref().and_then(field_fragment);
                    view! {
                        <li>
                            {match fragment {
                                Some(href) => view! { <a href=href>{error.message}</a> }.into_any(),
                                None => error.message.into_any(),
                            }}
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
}

/// A persistent polite live region for an action's current status.
///
/// Keep the component mounted and update `message` for announcements. An empty
/// message clears the content without removing the live region. Status text is
/// escaped and uses the surrounding page's typography.
#[component]
pub fn StatusMessage(
    /// The current status; an empty string clears it.
    #[prop(into)]
    message: Signal<String>,
) -> impl IntoView {
    view! { <div role="status" aria-atomic="true">{move || message.get()}</div> }
}

/// An h1 page heading with optional descriptive text and action children.
///
/// Place actions such as navigation links in `children`. This uses the same
/// heading and page-header classes as Stucco's static renderer.
#[component]
pub fn PageHeader(
    /// The page's visible h1 heading.
    #[prop(into)]
    heading: String,
    /// Optional context for the page.
    #[prop(optional, into)]
    description: Option<String>,
    /// Optional action content rendered after the description.
    #[prop(optional)]
    children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="st-page-header">
            <h1 class="st-heading" data-size="3xl">{heading}</h1>
            {description.map(|text| view! { <div class="st-page-header-description">{text}</div> })}
            {children.map(|actions| actions())}
        </div>
    }
}

/// A controlled text input with a visible label, hint and reactive error.
///
/// The caller owns the value and updates it from `on_input`. Supply a stable,
/// page-unique `id`; the field reserves its `{id}-hint` and `{id}-error` IDs.
/// Text is preserved verbatim, including invalid numeric input. Use
/// `inputmode="numeric"` to suggest a numeric keyboard without losing that text.
///
/// During hydration, text entered before the component starts is sent through
/// `on_input` before synchronizing later signal changes back into the input.
/// Browser validation is supplementary; validate submitted data on the server.
#[component]
pub fn TextField(
    /// Stable, page-unique input ID, identical on the server and browser.
    #[prop(into)]
    id: String,
    /// Native form field name.
    #[prop(into)]
    name: String,
    /// Visible text of the input's label.
    #[prop(into)]
    label: TextProp,
    /// The caller's current, unparsed input value.
    #[prop(into)]
    value: Signal<String>,
    /// Receives the complete text whenever the user edits the input.
    #[prop(into)]
    on_input: Callback<String>,
    /// Optional help text referenced by `aria-describedby`.
    #[prop(optional, into)]
    hint: Option<String>,
    /// A validation message; an empty string means the field has no error.
    #[prop(default = Signal::derive(String::new), into)]
    error: Signal<String>,
    /// Native keyboard hint such as `"numeric"` or `"decimal"`.
    #[prop(optional, into)]
    inputmode: Option<String>,
    /// Whether a value is required; also controls the visible required marker.
    #[prop(default = Signal::derive(|| false), into)]
    required: Signal<bool>,
    /// Whether the control is disabled.
    #[prop(default = Signal::derive(|| false), into)]
    disabled: Signal<bool>,
    /// Prevents editing while retaining focus and native form submission.
    #[prop(default = Signal::derive(|| false), into)]
    read_only: Signal<bool>,
    /// Optional native datalist ID. [`Autocomplete`] supplies this itself.
    #[prop(optional, into)]
    list: Option<String>,
    /// Localizable prefix announced before a validation message.
    #[prop(default = "Error: ".to_owned(), into)]
    error_prefix: String,
    /// Optional BCP 47 language tag; otherwise inherits from the page.
    #[prop(optional, into)]
    lang: Option<String>,
    /// Optional native direction (`"ltr"`, `"rtl"` or `"auto"`).
    #[prop(optional, into)]
    dir: Option<String>,
) -> impl IntoView {
    let initial_value = value.get_untracked();
    let input = NodeRef::<leptos::html::Input>::new();
    let hint_id = format!("{id}-hint");
    let error_id = format!("{id}-error");
    let described_hint = hint.as_ref().map(|_| hint_id.clone());
    let described_error = error_id.clone();

    // A property binding would overwrite text typed into the SSR input before
    // hydration. Adopt that text once, then synchronize subsequent caller updates.
    #[cfg(feature = "hydrate")]
    {
        let original = initial_value.clone();
        let initialized = std::cell::Cell::new(false);
        Effect::new(move |_| {
            let Some(element) = input.get() else {
                return;
            };
            let next = value.get();
            let current = element.value();
            // A client-side component rebuild may reuse the input node. Adopt
            // pre-hydration typing only once per DOM element, not again when a
            // new component owner supplies a deliberately reloaded value.
            let first = !initialized.replace(true);
            let adopt = first && !element.has_attribute("data-st-initialized");
            if first {
                let _ = element.set_attribute("data-st-initialized", "");
            }
            if adopt && current != original {
                on_input.run(current);
            } else if current != next {
                element.set_value(&next);
            }
        });
    }

    let described_by = move || {
        let mut ids = Vec::with_capacity(2);
        if let Some(id) = &described_hint {
            ids.push(id.as_str());
        }
        if !error.get().is_empty() {
            ids.push(described_error.as_str());
        }
        (!ids.is_empty()).then(|| ids.join(" "))
    };

    view! {
        <div class="st-field" lang=lang dir=dir>
            <label class="st-label" for=id.clone()>
                {move || label.get()}
                {move || required.get().then(|| view! {
                    <span class="st-required" aria-hidden="true">"*"</span>
                })}
            </label>
            <input
                node_ref=input
                class="st-input"
                type="text"
                id=id
                name=name
                value=initial_value
                inputmode=inputmode
                list=list
                required=move || required.get()
                disabled=move || disabled.get()
                readonly=move || read_only.get()
                aria-describedby=described_by
                aria-invalid=move || (!error.get().is_empty()).then_some("true")
                on:input=move |event| on_input.run(event_target_value(&event))
            />
            {hint.map(|text| view! {
                <p class="st-field-hint" id=hint_id>{text}</p>
            })}
            {move || {
                let message = error.get();
                (!message.is_empty()).then(|| view! {
                    <p class="st-field-error" id=error_id.clone()>
                        <span class="st-sr-only">{error_prefix.clone()}</span>
                        {message}
                    </p>
                })
            }}
        </div>
    }
}

/// Builds CSS from Stucco's existing theme and standard component styles.
///
/// Call this once on the server and serve the result as a stylesheet. It contains
/// no scripts and does not load the static renderer's browser runtime. The shared
/// styles cover forms, actions, layout, typography, data, feedback and navigation.
///
/// ```
/// use stucco_leptos::stylesheet;
/// use stucco_theme::Preset;
///
/// let css = stylesheet(Preset::Slate);
/// assert!(css.contains(".st-input"));
/// ```
///
/// # Panics
///
/// A hand-tuned [`stucco_theme::Theme`] that fails its contrast checks panics when
/// converted to a built theme. Call `Theme::build` first to handle that error.
#[cfg(feature = "ssr")]
pub fn stylesheet(theme: impl Into<stucco_theme::BuiltTheme>) -> String {
    let mut cx = stucco_core::Cx::new();
    for asset in stucco_ui::ui_assets() {
        cx.require(asset);
    }
    let (_, required) = cx.finish();
    stucco_core::Bundle::new(theme).css_for(&required)
}
