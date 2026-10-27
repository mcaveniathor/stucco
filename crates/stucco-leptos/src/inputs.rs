use leptos::prelude::*;

use crate::TextField;

/// A suggestion for [`Autocomplete`]. Suggestions do not constrain valid input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AutocompleteOption {
    /// Text inserted into the input when this suggestion is selected.
    pub value: String,
    /// Optional explanatory label. Its presentation is browser-dependent.
    pub label: Option<String>,
}

/// A controlled text field with a native datalist of suggestions.
///
/// It remains an ordinary text input without hydration. Browser suggestions
/// supplement typing; their presentation and assistive-technology support vary.
/// Do not use this component to require selection from a closed set. Use a native
/// select for that requirement, and validate all submitted text on the server.
/// The caller owns loading, read errors and stale-response handling for remote
/// suggestions. Reserve `{id}-options`, `{id}-hint` and `{id}-error`.
#[component]
pub fn Autocomplete(
    /// Stable, page-unique input ID.
    #[prop(into)]
    id: String,
    /// Native form field name.
    #[prop(into)]
    name: String,
    /// Visible input label, optionally reactive.
    #[prop(into)]
    label: TextProp,
    /// Current raw text; update it from `on_input`.
    #[prop(into)]
    value: Signal<String>,
    /// Receives edits and text adopted from before hydration.
    #[prop(into)]
    on_input: Callback<String>,
    /// Current suggestions, in display order.
    #[prop(into)]
    options: Signal<Vec<AutocompleteOption>>,
    /// Optional help text.
    #[prop(optional, into)]
    hint: Option<String>,
    /// Validation message; empty means valid.
    #[prop(default = Signal::derive(String::new), into)]
    error: Signal<String>,
    /// Whether a nonempty value is required.
    #[prop(default = Signal::derive(|| false), into)]
    required: Signal<bool>,
    /// Whether the control is disabled.
    #[prop(default = Signal::derive(|| false), into)]
    disabled: Signal<bool>,
    /// Prevents edits while retaining native submission.
    #[prop(default = Signal::derive(|| false), into)]
    read_only: Signal<bool>,
    /// Localizable prefix announced before an error.
    #[prop(default = "Error: ".to_owned(), into)]
    error_prefix: String,
    /// Optional BCP 47 language tag; otherwise inherits from the page.
    #[prop(optional, into)]
    lang: Option<String>,
    /// Optional native direction (`"ltr"`, `"rtl"` or `"auto"`).
    #[prop(optional, into)]
    dir: Option<String>,
) -> impl IntoView {
    let list_id = format!("{id}-options");
    view! {
        <div lang=lang dir=dir>
            <TextField
                id=id name=name label=label value=value on_input=on_input
                list=list_id.clone() nostrip:hint=hint error=error required=required
                disabled=disabled read_only=read_only error_prefix=error_prefix
            />
            <datalist id=list_id>
                {move || options.get().into_iter().map(|option| view! {
                    <option value=option.value label=option.label></option>
                }).collect_view()}
            </datalist>
        </div>
    }
}

/// Public metadata about a locally selected file; no file bytes are retained.
///
/// Names, sizes and media types are browser input, not authoritative server
/// validation. Applications may format this metadata in their own locale.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedFile {
    /// Browser-provided file name without an application-supplied path.
    pub name: String,
    /// Browser-reported size in bytes.
    pub size: u64,
    /// Browser-reported media type, which may be empty or inaccurate.
    pub media_type: String,
}

/// A labeled native file picker for a multipart form.
///
/// The component never binds or clears the selected files during hydration.
/// `on_change` reports metadata, including selections made before hydration; the
/// browser retains the actual files in the input. Submit the containing form
/// with `method="post"` and `enctype="multipart/form-data"`, or let the application
/// read its `FormData`. Enforce authorization, type and byte limits on the server;
/// `accept` is only a picker hint. Re-rendered responses require file reselection.
/// A native form reset does not emit `change`; the owning application must clear
/// its displayed metadata when it resets the form.
/// Reserve `{id}-hint` and `{id}-error`.
#[component]
pub fn FileInput(
    /// Stable, page-unique input ID.
    #[prop(into)]
    id: String,
    /// Native multipart field name.
    #[prop(into)]
    name: String,
    /// Visible label.
    #[prop(into)]
    label: String,
    /// Optional native accept hint, such as `".csv,text/csv"`.
    #[prop(optional, into)]
    accept: Option<String>,
    /// Allow multiple files in the picker.
    #[prop(optional)]
    multiple: bool,
    /// Whether at least one file is required.
    #[prop(default = Signal::derive(|| false), into)]
    required: Signal<bool>,
    /// Whether the picker is disabled.
    #[prop(default = Signal::derive(|| false), into)]
    disabled: Signal<bool>,
    /// Optional help text, for example accepted types and size limits.
    #[prop(optional, into)]
    hint: Option<String>,
    /// Validation message; empty means valid.
    #[prop(default = Signal::derive(String::new), into)]
    error: Signal<String>,
    /// Receives selected file metadata after hydration and subsequent changes.
    #[prop(default = Callback::new(|_: Vec<SelectedFile>| {}), into)]
    on_change: Callback<Vec<SelectedFile>>,
    /// Localizable prefix announced before an error.
    #[prop(default = "Error: ".to_owned(), into)]
    error_prefix: String,
    /// Optional BCP 47 language tag; otherwise inherits from the page.
    #[prop(optional, into)]
    lang: Option<String>,
    /// Optional native direction (`"ltr"`, `"rtl"` or `"auto"`).
    #[prop(optional, into)]
    dir: Option<String>,
) -> impl IntoView {
    let input = NodeRef::<leptos::html::Input>::new();
    let hint_id = format!("{id}-hint");
    let error_id = format!("{id}-error");
    let described_hint = hint.as_ref().map(|_| hint_id.clone());
    let described_error = error_id.clone();
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| {
        if let Some(element) = input.get() {
            untrack(|| on_change.run(selected_files(&element)));
        }
    });
    #[cfg(not(feature = "hydrate"))]
    let _ = on_change;

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
                {label}
                {move || required.get().then(|| view! {
                    <span class="st-required" aria-hidden="true">"*"</span>
                })}
            </label>
            <input
                node_ref=input id=id name=name class="st-input" type="file"
                accept=accept multiple=multiple required=move || required.get()
                disabled=move || disabled.get() aria-describedby=described_by
                aria-invalid=move || (!error.get().is_empty()).then_some("true")
                on:change=move |_| {
                    #[cfg(feature = "hydrate")]
                    if let Some(element) = input.get_untracked() {
                        on_change.run(selected_files(&element));
                    }
                }
            />
            {hint.map(|text| view! { <p class="st-field-hint" id=hint_id>{text}</p> })}
            {move || {
                let message = error.get();
                (!message.is_empty()).then(|| view! {
                    <p class="st-field-error" id=error_id.clone()>
                        <span class="st-sr-only">{error_prefix.clone()}</span>{message}
                    </p>
                })
            }}
        </div>
    }
}

#[cfg(feature = "hydrate")]
fn selected_files(input: &leptos::web_sys::HtmlInputElement) -> Vec<SelectedFile> {
    input.files().map_or_else(Vec::new, |files| {
        (0..files.length())
            .filter_map(|index| files.item(index))
            .map(|file| SelectedFile {
                name: file.name(),
                size: file.size() as u64,
                media_type: file.type_(),
            })
            .collect()
    })
}

/// A stable keyed row in [`RepeatedField`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatedValue {
    /// Unique ID that must not change when another row is removed or reordered.
    pub id: u64,
    /// Current raw input text.
    pub value: String,
}

/// A controlled list of native text inputs with hydrated add/remove actions.
///
/// Existing rows submit the same `name` once per row without hydration. The
/// application must decode repeated form keys as a list. Add/remove buttons stay
/// disabled until hydration; describe that enhancement next to the field when
/// it is essential. The callbacks must synchronously update `values`; this lets
/// the component focus the added row or an adjacent row after removal. The caller
/// owns ID allocation and server validation. Preserve unique IDs for the lifetime
/// of a draft; index keys would lose entered text and focus after removal.
/// Reserve `{id}-hint`, `{id}-add` and `{id}-{row_id}` and their field suffixes.
#[component]
pub fn RepeatedField(
    /// Stable, page-unique group ID.
    #[prop(into)]
    id: String,
    /// Native form field name shared by every row.
    #[prop(into)]
    name: String,
    /// Visible legend for the group.
    #[prop(into)]
    label: String,
    /// Current keyed rows in display order.
    #[prop(into)]
    values: Signal<Vec<RepeatedValue>>,
    /// Receives the stable row ID and raw text, including pre-hydration edits.
    #[prop(into)]
    on_input: Callback<(u64, String)>,
    /// Requests a new row. Update `values` synchronously with a fresh ID.
    #[prop(into)]
    on_add: Callback<()>,
    /// Requests removal of the given row. Update `values` synchronously.
    #[prop(into)]
    on_remove: Callback<u64>,
    /// Formats each input's visible label from its one-based position.
    #[prop(default = Callback::new(|position: usize| format!("Value {position}")), into)]
    item_label: Callback<usize, String>,
    /// Localizable label for the add button.
    #[prop(default = "Add value".to_owned(), into)]
    add_label: String,
    /// Formats each remove button's label from its one-based position.
    #[prop(default = Callback::new(|position: usize| format!("Remove value {position}")), into)]
    remove_label: Callback<usize, String>,
    /// Optional explanation of the group and hydration-dependent controls.
    #[prop(optional, into)]
    hint: Option<String>,
    /// Minimum number of rows the remove buttons permit. Validate on the server.
    #[prop(default = 0)]
    min_values: usize,
    /// Maximum number of rows the add button permits. Validate on the server.
    #[prop(optional)]
    max_values: Option<usize>,
    /// Disable all fields and actions.
    #[prop(default = Signal::derive(|| false), into)]
    disabled: Signal<bool>,
    /// Optional BCP 47 language tag; otherwise inherits from the page.
    #[prop(optional, into)]
    lang: Option<String>,
    /// Optional native direction (`"ltr"`, `"rtl"` or `"auto"`).
    #[prop(optional, into)]
    dir: Option<String>,
) -> impl IntoView {
    let hydrated = RwSignal::new(false);
    #[cfg(feature = "hydrate")]
    Effect::new(move |_| hydrated.set(true));
    let hint_id = format!("{id}-hint");
    let description = hint.as_ref().map(|_| hint_id.clone());
    let add_id = format!("{id}-add");
    let field_prefix = StoredValue::new(id.clone());
    let field_name = StoredValue::new(name);
    let can_remove = move || hydrated.get() && !disabled.get() && values.get().len() > min_values;
    let can_add = move || {
        hydrated.get() && !disabled.get() && max_values.is_none_or(|max| values.get().len() < max)
    };
    view! {
        <fieldset id=id class="st-fieldset" disabled=move || disabled.get()
            aria-describedby=description lang=lang dir=dir>
            <legend class="st-legend">{label}</legend>
            {hint.map(|text| view! { <p class="st-field-hint" id=hint_id>{text}</p> })}
            <For each=move || values.get() key=|row| row.id children=move |row| {
                let row_id = row.id;
                let input_id = format!("{}-{row_id}", field_prefix.get_value());
                let position = move || values.with(|rows| rows.iter().position(|row| row.id == row_id).map_or(1, |index| index + 1));
                view! {
                    <div class="st-form">
                        <TextField
                            id=input_id name=field_name.get_value()
                            label=move || item_label.run(position())
                            value=Signal::derive(move || values.with(|rows| rows.iter().find(|row| row.id == row_id).map_or_else(String::new, |row| row.value.clone())))
                            on_input=Callback::new(move |value| on_input.run((row_id, value)))
                            disabled=disabled
                        />
                        <button type="button" class="st-button" data-variant="secondary" data-size="sm"
                            disabled=move || !can_remove()
                            on:click=move |_| {
                                if !can_remove() { return; }
                                #[cfg(feature = "hydrate")]
                                let previous_position = position().saturating_sub(1);
                                on_remove.run(row_id);
                                #[cfg(feature = "hydrate")]
                                {
                                    let rows = values.get_untracked();
                                    let target = rows.get(previous_position).or_else(|| rows.last())
                                        .map(|row| format!("{}-{}", field_prefix.get_value(), row.id))
                                        .unwrap_or_else(|| format!("{}-add", field_prefix.get_value()));
                                    focus_after_render(target);
                                }
                            }
                        >{move || remove_label.run(position())}</button>
                    </div>
                }
            }/>
            <button type="button" id=add_id class="st-button" data-variant="secondary" data-size="md"
                disabled=move || !can_add()
                on:click=move |_| {
                    if !can_add() { return; }
                    #[cfg(feature = "hydrate")]
                    let previous: Vec<_> = values.get_untracked().iter().map(|row| row.id).collect();
                    on_add.run(());
                    #[cfg(feature = "hydrate")]
                    if let Some(row) = values.get_untracked().iter().find(|row| !previous.contains(&row.id)) {
                        focus_after_render(format!("{}-{}", field_prefix.get_value(), row.id));
                    }
                }
            >{add_label}</button>
        </fieldset>
    }
}

#[cfg(feature = "hydrate")]
fn focus_after_render(id: String) {
    use leptos::wasm_bindgen::JsCast;
    request_animation_frame(move || {
        if let Some(element) = document()
            .get_element_by_id(&id)
            .and_then(|element| element.dyn_into::<leptos::web_sys::HtmlElement>().ok())
        {
            let _ = element.focus();
        }
    });
}
