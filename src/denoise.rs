//! [k] Denoise: smooths grain (brightness noise) and colour speckles with edge-aware guided
//! filters, keeping edges and some of the finest texture. No host calls.

use crate::color::LUMA_601;

/// The Denoise options, as the panel sets them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    /// 0–100 (default 50). The effect is blended in by amount × 3, so it is full from 34 up;
    /// above that, a higher amount uses larger filters.
    pub amount: f32,
    pub grain: bool,
    pub color: bool,
    pub detail: bool,
    pub strong: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            amount: 50.0,
            grain: true,
            color: true,
            detail: true,
            strong: false,
        }
    }
}

/// Filter sizes for one run, in pixels of the buffer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sizes {
    pub grain_radius: usize,
    pub grain_edge: f32,
    pub color_radius: usize,
    pub color_edge: f32,
}

/// The sheet's table at a 2048 px long side: (amount, grain radius, grain edge limit, colour radius).
const TABLE: [(f32, f32, f32, f32); 3] = [
    (25.0, 2.0, 0.019, 4.0),
    (50.0, 2.0, 0.027, 5.0),
    (100.0, 3.0, 0.045, 8.0),
];
const TABLE_STRONG: [(f32, f32, f32, f32); 3] = [
    (25.0, 2.0, 0.032, 6.0),
    (50.0, 3.0, 0.048, 8.0),
    (100.0, 5.0, 0.078, 12.0),
];
const COLOR_EDGE: f32 = 0.012;

/// Largest radius the filter may use: its pixels must stay inside the band the host hands over
/// (the manifest's `overlap`). A guided filter reaches twice its radius, plus one for the
/// fine-detail pass.
pub const MAX_RADIUS: usize = {
    let room = (crate::OVERLAP as usize - 1 - 2 * crate::film::SOFTNESS_MAX_RADIUS) / 2;
    if room < 9 { room } else { 9 }
};

impl Sizes {
    /// The sizes for `opts` on a photo whose long side is `long_side` pixels: the table read at the
    /// amount (between rows, a straight-line blend), its radii scaled by long side / 2048
    /// (×0.35 to ×2.5).
    pub fn for_photo(opts: &Options, long_side: f32) -> Sizes {
        let t = if opts.strong { &TABLE_STRONG } else { &TABLE };
        let a = opts.amount.clamp(t[0].0, t[2].0);
        let (lo, hi) = if a <= t[1].0 {
            (t[0], t[1])
        } else {
            (t[1], t[2])
        };
        let f = (a - lo.0) / (hi.0 - lo.0);
        let lerp = |x: f32, y: f32| x + (y - x) * f;
        let scale = (long_side / 2048.0).clamp(0.35, 2.5);
        let px = |r: f32| ((r * scale).round() as usize).clamp(1, MAX_RADIUS);
        Sizes {
            grain_radius: px(lerp(lo.1, hi.1)),
            grain_edge: lerp(lo.2, hi.2),
            color_radius: px(lerp(lo.3, hi.3)),
            color_edge: COLOR_EDGE,
        }
    }
}

/// How much of the effect is blended in: amount × 3 %, full from 34 up.
pub fn blend(opts: &Options) -> f32 {
    (opts.amount * 3.0 / 100.0).clamp(0.0, 1.0)
}

/// Mean over a (2r+1)² window, edge pixels repeated (separable running sums).
pub(crate) fn box_mean(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let mut tmp = vec![0.0f32; src.len()];
    let mut out = vec![0.0f32; src.len()];
    let n = (2 * r + 1) as f64;
    let ri = r as isize;
    // rows
    for y in 0..h {
        let row = &src[y * w..(y + 1) * w];
        let at = |x: isize| row[x.clamp(0, w as isize - 1) as usize] as f64;
        let mut sum: f64 = (-ri..=ri).map(at).sum();
        for x in 0..w {
            tmp[y * w + x] = (sum / n) as f32;
            sum += at(x as isize + ri + 1) - at(x as isize - ri);
        }
    }
    // columns
    for x in 0..w {
        let at = |y: isize| tmp[y.clamp(0, h as isize - 1) as usize * w + x] as f64;
        let mut sum: f64 = (-ri..=ri).map(at).sum();
        for y in 0..h {
            out[y * w + x] = (sum / n) as f32;
            sum += at(y as isize + ri + 1) - at(y as isize - ri);
        }
    }
    out
}

fn mul(a: &[f32], b: &[f32]) -> Vec<f32> {
    a.iter().zip(b).map(|(x, y)| x * y).collect()
}

/// Edge-aware smoothing of `p` guided by itself: flat areas are averaged, edges whose local
/// spread is well above `edge` are kept.
fn guided_self(p: &[f32], w: usize, h: usize, r: usize, edge: f32) -> Vec<f32> {
    let eps = edge * edge;
    let m = box_mean(p, w, h, r);
    let mm = box_mean(&mul(p, p), w, h, r);
    let mut a = vec![0.0f32; p.len()];
    let mut b = vec![0.0f32; p.len()];
    for i in 0..p.len() {
        let var = (mm[i] - m[i] * m[i]).max(0.0);
        a[i] = var / (var + eps);
        b[i] = m[i] - a[i] * m[i];
    }
    let (ma, mb) = (box_mean(&a, w, h, r), box_mean(&b, w, h, r));
    (0..p.len()).map(|i| ma[i] * p[i] + mb[i]).collect()
}

/// Smoothing of each of `ps` guided by `g`: a change in `p` survives only where `g` changes too,
/// so colour doesn't bleed across brightness edges.
fn guided<const N: usize>(
    g: &[f32],
    ps: [&[f32]; N],
    w: usize,
    h: usize,
    r: usize,
    edge: f32,
) -> [Vec<f32>; N] {
    let eps = edge * edge;
    let mg = box_mean(g, w, h, r);
    let mgg = box_mean(&mul(g, g), w, h, r);
    ps.map(|p| {
        let mp = box_mean(p, w, h, r);
        let mgp = box_mean(&mul(g, p), w, h, r);
        let mut a = vec![0.0f32; p.len()];
        let mut b = vec![0.0f32; p.len()];
        for i in 0..p.len() {
            let var = (mgg[i] - mg[i] * mg[i]).max(0.0);
            a[i] = (mgp[i] - mg[i] * mp[i]) / (var + eps);
            b[i] = mp[i] - a[i] * mg[i];
        }
        let (ma, mb) = (box_mean(&a, w, h, r), box_mean(&b, w, h, r));
        (0..p.len()).map(|i| ma[i] * g[i] + mb[i]).collect()
    })
}

/// Puts back `keep` of the finest texture the smoothing took out of `orig` (what was removed,
/// less its own 3×3 average).
fn keep_fine(smoothed: &mut [f32], orig: &[f32], w: usize, h: usize, keep: f32) {
    let removed: Vec<f32> = orig
        .iter()
        .zip(smoothed.iter())
        .map(|(o, s)| o - s)
        .collect();
    let coarse = box_mean(&removed, w, h, 1);
    for i in 0..smoothed.len() {
        smoothed[i] += keep * (removed[i] - coarse[i]);
    }
}

/// Denoises `rgb` (interleaved screen values, `w × h`) in place.
pub fn apply(rgb: &mut [f32], w: usize, h: usize, opts: &Options, sizes: &Sizes) {
    let mix = blend(opts);
    if mix <= 0.0 || (!opts.grain && !opts.color) || w == 0 || h == 0 {
        return;
    }
    let n = w * h;
    // 1) brightness Y and colour (B − Y, R − Y)
    let mut y = Vec::with_capacity(n);
    let mut cb = Vec::with_capacity(n);
    let mut cr = Vec::with_capacity(n);
    for p in rgb.chunks_exact(3) {
        let l = LUMA_601[0] * p[0] + LUMA_601[1] * p[1] + LUMA_601[2] * p[2];
        y.push(l);
        cb.push(p[2] - l);
        cr.push(p[0] - l);
    }
    // 2) grain
    let yd = if opts.grain {
        let mut s = guided_self(&y, w, h, sizes.grain_radius, sizes.grain_edge);
        if opts.detail {
            keep_fine(&mut s, &y, w, h, if opts.strong { 0.20 } else { 0.32 });
        }
        s
    } else {
        y.clone()
    };
    // 3) colour speckles, guided by brightness
    let (cbd, crd) = if opts.color {
        let [mut b, mut r] = guided(&y, [&cb, &cr], w, h, sizes.color_radius, sizes.color_edge);
        if opts.detail {
            keep_fine(&mut b, &cb, w, h, 0.08);
            keep_fine(&mut r, &cr, w, h, 0.08);
        }
        (b, r)
    } else {
        (cb, cr)
    };
    // 4) back to RGB, blended with the original
    for (i, p) in rgb.chunks_exact_mut(3).enumerate() {
        let r = yd[i] + crd[i];
        let b = yd[i] + cbd[i];
        let g = (yd[i] - LUMA_601[0] * r - LUMA_601[2] * b) / LUMA_601[1];
        p[0] += (r - p[0]) * mix;
        p[1] += (g - p[1]) * mix;
        p[2] += (b - p[2]) * mix;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noisy(w: usize, h: usize) -> Vec<f32> {
        let mut v = Vec::with_capacity(w * h * 3);
        for yy in 0..h {
            for xx in 0..w {
                let base = if xx < w / 2 { 0.3 } else { 0.7 };
                for c in 0..3 {
                    v.push(base + 0.04 * (crate::color::hash01(xx as i64, yy as i64, c) - 0.5));
                }
            }
        }
        v
    }

    fn spread(v: &[f32], w: usize, x0: usize, x1: usize) -> f32 {
        let vals: Vec<f32> = (0..v.len() / 3)
            .filter(|i| (x0..x1).contains(&(i % w)))
            .map(|i| v[i * 3 + 1])
            .collect();
        let m = vals.iter().sum::<f32>() / vals.len() as f32;
        (vals.iter().map(|x| (x - m) * (x - m)).sum::<f32>() / vals.len() as f32).sqrt()
    }

    #[test]
    fn smooths_flat_areas_and_keeps_the_edge() {
        let (w, h) = (64, 32);
        let orig = noisy(w, h);
        let mut out = orig.clone();
        let opts = Options::default();
        apply(&mut out, w, h, &opts, &Sizes::for_photo(&opts, 2048.0));
        assert!(spread(&out, w, 4, 24) < spread(&orig, w, 4, 24) * 0.6);
        // the step between the halves is still there
        let row = 16 * w;
        assert!(out[(row + 40) * 3 + 1] - out[(row + 24) * 3 + 1] > 0.3);
    }

    #[test]
    fn amount_zero_does_nothing() {
        let (w, h) = (16, 16);
        let orig = noisy(w, h);
        let mut out = orig.clone();
        let opts = Options {
            amount: 0.0,
            ..Options::default()
        };
        apply(&mut out, w, h, &opts, &Sizes::for_photo(&opts, 2048.0));
        assert_eq!(out, orig);
    }

    #[test]
    fn sizes_follow_the_table_and_the_photo() {
        let o = Options::default();
        assert_eq!(
            Sizes::for_photo(&o, 2048.0),
            Sizes {
                grain_radius: 2,
                grain_edge: 0.027,
                color_radius: 5,
                color_edge: 0.012
            }
        );
        let big = Sizes::for_photo(
            &Options {
                amount: 100.0,
                strong: true,
                ..o
            },
            8000.0,
        );
        assert_eq!(big.grain_radius, MAX_RADIUS.min(13));
        assert!(big.color_radius <= MAX_RADIUS);
        assert_eq!(Sizes::for_photo(&o, 300.0).grain_radius, 1);
    }
}
