//! Gallery sections for stucco-leptos components beyond forms and dialogs.

use leptos::prelude::*;
use stucco_leptos::{
    BreadcrumbCurrent, BreadcrumbLink, Breadcrumbs, Cell, CellKind, CursorPagination, HeaderCell,
    NavLink, Pagination, Row, StepItem, Steps, Table, TableBody, TableHead, Tabs,
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

/// Tables: a semantic table that becomes labelled cards in narrow containers.
#[component]
pub(crate) fn TablesSection(arabic: bool) -> impl IntoView {
    let rows = [
        ("Ada Lovelace", "أدا لوفلايس", "2026-10-01", "$128.00"),
        ("Grace Hopper", "غريس هوبر", "2026-10-09", "$64.50"),
        (
            "Katherine Johnson",
            "كاثرين جونسون",
            "2026-10-18",
            "$212.00",
        ),
    ];
    let customer = text(arabic, "Customer", "العميل");
    let placed = text(arabic, "Placed", "تاريخ الطلب");
    let total = text(arabic, "Total", "الإجمالي");
    view! {
        <section id="gallery-tables" class="st-surface st-stack" data-level="raised"
            data-border="" data-padding="5" data-space="4"
            aria-labelledby="gallery-tables-heading">
            <h2 id="gallery-tables-heading">{text(arabic, "Tables", "الجداول")}</h2>
            <Table caption=text(arabic, "Recent orders", "الطلبات الأخيرة") cards=true>
                <TableHead>
                    <Row>
                        <HeaderCell aria_sort="ascending">
                            <a href="#gallery-tables">{customer}</a>
                        </HeaderCell>
                        <HeaderCell kind=CellKind::Date>{placed}</HeaderCell>
                        <HeaderCell kind=CellKind::Number>{total}</HeaderCell>
                    </Row>
                </TableHead>
                <TableBody>
                    {rows.into_iter().map(|(english, arabic_name, date, amount)| view! {
                        <Row>
                            <HeaderCell label=customer>{text(arabic, english, arabic_name)}</HeaderCell>
                            <Cell kind=CellKind::Date label=placed>{date}</Cell>
                            <Cell kind=CellKind::Number label=total>{amount}</Cell>
                        </Row>
                    }).collect_view()}
                </TableBody>
            </Table>
        </section>
    }
}
