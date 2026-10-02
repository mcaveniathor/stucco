//! 12-step colour scales for light and dark schemes.

use crate::Color;

/// Light or dark colour scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scheme {
    /// Light backgrounds, dark text.
    Light,
    /// Dark backgrounds, light text.
    Dark,
}

impl Scheme {
    /// Both schemes, light first.
    pub const BOTH: [Scheme; 2] = [Scheme::Light, Scheme::Dark];
}

const LIGHT_L: [f64; 12] = [
    0.99, 0.975, 0.95, 0.92, 0.885, 0.845, 0.79, 0.71, 0.60, 0.55, 0.45, 0.24,
];
const DARK_L: [f64; 12] = [
    0.16, 0.19, 0.23, 0.26, 0.30, 0.34, 0.40, 0.48, 0.60, 0.66, 0.80, 0.95,
];
const CHROMA: [f64; 12] = [
    0.10, 0.15, 0.25, 0.35, 0.45, 0.55, 0.65, 0.80, 1.0, 1.0, 0.85, 0.45,
];

/// A 12-step scale of one hue, for both schemes.
///
/// Steps: 1–2 app backgrounds, 3–5 component backgrounds, 6–8 borders,
/// 9–10 solid fills, 11 low-contrast text, 12 high-contrast text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scale {
    /// Light-scheme steps 1–12.
    pub light: [Color; 12],
    /// Dark-scheme steps 1–12.
    pub dark: [Color; 12],
}

impl Scale {
    /// A scale of `hue` whose solid steps (9–10) have `chroma`.
    pub fn new(hue: f64, chroma: f64) -> Scale {
        let build =
            |ls: [f64; 12]| std::array::from_fn(|i| Color::oklch(ls[i], chroma * CHROMA[i], hue));
        Scale {
            light: build(LIGHT_L),
            dark: build(DARK_L),
        }
    }

    /// Step `n` (1–12) in `scheme`. Panics outside 1–12.
    pub fn step(&self, scheme: Scheme, n: usize) -> Color {
        assert!((1..=12).contains(&n), "scale step out of range: {n}");
        match scheme {
            Scheme::Light => self.light[n - 1],
            Scheme::Dark => self.dark[n - 1],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contrast;

    #[test]
    fn steps_follow_the_curves() {
        let s = Scale::new(250.0, 0.15);
        assert_eq!(s.step(Scheme::Light, 1).l, 0.99);
        assert_eq!(s.step(Scheme::Dark, 12).l, 0.95);
        assert_eq!(s.step(Scheme::Light, 9).c, 0.15);
        assert!((s.step(Scheme::Light, 1).c - 0.015).abs() < 1e-9);
    }

    #[test]
    fn text_steps_contrast_with_backgrounds() {
        for hue in [0.0, 60.0, 145.0, 250.0, 320.0] {
            let s = Scale::new(hue, 0.15);
            for scheme in Scheme::BOTH {
                assert!(
                    contrast(s.step(scheme, 12), s.step(scheme, 1)) >= 7.0,
                    "{hue} {scheme:?}"
                );
                assert!(
                    contrast(s.step(scheme, 11), s.step(scheme, 2)) >= 4.5,
                    "{hue} {scheme:?}"
                );
            }
        }
    }
}
