use super::DATA;
use crate::passthrough::apply;
use stucco_core::{Attrs, Cx, Render, el};

/// A compact trend with a visible text equivalent and no pointer interaction.
/// Non-finite samples are omitted. Empty series say “No data”, a single sample
/// is a centered point, and a constant series is a horizontal line.
#[derive(Debug)]
pub struct Sparkline {
    attrs: Attrs,
    summary: String,
    values: Vec<f64>,
}

impl Sparkline {
    /// `summary` describes the measure, period and trend in words. Supply
    /// already ordered samples; this component never fetches or aggregates data.
    pub fn new(summary: impl Into<String>, values: impl IntoIterator<Item = f64>) -> Self {
        Self {
            attrs: Attrs::default(),
            summary: summary.into(),
            values: values.into_iter().filter(|v| v.is_finite()).collect(),
        }
    }
}
passthrough!(Sparkline);
impl Render for Sparkline {
    fn render(&self, cx: &mut Cx) {
        cx.require(&DATA);
        let mut figure = el::figure()
            .class("st-sparkline")
            .child(el::figcaption().text(self.summary.clone()));
        if self.values.is_empty() {
            figure = figure.child(el::p().text("No data"));
        } else {
            // Normalize before subtraction so opposite f64 extrema cannot overflow.
            let scale = self.values.iter().map(|v| v.abs()).fold(1.0, f64::max);
            let lo = self
                .values
                .iter()
                .map(|v| v / scale)
                .fold(f64::INFINITY, f64::min);
            let hi = self
                .values
                .iter()
                .map(|v| v / scale)
                .fold(f64::NEG_INFINITY, f64::max);
            let points = self
                .values
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let x = if self.values.len() == 1 {
                        100.0
                    } else {
                        4.0 + 192.0 * i as f64 / (self.values.len() - 1) as f64
                    };
                    let y = if hi == lo {
                        30.0
                    } else {
                        56.0 - 52.0 * (v / scale - lo) / (hi - lo)
                    };
                    format!("{x:.2},{y:.2}")
                })
                .collect::<Vec<_>>()
                .join(" ");
            let svg = el::svg()
                .attr("viewBox", "0 0 200 60")
                .aria("hidden", "true")
                .attr("focusable", "false")
                .child(if self.values.len() == 1 {
                    el::circle()
                        .attr("cx", "100")
                        .attr("cy", "30")
                        .attr("r", "3")
                } else {
                    el::polyline()
                        .attr("points", points)
                        .attr("fill", "none")
                        .attr("stroke", "currentColor")
                        .attr("stroke-width", "2")
                        .attr("vector-effect", "non-scaling-stroke")
                });
            figure = figure
                .child(svg)
                .child(el::p().class("st-sr-only").text(format!(
                    "Values in order: {}",
                    self.values
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )));
        }
        apply(figure, &self.attrs, &[]).render(cx);
    }
}
