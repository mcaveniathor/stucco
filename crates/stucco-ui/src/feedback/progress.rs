use super::FEEDBACK;
use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Render, el};

/// `v` clamped into `[lo, hi]`; NaN and infinities become `lo`.
fn clamp_finite(v: f64, lo: f64, hi: f64) -> f64 {
    if v.is_finite() { v.clamp(lo, hi) } else { lo }
}

/// A number as an attribute value: `42`, not `42.0`, and never `-0`.
fn num(v: f64) -> String {
    format!("{}", v + 0.0)
}

/// How far a task has got, as a native `<progress>` with a visible label.
///
/// ```
/// use stucco_ui::feedback::Progress;
/// let html = stucco_core::to_html(&Progress::new("Import", 42.0, 100.0));
/// assert!(html.contains(r#"value="42" max="100""#));
/// ```
#[derive(Debug)]
pub struct Progress {
    attrs: Attrs,
    label: String,
    value: Option<f64>,
    max: f64,
    show_value: bool,
}

impl Progress {
    /// `value` out of `max`. A `max` that isn't positive and finite becomes 1;
    /// the value is clamped into range.
    pub fn new(label: impl Into<String>, value: f64, max: f64) -> Self {
        let max = if max.is_finite() && max > 0.0 {
            max
        } else {
            1.0
        };
        Progress {
            attrs: Attrs::default(),
            label: label.into(),
            value: Some(clamp_finite(value, 0.0, max)),
            max,
            show_value: false,
        }
    }

    /// Work whose amount isn't known yet.
    pub fn indeterminate(label: impl Into<String>) -> Self {
        Progress {
            attrs: Attrs::default(),
            label: label.into(),
            value: None,
            max: 1.0,
            show_value: false,
        }
    }

    /// Shows the percentage beside the bar.
    pub fn show_value(mut self) -> Self {
        self.show_value = true;
        self
    }
}

passthrough!(Progress);

impl Render for Progress {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FEEDBACK);
        let id = cx.id("progress");
        let percent = self
            .value
            .map(|v| format!("{}%", (v / self.max * 100.0).round() as u32));
        let mut bar = el::progress().id(id.clone());
        if let Some(value) = self.value {
            bar = bar.attr("value", num(value)).attr("max", num(self.max));
        }
        let bar = bar.text(percent.clone().unwrap_or_default());
        let shown = percent
            .filter(|_| self.show_value)
            .map(|p| el::span().class("st-progress-value").text(p));
        let root = el::div()
            .class("st-progress")
            .child(el::label().attr("for", id).text(self.label.clone()))
            .child(bar)
            .child(shown);
        apply(root, &self.attrs, &[]).render(cx);
    }
}

/// A value within a known range that isn't progress, such as storage used,
/// as a native `<meter>`. Browsers colour it by the `low`, `high` and
/// `optimum` bands.
///
/// ```
/// use stucco_ui::feedback::Meter;
/// let html = stucco_core::to_html(&Meter::new("Storage", 7.2, 0.0, 10.0).high(8.0));
/// assert!(html.contains(r#"high="8""#));
/// ```
#[derive(Debug)]
pub struct Meter {
    attrs: Attrs,
    label: String,
    value: f64,
    min: f64,
    max: f64,
    low: Option<f64>,
    high: Option<f64>,
    optimum: Option<f64>,
    text: Option<String>,
}

impl Meter {
    /// `value` between `min` and `max`. A non-finite `min` becomes 0; a `max`
    /// that isn't above `min` becomes `min + 1`; every value is clamped.
    pub fn new(label: impl Into<String>, value: f64, min: f64, max: f64) -> Self {
        let min = if min.is_finite() { min } else { 0.0 };
        let max = if max.is_finite() && max > min {
            max
        } else {
            min + 1.0
        };
        Meter {
            attrs: Attrs::default(),
            label: label.into(),
            value: clamp_finite(value, min, max),
            min,
            max,
            low: None,
            high: None,
            optimum: None,
            text: None,
        }
    }

    /// Values at or below this are low.
    pub fn low(mut self, low: f64) -> Self {
        self.low = Some(clamp_finite(low, self.min, self.max));
        self
    }

    /// Values at or above this are high.
    pub fn high(mut self, high: f64) -> Self {
        self.high = Some(clamp_finite(high, self.min, self.max));
        self
    }

    /// The best value; which band it falls in decides the colours.
    pub fn optimum(mut self, optimum: f64) -> Self {
        self.optimum = Some(clamp_finite(optimum, self.min, self.max));
        self
    }

    /// Visible text for the value, such as "7.2 of 10 GB".
    pub fn value_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }
}

passthrough!(Meter);

impl Render for Meter {
    fn render(&self, cx: &mut Cx) {
        cx.require(&FEEDBACK);
        let id = cx.id("meter");
        let mut meter = el::meter()
            .id(id.clone())
            .attr("value", num(self.value))
            .attr("min", num(self.min))
            .attr("max", num(self.max));
        for (name, v) in [
            ("low", self.low),
            ("high", self.high),
            ("optimum", self.optimum),
        ] {
            if let Some(v) = v {
                meter = meter.attr(name, num(v));
            }
        }
        let mut described = None;
        if let Some(text) = &self.text {
            let text_id = cx.id("metertext");
            meter = meter.aria("describedby", text_id.clone());
            described = Some(
                el::span()
                    .id(text_id)
                    .class("st-meter-text")
                    .text(text.clone()),
            );
        }
        let root = el::div()
            .class("st-meter")
            .child(el::label().attr("for", id).text(self.label.clone()))
            .child(meter.text(num(self.value)))
            .child(described);
        apply(root, &self.attrs, &[]).render(cx);
    }
}
