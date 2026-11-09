//! Colour helpers shared by the filter's steps: sRGB encoding, tone curves, HSL colour bands.
//! No host calls; everything here runs natively in the tests.

pub type Rgb = [f32; 3];

/// Rec.709 luminance weights (linear light).
pub const LUMA_709: Rgb = [0.2126, 0.7152, 0.0722];
/// Rec.601 luma weights (screen values), the Denoise step's brightness.
pub const LUMA_601: Rgb = [0.299, 0.587, 0.114];

#[inline]
pub fn dot(a: Rgb, b: Rgb) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------------------------------------
// sRGB transfer, through tables (the interpreter that runs plug-ins makes `powf` costly)
// ---------------------------------------------------------------------------------------------

const ENC_N: usize = 4096;

/// Screen value ↔ linear light, tabulated once per call.
pub struct Transfer {
    /// linear(screen), indexed by screen value.
    decode: Vec<f32>,
    /// screen(linear), indexed by sqrt(linear), so the shadows keep their precision.
    encode: Vec<f32>,
}

pub fn decode_exact(v: f32) -> f32 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

pub fn encode_exact(v: f32) -> f32 {
    if v <= 0.003_130_8 {
        v * 12.92
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    }
}

impl Transfer {
    pub fn new() -> Self {
        let decode = (0..=ENC_N)
            .map(|i| decode_exact(i as f32 / ENC_N as f32))
            .collect();
        let encode = (0..=ENC_N).map(|i| {
            let s = i as f32 / ENC_N as f32;
            encode_exact(s * s)
        });
        Self {
            decode,
            encode: encode.collect(),
        }
    }

    #[inline]
    fn lookup(t: &[f32], x: f32) -> f32 {
        let f = x * ENC_N as f32;
        let i = (f as usize).min(ENC_N - 1);
        let t0 = t[i];
        t0 + (t[i + 1] - t0) * (f - i as f32)
    }

    /// Screen value → linear light.
    #[inline]
    pub fn to_linear(&self, v: f32) -> f32 {
        if (0.0..=1.0).contains(&v) {
            Self::lookup(&self.decode, v)
        } else if v < 0.0 {
            v / 12.92
        } else {
            decode_exact(v)
        }
    }

    /// Linear light → screen value.
    #[inline]
    pub fn to_screen(&self, v: f32) -> f32 {
        if (0.0..=1.0).contains(&v) {
            Self::lookup(&self.encode, v.sqrt())
        } else if v < 0.0 {
            v * 12.92
        } else {
            encode_exact(v)
        }
    }
}

impl Default for Transfer {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------------------------
// Tone curves
// ---------------------------------------------------------------------------------------------

const LUT_N: usize = 2048;

/// A tone curve on 0–1 screen values: a smooth, monotone curve through its points
/// (monotone cubic, so it never overshoots between points), tabulated.
#[derive(Clone)]
pub struct Curve {
    lut: Vec<f32>,
}

impl Curve {
    /// The curve through `pts` (`input → output`, inputs ascending from 0 to 1).
    pub fn through(pts: &[(f32, f32)]) -> Self {
        let f = monotone_cubic(pts);
        Self::from_fn(f)
    }

    /// A curve tabulated from any function of the screen value.
    pub fn from_fn(f: impl Fn(f32) -> f32) -> Self {
        Self {
            lut: (0..=LUT_N).map(|i| f(i as f32 / LUT_N as f32)).collect(),
        }
    }

    /// The curve's output for `x` (clamped into 0–1).
    #[inline]
    pub fn eval(&self, x: f32) -> f32 {
        let f = x.clamp(0.0, 1.0) * LUT_N as f32;
        let i = (f as usize).min(LUT_N - 1);
        let t0 = self.lut[i];
        t0 + (self.lut[i + 1] - t0) * (f - i as f32)
    }

    /// This curve, then `next` on its output.
    pub fn then(&self, next: &Curve) -> Curve {
        Curve::from_fn(|x| next.eval(self.eval(x)))
    }
}

/// Fritsch–Carlson monotone cubic Hermite interpolation through `pts`.
pub(crate) fn monotone_cubic(pts: &[(f32, f32)]) -> impl Fn(f32) -> f32 {
    let xs: Vec<f32> = pts.iter().map(|p| p.0).collect();
    let ys: Vec<f32> = pts.iter().map(|p| p.1).collect();
    let n = xs.len();
    let d: Vec<f32> = (0..n - 1)
        .map(|k| (ys[k + 1] - ys[k]) / (xs[k + 1] - xs[k]))
        .collect();
    let mut m = vec![0.0f32; n];
    m[0] = d[0];
    m[n - 1] = d[n - 2];
    for k in 1..n - 1 {
        m[k] = if d[k - 1] * d[k] <= 0.0 {
            0.0
        } else {
            (d[k - 1] + d[k]) / 2.0
        };
    }
    for k in 0..n - 1 {
        if d[k] == 0.0 {
            m[k] = 0.0;
            m[k + 1] = 0.0;
        } else {
            let (a, b) = (m[k] / d[k], m[k + 1] / d[k]);
            let s = a * a + b * b;
            if s > 9.0 {
                let t = 3.0 / s.sqrt();
                m[k] = t * a * d[k];
                m[k + 1] = t * b * d[k];
            }
        }
    }
    move |x: f32| {
        let k = match xs.iter().rposition(|&xk| xk <= x) {
            Some(k) => k.min(n - 2),
            None => 0,
        };
        let hk = xs[k + 1] - xs[k];
        let t = ((x - xs[k]) / hk).clamp(0.0, 1.0);
        let (t2, t3) = (t * t, t * t * t);
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + t;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        h00 * ys[k] + h10 * hk * m[k] + h01 * ys[k + 1] + h11 * hk * m[k + 1]
    }
}

// ---------------------------------------------------------------------------------------------
// Saturation and channel mixing
// ---------------------------------------------------------------------------------------------

/// Scales saturation by `k` around brightness (Rec.709 weights on screen values): greys stay grey.
#[inline]
pub fn saturate(c: Rgb, k: f32) -> Rgb {
    let y = dot(c, LUMA_709);
    [y + k * (c[0] - y), y + k * (c[1] - y), y + k * (c[2] - y)]
}

/// `m × c`: each new channel is a mix of the old ones.
#[inline]
pub fn mix3(m: &[Rgb; 3], c: Rgb) -> Rgb {
    [dot(m[0], c), dot(m[1], c), dot(m[2], c)]
}

#[inline]
pub fn clamp01(c: Rgb) -> Rgb {
    [
        c[0].clamp(0.0, 1.0),
        c[1].clamp(0.0, 1.0),
        c[2].clamp(0.0, 1.0),
    ]
}

#[inline]
pub fn lerp3(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

// ---------------------------------------------------------------------------------------------
// HSL colour bands (eight bands around the hue circle)
// ---------------------------------------------------------------------------------------------

/// One colour band's change: hue in degrees, saturation and lightness in percent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Band {
    pub hue: f32,
    pub sat: f32,
    pub light: f32,
}

pub const fn band(hue: f32, sat: f32, light: f32) -> Band {
    Band { hue, sat, light }
}

/// Red, Orange, Yellow, Green, Aqua, Blue, Purple, Magenta.
pub type Bands = [Band; 8];

/// Each band's centre hue, in degrees; the ninth closes the circle back at red.
const CENTRES: [f32; 9] = [0.0, 30.0, 60.0, 120.0, 180.0, 240.0, 270.0, 300.0, 360.0];

pub fn rgb_to_hsl(c: Rgb) -> (f32, f32, f32) {
    let max = c[0].max(c[1]).max(c[2]);
    let min = c[0].min(c[1]).min(c[2]);
    let l = (max + min) * 0.5;
    let d = max - min;
    if d <= 1e-7 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs()).max(1e-7);
    let h = if max == c[0] {
        60.0 * ((c[1] - c[2]) / d)
    } else if max == c[1] {
        60.0 * ((c[2] - c[0]) / d + 2.0)
    } else {
        60.0 * ((c[0] - c[1]) / d + 4.0)
    };
    (h.rem_euclid(360.0), s.min(1.0), l)
}

pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> Rgb {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(360.0) / 60.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c * 0.5;
    [r + m, g + m, b + m]
}

/// The band change at hue `h`, blended smoothly between the two neighbouring bands.
fn band_at(bands: &Bands, h: f32) -> Band {
    let i = (0..8).rev().find(|&i| CENTRES[i] <= h).unwrap_or(0);
    let t = smoothstep(0.0, 1.0, (h - CENTRES[i]) / (CENTRES[i + 1] - CENTRES[i]));
    let (a, b) = (bands[i], bands[(i + 1) % 8]);
    Band {
        hue: a.hue + (b.hue - a.hue) * t,
        sat: a.sat + (b.sat - a.sat) * t,
        light: a.light + (b.light - a.light) * t,
    }
}

/// Moves each colour's hue, saturation and lightness by its band's amounts. Greys don't move;
/// the stronger the colour, the more of the change it takes.
pub fn apply_bands(c: Rgb, bands: &Bands) -> Rgb {
    apply_bands_gated(c, bands, BandGate::STRONG)
}

/// How much of a band's change a colour takes, by its saturation and chroma: full from
/// `saturation` / `chroma` up, easing in from grey.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandGate {
    pub saturation: f32,
    pub chroma: f32,
}

impl BandGate {
    /// Only clearly coloured pixels change much (ChromaGold 200).
    pub const STRONG: BandGate = BandGate {
        saturation: 0.5,
        chroma: 0.15,
    };
    /// Muted colours change too (ChromaKode 64); greys still don't.
    pub const MUTED: BandGate = BandGate {
        saturation: 0.15,
        chroma: 0.05,
    };
}

/// [`apply_bands`] with a chosen [`BandGate`].
pub fn apply_bands_gated(c: Rgb, bands: &Bands, gate: BandGate) -> Rgb {
    let (h, s, l) = rgb_to_hsl(c);
    let chroma = c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2]);
    let k = smoothstep(0.0, gate.saturation, s) * smoothstep(0.0, gate.chroma, chroma);
    if k <= 0.0 {
        return c;
    }
    let b = band_at(bands, h);
    let h2 = h + b.hue * k;
    let s2 = (s * (1.0 + b.sat * k / 100.0)).clamp(0.0, 1.0);
    let l2 = (l * (1.0 + b.light * k / 100.0)).clamp(0.0, 1.0);
    hsl_to_rgb(h2, s2, l2)
}

// ---------------------------------------------------------------------------------------------
// Position-fixed noise (the same photo always gets the same pattern)
// ---------------------------------------------------------------------------------------------

/// A uniform number in [0, 1) fixed by pixel position and `seed`.
#[inline]
pub fn hash01(x: i64, y: i64, seed: u32) -> f32 {
    (hash32(x, y, seed) >> 8) as f32 / (1u32 << 24) as f32
}

/// 32 well-mixed bits for a position and seed (the source of [`hash01`]).
pub fn hash32(x: i64, y: i64, seed: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343)
        ^ (y as u32).wrapping_mul(0xd816_3841)
        ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb_352d);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846c_a68b);
    h ^= h >> 16;
    h
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_round_trips() {
        let t = Transfer::new();
        for i in 0..=100 {
            let v = i as f32 / 100.0;
            assert!((t.to_screen(t.to_linear(v)) - v).abs() < 2e-4, "{v}");
            assert!((t.to_linear(v) - decode_exact(v)).abs() < 1e-4);
        }
    }

    #[test]
    fn curves_pass_through_their_points_and_stay_monotone() {
        let pts = [
            (0.0, 0.0),
            (0.06, 0.03),
            (0.25, 0.205),
            (0.5, 0.52),
            (0.75, 0.805),
            (0.92, 0.935),
            (1.0, 0.975),
        ];
        let c = Curve::through(&pts);
        for (x, y) in pts {
            assert!(
                (c.eval(x) - y).abs() < 1e-3,
                "{x} → {} (want {y})",
                c.eval(x)
            );
        }
        let mut last = -1.0;
        for i in 0..=1000 {
            let v = c.eval(i as f32 / 1000.0);
            assert!(v >= last - 1e-6);
            last = v;
        }
    }

    #[test]
    fn hsl_round_trips() {
        for c in [
            [0.8, 0.2, 0.1],
            [0.1, 0.6, 0.3],
            [0.2, 0.3, 0.9],
            [0.5, 0.5, 0.5],
            [1.0, 1.0, 0.0],
        ] {
            let (h, s, l) = rgb_to_hsl(c);
            let back = hsl_to_rgb(h, s, l);
            for i in 0..3 {
                assert!((back[i] - c[i]).abs() < 1e-5, "{c:?} → {back:?}");
            }
        }
    }

    #[test]
    fn bands_leave_greys_alone() {
        let all = [band(20.0, 50.0, -30.0); 8];
        for g in [0.0, 0.18, 0.5, 1.0] {
            assert_eq!(apply_bands([g, g, g], &all), [g, g, g]);
        }
    }

    #[test]
    fn hash_is_stable_and_spread() {
        assert_eq!(hash01(10, 20, 1), hash01(10, 20, 1));
        let mean: f32 = (0..10_000)
            .map(|i| hash01(i % 100, i / 100, 7))
            .sum::<f32>()
            / 10_000.0;
        assert!((mean - 0.5).abs() < 0.02);
    }
}
