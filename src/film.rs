//! The film's own character beyond its colour: the highlight shoulder,
//! halation, softness and grain. These are part of the base look (on by default, each with its
//! own slider), not modifiers.
//!
//! Every size here is for a 2400 px long side and scales with the photo, so a preview and a big
//! export look alike. Grain is fixed by pixel position, and every neighbourhood step stays inside
//! the host's band overlap, so bands join without seams.

use crate::color::*;
use crate::denoise::box_mean;

/// The long side the sizes below are for.
pub const REFERENCE_LONG_SIDE: f32 = 2400.0;

/// Highlight shoulder: where linear values start rolling off when Normalize brightens a photo.
pub const SHOULDER_KNEE: f32 = 0.5;

/// Halation: where the glow starts (linear luminance), its colour, and its radius at 2400 px
/// (three box passes, so the support is three times this).
pub const HALATION_START: f32 = 0.45;
pub const HALATION_TINT: Rgb = [1.0, 0.45, 0.25];
pub const HALATION_RADIUS: f32 = 6.0;
pub const HALATION_MAX_RADIUS: usize = crate::OVERLAP as usize / 3;
/// Halation at 100 %: how much of the blurred highlight comes back as glow.
pub const HALATION_GAIN: f32 = 1.6;

/// One film's halation: where it starts (linear luminance), its colour, its radius at 2400 px
/// (three box passes, so the support is three times this) and its strength at 100 %.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HalationStyle {
    /// Which light feeds it: luminance for most films; red only for a film whose halation comes
    /// back into its red layer alone (ChromaStill: no rem-jet).
    pub source: Rgb,
    pub start: f32,
    pub tint: Rgb,
    pub radius: f32,
    pub gain: f32,
}

impl Default for HalationStyle {
    /// ChromaKode 64's.
    fn default() -> Self {
        HalationStyle {
            source: LUMA_709,
            start: HALATION_START,
            tint: HALATION_TINT,
            radius: HALATION_RADIUS,
            gain: HALATION_GAIN,
        }
    }
}

/// Softness: a tent blur (two box passes) of this radius at 2400 px, mixed in at up to this much.
pub const SOFTNESS_RADIUS: f32 = 1.0;
pub const SOFTNESS_MAX_RADIUS: usize = 2;
pub const SOFTNESS_MIX: f32 = 0.8;

/// Grain: the clump size at 2400 px (px), the size at 100 % (screen steps of 1/255 at middle
/// grey), and how colourful it is (0 = grey grain). These are ChromaKode 64's; each film has its
/// own [`GrainStyle`].
pub const GRAIN_SIZE: f32 = 1.8;
pub const GRAIN_AMOUNT: f32 = 33.0 / 255.0;
pub const GRAIN_COLOR: f32 = 0.3;

/// One film's grain: clump size at 2400 px, size at 100 %, and colourfulness (see above), plus
/// - `fine`: how much per-pixel bite goes on top of the clumps (ChromaKode 64's crisp grain);
/// - `cloud`: a second, smaller layer of clumps on an offset lattice, so the grain reads as soft,
///   irregular dye clouds instead of a grid (ChromaGold 200);
/// - `shadow`: extra grain toward the shadows, as in a print from a negative.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GrainStyle {
    pub size: f32,
    pub amount: f32,
    pub color: f32,
    pub fine: f32,
    pub cloud: f32,
    pub shadow: f32,
}

impl Default for GrainStyle {
    fn default() -> Self {
        GrainStyle {
            size: GRAIN_SIZE,
            amount: GRAIN_AMOUNT,
            color: GRAIN_COLOR,
            fine: 0.35,
            cloud: 0.0,
            shadow: 0.0,
        }
    }
}

/// How much film character, each 0–1 (the panel's 0–100 sliders).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Character {
    pub grain: f32,
    pub halation: f32,
    pub softness: f32,
}

impl Default for Character {
    fn default() -> Self {
        Character {
            grain: 0.5,
            halation: 0.5,
            softness: 0.3,
        }
    }
}

impl Character {
    pub const NONE: Character = Character {
        grain: 0.0,
        halation: 0.0,
        softness: 0.0,
    };
}

/// How the sizes scale for a photo with this long side.
pub fn scale(long_side: f32) -> f32 {
    (long_side / REFERENCE_LONG_SIDE).clamp(0.35, 2.5)
}

/// The highlight shoulder for a linear gain `g`: values up to the knee are untouched, and
/// `[knee, g]` (where white lands) rolls smoothly onto `[knee, 1]`, so brightening never clips.
/// Tabulated, as `powf` per sample is too dear for the host's instruction budget.
#[derive(Clone)]
pub struct Shoulder {
    top: f32,
    roll: Option<Curve>,
}

impl Shoulder {
    pub fn new(gain: f32) -> Self {
        let top = gain.max(1.0);
        let power = (top - SHOULDER_KNEE) / (1.0 - SHOULDER_KNEE);
        let roll = (power > 1.0 + 1e-4).then(|| Curve::from_fn(|t| 1.0 - (1.0 - t).powf(power)));
        Shoulder { top, roll }
    }

    pub fn apply(&self, y: f32) -> f32 {
        match &self.roll {
            Some(roll) if y > SHOULDER_KNEE => {
                SHOULDER_KNEE
                    + (1.0 - SHOULDER_KNEE)
                        * roll.eval((y - SHOULDER_KNEE) / (self.top - SHOULDER_KNEE))
            }
            _ => y,
        }
    }
}

/// What one linear pixel gives to halation: the part of its luminance above [`HALATION_START`].
pub fn halation_source(c: Rgb) -> f32 {
    halation_source_from(c, HALATION_START)
}

/// [`halation_source`] with its own starting level.
pub fn halation_source_from(c: Rgb, start: f32) -> f32 {
    halation_source_weighted(c, LUMA_709, start)
}

/// [`halation_source`] fed by a chosen mix of the channels.
pub fn halation_source_weighted(c: Rgb, weights: Rgb, start: f32) -> f32 {
    (dot(c, weights) - start).clamp(0.0, 2.0)
}

/// Halation's glow (linear, before the tint): every pixel's [`halation_source`], blurred wide.
/// `None` when it would add nothing.
pub fn halation_glow(
    mask: Vec<f32>,
    w: usize,
    h: usize,
    amount: f32,
    long_side: f32,
) -> Option<Vec<f32>> {
    halation_glow_styled(mask, w, h, amount, long_side, &HalationStyle::default())
}

/// [`halation_glow`] in a film's own style.
pub fn halation_glow_styled(
    mut mask: Vec<f32>,
    w: usize,
    h: usize,
    amount: f32,
    long_side: f32,
    style: &HalationStyle,
) -> Option<Vec<f32>> {
    if amount <= 0.0 || mask.iter().all(|&m| m == 0.0) {
        return None;
    }
    let r = ((style.radius * scale(long_side)).round() as usize).clamp(1, HALATION_MAX_RADIUS);
    for _ in 0..3 {
        mask = box_mean(&mask, w, h, r);
    }
    let k = amount * style.gain;
    mask.iter_mut().for_each(|m| *m *= k);
    Some(mask)
}

/// Softness: mixes in a small tent blur of the screen-value buffer.
pub fn soften(rgb: &mut [f32], w: usize, h: usize, amount: f32, long_side: f32) {
    if amount <= 0.0 || w == 0 || h == 0 {
        return;
    }
    let r = ((SOFTNESS_RADIUS * scale(long_side)).round() as usize).clamp(1, SOFTNESS_MAX_RADIUS);
    // the tent's weights: 1 2 1 or 1 2 3 2 1
    let taps: Vec<f32> = (0..=2 * r)
        .map(|i| (r + 1 - i.abs_diff(r)) as f32 / ((r + 1) * (r + 1)) as f32)
        .collect();
    let k = amount * SOFTNESS_MIX;
    let ri = r as isize;
    let mut tmp = vec![0.0f32; rgb.len()];
    // rows (edges clamped)
    for y in 0..h {
        let row = &rgb[y * w * 3..(y + 1) * w * 3];
        let out = &mut tmp[y * w * 3..(y + 1) * w * 3];
        for x in 0..w {
            let mut acc = [0.0f32; 3];
            for (j, &t) in taps.iter().enumerate() {
                let xx = (x as isize + j as isize - ri).clamp(0, w as isize - 1) as usize * 3;
                acc[0] += row[xx] * t;
                acc[1] += row[xx + 1] * t;
                acc[2] += row[xx + 2] * t;
            }
            out[x * 3..x * 3 + 3].copy_from_slice(&acc);
        }
    }
    // columns, mixed straight into the picture
    for y in 0..h {
        for x in 0..w {
            let mut acc = [0.0f32; 3];
            for (j, &t) in taps.iter().enumerate() {
                let yy = (y as isize + j as isize - ri).clamp(0, h as isize - 1) as usize;
                let i = (yy * w + x) * 3;
                acc[0] += tmp[i] * t;
                acc[1] += tmp[i + 1] * t;
                acc[2] += tmp[i + 2] * t;
            }
            let i = (y * w + x) * 3;
            for c in 0..3 {
                rgb[i + c] += (acc[c] - rgb[i + c]) * k;
            }
        }
    }
}

/// Film grain, fixed by position in the photo.
pub struct Grain {
    cell: f32,
    amount: f32,
    color: f32,
    fine: f32,
    cloud: f32,
    shadow: f32,
}

impl Grain {
    pub fn new(amount: f32, long_side: f32) -> Option<Self> {
        Self::styled(amount, long_side, &GrainStyle::default())
    }

    /// Grain in a film's own style.
    pub fn styled(amount: f32, long_side: f32, style: &GrainStyle) -> Option<Self> {
        (amount > 0.0).then(|| Grain {
            cell: (style.size * scale(long_side)).max(1.0),
            amount: amount * style.amount,
            color: style.color,
            fine: style.fine,
            cloud: style.cloud,
            shadow: style.shadow,
        })
    }

    /// Four noise values (grey, then red, green, blue) from one hash: each byte, centred.
    fn four(h: u32) -> [f32; 4] {
        [24, 16, 8, 0].map(|shift| ((h >> shift) & 0xff) as f32 / 255.0 - 0.5)
    }

    /// Smooth value noise over a lattice of `cell`-sized clumps, four channels at once (grey, red,
    /// green, blue), each about −0.7…0.7. One hash per lattice corner and one per pixel keeps it
    /// cheap enough for the host's instruction budget.
    fn noise(&self, px: i64, py: i64) -> [f32; 4] {
        const SEED: u32 = 0x9e37;
        let clumps = Self::layer(px as f32 / self.cell, py as f32 / self.cell, SEED);
        if self.cloud > 0.0 {
            // ChromaGold: a second, smaller layer on an offset lattice and no per-pixel bite, so
            // the clumps blend into soft, irregular clouds with no grid and no single-pixel specks
            let small = self.cell * 0.57;
            let cl = Self::layer(
                px as f32 / small + 0.37,
                py as f32 / small + 0.71,
                SEED ^ 0x5bd1_e995,
            );
            return std::array::from_fn(|k| clumps[k] * 1.25 + cl[k] * self.cloud);
        }
        // a little per-pixel bite on top of the clumps, so the grain isn't mushy
        let fine = Self::four(hash32(px, py, SEED ^ 0x27d4_eb2f));
        std::array::from_fn(|k| clumps[k] * 1.4 + fine[k] * self.fine)
    }

    /// One layer of smooth value noise at lattice coordinates `(fx, fy)`, four channels at once.
    fn layer(fx: f32, fy: f32, seed: u32) -> [f32; 4] {
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (tx, ty) = (tx * tx * (3.0 - 2.0 * tx), ty * ty * (3.0 - 2.0 * ty));
        let (x0, y0) = (x0 as i64, y0 as i64);
        let a = Self::four(hash32(x0, y0, seed));
        let b = Self::four(hash32(x0 + 1, y0, seed));
        let c = Self::four(hash32(x0, y0 + 1, seed));
        let d = Self::four(hash32(x0 + 1, y0 + 1, seed));
        std::array::from_fn(|k| {
            let top = a[k] + (b[k] - a[k]) * tx;
            let bottom = c[k] + (d[k] - c[k]) * tx;
            top + (bottom - top) * ty
        })
    }

    /// Adds grain to a screen-value pixel: a little stronger in the midtones, mostly grey with
    /// some colour.
    pub fn apply(&self, c: Rgb, px: i64, py: i64) -> Rgb {
        let l = dot(c, LUMA_709).clamp(0.0, 1.0);
        let a = self.amount * (0.65 + 1.4 * l * (1.0 - l) + self.shadow * (1.0 - l));
        let n = self.noise(px, py);
        let (k, grey) = (self.color, n[0] * (1.0 - self.color));
        [
            c[0] + a * (grey + n[1] * k),
            c[1] + a * (grey + n[2] * k),
            c[2] + a * (grey + n[3] * k),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shoulder_keeps_shadows_and_lands_white_on_one() {
        let s = Shoulder::new(1.8);
        assert_eq!(s.apply(0.3), 0.3);
        assert!((s.apply(1.8) - 1.0).abs() < 1e-6);
        // smooth at the knee, never decreasing
        let mut last = 0.0;
        for i in 0..=200 {
            let y = s.apply(i as f32 / 100.0);
            assert!(y >= last - 1e-6);
            last = y;
        }
        assert!((s.apply(SHOULDER_KNEE + 0.001) - (SHOULDER_KNEE + 0.001)).abs() < 1e-3);
        // no gain, no shoulder
        assert_eq!(Shoulder::new(1.0).apply(0.95), 0.95);
    }

    #[test]
    fn grain_is_fixed_by_position_and_centred() {
        let g = Grain::new(1.0, 2400.0).unwrap();
        assert_eq!(g.apply([0.5; 3], 10, 20), g.apply([0.5; 3], 10, 20));
        let (mut sum, mut sq, n) = (0.0f64, 0.0f64, 200 * 200);
        for y in 0..200 {
            for x in 0..200 {
                let d = (g.apply([0.5; 3], x, y)[1] - 0.5) as f64;
                sum += d;
                sq += d * d;
            }
        }
        let (mean, sd) = (sum / n as f64, (sq / n as f64).sqrt());
        assert!(mean.abs() < 0.002, "{mean}");
        assert!(sd > 1.0 / 255.0 && sd < 12.0 / 255.0, "{sd}");
    }

    #[test]
    fn halation_glows_only_near_highlights() {
        let (w, h) = (64, 8);
        let mut img = vec![0.05f32; w * h * 3];
        for y in 0..h {
            for x in 0..8 {
                img[(y * w + x) * 3..][..3].copy_from_slice(&[1.0; 3]);
            }
        }
        let src = |img: &[f32]| {
            img.chunks_exact(3)
                .map(|c| halation_source([c[0], c[1], c[2]]))
                .collect::<Vec<_>>()
        };
        let glow = halation_glow(src(&img), w, h, 1.0, 2400.0).unwrap();
        assert!(glow[4 * w + 10] > 0.01);
        assert_eq!(glow[4 * w + 60], 0.0);
        assert!(halation_glow(src(&vec![0.05f32; w * h * 3]), w, h, 1.0, 2400.0).is_none());
    }
}
