//! ChromaKode Film: Chromaforge pixel filters (PhotoCraft plug-in ABI v1, with the v1.1
//! `pc_analyze` / `pc_merge` measurement), one per film, sharing one engine:
//!
//! - ChromaKode 64 (the default build): Kodachrome 64 slide film, the Kodachrome 64 Kit.
//! - ChromaGold 200 (`--features gold`): Kodak Gold 200 colour negative film, printed.
//!
//! - `kit.rs`: the steps, in order (no host calls).
//! - `film.rs`: the film's character: highlight shoulder, halation, softness, grain.
//! - `denoise.rs`: the Denoise modifier (no host calls).
//! - `color.rs`: curves, colour bands, sRGB, noise.
//! - this file: the manifest and the exports the host calls.

pub mod color;
pub mod denoise;
pub mod film;
pub mod kit;
pub mod print;

use serde_json::{Value, json};

use kit::{Encoding, Frame, Kit, Settings, Stock};

/// Pixels of context around each band the filter asks for: enough for halation's glow
/// ([`film::HALATION_MAX_RADIUS`]) and for softness plus Denoise ([`denoise::MAX_RADIUS`]).
#[cfg(not(feature = "still"))]
pub const OVERLAP: u32 = 24;
/// ChromaStill's red halation reaches much further (no rem-jet), so it asks for more context.
#[cfg(feature = "still")]
pub const OVERLAP: u32 = 48;

/// The film this build is.
#[cfg(not(any(feature = "gold", feature = "still", feature = "purple")))]
pub const STOCK: Stock = Stock::Kodachrome64;
#[cfg(feature = "gold")]
pub const STOCK: Stock = Stock::Gold200;
#[cfg(all(feature = "still", not(feature = "gold")))]
pub const STOCK: Stock = Stock::Still800T;
#[cfg(all(feature = "purple", not(any(feature = "gold", feature = "still"))))]
pub const STOCK: Stock = Stock::Purple;

#[cfg(not(any(feature = "gold", feature = "still", feature = "purple")))]
mod pictures {
    pub const TEMPLATE: &str = include_str!("manifest.json");
    pub const ICON: &str = include_str!("../brand/icon.svg");
    pub const HEADER_DARK: &str = include_str!("../brand/logo-dark.svg");
    pub const HEADER_LIGHT: &str = include_str!("../brand/logo-light.svg");
}
#[cfg(feature = "gold")]
mod pictures {
    pub const TEMPLATE: &str = include_str!("manifest_gold.json");
    pub const ICON: &str = include_str!("../brand/gold-icon.svg");
    pub const HEADER_DARK: &str = include_str!("../brand/gold-logo-dark.svg");
    pub const HEADER_LIGHT: &str = include_str!("../brand/gold-logo-light.svg");
}
#[cfg(all(feature = "still", not(feature = "gold")))]
mod pictures {
    pub const TEMPLATE: &str = include_str!("manifest_still.json");
    pub const ICON: &str = include_str!("../brand/still-icon.svg");
    pub const HEADER_DARK: &str = include_str!("../brand/still-logo-dark.svg");
    pub const HEADER_LIGHT: &str = include_str!("../brand/still-logo-light.svg");
}
#[cfg(all(feature = "purple", not(any(feature = "gold", feature = "still"))))]
mod pictures {
    pub const TEMPLATE: &str = include_str!("manifest_purple.json");
    pub const ICON: &str = include_str!("../brand/purple-icon.svg");
    pub const HEADER_DARK: &str = include_str!("../brand/purple-logo-dark.svg");
    pub const HEADER_LIGHT: &str = include_str!("../brand/purple-logo-light.svg");
}
use pictures::*;

fn json_string(s: &str) -> String {
    Value::from(s).to_string()
}

/// The manifest the host reads, with the version and the pictures filled in.
pub fn manifest() -> String {
    TEMPLATE
        .replace("\"@version\"", &json_string(env!("CARGO_PKG_VERSION")))
        .replace("\"@icon\"", &json_string(ICON))
        .replace("\"@header_dark\"", &json_string(HEADER_DARK))
        .replace("\"@header_light\"", &json_string(HEADER_LIGHT))
}

// ---------------------------------------------------------------------------------------------
// Parameters
// ---------------------------------------------------------------------------------------------

fn flag(p: &Value, k: &str, default: bool) -> bool {
    p.get(k).and_then(Value::as_bool).unwrap_or(default)
}

fn number(p: &Value, k: &str, default: f64) -> f64 {
    p.get(k)
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .unwrap_or(default)
}

/// A 0–100 slider as 0–1.
fn percent(p: &Value, k: &str, default: f64) -> f32 {
    (number(p, k, default) / 100.0).clamp(0.0, 1.0) as f32
}

/// The Denoise menu as options: one choice instead of six controls. Every level smooths grain and
/// colour speckles and keeps some fine detail; the levels are the table's rows.
pub fn denoise_level(p: &Value) -> Option<denoise::Options> {
    let base = denoise::Options::default();
    match p.get("denoise").and_then(Value::as_str)? {
        "Light" => Some(denoise::Options {
            amount: 25.0,
            ..base
        }),
        "Medium" => Some(denoise::Options {
            amount: 50.0,
            ..base
        }),
        "Strong" => Some(denoise::Options {
            amount: 100.0,
            ..base
        }),
        "Max" => Some(denoise::Options {
            amount: 100.0,
            strong: true,
            ..base
        }),
        _ => None,
    }
}

/// This build's settings before the panel changes anything.
pub fn default_settings() -> Settings {
    let p = STOCK.profile();
    Settings {
        normalize: false,
        film: p.film,
        cool: p.cool,
        hue: p.hue,
        ..Settings::default()
    }
}

/// The panel's values as the filter's settings. Controls a film's panel doesn't have stay off.
pub fn settings(p: &Value) -> Settings {
    let defaults = default_settings();
    let d = defaults.film;
    let s = Settings {
        normalize: flag(p, "normalize", false),
        look: flag(p, "look", true),
        strength: percent(p, "strength", 100.0),
        film: film::Character {
            grain: percent(p, "grain", d.grain as f64 * 100.0),
            halation: percent(p, "halation", d.halation as f64 * 100.0),
            softness: percent(p, "softness", d.softness as f64 * 100.0),
        },
        cool: number(p, "coolness", defaults.cool as f64).clamp(0.0, 100.0) as f32,
        denoise: denoise_level(p),
        color_shift: flag(p, "colorShift", false),
        hc: flag(p, "hc", false),
        found: flag(p, "found", false),
        fade: flag(p, "liftedFade", false),
        exposure: match p.get("iso").and_then(Value::as_str) {
            // ChromaLome: the speed the film was rated at
            Some(iso) => kit::iso_stops(iso.trim_start_matches("ISO ").parse().unwrap_or(100)),
            None => number(p, "exposure", 0.0).clamp(-2.0, 3.0) as f32,
        },
        sun: percent(p, "sunCast", 0.0),
        // the film's tint control: Hue (ChromaStill) or Warmth (ChromaLome)
        hue: if p.get("warmth").is_some() {
            percent(p, "warmth", 0.0)
        } else {
            percent(p, "hue", defaults.hue as f64 * 100.0)
        },
        banding: flag(p, "bandingFix", false),
    };
    if flag(p, "bypass", false) {
        s.base_only()
    } else {
        s
    }
}

/// Where the band sits in the photo, from `_image` (the whole buffer when it's missing).
pub fn frame(p: &Value, w: usize, h: usize) -> Frame {
    let img = &p["_image"];
    let int = |k: &str| img.get(k).and_then(Value::as_i64);
    let cw = int("canvasWidth").filter(|&v| v > 0).unwrap_or(w as i64);
    let ch = int("canvasHeight").filter(|&v| v > 0).unwrap_or(h as i64);
    Frame {
        x: int("x").unwrap_or(0),
        y: int("y").unwrap_or(0),
        canvas_w: cw as f32,
        canvas_h: ch as f32,
    }
}

/// Normalize's correction: the brightness from the photo's measurement (`_analysis`; none until
/// it's measured) and a typical photo's black point ([`kit::TYPICAL_BLACK`]). The measurement's
/// own black level isn't used: it's read before your develop edits, the filter runs after them.
pub fn normalize(p: &Value) -> Option<kit::Normalize> {
    let profile = STOCK.profile();
    if let Some(stops) = profile.fixed_exposure {
        return Some(kit::Normalize {
            gain: 2f32.powf(stops),
            black: kit::TYPICAL_BLACK,
        });
    }
    let target = profile.normalize_target;
    let gain = p
        .get("_analysis")
        .and_then(|a| a.get("logAverage"))
        .and_then(Value::as_f64)
        .map_or(1.0, |avg| kit::normalize_gain_to(avg, target));
    Some(kit::Normalize {
        gain,
        black: kit::TYPICAL_BLACK,
    })
}

fn measurement_json(avg: f64, n: u64, black: f32) -> Value {
    json!({"logAverage": avg, "samples": n, "black": black, "stops": (STOCK.profile().normalize_target as f64 / avg).log2()})
}

// ---------------------------------------------------------------------------------------------
// Running it on a buffer (shared by the exports and the native sample renderer)
// ---------------------------------------------------------------------------------------------

/// Filters `buf` (interleaved RGB `f32`, `w × h`) in place with the host's parameters `p`.
pub fn filter(buf: &mut [f32], w: usize, h: usize, p: &Value) {
    let encoding = match p.get("encoding").and_then(Value::as_str) {
        Some("linear") => Encoding::Linear,
        _ => Encoding::Screen,
    };
    Kit::for_profile(&STOCK.profile()).run(
        buf,
        w,
        h,
        frame(p, w, h),
        encoding,
        &settings(p),
        normalize(p),
    );
}

/// Normalize Exposure's measurement of a linear buffer, as the JSON the host stores.
pub fn analyze(linear: &[f32], w: usize, h: usize) -> Option<Value> {
    let black = kit::measure_black(linear, w, h).unwrap_or(0.0);
    kit::measure(linear, w, h).map(|(avg, n)| measurement_json(avg, n, black))
}

/// One measurement for a whole roll, from each frame's.
pub fn merge(list: &Value) -> Option<Value> {
    let ms: Vec<(f64, u64)> = list
        .as_array()?
        .iter()
        .filter_map(|m| {
            Some((
                m.get("logAverage")?.as_f64()?,
                m.get("samples").and_then(Value::as_u64).unwrap_or(1),
            ))
        })
        .collect();
    // the roll's black level: the frames' levels, weighted by how many points each read
    let (mut bsum, mut bn) = (0.0f64, 0u64);
    for m in list.as_array()? {
        if let Some(b) = m.get("black").and_then(Value::as_f64) {
            let n = m.get("samples").and_then(Value::as_u64).unwrap_or(1);
            bsum += b * n as f64;
            bn += n;
        }
    }
    let black = if bn > 0 {
        (bsum / bn as f64) as f32
    } else {
        0.0
    };
    kit::merge(&ms).map(|(avg, n)| measurement_json(avg, n, black))
}

// ---------------------------------------------------------------------------------------------
// The exports (PhotoCraft plug-in ABI v1, plus the v1.1 measurement)
// ---------------------------------------------------------------------------------------------

/// `pc_filter`'s `format` for 32-bit float RGB: depth 32, colour mode 3.
const FORMAT_RGB32: i32 = 32 | (3 << 16);

/// Hands a string to the host as `len << 32 | ptr`; it lives as long as the instance.
fn give(s: String) -> i64 {
    let s: &'static mut str = s.leak();
    ((s.len() as i64) << 32) | (s.as_ptr() as usize as i64 & 0xffff_ffff)
}

/// # Safety
/// `ptr..ptr+len` must be a block the host filled (from `pc_alloc`).
unsafe fn bytes<'a>(ptr: i32, len: i32) -> &'a [u8] {
    if len <= 0 || ptr == 0 {
        return &[];
    }
    unsafe { std::slice::from_raw_parts(ptr as usize as *const u8, len as usize) }
}

#[unsafe(no_mangle)]
pub extern "C" fn pc_abi_version() -> i32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn pc_manifest() -> i64 {
    give(manifest())
}

/// A block of `n` bytes, aligned for `f32` (0 when there's no memory left).
#[unsafe(no_mangle)]
pub extern "C" fn pc_alloc(n: i32) -> i32 {
    let words = (n.max(1) as usize).div_ceil(4);
    let mut v: Vec<u32> = Vec::new();
    if v.try_reserve_exact(words).is_err() {
        return 0;
    }
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p as usize as i32
}

/// The pixel buffer as `f32`s when its size and format are what the ABI promises.
///
/// # Safety
/// `buf..buf+len` must be the block the host filled.
unsafe fn pixels<'a>(
    buf: i32,
    len: i32,
    w: i32,
    h: i32,
    channels: i32,
    format: i32,
) -> Option<(&'a mut [f32], usize, usize)> {
    let (w, h) = (usize::try_from(w).ok()?, usize::try_from(h).ok()?);
    let n = w.checked_mul(h)?.checked_mul(3)?;
    if channels != 3
        || format != FORMAT_RGB32
        || len as usize != n.checked_mul(4)?
        || buf == 0
        || buf % 4 != 0
    {
        return None;
    }
    Some((
        unsafe { std::slice::from_raw_parts_mut(buf as usize as *mut f32, n) },
        w,
        h,
    ))
}

#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn pc_filter(
    buf: i32,
    len: i32,
    w: i32,
    h: i32,
    channels: i32,
    format: i32,
    params: i32,
    params_len: i32,
) -> i32 {
    let Some((px, w, h)) = (unsafe { pixels(buf, len, w, h, channels, format) }) else {
        return 1;
    };
    let Ok(p) = serde_json::from_slice::<Value>(unsafe { bytes(params, params_len) }) else {
        return 2;
    };
    filter(px, w, h, &p);
    0
}

/// Normalize Exposure's measurement: the photo's log-average brightness (linear sRGB in). Neither
/// filter exports it any more (both Normalizes are fixed lifts, so neither has a Measure button);
/// kept for the tools.
#[cfg(any())]
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn pc_analyze(
    buf: i32,
    len: i32,
    w: i32,
    h: i32,
    channels: i32,
    format: i32,
    _params: i32,
    _params_len: i32,
) -> i64 {
    let Some((px, w, h)) = (unsafe { pixels(buf, len, w, h, channels, format) }) else {
        return 0;
    };
    analyze(px, w, h).map_or(0, |v| give(v.to_string()))
}

/// A roll's measurement from its frames' (`buf`: the JSON list of `count` measurements).
#[cfg(any())]
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn pc_merge(
    buf: i32,
    len: i32,
    _count: i32,
    _a: i32,
    _b: i32,
    _c: i32,
    _params: i32,
    _params_len: i32,
) -> i64 {
    let Ok(list) = serde_json::from_slice::<Value>(unsafe { bytes(buf, len) }) else {
        return 0;
    };
    merge(&list).map_or(0, |v| give(v.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_is_valid_json_with_everything_filled_in() {
        let m: Value = serde_json::from_str(&manifest()).unwrap();
        let id = match STOCK {
            Stock::Gold200 => "io.github.nwh212.chromagold",
            Stock::Still800T => "io.github.nwh212.chromastill",
            Stock::Purple => "io.github.nwh212.chromalome",
            Stock::Kodachrome64 => "io.github.nwh212.chromakode",
        };
        assert_eq!(m["id"], id);
        assert_eq!(m["kind"], "filter");
        assert_eq!(m["overlap"], OVERLAP);
        assert_eq!(m["version"], env!("CARGO_PKG_VERSION"));
        for svg in [&m["icon"], &m["header"]["dark"], &m["header"]["light"]] {
            let s = svg.as_str().unwrap();
            assert!(s.starts_with("<svg") && s.len() < 24 * 1024);
        }
        assert!(manifest().len() < 64 * 1024);
        assert!(m["description"].as_str().unwrap().len() <= 1024);
    }

    #[test]
    fn every_section_names_real_parameters_once() {
        let m: Value = serde_json::from_str(&manifest()).unwrap();
        let params = m["params"].as_object().unwrap();
        assert!(params.len() <= 64);
        let mut seen = std::collections::HashSet::new();
        for s in m["sections"].as_array().unwrap() {
            for p in s["params"].as_array().unwrap() {
                let p = p.as_str().unwrap();
                assert!(params.contains_key(p), "{p}");
                assert!(seen.insert(p.to_string()), "{p} twice");
            }
        }
        for (k, spec) in params {
            assert!(spec["label"].as_str().unwrap().len() <= 64, "{k}");
        }
    }

    #[test]
    fn defaults_are_the_base_look() {
        assert_eq!(settings(&json!({})), default_settings());
        // the manifest's slider defaults are the film's
        let m: Value = serde_json::from_str(&manifest()).unwrap();
        let d = default_settings().film;
        for (k, v) in [
            ("grain", d.grain),
            ("halation", d.halation),
            ("softness", d.softness),
        ] {
            assert_eq!(
                m["params"][k]["default"].as_f64().unwrap(),
                (v * 100.0).round() as f64,
                "{k}"
            );
        }
    }

    #[test]
    fn bypass_keeps_the_base() {
        let s = settings(
            &json!({"hc": true, "bandingFix": true, "denoise": "Strong", "bypass": true, "strength": 40}),
        );
        assert!(!s.hc && !s.banding && s.denoise.is_none());
        assert!(s.look && (s.strength - 0.4).abs() < 1e-6);
    }

    #[test]
    fn coolness_and_film_sliders_are_read() {
        let s = settings(&json!({"coolness": 25, "grain": 0, "halation": 100, "softness": 50}));
        assert_eq!(s.cool, 25.0);
        assert_eq!(
            s.film,
            film::Character {
                grain: 0.0,
                halation: 1.0,
                softness: 0.5
            }
        );
        assert_eq!(kit::cool_gains(25.0), kit::COOL_25);
        let b = settings(&json!({"grain": 80, "bypass": true}));
        assert_eq!(b.film.grain, 0.8); // the film's character is part of the base
    }

    #[test]
    fn denoise_menu_maps_to_the_table() {
        assert_eq!(denoise_level(&json!({"denoise": "Off"})), None);
        assert_eq!(denoise_level(&json!({})), None);
        assert_eq!(
            denoise_level(&json!({"denoise": "Light"})).unwrap().amount,
            25.0
        );
        let max = denoise_level(&json!({"denoise": "Max"})).unwrap();
        assert!(max.strong && max.amount == 100.0 && max.grain && max.color && max.detail);
    }

    #[test]
    fn analysis_round_trips_through_the_gain() {
        let lin = vec![0.045f32; 30];
        let a = analyze(&lin, 5, 2).unwrap();
        let n = normalize(&json!({"_analysis": a})).unwrap();
        let profile = STOCK.profile();
        match profile.fixed_exposure {
            // ChromaKode: a fixed lift, measured or not
            Some(stops) => assert_eq!(n.gain, 2f32.powf(stops)),
            None => assert!(
                (n.gain - (profile.normalize_target / 0.045).min(2f32.powf(1.5))).abs() < 0.01
            ), // to the target, within ±1.5 stops
        }
        assert_eq!(n.black, kit::TYPICAL_BLACK); // never the photo's own
        let roll = merge(&json!([{"logAverage": 0.09, "samples": 10, "black": 0.002}, {"logAverage": 0.36, "samples": 30, "black": 0.006}])).unwrap();
        assert!(
            (roll["logAverage"].as_f64().unwrap()
                - (0.09f64.ln() * 0.25 + 0.36f64.ln() * 0.75).exp())
            .abs()
                < 1e-9
        );
        assert!((roll["black"].as_f64().unwrap() - 0.005).abs() < 1e-6);
        // before measuring (or ChromaKode, which never measures)
        let unmeasured = profile.fixed_exposure.map_or(1.0, |s| 2f32.powf(s));
        assert_eq!(
            normalize(&json!({})),
            Some(kit::Normalize {
                gain: unmeasured,
                black: kit::TYPICAL_BLACK
            })
        );
    }

    #[test]
    fn filter_reads_the_host_parameters() {
        let mut b = vec![0.5f32; 4 * 4 * 3];
        let p = json!({"encoding": "srgb", "look": false, "normalize": false, "coolness": 0, "hue": 0, "liftedFade": true, "grain": 0, "halation": 0, "softness": 0,
                        "_image": {"x": 0, "y": 0, "canvasWidth": 4, "canvasHeight": 4}});
        let fade = color::Curve::through(&STOCK.profile().look.fade);
        filter(&mut b, 4, 4, &p);
        assert!((b[0] - fade.eval(0.5)).abs() < 0.01); // the fade curve's mid point
        let mut black = vec![0.0f32; 3];
        filter(&mut black, 1, 1, &p);
        assert!((black[0] - fade.eval(0.0)).abs() < 1e-3);
    }
}
