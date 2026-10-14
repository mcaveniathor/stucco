use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Href, Render, el};

/// One application-evaluated prerequisite, with an optional remedy and role guidance.
#[derive(Debug)]
pub struct Requirement {
    label: String,
    ready: bool,
    reason: Option<String>,
    guidance: Option<String>,
    remedy: Option<(String, Href)>,
}
impl Requirement {
    /// A satisfied prerequisite.
    pub fn ready(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ready: true,
            reason: None,
            guidance: None,
            remedy: None,
        }
    }
    /// An unmet prerequisite and its concrete reason.
    pub fn blocked(label: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            ready: false,
            reason: Some(reason.into()),
            ..Self::ready(label)
        }
    }
    /// Guidance such as the role or person who can resolve a blocker.
    pub fn guidance(mut self, text: impl Into<String>) -> Self {
        self.guidance = Some(text.into());
        self
    }
    /// An ordinary link to the remedy. Use a context-carrying URL when needed.
    pub fn remedy(mut self, label: impl Into<String>, href: impl Into<Href>) -> Self {
        self.remedy = Some((label.into(), href.into()));
        self
    }
}

/// A labelled snapshot of prerequisites for an action, including concrete blockers.
/// The application owns eligibility, permission checks and submission controls;
/// always recheck prerequisites when the action executes.
#[derive(Debug)]
pub struct ReadinessPanel {
    attrs: Attrs,
    title: String,
    requirements: Vec<Requirement>,
}
impl ReadinessPanel {
    /// Creates a panel. With no requirements it says no checks are available.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            attrs: Attrs::default(),
            title: title.into(),
            requirements: vec![],
        }
    }
    /// Appends a prerequisite in display order.
    pub fn requirement(mut self, requirement: Requirement) -> Self {
        self.requirements.push(requirement);
        self
    }
}
passthrough!(ReadinessPanel);
impl Render for ReadinessPanel {
    fn render(&self, cx: &mut Cx) {
        cx.require(&super::FEEDBACK);
        let id = cx.id("readiness");
        let ready = self.requirements.iter().filter(|r| r.ready).count();
        let summary = if self.requirements.is_empty() {
            "No checks available.".to_owned()
        } else {
            format!("{ready} of {} checks complete.", self.requirements.len())
        };
        let mut panel = el::section()
            .class("st-readiness")
            .aria("labelledby", &id)
            .child(el::h2().id(id).text(&self.title))
            .child(el::p().text(summary));
        if !self.requirements.is_empty() {
            panel = panel.child(el::ul().class("st-readiness-list").children(
                self.requirements.iter().map(|r| {
                    el::li()
                        .data("ready", if r.ready { "true" } else { "false" })
                        .child(
                            el::div()
                                .class("st-readiness-label")
                                .child(el::strong().text(&r.label))
                                .child(el::span().class("st-readiness-status").text(if r.ready {
                                    "Ready"
                                } else {
                                    "Action needed"
                                })),
                        )
                        .child(r.reason.as_ref().map(|s| el::p().text(s)))
                        .child(r.guidance.as_ref().map(|s| el::p().text(s)))
                        .child(
                            r.remedy
                                .as_ref()
                                .map(|(label, href)| el::a().href(href.clone()).text(label)),
                        )
                }),
            ));
        }
        apply(panel, &self.attrs, &["aria-labelledby"]).render(cx);
    }
}
