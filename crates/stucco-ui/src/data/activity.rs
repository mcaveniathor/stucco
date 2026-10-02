//! Times and the history of a record.

use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Render, Slot, el};

/// A point in time: readable text for people and a machine-readable
/// `datetime` for software, in a `<time>` element.
///
/// Stucco does not format dates or convert time zones; the application
/// passes both forms. Name the time zone in the text, or with
/// [`Timestamp::zone`], whenever the time of day is shown: "10:00" alone is
/// ambiguous to anyone in another zone.
///
/// ```
/// use stucco_core::to_html;
/// use stucco_ui::data::Timestamp;
///
/// let html = to_html(&Timestamp::new("2026-09-04T10:00:00Z", "4 Sep 2026, 10:00").zone("UTC"));
/// assert_eq!(
///     html,
///     r#"<time class="st-time" datetime="2026-09-04T10:00:00Z">4 Sep 2026, 10:00 <span class="st-time-zone">UTC</span></time>"#
/// );
/// ```
#[derive(Clone, Debug)]
pub struct Timestamp {
    attrs: Attrs,
    datetime: String,
    text: String,
    zone: Option<String>,
}

impl Timestamp {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["datetime"];

    /// `text` for people, `datetime` (an ISO 8601 date, time or date-time)
    /// for software.
    pub fn new(datetime: impl Into<String>, text: impl Into<String>) -> Self {
        Timestamp {
            attrs: Attrs::default(),
            datetime: datetime.into(),
            text: text.into(),
            zone: None,
        }
    }

    /// A date shown as given (`YYYY-MM-DD`): no time of day, so no zone.
    pub fn date(date: impl Into<String>) -> Self {
        let date = date.into();
        Timestamp::new(date.clone(), date)
    }

    /// Names the time zone after the text ("UTC", "Europe/Paris").
    pub fn zone(mut self, zone: impl Into<String>) -> Self {
        self.zone = Some(zone.into());
        self
    }
}

passthrough!(Timestamp);

impl Render for Timestamp {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::DATA);
        let mut time = el::time()
            .class("st-time")
            .attr("datetime", &self.datetime)
            .text(&self.text);
        if let Some(zone) = &self.zone {
            time = time
                .text(" ")
                .child(el::span().class("st-time-zone").text(zone));
        }
        apply(time, &self.attrs, Self::RESERVED).render(cx);
    }
}

/// One event in an [`ActivityList`]: who did what, to what, and when.
#[derive(Debug)]
pub struct Activity<'a> {
    actor: Slot<'a>,
    action: Slot<'a>,
    target: Option<Slot<'a>>,
    time: Timestamp,
    detail: Option<Slot<'a>>,
}

impl<'a> Activity<'a> {
    /// `actor` did `action` at `time` ("Ada", "changed the status", …).
    pub fn new(actor: impl Render + 'a, action: impl Render + 'a, time: Timestamp) -> Self {
        Activity {
            actor: Slot::new(actor),
            action: Slot::new(action),
            target: None,
            time,
            detail: None,
        }
    }

    /// What the action was done to, when the list mixes records.
    pub fn target(mut self, target: impl Render + 'a) -> Self {
        self.target = Some(Slot::new(target));
        self
    }

    /// More about the event, behind a "Details" disclosure.
    ///
    /// Activity is often kept for a long time and shown to many people:
    /// leave secrets, tokens and passwords out of it.
    pub fn detail(mut self, detail: impl Render + 'a) -> Self {
        self.detail = Some(Slot::new(detail));
        self
    }
}

/// A record's history, newest first by convention: an ordered list of
/// [`Activity`] events, each with its actor, action, optional target, time
/// and optional details in a native disclosure.
///
/// ```
/// use stucco_core::to_html;
/// use stucco_ui::data::{Activity, ActivityList, Timestamp};
///
/// let list = ActivityList::new("Order history")
///     .item(Activity::new("Ada", "marked the order paid", Timestamp::date("2026-09-04")));
/// let html = to_html(&list);
/// assert!(html.contains(r#"<ol class="st-activity-list" aria-label="Order history">"#));
/// assert!(html.contains("marked the order paid"));
/// ```
#[derive(Debug)]
pub struct ActivityList<'a> {
    attrs: Attrs,
    label: String,
    items: Vec<Activity<'a>>,
    empty: String,
}

impl<'a> ActivityList<'a> {
    /// Reserved attributes (set by the component).
    pub const RESERVED: &'static [&'static str] = &["aria-label"];

    /// An empty list named `label` for assistive technology.
    pub fn new(label: impl Into<String>) -> Self {
        ActivityList {
            attrs: Attrs::default(),
            label: label.into(),
            items: Vec::new(),
            empty: "No activity yet.".to_owned(),
        }
    }

    /// Adds an event.
    pub fn item(mut self, activity: Activity<'a>) -> Self {
        self.items.push(activity);
        self
    }

    /// Adds every event.
    pub fn items(mut self, activities: impl IntoIterator<Item = Activity<'a>>) -> Self {
        self.items.extend(activities);
        self
    }

    /// The text shown when there are no events (default "No activity yet.").
    pub fn empty(mut self, text: impl Into<String>) -> Self {
        self.empty = text.into();
        self
    }
}

passthrough!(ActivityList<'_>);

impl Render for ActivityList<'_> {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::DATA);
        if self.items.is_empty() {
            let empty = el::p().class("st-activity-empty").text(&self.empty);
            apply(empty, &self.attrs, Self::RESERVED).render(cx);
            return;
        }
        let list = el::ol()
            .class("st-activity-list")
            .aria("label", &self.label)
            .children(self.items.iter().map(|item| {
                let summary = el::p()
                    .class("st-activity-summary")
                    .child(el::span().class("st-activity-actor").child(&item.actor))
                    .text(" ")
                    .child(&item.action)
                    .child(item.target.as_ref().map(|target| {
                        el::span()
                            .class("st-activity-target")
                            .text(" ")
                            .child(target)
                    }));
                el::li()
                    .class("st-activity")
                    .child(summary)
                    .child(&item.time)
                    .child(item.detail.as_ref().map(|detail| {
                        el::details()
                            .class("st-activity-detail")
                            .child(el::summary().text("Details"))
                            .child(el::div().child(detail))
                    }))
            }));
        apply(list, &self.attrs, Self::RESERVED).render(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stucco_core::to_html;

    #[test]
    fn activities_show_actor_action_target_time_and_details() {
        let html = to_html(
            &ActivityList::new("History").item(
                Activity::new("Ada <admin>", "archived", Timestamp::date("2026-09-04"))
                    .target("order 7")
                    .detail("Reason: duplicate"),
            ),
        );
        assert_eq!(
            html,
            concat!(
                r#"<ol class="st-activity-list" aria-label="History"><li class="st-activity">"#,
                r#"<p class="st-activity-summary"><span class="st-activity-actor">Ada &lt;admin&gt;</span> archived"#,
                r#"<span class="st-activity-target"> order 7</span></p>"#,
                r#"<time class="st-time" datetime="2026-09-04">2026-09-04</time>"#,
                r#"<details class="st-activity-detail"><summary>Details</summary><div>Reason: duplicate</div></details>"#,
                "</li></ol>"
            )
        );
    }

    #[test]
    fn an_empty_history_says_so() {
        let html = to_html(&ActivityList::new("History").empty("Nothing has happened."));
        assert_eq!(
            html,
            r#"<p class="st-activity-empty">Nothing has happened.</p>"#
        );
    }
}
