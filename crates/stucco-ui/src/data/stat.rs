use super::DATA;
use crate::{Icon, Tone, Trend, passthrough::apply};
use stucco_core::{Attrs, Cx, Render, el};

// Lucide `arrow-up`, `arrow-down` and `minus` (ISC; see LICENSE-LUCIDE).
const ARROW_UP: Icon = Icon::custom(
    "arrow-up",
    r#"<path d="m5 12 7-7 7 7" /><path d="M12 19V5" />"#,
);
const ARROW_DOWN: Icon = Icon::custom(
    "arrow-down",
    r#"<path d="M12 5v14" /><path d="m19 12-7 7-7-7" />"#,
);
const MINUS: Icon = Icon::custom("minus", r#"<path d="M5 12h14" />"#);

/// A metric: a label, a large value, and optionally how it changed.
///
/// The direction is in hidden text as well as the arrow. The change is
/// neutral unless [`change_tone`](Self::change_tone) says whether it is good
/// news: a rising cost is not.
///
/// ```
/// use stucco_ui::{Trend, data::Stat};
/// let html = stucco_core::to_html(&Stat::new("Orders", "128").change("12%", Trend::Up));
/// assert!(html.contains("Up </span>12%"));
/// ```
#[derive(Debug)]
pub struct Stat {
    attrs: Attrs,
    label: String,
    value: String,
    change: Option<(String, Trend)>,
    change_tone: Tone,
    context: Option<String>,
}

impl Stat {
    /// A label and its value.
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Stat {
            attrs: Attrs::default(),
            label: label.into(),
            value: value.into(),
            change: None,
            change_tone: Tone::Default,
            context: None,
        }
    }

    /// How much the value moved, and which way.
    pub fn change(mut self, text: impl Into<String>, trend: Trend) -> Self {
        self.change = Some((text.into(), trend));
        self
    }

    /// Colours the change (default neutral).
    pub fn change_tone(mut self, tone: Tone) -> Self {
        self.change_tone = tone;
        self
    }

    /// What the change is measured against, such as "vs last week".
    pub fn context(mut self, text: impl Into<String>) -> Self {
        self.context = Some(text.into());
        self
    }
}

passthrough!(Stat);

impl Render for Stat {
    fn render(&self, cx: &mut Cx) {
        cx.require(&DATA);
        let change = self.change.as_ref().map(|(text, trend)| {
            let (icon, said) = match trend {
                Trend::Up => (ARROW_UP, "Up "),
                Trend::Down => (ARROW_DOWN, "Down "),
                Trend::Flat => (MINUS, "No change "),
            };
            let delta = el::span()
                .class("st-stat-delta")
                .data("tone", self.change_tone.as_str())
                .data("trend", trend.as_str())
                .child(icon)
                .child(el::span().class("st-sr-only").text(said))
                .text(text.clone());
            let mut line = el::p().class("st-stat-change").child(delta);
            if let Some(context) = &self.context {
                line = line.text(format!(" {context}"));
            }
            line
        });
        let tile = el::div()
            .class("st-stat")
            .child(el::p().class("st-stat-label").text(self.label.clone()))
            .child(el::p().class("st-stat-value").text(self.value.clone()))
            .child(change);
        apply(tile, &self.attrs, &[]).render(cx);
    }
}
