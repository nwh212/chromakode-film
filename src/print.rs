//! ChromaGold 200's film: Kodak Gold 200 as a C-41 colour negative, printed on paper, built from
//! Kodak's data sheet (E-7022) rather than fitted.
//!
//! Kodachrome (ChromaKode 64) was a K-14 reversal film: no couplers in the emulsion (the dyes went
//! in during development, one layer at a time), no mask, viewed directly as a contrasty slide.
//! Gold is a C-41 negative, and its look comes from five things in a row:
//!
//! 1. **The layers' spectral sensitivities.** Each layer sees a broad band, overlapping its
//!    neighbours (the green-sensitive layer also has a bump in the violet). Camera colours become
//!    the three layers' exposures through a 3×3 mix worked out from the sheet's sensitivity curves
//!    against the sRGB primaries (see [`GOLD200`]); white exposes all three alike (daylight film).
//! 2. **The negative's characteristic curves.** Each layer's density against log exposure, read
//!    off the sheet (Status M): a low contrast (γ ≈ 0.6), a long straight line (two stops under to
//!    three over) and a toe, slightly different per layer. The base and the orange mask are the
//!    curves' offsets.
//! 3. **C-41 interimage effects.** DIR couplers let a layer's development hold back its
//!    neighbours'; in density, greys untouched. (Kept as a parameter; Gold's sheet curves already
//!    carry most of it.)
//! 4. **Printing.** The printer is balanced so a mid grey prints neutral, which drops out the
//!    orange mask and fog, and the paper sees 10^-density. A lab prints every frame back to a
//!    normal grey, so an under- or over-exposed negative comes back to normal brightness, but not
//!    to the normal curve: its shadows sit on the toe (flat, coloured, grainier) or its highlights
//!    on the shoulder.
//! 5. **The paper.** A steep S-curve (density against log exposure) between paper white and paper
//!    black, and dyes with unwanted absorptions, so strong colours print darker (subtractive
//!    colour). The scan sets paper white to white and paper black to black.
//!
//! Everything per pixel is lookup tables and two 3×3 mixes, cheap enough for the host's budget.

use crate::color::*;

/// The model's numbers for one film and paper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrintParams {
    /// Layer exposure (R, G, B layers) from linear sRGB.
    pub layer: [Rgb; 3],
    /// Characteristic curves: (log H, density) per layer, R, G, B.
    pub red: &'static [(f32, f32)],
    pub green: &'static [(f32, f32)],
    pub blue: &'static [(f32, f32)],
    /// Where a mid grey (18 %) lands on the curves, in log H.
    pub grey_logh: f32,
    pub interimage: f32,
    /// The paper: contrast at its mid point, white and black densities, and the reflectance a mid
    /// grey prints at.
    pub paper_gamma: f32,
    pub paper_dmin: f32,
    pub paper_dmax: f32,
    pub grey_reflectance: f32,
    /// The paper dyes' absorptions (rows: the red, green and blue light each density takes away).
    pub impurity: [Rgb; 3],
    /// The scan: colour balance (linear gains), then per-channel black point and contrast on
    /// screen values. Neutral (`[1; 3]`, `[0; 3]`, `[1; 3]`) for most films.
    pub balance: Rgb,
    pub scan_black: Rgb,
    pub scan_gamma: Rgb,
    /// How much of an off exposure's brightness the lab leaves in (0: printed fully back to
    /// normal; ChromaLome: partly, as Lomography's own ISO 100/200/400 frames show).
    pub exposure_carry: f32,
    /// Saturation of the scan (screen values, around brightness; 1: as printed).
    pub saturation: f32,
    /// The scan for a negative exposed two stops less (ChromaLome rated at 400, where the film
    /// looks right; at 100 it's over and the scan above, Lomography's chart, is the faded one).
    /// Exposures in between take a scan in between. `None`: one scan for every exposure.
    pub rated: Option<Scan>,
}

/// A scan's numbers (see [`PrintParams`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scan {
    pub balance: Rgb,
    pub scan_black: Rgb,
    pub scan_gamma: Rgb,
    pub exposure_carry: f32,
    pub saturation: f32,
}

impl PrintParams {
    /// These numbers with the scan `t` (0-1) of the way to `s`.
    pub fn toward(&self, s: &Scan, t: f32) -> Self {
        let l = |a: f32, b: f32| a + (b - a) * t;
        let l3 = |a: Rgb, b: Rgb| [l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2])];
        Self {
            balance: l3(self.balance, s.balance),
            scan_black: l3(self.scan_black, s.scan_black),
            scan_gamma: l3(self.scan_gamma, s.scan_gamma),
            exposure_carry: l(self.exposure_carry, s.exposure_carry),
            saturation: l(self.saturation, s.saturation),
            ..*self
        }
    }
}

/// The scan's black point bends in smoothly over this width (screen values), never with a corner:
/// a soft edge crossing a channel's black point (bokeh between two colours) stays smooth.
pub const SCAN_KNEE: f32 = 0.08;

fn soft_ramp(x: f32) -> f32 {
    0.5 * (x + (x * x + SCAN_KNEE * SCAN_KNEE).sqrt())
}

/// The neutral scan for films that don't need one.
pub const PLAIN_SCAN: (Rgb, Rgb, Rgb, f32) = ([1.0; 3], [0.0; 3], [1.0; 3], 0.0);

/// Kodak Gold 200, from E-7022.
///
/// The layer mix: the sheet's three log-sensitivity curves, digitized every 5–15 nm, against the
/// sRGB primaries as smooth spectra (least-norm from the CIE 1931 colour-matching functions),
/// each row scaled so white exposes the layer fully.
pub const GOLD200: PrintParams = PrintParams {
    layer: [
        [0.845, 0.127, 0.028],
        [0.020, 0.863, 0.117],
        [0.021, 0.088, 0.891],
    ],
    // "Characteristic Curves", Daylight, Status M (log H in lux-seconds)
    red: &[
        (-3.0, 0.26),
        (-2.5, 0.30),
        (-2.0, 0.48),
        (-1.5, 0.72),
        (-1.0, 0.96),
        (-0.5, 1.22),
        (0.0, 1.56),
        (0.5, 1.71),
        (0.85, 1.84),
    ],
    green: &[
        (-3.0, 0.66),
        (-2.5, 0.70),
        (-2.0, 0.93),
        (-1.5, 1.19),
        (-1.0, 1.42),
        (-0.5, 1.70),
        (0.0, 2.04),
        (0.5, 2.19),
        (0.85, 2.31),
    ],
    blue: &[
        (-3.0, 1.00),
        (-2.75, 0.97),
        (-2.5, 1.07),
        (-2.0, 1.19),
        (-1.5, 1.49),
        (-1.2, 1.74),
        (-1.0, 1.83),
        (-0.5, 2.11),
        (0.0, 2.45),
        (0.5, 2.62),
        (0.85, 2.75),
    ],
    grey_logh: -0.7,
    interimage: 0.0,
    paper_gamma: 2.8,
    paper_dmin: 0.06,
    paper_dmax: 2.2,
    grey_reflectance: 0.30,
    impurity: [[1.0, 0.08, 0.03], [0.04, 1.0, 0.18], [0.0, 0.03, 1.0]],
    balance: PLAIN_SCAN.0,
    scan_black: PLAIN_SCAN.1,
    scan_gamma: PLAIN_SCAN.2,
    exposure_carry: PLAIN_SCAN.3,
    saturation: 1.0,
    rated: None,
};

/// Kodak VISION3 500T 5219/7219, the stock CineStill 800T is made from (its rem-jet backing
/// removed so it runs in C-41), from Kodak's technical data (H-1-5219).
///
/// The layer mix: the sheet's "Spectral Sensitivity Curves" against the sRGB primaries, as for
/// Gold; rows scaled so white exposes every layer alike, i.e. the photo as if shot under the
/// 3200 K light the film is balanced for (the Hue control adds 800T's usual green-blue lean). The curves: "Sensitometric Curves", 3200 K tungsten, ECN-2: a very soft negative
/// (γ ≈ 0.45) made to be printed or scanned with a lot of contrast, so the paper is steeper than
/// Gold's. Rated 800 rather than 500, so a mid grey sits ⅔ stop lower on the curves.
pub const STILL800T: PrintParams = PrintParams {
    layer: [
        [0.884, 0.113, 0.003],
        [0.028, 0.897, 0.075],
        [0.032, 0.074, 0.894],
    ],
    red: &[
        (-4.0, 0.18),
        (-3.5, 0.19),
        (-3.0, 0.27),
        (-2.6, 0.39),
        (-2.0, 0.65),
        (-1.5, 0.86),
        (-1.0, 1.06),
        (-0.5, 1.27),
        (0.0, 1.47),
        (0.5, 1.74),
        (1.0, 1.86),
    ],
    green: &[
        (-4.0, 0.57),
        (-3.5, 0.58),
        (-3.0, 0.65),
        (-2.6, 0.79),
        (-2.0, 1.04),
        (-1.5, 1.27),
        (-1.0, 1.49),
        (-0.5, 1.72),
        (0.0, 1.95),
        (0.5, 2.31),
        (1.0, 2.58),
    ],
    blue: &[
        (-4.0, 0.83),
        (-3.5, 0.84),
        (-3.0, 0.92),
        (-2.6, 1.04),
        (-2.0, 1.29),
        (-1.5, 1.51),
        (-1.0, 1.74),
        (-0.5, 1.96),
        (0.0, 2.19),
        (0.5, 2.63),
        (1.0, 2.76),
    ],
    grey_logh: -1.7,
    interimage: 0.0,
    paper_gamma: 4.0,
    paper_dmin: 0.06,
    paper_dmax: 2.2,
    grey_reflectance: 0.32,
    impurity: [[1.0, 0.08, 0.03], [0.04, 1.0, 0.18], [0.0, 0.03, 1.0]],
    balance: PLAIN_SCAN.0,
    scan_black: PLAIN_SCAN.1,
    scan_gamma: PLAIN_SCAN.2,
    exposure_carry: PLAIN_SCAN.3,
    saturation: 1.0,
    rated: None,
};

/// LomoChrome Purple: a C-41 negative whose layers see swapped light, so greens print purple,
/// blues green-teal and yellows pink, while reds stay red. Lomography publishes no curves, so the
/// negative and paper are Gold 200's (an ordinary C-41 film); the swap and the scan are fitted to
/// a ColorChecker shot on LomoChrome Purple at ISO 100, from Lomography's own film guide (all 24
/// patches, 8.2/255 rms): the red layer sees red plus half the green (foliage goes purple), the
/// green layer mostly blue (skies and blues go teal), the blue layer green and blue; the scan
/// crushes red in the shadows (Purple's blue-teal darks), through a smooth knee. Every step is a mix in linear light or
/// a smooth curve per channel, so a blurred edge between two colours goes smoothly through the
/// colours between them.
pub const PURPLE: PrintParams = PrintParams {
    layer: [
        [1.3878, 0.5488, 0.2300],
        [0.1238, 0.0777, 1.3213],
        [0.0767, 0.5777, 0.6404],
    ],
    red: GOLD200.red,
    green: GOLD200.green,
    blue: GOLD200.blue,
    grey_logh: -0.7,
    interimage: 0.0,
    paper_gamma: 2.8,
    paper_dmin: 0.06,
    paper_dmax: 2.2,
    grey_reflectance: 0.30,
    impurity: [[1.0, 0.08, 0.03], [0.04, 1.0, 0.18], [0.0, 0.03, 1.0]],
    balance: [1.2029, 1.0421, 0.9312],
    scan_black: [0.3822, 0.0818, 0.0686],
    scan_gamma: [0.7561, 1.1141, 0.9421],
    exposure_carry: 0.6,
    saturation: 1.0,
    rated: Some(PURPLE_400),
};

/// LomoChrome Purple's scan two stops under the chart, at its rated 400: printed back to a normal
/// brightness, whites clean and a little lavender, foliage a deep violet, darks slate teal, more
/// colour.
pub const PURPLE_400: Scan = Scan {
    balance: [0.8856, 0.7348, 0.6340],
    scan_black: [-0.0200, -0.0200, -0.0200],
    scan_gamma: [1.7000, 1.1280, 0.8123],
    exposure_carry: 0.0,
    saturation: 1.50,
};

/// The film and paper as lookup tables, for one exposure.
pub struct Print {
    layer: [Rgb; 3],
    /// √(layer exposure) → paper density, per layer: the negative's curve, the printer's balance
    /// and the paper's curve in one table. (With interimage on, the negative and paper are two
    /// tables, as the interimage mix sits between them.)
    film: [Lut; 3],
    neg: Option<([Lut; 3], [Lut; 3])>,
    interimage: f32,
    impurity: [Rgb; 3],
    /// The scan's saturation (applied by the kit, after the print).
    pub saturation: f32,
    /// Paper density (after the dyes' absorptions) / [`D_SCALE`] → screen value of the scan, per
    /// channel (the scan's colour balance folded in).
    out: [Lut; 3],
}

const D_SCALE: f32 = 4.0;

/// A small lookup table over 0–1 (the film's and paper's curves are smooth, so 513 entries are
/// plenty, and building them for every band the host sends stays cheap).
struct Lut {
    v: [f32; LUT + 1],
}

const LUT: usize = 512;

impl Lut {
    fn from_fn(f: impl Fn(f32) -> f32) -> Self {
        Lut {
            v: std::array::from_fn(|i| f(i as f32 / LUT as f32)),
        }
    }

    #[inline]
    fn eval(&self, x: f32) -> f32 {
        let f = x.clamp(0.0, 1.0) * LUT as f32;
        let i = (f as usize).min(LUT - 1);
        let a = self.v[i];
        a + (self.v[i + 1] - a) * (f - i as f32)
    }
}

impl Print {
    /// The model for a negative exposed `stops` away from normal (and printed back to normal).
    pub fn new(p: &PrintParams, stops: f32) -> Self {
        Self::balanced(p, stops, [1.0; 3])
    }

    /// [`Print::new`] with the print's or scan's colour balance: linear gains on its output.
    pub fn balanced(p: &PrintParams, stops: f32, balance: Rgb) -> Self {
        let rated = p
            .rated
            .filter(|_| stops < 0.0)
            .map(|s| p.toward(&s, (-stops / 2.0).min(1.0)));
        let p = rated.as_ref().unwrap_or(p);
        let shift = stops * std::f32::consts::LOG10_2;
        let curves = [p.red, p.green, p.blue].map(extended);
        let hd: Vec<_> = curves.iter().map(|c| monotone_cubic(c)).collect();
        let density =
            |k: usize, lin: f32| hd[k]((lin.max(1e-6) / 0.18).log10() + p.grey_logh + shift);
        // the lab prints a mid grey of *this* negative neutral and normal
        let grey: [f32; 3] = std::array::from_fn(|k| {
            let row = p.layer[k][0] + p.layer[k][1] + p.layer[k][2];
            density(
                k,
                if (row - 1.0).abs() > 1e-4 {
                    0.18 * row
                } else {
                    0.18
                },
            )
        });
        let grey_mean = (grey[0] + grey[1] + grey[2]) / 3.0;
        let grey_d = grey.map(|g| g + p.interimage * (g - grey_mean));
        let neg_of = |k: usize| Lut::from_fn(|u| density(k, u * u));
        // paper: logistic in log exposure, through the mid grey's density at the balance point
        let (dmin, dmax) = (p.paper_dmin, p.paper_dmax);
        let kp = 4.0 * p.paper_gamma / (dmax - dmin);
        let grey_paper = -p.grey_reflectance.log10();
        let x0 = ((dmax - dmin) / (grey_paper - dmin) - 1.0).ln() / kp;
        let paper_d = move |k: usize, d: f32| {
            dmin + (dmax - dmin) / (1.0 + (-kp * (-(d - grey_d[k]) - x0)).exp())
        };
        let (film, neg) = if p.interimage == 0.0 {
            (
                std::array::from_fn(|k| Lut::from_fn(|u| paper_d(k, density(k, u * u)))),
                None,
            )
        } else {
            let paper: [Lut; 3] =
                std::array::from_fn(|k| Lut::from_fn(move |v| paper_d(k, v * D_SCALE)));
            let negs: [Lut; 3] = std::array::from_fn(neg_of);
            (
                std::array::from_fn(|_| Lut::from_fn(|_| 0.0)),
                Some((negs, paper)),
            )
        };
        // the dyes' unwanted absorptions, rows scaled so greys stay grey
        let impurity = p.impurity.map(|r| {
            let s = r[0] + r[1] + r[2];
            [r[0] / s, r[1] / s, r[2] / s]
        });
        let (white, black) = (10f32.powf(-dmin), 10f32.powf(-dmax));
        let carry = if p.exposure_carry != 0.0 {
            2f32.powf(p.exposure_carry * stops)
        } else {
            1.0
        };
        let out = std::array::from_fn(|k| {
            let g = if p.balance[k] != 1.0 || carry != 1.0 {
                balance[k] * p.balance[k] * carry
            } else {
                balance[k]
            };
            let (sb, sg) = (p.scan_black[k], p.scan_gamma[k]);
            let plain = sb == 0.0 && sg == 1.0;
            Lut::from_fn(move |w| {
                let v = encode_exact(
                    (((10f32.powf(-w * D_SCALE) - black) / (white - black)).clamp(0.0, 1.0) * g)
                        .min(1.0),
                );
                if plain {
                    v
                } else {
                    (soft_ramp(v - sb) / soft_ramp(1.0 - sb))
                        .clamp(0.0, 1.0)
                        .powf(sg)
                }
            })
        });
        Print {
            layer: p.layer,
            film,
            neg,
            interimage: p.interimage,
            impurity,
            saturation: p.saturation,
            out,
        }
    }

    /// One screen-value pixel through film, printer, paper and scan. `t`: the sRGB transfer.
    pub fn apply(&self, c: Rgb, t: &Transfer) -> Rgb {
        let lin = [t.to_linear(c[0]), t.to_linear(c[1]), t.to_linear(c[2])];
        let e = mix3(&self.layer, lin);
        let u = e.map(|v| v.clamp(0.0, 1.0).sqrt());
        let dp: Rgb = match &self.neg {
            None => std::array::from_fn(|k| self.film[k].eval(u[k])),
            Some((neg, paper)) => {
                let d: Rgb = std::array::from_fn(|k| neg[k].eval(u[k]));
                let m = (d[0] + d[1] + d[2]) / 3.0;
                std::array::from_fn(|k| {
                    paper[k].eval((d[k] + self.interimage * (d[k] - m)) / D_SCALE)
                })
            }
        };
        let dout = mix3(&self.impurity, dp);
        std::array::from_fn(|k| self.out[k].eval(dout[k] / D_SCALE))
    }
}

/// A characteristic curve held flat below its first point and continued along its last slope above
/// its last, so any exposure has a density.
fn extended(pts: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let n = pts.len();
    let (a, b) = (pts[n - 2], pts[n - 1]);
    let slope = (b.1 - a.1) / (b.0 - a.0);
    let mut v = vec![(pts[0].0 - 3.0, pts[0].1)];
    v.extend_from_slice(pts);
    v.push((b.0 + 3.0, b.1 + 3.0 * slope));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greys_print_grey_and_black_to_white_spans_the_paper() {
        let t = Transfer::new();
        let pr = Print::new(&GOLD200, 0.0);
        // neutral from the midtones up
        for v in [0.46, 0.7, 0.9] {
            let o = pr.apply([v; 3], &t);
            assert!(
                (o[0] - o[1]).abs() < 0.02 && (o[1] - o[2]).abs() < 0.02,
                "{v}: {o:?}"
            );
        }
        // the toe: dark greys print warm brown (red above blue), within reason
        for v in [0.05, 0.2] {
            let o = pr.apply([v; 3], &t);
            assert!(o[0] >= o[2] && o[0] - o[2] < 0.08, "{v}: {o:?}");
        }
        let mut last = -1.0;
        for i in 0..=20 {
            let o = pr.apply([i as f32 / 20.0; 3], &t)[1];
            assert!(o >= last - 1e-4, "never darker as the scene gets brighter");
            last = o;
        }
        assert!(pr.apply([0.0; 3], &t)[1] < 0.06);
        assert!(pr.apply([1.0; 3], &t)[1] > 0.9);
    }

    #[test]
    fn an_off_exposure_prints_back_to_a_normal_mid_grey() {
        let t = Transfer::new();
        let normal = Print::new(&GOLD200, 0.0).apply([0.46; 3], &t);
        for stops in [-2.0, 3.0] {
            let o = Print::new(&GOLD200, stops).apply([0.46; 3], &t);
            assert!(
                (o[1] - normal[1]).abs() < 0.03,
                "{stops}: {o:?} vs {normal:?}"
            );
        }
    }
}
