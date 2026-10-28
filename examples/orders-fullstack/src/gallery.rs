//! Component demonstrations independent of Orders services and domain models.

use leptos::prelude::*;
use leptos_meta::Title;
use leptos_router::hooks::use_query_map;

use crate::gallery_sections::NavigationSection;
use stucco_leptos::{
    Autocomplete, AutocompleteOption, Dialog, FileInput, PageHeader, RepeatedField, RepeatedValue,
    SelectedFile, StatusMessage, SubmitButton,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct GalleryState {
    arabic: bool,
    rtl: bool,
    details: bool,
    submitted: bool,
    page: u64,
    city: String,
    recipients: Vec<String>,
}

pub(crate) fn text(arabic: bool, english: &'static str, arabic_text: &'static str) -> &'static str {
    if arabic { arabic_text } else { english }
}

#[component]
pub(crate) fn ComponentGallery() -> impl IntoView {
    let query = use_query_map();
    // A query change is a new native-form result, including browser Back/Forward.
    // Keying it also keeps SSR and hydration's starting values identical.
    let state = move || {
        let query = query.get();
        let arabic = query.get("lang").is_some_and(|lang| lang == "ar");
        GalleryState {
            arabic,
            rtl: query.get("dir").map_or(arabic, |dir| dir == "rtl"),
            details: query.get("details").is_some_and(|value| value == "open"),
            submitted: query.get("preview").is_some_and(|value| value == "1"),
            page: query
                .get("page")
                .and_then(|page| page.parse::<u64>().ok())
                .map_or(3, |page| page.clamp(1, 10)),
            city: query.get("city").unwrap_or_else(|| "Paris".to_owned()),
            recipients: query.get_all("recipient").map_or_else(
                || vec!["Ada".to_owned(), "Grace".to_owned()],
                |values| values.into_iter().take(5).collect(),
            ),
        }
    };
    view! {
        <For each=move || vec![state()] key=|state| state.clone()
            children=|state| view! { <GalleryExamples state/> }/>
    }
}

#[component]
fn GalleryExamples(state: GalleryState) -> impl IntoView {
    let ar = state.arabic;
    let lang = if ar { "ar" } else { "en" };
    let dir = if state.rtl { "rtl" } else { "ltr" };
    let hydrated = RwSignal::new(false);
    Effect::new(move |_| hydrated.set(true));
    let city = RwSignal::new(state.city);
    let rows = RwSignal::new(
        state
            .recipients
            .into_iter()
            .enumerate()
            .map(|(index, value)| RepeatedValue {
                id: index as u64 + 1,
                value,
            })
            .collect::<Vec<_>>(),
    );
    let next_id = RwSignal::new(rows.get_untracked().len() as u64 + 1);
    let files = RwSignal::new(Vec::<SelectedFile>::new());
    let options = Signal::derive(move || {
        vec![
            AutocompleteOption {
                value: "Paris".into(),
                label: Some(text(ar, "France", "فرنسا").into()),
            },
            AutocompleteOption {
                value: "Cairo".into(),
                label: Some(text(ar, "Egypt", "مصر").into()),
            },
            AutocompleteOption {
                value: "Tokyo".into(),
                label: Some(text(ar, "Japan", "اليابان").into()),
            },
        ]
    });
    let summary = Signal::derive(move || {
        let names = rows
            .get()
            .into_iter()
            .map(|row| row.value)
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "{}: {} · {}: {}",
            text(ar, "City", "المدينة"),
            city.get(),
            text(ar, "Recipients", "المستلمون"),
            names
        )
    });
    let fallback = format!("/components?lang={lang}&dir={dir}&details=open");
    let dialog_page = fallback.clone();
    let return_href = format!("/components?lang={lang}&dir={dir}");
    let help_title = text(ar, "About this gallery", "عن هذا المعرض");
    view! {
        <Title text="Component gallery · Stucco"/>
        <article id="component-gallery" class="st-stack" data-space="6"
            data-hydrated=move || hydrated.get().to_string() lang=lang dir=dir>
            <PageHeader heading=text(ar, "Component gallery", "معرض المكونات")
                description=text(ar,
                    "Reusable Rust components with native HTML fallbacks. These examples do not save data.",
                    "مكونات Rust قابلة لإعادة الاستخدام مع نماذج HTML أصلية. هذه الأمثلة لا تحفظ البيانات.")/>
            <nav aria-label=text(ar, "Gallery language and direction", "لغة المعرض واتجاهه")
                class="st-cluster" data-space="4">
                <a href="/components?lang=en&dir=ltr" lang="en">"English · LTR"</a>
                <a href="/components?lang=en&dir=rtl" lang="en">"English · RTL"</a>
                <a href="/components?lang=ar&dir=rtl" lang="ar">"العربية · RTL"</a>
            </nav>
            {state.details.then(|| view! {
                <section id="gallery-help-fallback" class="st-surface st-stack"
                    data-level="raised" data-border="" data-padding="5" data-space="4"
                    aria-labelledby="gallery-help-fallback-title">
                    <h2 id="gallery-help-fallback-title">{help_title}</h2>
                    <HelpText arabic=ar/>
                    <a href=return_href>{text(ar, "Back to gallery", "العودة إلى المعرض")}</a>
                </section>
            })}
            <section class="st-surface st-stack" data-level="raised" data-border=""
                data-padding="5" data-space="4" aria-labelledby="gallery-overlay-heading">
                <h2 id="gallery-overlay-heading">{text(ar, "A native dialog", "حوار أصلي")}</h2>
                <p>{text(ar, "Open help in place after hydration, or follow its page link before it loads.",
                    "افتح المساعدة في حوار بعد تحميل WASM أو انتقل إلى صفحتها قبل ذلك.")}</p>
                <Dialog id="gallery-help" title=help_title trigger_label=text(ar, "Open gallery help", "فتح مساعدة المعرض")
                    fallback_href=fallback close_label=text(ar, "Close help", "إغلاق المساعدة")>
                    <HelpText arabic=ar/>
                    <a href=dialog_page>{text(ar, "Read help as a page", "قراءة المساعدة في صفحة")}</a>
                </Dialog>
            </section>
            <section class="st-surface st-stack" data-level="raised" data-border=""
                data-padding="5" data-space="4" aria-labelledby="gallery-fields-heading">
                <h2 id="gallery-fields-heading">{text(ar, "Suggestions and repeated values", "اقتراحات وقيم متكررة")}</h2>
                <form id="gallery-form" class="st-form" method="get" action="/components">
                    <input type="hidden" name="lang" value=lang/>
                    <input type="hidden" name="dir" value=dir/>
                    <input type="hidden" name="preview" value="1"/>
                    <Autocomplete id="gallery-city" name="city" label=text(ar, "City", "المدينة")
                        value=Signal::derive(move || city.get())
                        on_input=Callback::new(move |value| city.set(value)) options=options
                        hint=text(ar, "Choose a suggestion or type any city.", "اختر اقتراحاً أو اكتب أي مدينة.")/>
                    <RepeatedField id="gallery-recipients" name="recipient" label=text(ar, "Recipients", "المستلمون")
                        values=Signal::derive(move || rows.get())
                        on_input=Callback::new(move |(id, value)| rows.update(|rows| {
                            if let Some(row) = rows.iter_mut().find(|row| row.id == id) { row.value = value; }
                        }))
                        on_add=Callback::new(move |()| {
                            let id = next_id.get_untracked();
                            if let Some(next) = id.checked_add(1) {
                                rows.update(|rows| rows.push(RepeatedValue {id, value: String::new()}));
                                next_id.set(next);
                            }
                        })
                        on_remove=Callback::new(move |id| rows.update(|rows| rows.retain(|row| row.id != id)))
                        item_label=Callback::new(move |position| format!("{} {position}", text(ar, "Recipient", "المستلم")))
                        remove_label=Callback::new(move |position| format!("{} {position}", text(ar, "Remove recipient", "حذف المستلم")))
                        add_label=text(ar, "Add recipient", "إضافة مستلم")
                        min_values=1 max_values=5
                        hint=text(ar, "Edit these values with or without WASM. Add and remove require hydration; up to five recipients.",
                            "يمكن تعديل القيم مع WASM أو بدونه. الإضافة والحذف يتطلبان التحميل؛ خمسة مستلمين كحد أقصى.")/>
                    <SubmitButton label=text(ar, "Preview submitted values", "معاينة القيم المرسلة")/>
                </form>
                {state.submitted.then(|| view! {
                    <p role="status">{text(ar, "Native form values received. No data was saved.", "تم استلام قيم النموذج. لم تُحفظ أي بيانات.")}</p>
                })}
                <div id="gallery-value-preview"><StatusMessage message=summary/></div>
                <p id="gallery-formatted-amount">{text(ar, "Application-formatted amount: €1,234.50", "مبلغ منسّق بواسطة التطبيق: ١٬٢٣٤٫٥٠ €")}</p>
            </section>
            <section class="st-surface st-stack" data-level="raised" data-border=""
                data-padding="5" data-space="4" aria-labelledby="gallery-files-heading">
                <h2 id="gallery-files-heading">{text(ar, "File selection", "اختيار الملفات")}</h2>
                <FileInput id="gallery-files" name="files" label=text(ar, "Example files", "ملفات تجريبية") multiple=true
                    hint=text(ar, "This example shows file names and sizes after hydration. It does not upload the files.",
                        "يعرض المثال أسماء الملفات وأحجامها بعد التحميل. لا يرفع الملفات.")
                    on_change=Callback::new(move |selected| files.set(selected))/>
                <div id="gallery-file-preview" role="status" aria-atomic="true">
                    {move || if files.get().is_empty() {
                        text(ar, "No files selected.", "لم يتم اختيار ملفات.").into_any()
                    } else {
                        view! {
                            <ul>{files.get().into_iter().map(|file| view! {
                                <li>{format!("{} · {} {}", file.name, file.size, text(ar, "bytes", "بايت"))}</li>
                            }).collect_view()}</ul>
                        }.into_any()
                    }}
                </div>
            </section>
            <NavigationSection arabic=ar lang=lang dir=dir page=state.page hydrated=hydrated/>
        </article>
    }
}

#[component]
fn HelpText(arabic: bool) -> impl IntoView {
    view! {
        <p>{text(arabic, "The same component works inside any Leptos page. Labels, direction and formatted values belong to the application.",
            "يعمل المكون نفسه داخل أي صفحة Leptos. يحدد التطبيق التسميات والاتجاه وتنسيق القيم.")}</p>
        <p>{text(arabic, "Use Tab to move between dialog controls. Escape closes the hydrated dialog and returns focus to its opener.",
            "استخدم Tab للتنقل بين عناصر الحوار. يغلق Escape الحوار ويعيد التركيز إلى رابط فتحه.")}</p>
    }
}
