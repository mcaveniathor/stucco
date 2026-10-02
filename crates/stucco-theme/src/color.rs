//! OKLCH colours, sRGB gamut mapping and WCAG contrast.

/// A colour in the OKLCH space: lightness `l` (0–1), chroma `c` (≥ 0) and hue
/// `h` in degrees (0–360).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Color {
    /// Perceptual lightness, 0 (black) to 1 (white).
    pub l: f64,
    /// Chroma (colourfulness), 0 for greys.
    pub c: f64,
    /// Hue angle in degrees, `[0, 360)`.
    pub h: f64,
}

impl Color {
    /// Builds a colour, clamping `l` to 0–1 and `c` to ≥ 0, normalising the
    /// hue into `[0, 360)` and treating a `NaN` hue as 0.
    pub fn oklch(l: f64, c: f64, h: f64) -> Color {
        let h = if h.is_nan() { 0.0 } else { h.rem_euclid(360.0) };
        Color {
            l: if l.is_nan() { 0.0 } else { l.clamp(0.0, 1.0) },
            c: if c.is_nan() { 0.0 } else { c.max(0.0) },
            h,
        }
    }

    /// The same colour with chroma reduced until it fits sRGB, keeping
    /// lightness and hue.
    pub fn gamut_mapped(self) -> Color {
        if in_gamut(self.linear()) {
            return self;
        }
        let (mut lo, mut hi) = (0.0, self.c);
        for _ in 0..20 {
            let mid = (lo + hi) / 2.0;
            if in_gamut(Color { c: mid, ..self }.linear()) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        Color { c: lo, ..self }
    }

    /// Gamma-encoded sRGB channels in 0–1, after gamut mapping.
    pub fn to_srgb(self) -> [f64; 3] {
        self.gamut_mapped()
            .linear()
            .map(|v| encode(v.clamp(0.0, 1.0)).clamp(0.0, 1.0))
    }

    /// CSS `oklch(L% C H)` of the gamut-mapped colour.
    pub fn css(self) -> String {
        let m = self.gamut_mapped();
        format!("oklch({:.2}% {:.4} {:.2})", m.l * 100.0, m.c, m.h)
    }

    /// WCAG relative luminance.
    pub fn luminance(self) -> f64 {
        srgb_luminance(self.to_srgb())
    }

    /// Linear sRGB via OKLab (Björn Ottosson's matrices), unclamped.
    fn linear(self) -> [f64; 3] {
        let (a, b) = {
            let h = self.h.to_radians();
            (self.c * h.cos(), self.c * h.sin())
        };
        let l_ = self.l + 0.396_337_777_4 * a + 0.215_803_757_3 * b;
        let m_ = self.l - 0.105_561_345_8 * a - 0.063_854_172_8 * b;
        let s_ = self.l - 0.089_484_177_5 * a - 1.291_485_548_0 * b;
        let (l, m, s) = (l_.powi(3), m_.powi(3), s_.powi(3));
        [
            4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
            -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
            -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701_0 * s,
        ]
    }
}

/// WCAG contrast ratio between two colours (1–21, order-independent).
pub fn contrast(a: Color, b: Color) -> f64 {
    luminance_ratio(a.luminance(), b.luminance())
}

/// WCAG relative luminance of gamma-encoded sRGB channels in 0–1.
pub(crate) fn srgb_luminance(srgb: [f64; 3]) -> f64 {
    let [r, g, b] = srgb.map(decode);
    0.2126 * r + 0.7152 * g + 0.0722 * b
}

/// WCAG contrast ratio between two relative luminances.
pub(crate) fn luminance_ratio(la: f64, lb: f64) -> f64 {
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

fn in_gamut(linear: [f64; 3]) -> bool {
    linear.iter().all(|v| (-1e-4..=1.0 + 1e-4).contains(v))
}

/// sRGB transfer function (linear → gamma-encoded).
fn encode(v: f64) -> f64 {
    if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

/// Inverse sRGB transfer function (gamma-encoded → linear).
fn decode(v: f64) -> f64 {
    if v <= 0.040_45 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn known_colours_convert() {
        assert!(
            Color::oklch(1.0, 0.0, 0.0)
                .to_srgb()
                .iter()
                .all(|v| close(*v, 1.0))
        );
        let red = Color::oklch(0.62796, 0.25768, 29.2339).to_srgb();
        assert!(
            close(red[0], 1.0) && close(red[1], 0.0) && close(red[2], 0.0),
            "{red:?}"
        );
    }

    #[test]
    fn contrast_matches_wcag() {
        let (k, w) = (Color::oklch(0.0, 0.0, 0.0), Color::oklch(1.0, 0.0, 0.0));
        assert!(close(contrast(k, w), 21.0) && close(contrast(w, k), 21.0));
    }

    #[test]
    fn inputs_are_normalised() {
        assert_eq!(Color::oklch(0.5, 0.1, -30.0).h, 330.0);
        assert!(close(Color::oklch(0.5, 0.1, 725.0).h, 5.0));
        assert_eq!(Color::oklch(0.5, 0.1, f64::NAN).h, 0.0);
    }

    #[test]
    fn out_of_gamut_chroma_is_reduced() {
        let m = Color::oklch(0.7, 0.5, 145.0).gamut_mapped();
        assert!(close(m.l, 0.7) && close(m.h, 145.0) && m.c < 0.5);
        assert!(m.to_srgb().iter().all(|v| (0.0..=1.0).contains(v)));
    }

    #[test]
    fn css_uses_percent_lightness() {
        assert_eq!(
            Color::oklch(0.5, 0.1, 250.0).css(),
            "oklch(50.00% 0.1000 250.00)"
        );
    }
}
