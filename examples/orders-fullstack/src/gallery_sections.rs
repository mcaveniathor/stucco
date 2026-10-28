//! Gallery sections for stucco-leptos components beyond forms and dialogs.

use leptos::prelude::*;
use stucco_leptos::{
    BreadcrumbCurrent, BreadcrumbLink, Breadcrumbs, CursorPagination, NavLink, Pagination,
    StepItem, Steps, Tabs,
};

use crate::gallery::text;

const PAGES: u64 = 10;

/// Navigation components: links first, with a button flipping their reactive state.
#[component]
pub(crate) fn NavigationSection(
    arabic: bool,
    lang: &'static str,
    dir: &'static str,
    page: u64,
    hydrated: RwSignal<bool>,
) -> impl IntoView {
    let reports = RwSignal::new(false);
    let step = RwSignal::new(0usize);
    let base = format!("/components?lang={lang}&dir={dir}");
    let page_href = {
        let base = base.clone();
        Callback::new(move |page| format!("{base}&page={page}#gallery-navigation"))
    };
    let previous = (page > 1).then(|| format!("{base}&page={}#gallery-navigation", page - 1));
    let next = (page < PAGES).then(|| format!("{base}&page={}#gallery-navigation", page + 1));
    let steps = vec![
        StepItem {
            label: text(arabic, "Cart", "السلة").into(),
            href: Some(format!("{base}#gallery-navigation")),
        },
        StepItem {
            label: text(arabic, "Payment", "الدفع").into(),
            href: Some(format!("{base}#gallery-navigation")),
        },
        StepItem {
            label: text(arabic, "Done", "تم").into(),
            href: None,
        },
    ];
    let section_href = format!("{base}#gallery-navigation");
    view! {
        <section id="gallery-navigation" class="st-surface st-stack" data-level="raised"
            data-border="" data-padding="5" data-space="4"
            aria-labelledby="gallery-navigation-heading">
            <h2 id="gallery-navigation-heading">{text(arabic, "Navigation", "التنقل")}</h2>
            <Breadcrumbs label=text(arabic, "Gallery location", "موقع المعرض")>
                <BreadcrumbLink href="/orders">{text(arabic, "Orders", "الطلبات")}</BreadcrumbLink>
                <BreadcrumbLink href="/components">{text(arabic, "Components", "المكونات")}</BreadcrumbLink>
                <BreadcrumbCurrent>{text(arabic, "Navigation", "التنقل")}</BreadcrumbCurrent>
            </Breadcrumbs>
            <Tabs label=text(arabic, "Gallery sections", "أقسام المعرض")>
                <NavLink label=text(arabic, "Overview", "نظرة عامة") href=section_href.clone()
                    current=Signal::derive(move || !reports.get())/>
                <NavLink label=text(arabic, "Reports", "التقارير") href=section_href
                    current=Signal::derive(move || reports.get())/>
            </Tabs>
            <div class="st-cluster" data-space="3">
                <button id="gallery-mark-reports" type="button" class="st-button"
                    data-variant="secondary" data-size="md" disabled=move || !hydrated.get()
                    on:click=move |_| reports.update(|on| *on = !*on)>
                    {text(arabic, "Toggle current tab", "تبديل التبويب الحالي")}
                </button>
            </div>
            <Steps label=text(arabic, "Checkout progress", "تقدم الدفع") steps=steps
                current=Signal::derive(move || step.get())
                completed_label=text(arabic, " (completed)", " (مكتمل)")
                not_started_label=text(arabic, " (not started)", " (لم يبدأ)")/>
            <div class="st-cluster" data-space="3">
                <button id="gallery-next-step" type="button" class="st-button"
                    data-variant="secondary" data-size="md" disabled=move || !hydrated.get()
                    on:click=move |_| step.update(|step| *step = (*step + 1).min(2))>
                    {text(arabic, "Next step", "الخطوة التالية")}
                </button>
            </div>
            <Pagination label=text(arabic, "Gallery pages", "صفحات المعرض") page=page
                total_pages=PAGES href=page_href
                page_label=Callback::new(move |page| format!("{} {page}", text(arabic, "Page", "الصفحة")))/>
            <CursorPagination label=text(arabic, "Gallery cursor pages", "صفحات المؤشر")
                previous_href=previous next_href=next
                previous_label=text(arabic, "Previous", "السابق") next_label=text(arabic, "Next", "التالي")/>
        </section>
    }
}
