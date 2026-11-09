//! The Kodachrome 64 Kit: every step of the recipe sheet, run in the sheet's order on one buffer.
//!
//! Normalize Exposure → Coolness → Denoise → Kodachrome 64 → Color Shift → HC → Found →
//! Lifted Fade → Warm Band
//!
//! with the film's own character (highlight shoulder, halation, softness, grain: `film.rs`) woven
//! in where film has it: the shoulder and halation as light reaches the film, softness and grain
//! in the film itself.
//!
//! No host calls; the tests and the sample renderer run it natively.

use crate::color::*;
use crate::denoise;
use crate::film::{self, Character, Grain, GrainStyle, HalationStyle, Shoulder};
use crate::print::{GOLD200, PURPLE, Print, PrintParams, STILL800T};

// ---------------------------------------------------------------------------------------------
// The recipe sheet's numbers
// ---------------------------------------------------------------------------------------------

// The Kodachrome 64 step's numbers start from the recipe sheet's and are fitted to the Lightroom kit
// itself (Lightroom's own rendering of a photo, before and after the kit, with Normalize, Cool 25,
// Color Shift, HC and Lifted Fade), with hue shifts held close to the sheet's and neighbouring
// bands' shifts kept close, so colours change smoothly.

/// Kodachrome 64: the dye matrix (rows add up to 1, so greys stay grey).
pub const DYE_MATRIX: [Rgb; 3] = [
    [1.023, -0.059, 0.036],
    [-0.049, 1.063, -0.014],
    [0.038, -0.035, 0.997],
];

/// Kodachrome 64: colour bands.
pub const K64_BANDS: Bands = [
    band(-0.3, 21.0, 1.0),   // red
    band(-2.7, -0.1, 0.5),   // orange
    band(-2.1, -37.6, -9.0), // yellow
    band(-3.6, -12.0, -7.0), // green
    band(1.4, -3.1, -0.8),   // aqua
    band(-1.9, 11.0, -4.3),  // blue
    band(-0.6, -10.0, -3.8), // purple
    band(-0.2, -5.7, 0.4),   // magenta
];

/// Kodachrome 64: master curve, then red and blue on top.
pub const K64_MASTER: [(f32, f32); 7] = [
    (0.000, 0.000),
    (0.060, 0.030),
    (0.250, 0.250),
    (0.500, 0.464),
    (0.750, 0.684),
    (0.920, 0.824),
    (1.000, 0.964),
];
pub const K64_RED: [(f32, f32); 5] = [
    (0.000, 0.000),
    (0.250, 0.186),
    (0.500, 0.461),
    (0.750, 0.657),
    (1.000, 0.876),
];
pub const K64_GREEN: [(f32, f32); 5] = [
    (0.000, 0.000),
    (0.250, 0.236),
    (0.500, 0.493),
    (0.750, 0.705),
    (1.000, 0.931),
];
pub const K64_BLUE: [(f32, f32); 5] = [
    (0.000, 0.000),
    (0.250, 0.226),
    (0.500, 0.526),
    (0.750, 0.790),
    (1.000, 1.000),
];
/// Kodachrome 64: saturation around brightness.
pub const K64_SATURATION: f32 = 1.115;

/// Color Shift: colour bands (hue and saturation only).
pub const SHIFT_BANDS: Bands = [
    band(4.0, 5.0, 0.0),    // red
    band(-6.0, 4.0, 0.0),   // orange
    band(-10.0, 2.0, 0.0),  // yellow
    band(-12.0, -4.0, 0.0), // green
    band(10.0, 0.0, 0.0),   // aqua
    band(-8.0, 5.0, 0.0),   // blue
    band(-6.0, -5.0, 0.0),  // purple
    band(6.0, 0.0, 0.0),    // magenta
];

/// Cool 25: white-balance gains in linear light (fitted to the Lightroom kit's Cool 25; overall
/// brightness kept).
/// Coolness scales them: 25 steps is exactly this, 50 is this twice over, and so on.
pub const COOL_25: Rgb = [0.888, 1.025, 1.078];

/// Coolness's white-balance gains for a number of steps (0 = none).
pub fn cool_gains(steps: f32) -> Rgb {
    cool_gains_from(COOL_25, steps)
}

/// Coolness's gains for a number of steps, from the gains at 25 steps.
pub fn cool_gains_from(at_25: Rgb, steps: f32) -> Rgb {
    let k = steps / 25.0;
    at_25.map(|g| g.powf(k))
}

/// HC: high-contrast S-curve.
pub const HC_CURVE: [(f32, f32); 5] = [
    (0.0, 0.0),
    (0.22, 0.16),
    (0.5, 0.5),
    (0.78, 0.85),
    (1.0, 1.0),
];

/// Found: saturation kept, channel curves, corner darkening.
pub const FOUND_SATURATION: f32 = 0.85;
pub const FOUND_RED: [(f32, f32); 3] = [(0.0, 0.03), (0.5, 0.52), (1.0, 0.99)];
pub const FOUND_GREEN: [(f32, f32); 3] = [(0.0, 0.015), (0.5, 0.49), (1.0, 0.97)];
pub const FOUND_BLUE: [(f32, f32); 3] = [(0.0, 0.02), (0.5, 0.47), (1.0, 0.91)];
pub const FOUND_VIGNETTE: f32 = 0.30;
pub const FOUND_VIGNETTE_START: f32 = 0.45;

/// Lifted Fade: matte curve (black → 22/255, white → 249/255).
pub const FADE_CURVE: [(f32, f32); 4] = [(0.0, 0.085), (0.18, 0.2), (0.5, 0.5), (1.0, 0.975)];

/// Normalize Exposure: target average (linear) and largest change (stops).
/// ChromaKode 64's Normalize: a fixed exposure lift in stops (the Lightroom kit's Normalize Exposure).
pub const NORMALIZE_EXPOSURE: f32 = 0.577;
pub const NORMALIZE_TARGET: f32 = 0.153;
/// Normalize's black point, as a multiple of [`TYPICAL_BLACK`] (0: none, as the sheet has it;
/// ChromaGold 200's print has one).
pub const NORMALIZE_BLACK: f32 = 0.0;
pub const NORMALIZE_MAX_BLACK: f32 = 0.05;
/// The black level Normalize's black point is set from: a typical photo's. (Measure reads the
/// photo before your develop edits while the filter runs after them, so a black point from the
/// photo's own darkest tones could crush an edited photo's shadows.)
pub const TYPICAL_BLACK: f32 = 0.004;
pub const NORMALIZE_MAX_STOPS: f32 = 1.5;
/// Most points Normalize Exposure's measurement reads.
pub const NORMALIZE_SAMPLES: usize = 250_000;

/// Warm Band: the Lightroom kit's Banding step, measured from it on two photos: brighter, warm midtones, blue
/// shadows pulled down, a little more saturation; then ±1.2 steps of 255 of triangular noise so
/// smooth skies don't show steps.
pub const WARM_SATURATION: f32 = 1.159;
pub const WARM_RED: [(f32, f32); 6] = [
    (0.000, 0.012),
    (0.125, 0.157),
    (0.250, 0.323),
    (0.500, 0.600),
    (0.750, 0.787),
    (1.000, 1.000),
];
pub const WARM_GREEN: [(f32, f32); 6] = [
    (0.000, 0.000),
    (0.125, 0.142),
    (0.250, 0.282),
    (0.500, 0.583),
    (0.750, 0.770),
    (1.000, 1.000),
];
pub const WARM_BLUE: [(f32, f32); 6] = [
    (0.000, 0.043),
    (0.125, 0.085),
    (0.250, 0.209),
    (0.500, 0.517),
    (0.750, 0.740),
    (1.000, 0.990),
];
pub const BANDING_NOISE: f32 = 1.2 / 255.0;

/// The Kodachrome 64 step's numbers in one place: the dye matrix, the colour bands, the master
/// curve with a curve per channel on top, and saturation.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    pub dye: [Rgb; 3],
    pub bands: Bands,
    pub master: Vec<(f32, f32)>,
    pub red: Vec<(f32, f32)>,
    pub green: Vec<(f32, f32)>,
    pub blue: Vec<(f32, f32)>,
    pub saturation: f32,
    /// Coolness's white-balance gains at 25 steps.
    pub cool25: Rgb,
    /// How much muted colours take of the colour bands (the look's and Color Shift's).
    pub gate: BandGate,
    /// Color Shift's bands, HC's curve and Lifted Fade's curve.
    pub shift: Bands,
    pub hc: Vec<(f32, f32)>,
    pub fade: Vec<(f32, f32)>,
}

impl Look {
    /// The kit's own numbers (the constants above).
    pub fn standard() -> Self {
        Look {
            dye: DYE_MATRIX,
            bands: K64_BANDS,
            master: K64_MASTER.to_vec(),
            red: K64_RED.to_vec(),
            green: K64_GREEN.to_vec(),
            blue: K64_BLUE.to_vec(),
            saturation: K64_SATURATION,
            cool25: COOL_25,
            gate: BandGate::MUTED,
            shift: K64_SHIFT_BANDS,
            hc: K64_HC_CURVE.to_vec(),
            fade: K64_FADE_CURVE.to_vec(),
        }
    }
}

impl Default for Look {
    fn default() -> Self {
        Self::standard()
    }
}

// ---------------------------------------------------------------------------------------------
// ChromaGold 200: Kodak Gold 200 colour negative film, printed
// ---------------------------------------------------------------------------------------------
//
// The film itself is a physical model of the C-41 negative and its print, from Kodak's data sheet
// (E-7022): `print.rs`. On top of it, only what Gold 200 is known for beyond its sheet: yellows
// and oranges a little brighter, greens toward olive, reds a touch toward carmine.
// As in Lightroom's colour mixer, muted colours take these too; greys don't.

pub const GOLD_BANDS: Bands = [
    band(-2.0, -4.0, 0.0),  // red: a touch toward carmine
    band(3.0, 8.0, 6.0),    // orange: brighter (and skin, between red and orange, leans golden)
    band(-2.0, 10.0, 6.0),  // yellow: bright, a touch golden
    band(-4.0, -12.0, 0.0), // green: toward olive and grass green
    band(0.0, -8.0, 0.0),   // aqua
    band(0.0, -4.0, 2.0),   // blue: true, a little pale
    band(0.0, -4.0, 0.0),   // purple
    band(0.0, -6.0, 0.0),   // magenta
];
pub const GOLD_SATURATION: f32 = 0.88;
/// Normalize (Gold): a fixed lift in stops, as ChromaKode's.
pub const GOLD_NORMALIZE_EXPOSURE: f32 = 0.33;
pub const GOLD_NORMALIZE_TARGET: f32 = 0.15;
pub const GOLD_NORMALIZE_BLACK: f32 = 1.0;
pub const GOLD_GRAIN: GrainStyle = GrainStyle {
    size: 2.4,
    amount: 34.0 / 255.0,
    color: 0.22,
    fine: 0.0,
    cloud: 0.8,
    shadow: 0.35,
};
/// Halation relative to ChromaKode 64's: negative film has an anti-halation layer.
pub const GOLD_HALATION: f32 = 0.6;

/// Exposure (Gold): an under-exposed negative's thin shadows print grainier (the colour and tone of
/// an off exposure come from the print model).
pub const UNDER_GRAIN: f32 = 0.8;
/// Sun Cast (Gold): the soft yellow cast Gold takes in harsh midday sun, strongest in the light.
pub const SUN_TINT: Rgb = [0.035, 0.02, -0.06];

/// ChromaKode 64's Color Shift, HC and Lifted Fade, fitted to the Lightroom kit's (ChromaGold 200
/// keeps the sheet's: [`SHIFT_BANDS`], [`HC_CURVE`], [`FADE_CURVE`]).
pub const K64_SHIFT_BANDS: Bands = [
    band(1.7, 16.9, -0.6),   // red
    band(-2.0, 11.4, -1.7),  // orange
    band(-4.2, -11.2, -6.3), // yellow
    band(-4.5, -4.8, -5.0),  // green
    band(3.6, 0.4, 4.2),     // aqua
    band(-3.4, 3.9, -6.0),   // blue
    band(-3.1, -5.1, -0.2),  // purple
    band(2.5, 0.2, -0.1),    // magenta
];
pub const K64_HC_CURVE: [(f32, f32); 5] = [
    (0.000, 0.000),
    (0.220, 0.146),
    (0.500, 0.485),
    (0.780, 0.799),
    (1.000, 1.000),
];
pub const K64_FADE_CURVE: [(f32, f32); 4] = [
    (0.000, 0.017),
    (0.180, 0.197),
    (0.500, 0.500),
    (1.000, 0.995),
];

// ---------------------------------------------------------------------------------------------
// ChromaStill 800T: CineStill 800T (Kodak VISION3 500T with its rem-jet removed)
// ---------------------------------------------------------------------------------------------
//
// The film is the same physical negative-and-print model as Gold's, with 5219's sheet (`print.rs`).
// What sets it apart is its halation: without the rem-jet backing, light that goes through the
// emulsion bounces off the back of the base and comes back into the bottom (red-sensitive) layer,
// so bright lights and edges get a saturated red ring: almost pure red-layer exposure, peaking
// about 0.2-0.4 mm out from the light and gone by about 0.5 mm, around anything near or above white.

pub const STILL_HALATION: HalationStyle = HalationStyle {
    source: [1.0, 0.0, 0.0],
    start: 0.55,
    tint: [1.0, 0.1, 0.02],
    radius: 14.0,
    gain: 3.2,
};
pub const STILL_GRAIN: GrainStyle = GrainStyle {
    size: 2.6,
    amount: 38.0 / 255.0,
    color: 0.3,
    fine: 0.0,
    cloud: 0.8,
    shadow: 0.4,
};
pub const STILL_SATURATION: f32 = 0.95;
pub const STILL_NORMALIZE_EXPOSURE: f32 = 0.33;
/// Hue (ChromaStill): how far the picture leans green-blue, as 800T usually does (a tungsten
/// film under mixed and daylight light, printed or scanned): the print's colour balance at 100 %,
/// as linear gains applied after the film.
pub const HUE_GAINS: Rgb = [0.7, 1.1, 1.3];
/// Hue as the filter loads: the gentle green-blue 800T is known for.
pub const STILL_HUE: f32 = 0.3;

// ---------------------------------------------------------------------------------------------
// ChromaLome Purple: LomoChrome Purple
// ---------------------------------------------------------------------------------------------
//
// The colour swap is the film model's (`print.rs`, [`PURPLE`]). The look comes from the film alone:
// no colour bands, which shift hues by band and could make a blurred edge between two colours
// do something odd; every step here is smooth.

pub const PURPLE_GRAIN: GrainStyle = GrainStyle {
    size: 2.0,
    amount: 30.0 / 255.0,
    color: 0.25,
    fine: 0.0,
    cloud: 0.8,
    shadow: 0.3,
};
/// Warmth (ChromaLome): the scan's lean toward cream-yellow highlights, at 100 %.
pub const WARM_TINT: Rgb = [1.18, 1.05, 0.78];
/// Warmth as the filter loads.
pub const PURPLE_WARMTH: f32 = 0.3;
/// ISO (ChromaLome): the speed the film was rated at, as stops of exposure from ISO 100. 100 is two
/// stops over (Lomography's chart: faded, pastel); 400 is where the film looks right (the print
/// model's rated scan, [`crate::print::PURPLE_400`]).
pub fn iso_stops(iso: u32) -> f32 {
    match iso {
        200 => -1.0,
        400 => -2.0,
        _ => 0.0,
    }
}

/// Hue's gains for an amount 0-1, brightness kept (ChromaStill's green-blue).
pub fn hue_gains(amount: f32) -> Rgb {
    tint_gains(HUE_GAINS, amount)
}

/// A film's print/scan tint for an amount 0-1, from its gains at 100 %, brightness kept.
pub fn tint_gains(at_full: Rgb, amount: f32) -> Rgb {
    let g = at_full.map(|v| v.powf(amount));
    let norm = 1.0 / dot(g, LUMA_709);
    g.map(|v| v * norm)
}

/// Which film a filter is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stock {
    Kodachrome64,
    Gold200,
    Still800T,
    Purple,
}

/// Everything that differs between the films: the look's numbers, Normalize's level and black
/// point, the grain, halation's strength, the Film sliders' defaults, and Warm Band's grade.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    pub stock: Stock,
    pub look: Look,
    pub normalize_target: f32,
    pub normalize_black: f32,
    pub grain: GrainStyle,
    pub halation: f32,
    pub halation_style: HalationStyle,
    pub film: Character,
    /// The film-and-print model (ChromaGold 200), in place of the dye matrix and curves.
    pub print: Option<PrintParams>,
    /// Normalize as a fixed exposure lift, in stops, when the film has one (ChromaKode 64, as the
    /// Lightroom kit's Normalize Exposure preset); `None`: Normalize is measured (ChromaGold 200).
    pub fixed_exposure: Option<f32>,
    /// Coolness as the filter loads (ChromaKode 64: 25, the kit's Cool 25; Gold has none).
    pub cool: f32,
    /// The print/scan tint the film's tint control (Hue, Warmth) leans toward at 100 %, and how
    /// far it leans as the filter loads.
    pub tint: Rgb,
    pub hue: f32,
    pub warm_band: bool,
}

impl Stock {
    pub fn profile(self) -> Profile {
        match self {
            Stock::Kodachrome64 => Profile {
                stock: self,
                look: Look::standard(),
                normalize_target: NORMALIZE_TARGET,
                normalize_black: NORMALIZE_BLACK,
                grain: GrainStyle::default(),
                halation: 1.0,
                halation_style: HalationStyle::default(),
                film: Character::default(),
                print: None,
                fixed_exposure: Some(NORMALIZE_EXPOSURE),
                cool: 25.0,
                tint: [1.0; 3],
                hue: 0.0,
                warm_band: true,
            },
            Stock::Gold200 => Profile {
                stock: self,
                look: Look {
                    dye: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    bands: GOLD_BANDS,
                    master: vec![(0.0, 0.0), (1.0, 1.0)],
                    red: vec![(0.0, 0.0), (1.0, 1.0)],
                    green: vec![(0.0, 0.0), (1.0, 1.0)],
                    blue: vec![(0.0, 0.0), (1.0, 1.0)],
                    saturation: GOLD_SATURATION,
                    cool25: COOL_25,
                    gate: BandGate::MUTED,
                    shift: SHIFT_BANDS,
                    hc: HC_CURVE.to_vec(),
                    fade: K64_FADE_CURVE.to_vec(),
                },
                normalize_target: GOLD_NORMALIZE_TARGET,
                normalize_black: GOLD_NORMALIZE_BLACK,
                grain: GOLD_GRAIN,
                halation: GOLD_HALATION,
                halation_style: HalationStyle::default(),
                film: Character {
                    grain: 0.55,
                    halation: 0.3,
                    softness: 0.3,
                },
                print: Some(GOLD200),
                fixed_exposure: Some(GOLD_NORMALIZE_EXPOSURE),
                cool: 0.0,
                tint: [1.0; 3],
                hue: 0.0,
                warm_band: false,
            },
            Stock::Still800T => Profile {
                stock: self,
                look: Look {
                    dye: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    bands: [band(0.0, 0.0, 0.0); 8],
                    master: vec![(0.0, 0.0), (1.0, 1.0)],
                    red: vec![(0.0, 0.0), (1.0, 1.0)],
                    green: vec![(0.0, 0.0), (1.0, 1.0)],
                    blue: vec![(0.0, 0.0), (1.0, 1.0)],
                    saturation: STILL_SATURATION,
                    cool25: COOL_25,
                    gate: BandGate::MUTED,
                    shift: SHIFT_BANDS,
                    hc: HC_CURVE.to_vec(),
                    fade: FADE_CURVE.to_vec(), // the sheet's real matte: 800T's dense blacks swallow the gentle one
                },
                normalize_target: GOLD_NORMALIZE_TARGET,
                normalize_black: GOLD_NORMALIZE_BLACK,
                grain: STILL_GRAIN,
                halation: 1.0,
                halation_style: STILL_HALATION,
                film: Character {
                    grain: 0.6,
                    halation: 0.7,
                    softness: 0.25,
                },
                print: Some(STILL800T),
                fixed_exposure: Some(STILL_NORMALIZE_EXPOSURE),
                cool: 0.0,
                tint: HUE_GAINS,
                hue: STILL_HUE,
                warm_band: false,
            },
            Stock::Purple => Profile {
                stock: self,
                look: Look {
                    dye: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    bands: [band(0.0, 0.0, 0.0); 8],
                    master: vec![(0.0, 0.0), (1.0, 1.0)],
                    red: vec![(0.0, 0.0), (1.0, 1.0)],
                    green: vec![(0.0, 0.0), (1.0, 1.0)],
                    blue: vec![(0.0, 0.0), (1.0, 1.0)],
                    saturation: 1.0,
                    cool25: COOL_25,
                    gate: BandGate::MUTED,
                    shift: SHIFT_BANDS,
                    hc: HC_CURVE.to_vec(),
                    fade: FADE_CURVE.to_vec(), // a real matte, as ChromaStill's
                },
                normalize_target: GOLD_NORMALIZE_TARGET,
                normalize_black: GOLD_NORMALIZE_BLACK,
                grain: PURPLE_GRAIN,
                halation: GOLD_HALATION,
                halation_style: HalationStyle::default(),
                film: Character {
                    grain: 0.45,
                    halation: 0.25,
                    softness: 0.25,
                },
                print: Some(PURPLE),
                fixed_exposure: Some(STILL_NORMALIZE_EXPOSURE),
                cool: 0.0,
                tint: WARM_TINT,
                hue: PURPLE_WARMTH,
                warm_band: false,
            },
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------------------------

/// Which steps are on, as the panel sets them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    // [BASE]
    pub normalize: bool,
    pub look: bool,
    /// Look strength, 0–1.
    pub strength: f32,
    /// Film character: grain, halation, softness (part of the base).
    pub film: Character,
    // [k] modifiers
    /// Coolness, in white-balance steps toward blue (the sheet's Cool 25 is 25).
    pub cool: f32,
    pub denoise: Option<denoise::Options>,
    pub color_shift: bool,
    pub hc: bool,
    pub found: bool,
    pub fade: bool,
    /// Exposure (Gold), in stops: how the negative was exposed (−2 … +3).
    pub exposure: f32,
    /// Sun Cast (Gold), 0–1.
    pub sun: f32,
    /// Hue (ChromaStill), 0–1: how far the picture leans green-blue.
    pub hue: f32,
    // [x] extras
    pub banding: bool,
}

impl Default for Settings {
    /// The kit as it loads: the two base steps on, every modifier off.
    fn default() -> Self {
        Self {
            normalize: true,
            look: true,
            strength: 1.0,
            film: Character::default(),
            cool: 0.0,
            denoise: None,
            color_shift: false,
            hc: false,
            found: false,
            fade: false,
            exposure: 0.0,
            sun: 0.0,
            hue: 0.0,
            banding: false,
        }
    }
}

impl Settings {
    /// These settings with every [k] and [x] modifier off and the base steps kept: what the kit's
    /// `.RESET MODS` did, without losing the modifiers' own settings.
    pub fn base_only(self) -> Self {
        Settings {
            normalize: self.normalize,
            look: self.look,
            strength: self.strength,
            film: self.film,
            ..Settings {
                normalize: false,
                look: false,
                ..Default::default()
            }
        }
    }
}

/// Where the buffer sits in the whole photo (the host's `_image`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    /// The buffer's top-left pixel in the photo (may be negative: the host pads bands).
    pub x: i64,
    pub y: i64,
    pub canvas_w: f32,
    pub canvas_h: f32,
}

impl Frame {
    pub fn whole(w: usize, h: usize) -> Self {
        Frame {
            x: 0,
            y: 0,
            canvas_w: w as f32,
            canvas_h: h as f32,
        }
    }
}

/// How the host encodes the samples it hands over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Encoding {
    /// sRGB-curve screen values (the Display stage).
    Screen,
    /// Linear light (the Source stage, HDR renders).
    Linear,
}

// ---------------------------------------------------------------------------------------------
// Normalize Exposure's measurement
// ---------------------------------------------------------------------------------------------

/// The photo's log-average linear luminance over at most [`NORMALIZE_SAMPLES`] evenly spread
/// points, and how many points it read.
pub fn measure(linear: &[f32], w: usize, h: usize) -> Option<(f64, u64)> {
    if w == 0 || h == 0 || linear.len() < w * h * 3 {
        return None;
    }
    let step = ((w * h) as f64 / NORMALIZE_SAMPLES as f64)
        .sqrt()
        .ceil()
        .max(1.0) as usize;
    let (mut sum, mut n) = (0.0f64, 0u64);
    for y in (step / 2..h).step_by(step) {
        for x in (step / 2..w).step_by(step) {
            let i = (y * w + x) * 3;
            let l = dot([linear[i], linear[i + 1], linear[i + 2]], LUMA_709);
            if l.is_finite() {
                sum += (1e-4 + l.max(0.0) as f64).ln();
                n += 1;
            }
        }
    }
    (n > 0).then(|| ((sum / n as f64).exp(), n))
}

/// The photo's black level: the linear luminance its darkest 0.5 % of points sit under, over the
/// same points as [`measure`].
pub fn measure_black(linear: &[f32], w: usize, h: usize) -> Option<f32> {
    if w == 0 || h == 0 || linear.len() < w * h * 3 {
        return None;
    }
    let step = ((w * h) as f64 / NORMALIZE_SAMPLES as f64)
        .sqrt()
        .ceil()
        .max(1.0) as usize;
    let mut ls = Vec::new();
    for y in (step / 2..h).step_by(step) {
        for x in (step / 2..w).step_by(step) {
            let i = (y * w + x) * 3;
            let l = dot([linear[i], linear[i + 1], linear[i + 2]], LUMA_709);
            if l.is_finite() {
                ls.push(l.max(0.0));
            }
        }
    }
    if ls.is_empty() {
        return None;
    }
    let k = ls.len() / 200;
    Some(*ls.select_nth_unstable_by(k, |a, b| a.total_cmp(b)).1)
}

/// Normalize's correction for one photo, from its measurement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Normalize {
    /// Linear gain.
    pub gain: f32,
    /// The measured black level ([`measure_black`]); 0 when the measurement has none.
    pub black: f32,
}

impl Normalize {
    /// The linear level that goes to black.
    pub fn black_point(&self, strength: f32) -> f32 {
        (self.black * strength).clamp(0.0, NORMALIZE_MAX_BLACK)
    }
}

/// The linear gain that brings a log average to [`NORMALIZE_TARGET`], at most ±1.5 stops.
pub fn normalize_gain(log_average: f64) -> f32 {
    normalize_gain_to(log_average, NORMALIZE_TARGET)
}

/// The linear gain that brings a log average to `target`, at most ±1.5 stops.
pub fn normalize_gain_to(log_average: f64, target: f32) -> f32 {
    if !(log_average.is_finite() && log_average > 0.0) {
        return 1.0;
    }
    let stops = (target as f64 / log_average)
        .log2()
        .clamp(-NORMALIZE_MAX_STOPS as f64, NORMALIZE_MAX_STOPS as f64);
    2f64.powf(stops) as f32
}

/// One measurement of a whole roll: the log average over every frame's points.
pub fn merge(measurements: &[(f64, u64)]) -> Option<(f64, u64)> {
    let (mut sum, mut n) = (0.0f64, 0u64);
    for &(avg, count) in measurements {
        if avg.is_finite() && avg > 0.0 && count > 0 {
            sum += avg.ln() * count as f64;
            n += count;
        }
    }
    (n > 0).then(|| ((sum / n as f64).exp(), n))
}

// ---------------------------------------------------------------------------------------------
// The kit
// ---------------------------------------------------------------------------------------------

/// The curves the steps use, built once per call.
pub struct Kit {
    transfer: Transfer,
    normalize_black: f32,
    cool25: Rgb,
    gate: BandGate,
    shift: Bands,
    print: Option<PrintParams>,
    halation_style: HalationStyle,
    tint: Rgb,
    grain_style: GrainStyle,
    halation: f32,
    warm_band: bool,
    dye: [Rgb; 3],
    bands: Bands,
    saturation: f32,
    k64: [Curve; 3],
    hc: Curve,
    found: [Curve; 3],
    fade: Curve,
    warm: [Curve; 3],
}

impl Kit {
    pub fn new() -> Self {
        Self::for_profile(&Stock::Kodachrome64.profile())
    }

    /// ChromaKode 64 with its film look's numbers swapped (the fitting tools use this).
    pub fn with_look(look: &Look) -> Self {
        Self::for_profile(&Profile {
            look: look.clone(),
            ..Stock::Kodachrome64.profile()
        })
    }

    /// The kit for one film.
    pub fn for_profile(profile: &Profile) -> Self {
        let look = &profile.look;
        let master = Curve::through(&look.master);
        Kit {
            transfer: Transfer::new(),
            normalize_black: profile.normalize_black,
            cool25: look.cool25,
            gate: look.gate,
            shift: look.shift,
            print: profile.print,
            halation_style: profile.halation_style,
            tint: profile.tint,
            grain_style: profile.grain,
            halation: profile.halation,
            warm_band: profile.warm_band,
            dye: look.dye,
            bands: look.bands,
            saturation: look.saturation,
            k64: [&look.red, &look.green, &look.blue].map(|pts| master.then(&Curve::through(pts))),
            hc: Curve::through(&look.hc),
            found: [
                Curve::through(&FOUND_RED),
                Curve::through(&FOUND_GREEN),
                Curve::through(&FOUND_BLUE),
            ],
            fade: Curve::through(&look.fade),
            warm: [
                Curve::through(&WARM_RED),
                Curve::through(&WARM_GREEN),
                Curve::through(&WARM_BLUE),
            ],
        }
    }

    /// Kodachrome 64 on one screen-value pixel, before the Look strength blend.
    pub fn kodachrome(&self, c: Rgb) -> Rgb {
        self.film_look(c, None)
    }

    /// The film's look on one screen-value pixel: the print model when the film has one
    /// (ChromaGold), else the dye matrix and curves (ChromaKode); then the colour bands and
    /// saturation.
    fn film_look(&self, c: Rgb, print: Option<&Print>) -> Rgb {
        if let Some(print) = print {
            let c = clamp01(print.apply(c, &self.transfer));
            let c = apply_bands_gated(c, &self.bands, self.gate);
            return clamp01(saturate(c, self.saturation * print.saturation));
        }
        let c = clamp01(mix3(&self.dye, c));
        let c = apply_bands_gated(c, &self.bands, self.gate);
        let c = [
            self.k64[0].eval(c[0]),
            self.k64[1].eval(c[1]),
            self.k64[2].eval(c[2]),
        ];
        clamp01(saturate(c, self.saturation))
    }

    /// Runs every step that's on over `buf` (interleaved RGB, `w × h`, in `encoding`), in place.
    /// `norm`: Normalize's correction (from the photo's measurement), or `None` when it hasn't
    /// been measured.
    pub fn run(
        &self,
        buf: &mut [f32],
        w: usize,
        h: usize,
        frame: Frame,
        encoding: Encoding,
        s: &Settings,
        norm: Option<Normalize>,
    ) {
        let t = &self.transfer;
        let n = w * h;
        if buf.len() < n * 3 {
            return;
        }
        let buf = &mut buf[..n * 3];

        let long = frame.canvas_w.max(frame.canvas_h);

        // 1) Normalize Exposure and 2) Coolness: gains in linear light. Halation's source is read
        // here, from the light as it reaches the film; then the highlight shoulder rolls off
        // whatever the gain pushed past white, and everything goes to screen values.
        let norm = if s.normalize { norm } else { None };
        let g = norm.map_or(1.0, |n| n.gain);
        let kb = norm.map_or(0.0, |n| n.black_point(self.normalize_black));
        let wb = cool_gains_from(self.cool25, s.cool);
        let hue = (s.hue > 0.0).then(|| tint_gains(self.tint, s.hue));
        let gains = [g * wb[0], g * wb[1], g * wb[2]];
        let unity = gains == [1.0; 3] && kb == 0.0;
        let shoulder = Shoulder::new(g.max(wb[0] * g).max(wb[1] * g).max(wb[2] * g));
        let want_halation = s.film.halation > 0.0;
        let mut source = if want_halation {
            Vec::with_capacity(n)
        } else {
            Vec::new()
        };
        for v in buf.chunks_exact_mut(3) {
            if unity && encoding == Encoding::Screen && !want_halation {
                v.iter_mut().for_each(|x| *x = x.clamp(0.0, 1.0));
                continue;
            }
            let mut lin = [0.0f32; 3];
            for c in 0..3 {
                let x = match encoding {
                    Encoding::Screen => t.to_linear(v[c].clamp(0.0, 1.0)),
                    Encoding::Linear => v[c].max(0.0),
                };
                lin[c] = ((x - kb).max(0.0) / (1.0 - kb)) * gains[c];
            }
            if want_halation {
                source.push(film::halation_source_weighted(
                    lin,
                    self.halation_style.source,
                    self.halation_style.start,
                ));
            }
            for c in 0..3 {
                v[c] = t.to_screen(shoulder.apply(lin[c]).min(1.0)).clamp(0.0, 1.0);
            }
        }
        let glow = if want_halation {
            film::halation_glow_styled(
                source,
                w,
                h,
                s.film.halation * self.halation,
                long,
                &self.halation_style,
            )
        } else {
            None
        };

        // 3) Denoise, then the film's softness
        if let Some(opts) = &s.denoise {
            denoise::apply(buf, w, h, opts, &denoise::Sizes::for_photo(opts, long));
        }
        film::soften(buf, w, h, s.film.softness, long);
        let under = (-s.exposure / 2.0).clamp(0.0, 1.0);
        // Hue (ChromaStill) is the print's or scan's colour balance, so it goes in the print model
        // (after the film, so the halation the film made from the scene's light keeps its red)
        let print = self
            .print
            .as_ref()
            .map(|p| Print::balanced(p, s.exposure, hue.unwrap_or([1.0; 3])));
        let grain = Grain::styled(
            s.film.grain * (1.0 + UNDER_GRAIN * under),
            long,
            &self.grain_style,
        );

        // 4) – 9): the per-pixel steps
        for (i, v) in buf.chunks_exact_mut(3).enumerate() {
            let px = frame.x + (i % w) as i64;
            let py = frame.y + (i / w) as i64;
            let mut c: Rgb = [v[0], v[1], v[2]];

            // halation: the glow from nearby highlights, added as light (red-orange, as it
            // bounces back through the film's red layer)
            if let Some(glow) = &glow {
                // it shows in the darker parts next to a highlight, not on the highlight itself
                let shade = 1.0 - dot(c, LUMA_709).clamp(0.0, 1.0);
                let k = glow[i] * shade * shade;
                if k > 0.0 {
                    let tint = self.halation_style.tint;
                    c = [0, 1, 2].map(|j| t.to_screen((t.to_linear(c[j]) + k * tint[j]).min(1.0)));
                }
            }

            // 4) Kodachrome 64, blended with the photo before it by Look strength
            if s.look && s.strength > 0.0 {
                c = lerp3(c, self.film_look(c, print.as_ref()), s.strength);
            }
            // Sun Cast (Gold)
            if s.sun > 0.0 {
                let l = dot(c, LUMA_709).clamp(0.0, 1.0);
                c = [0, 1, 2].map(|k| c[k] + SUN_TINT[k] * s.sun * l);
            }
            // 5) Color Shift
            if s.color_shift {
                c = apply_bands_gated(c, &self.shift, self.gate);
            }
            // 6) HC
            if s.hc {
                c = [self.hc.eval(c[0]), self.hc.eval(c[1]), self.hc.eval(c[2])];
            }
            // 7) Found
            if s.found {
                let k = clamp01(saturate(c, FOUND_SATURATION));
                c = [
                    self.found[0].eval(k[0]),
                    self.found[1].eval(k[1]),
                    self.found[2].eval(k[2]),
                ];
                let dx = ((px as f32 + 0.5) / frame.canvas_w - 0.5) * 2.0;
                let dy = ((py as f32 + 0.5) / frame.canvas_h - 0.5) * 2.0;
                let r = (dx * dx + dy * dy).sqrt() / std::f32::consts::SQRT_2;
                let dark = 1.0 - FOUND_VIGNETTE * smoothstep(FOUND_VIGNETTE_START, 1.0, r);
                c = [c[0] * dark, c[1] * dark, c[2] * dark];
            }
            // 8) Lifted Fade
            if s.fade {
                c = [
                    self.fade.eval(c[0]),
                    self.fade.eval(c[1]),
                    self.fade.eval(c[2]),
                ];
            }
            // 9) Warm Band's grade
            if s.banding && self.warm_band {
                let k = clamp01(saturate(clamp01(c), WARM_SATURATION));
                c = [
                    self.warm[0].eval(k[0]),
                    self.warm[1].eval(k[1]),
                    self.warm[2].eval(k[2]),
                ];
            }
            // the film's grain
            if let Some(grain) = &grain {
                c = grain.apply(c, px, py);
            }
            // and Warm Band's noise: triangular, fixed by position, the same on R, G and B
            if s.banding {
                let tri = hash01(px, py, 0x6b64) + hash01(px, py, 0x1e57) - 1.0;
                let d = tri * BANDING_NOISE;
                c = [c[0] + d, c[1] + d, c[2] + d];
            }

            let c = clamp01(c);
            for k in 0..3 {
                v[k] = match encoding {
                    Encoding::Screen => c[k],
                    Encoding::Linear => t.to_linear(c[k]),
                };
            }
        }
    }
}

impl Default for Kit {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grey(w: usize, h: usize, v: f32) -> Vec<f32> {
        vec![v; w * h * 3]
    }

    #[test]
    fn dye_matrix_rows_add_up_to_one() {
        for row in DYE_MATRIX {
            assert!((row.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn kodachrome_keeps_greys_nearly_neutral_and_whites_short_of_paper() {
        let kit = Kit::new();
        // cool, as the Lightroom kit is, but no colour cast beyond that
        let mid = kit.kodachrome([0.5, 0.5, 0.5]);
        assert!(
            mid[2] >= mid[0]
                && mid[2] - mid[0] < 0.12
                && (mid[1] - (mid[0] + mid[2]) / 2.0).abs() < 0.03,
            "{mid:?}"
        );
        let white = kit.kodachrome([1.0, 1.0, 1.0]);
        assert!(
            white.iter().all(|&v| v <= 1.0) && white[0].min(white[1]) < 0.99,
            "{white:?}"
        );
        let black = kit.kodachrome([0.0, 0.0, 0.0]);
        assert!(black.iter().all(|&v| v < 0.01));
    }

    #[test]
    fn kodachrome_makes_reds_richer() {
        let kit = Kit::new();
        let (_, s0, _) = rgb_to_hsl([0.7, 0.25, 0.2]);
        let (_, s1, _) = rgb_to_hsl(kit.kodachrome([0.7, 0.25, 0.2]));
        assert!(s1 > s0);
    }

    #[test]
    fn look_strength_zero_leaves_the_photo() {
        let kit = Kit::new();
        let mut b = vec![0.2, 0.4, 0.6, 0.9, 0.1, 0.3];
        let orig = b.clone();
        let s = Settings {
            normalize: false,
            strength: 0.0,
            film: Character::NONE,
            ..Settings::default()
        };
        kit.run(&mut b, 2, 1, Frame::whole(2, 1), Encoding::Screen, &s, None);
        for (a, o) in b.iter().zip(&orig) {
            assert!((a - o).abs() < 1e-6);
        }
    }

    #[test]
    fn normalize_moves_the_average_to_the_target_within_limits() {
        let lin = grey(10, 10, NORMALIZE_TARGET / 2.0 - 1e-4);
        let (avg, n) = measure(&lin, 10, 10).unwrap();
        assert_eq!(n, 100);
        assert!((normalize_gain(avg) - 2.0).abs() < 0.01);
        assert!((normalize_gain(0.001) - 2f32.powf(1.5)).abs() < 1e-3);
        assert!((normalize_gain(10.0) - 2f32.powf(-1.5)).abs() < 1e-3);
    }

    #[test]
    fn merge_weights_by_points() {
        let (m, n) = merge(&[(0.1, 100), (0.4, 100)]).unwrap();
        assert_eq!(n, 200);
        assert!((m - 0.2).abs() < 1e-9);
        assert!(merge(&[]).is_none());
    }

    #[test]
    fn lifted_fade_lifts_black() {
        let kit = Kit::new();
        let mut b = grey(1, 1, 0.0);
        let s = Settings {
            normalize: false,
            look: false,
            fade: true,
            film: Character::NONE,
            ..Settings::default()
        };
        kit.run(&mut b, 1, 1, Frame::whole(1, 1), Encoding::Screen, &s, None);
        assert!((b[0] - K64_FADE_CURVE[0].1).abs() < 1e-3 && b[0] > 0.0); // ChromaKode: the Lightroom kit's gentle lift
        let gold = Kit::for_profile(&Stock::Gold200.profile());
        let mut b = grey(1, 1, 0.0);
        gold.run(&mut b, 1, 1, Frame::whole(1, 1), Encoding::Screen, &s, None);
        assert!((b[0] - K64_FADE_CURVE[0].1).abs() < 1e-3); // ChromaGold: the same gentle lift
    }

    #[test]
    fn coolness_is_bluer_and_keeps_greys_balanced() {
        let kit = Kit::new();
        let mut b = grey(1, 1, 0.5);
        let s = Settings {
            normalize: false,
            look: false,
            cool: 25.0,
            film: Character::NONE,
            ..Settings::default()
        };
        kit.run(&mut b, 1, 1, Frame::whole(1, 1), Encoding::Screen, &s, None);
        assert!(b[2] > b[1] && b[1] > b[0]);
    }

    #[test]
    fn warm_band_warms_and_its_noise_is_small_and_repeatable() {
        let kit = Kit::new();
        let s = Settings {
            normalize: false,
            look: false,
            banding: true,
            film: Character::NONE,
            ..Settings::default()
        };
        let mut a = grey(32, 32, 0.5);
        let mut b = grey(32, 32, 0.5);
        kit.run(
            &mut a,
            32,
            32,
            Frame::whole(32, 32),
            Encoding::Screen,
            &s,
            None,
        );
        kit.run(
            &mut b,
            32,
            32,
            Frame::whole(32, 32),
            Encoding::Screen,
            &s,
            None,
        );
        assert_eq!(a, b);
        let graded = [
            kit.warm[0].eval(0.5),
            kit.warm[1].eval(0.5),
            kit.warm[2].eval(0.5),
        ];
        assert!(graded[0] > graded[2] + 0.04, "{graded:?}");
        for px in a.chunks_exact(3) {
            for k in 0..3 {
                assert!((px[k] - graded[k]).abs() <= BANDING_NOISE + 1e-6);
            }
        }
        assert!(a.chunks_exact(3).any(|px| (px[0] - graded[0]).abs() > 1e-4));
    }

    #[test]
    fn bands_split_the_same_as_the_whole() {
        let s = Settings {
            found: true,
            banding: true,
            cool: 40.0,
            denoise: Some(denoise::Options {
                amount: 100.0,
                strong: true,
                ..Default::default()
            }),
            film: Character {
                grain: 1.0,
                halation: 1.0,
                softness: 1.0,
            },
            ..Settings::default()
        };
        bands_match(&Kit::new(), &s);
    }

    #[test]
    fn gold_bands_split_the_same_as_the_whole() {
        let gold = Kit::for_profile(&Stock::Gold200.profile());
        for exposure in [-2.0, 3.0] {
            let s = Settings {
                banding: true,
                fade: true,
                exposure,
                sun: 0.7,
                denoise: Some(denoise::Options {
                    amount: 100.0,
                    strong: true,
                    ..Default::default()
                }),
                film: Character {
                    grain: 1.0,
                    halation: 1.0,
                    softness: 1.0,
                },
                ..Settings::default()
            };
            bands_match(&gold, &s);
        }
    }

    #[test]
    fn still_bands_split_the_same_as_the_whole() {
        let still = Kit::for_profile(&Stock::Still800T.profile());
        let s = Settings {
            banding: true,
            fade: true,
            exposure: -1.0,
            hue: 0.6,
            denoise: Some(denoise::Options {
                amount: 100.0,
                strong: true,
                ..Default::default()
            }),
            film: Character {
                grain: 1.0,
                halation: 1.0,
                softness: 1.0,
            },
            ..Settings::default()
        };
        bands_match(&still, &s);
    }

    #[test]
    fn still_halation_is_red() {
        // a bright light on a dark ground: the ring around it gains red, hardly any green or blue
        let still = Kit::for_profile(&Stock::Still800T.profile());
        let (w, h) = (80, 80);
        let mut with = vec![0.1f32; w * h * 3];
        for y in 36..44 {
            for x in 36..44 {
                with[(y * w + x) * 3..][..3].copy_from_slice(&[1.0; 3]);
            }
        }
        let mut without = with.clone();
        let on = Settings {
            normalize: false,
            look: false,
            film: Character {
                grain: 0.0,
                halation: 1.0,
                softness: 0.0,
            },
            ..Settings::default()
        };
        let off = Settings {
            film: Character::NONE,
            ..on
        };
        let frame = Frame {
            x: 0,
            y: 0,
            canvas_w: 2400.0,
            canvas_h: 1600.0,
        };
        still.run(&mut with, w, h, frame, Encoding::Screen, &on, None);
        still.run(&mut without, w, h, frame, Encoding::Screen, &off, None);
        let i = (40 * w + 52) * 3; // 8 px from the light's edge
        let d: Vec<f32> = (0..3).map(|k| with[i + k] - without[i + k]).collect();
        assert!(d[0] > 0.05 && d[0] > 4.0 * d[1].max(d[2]), "{d:?}");
    }

    #[test]
    fn purple_bands_split_the_same_as_the_whole() {
        let purple = Kit::for_profile(&Stock::Purple.profile());
        let s = Settings {
            fade: true,
            exposure: -2.0,
            hue: 0.3,
            denoise: Some(denoise::Options {
                amount: 100.0,
                strong: true,
                ..Default::default()
            }),
            film: Character {
                grain: 1.0,
                halation: 1.0,
                softness: 1.0,
            },
            ..Settings::default()
        };
        bands_match(&purple, &s);
    }

    #[test]
    fn purple_turns_a_soft_edge_between_two_colours_smoothly() {
        // a bokeh edge: green leaves blurring into orange flowers, and blue sky into red, in small
        // steps (blended in linear light, as a lens blurs); every step of the result must stay
        // close to its neighbours, with no jump or spike in colour along the way
        let purple = Kit::for_profile(&Stock::Purple.profile());
        let t = Transfer::new();
        for iso in [100, 200, 400] {
            let s = Settings {
                normalize: false,
                film: Character::NONE,
                exposure: iso_stops(iso),
                ..Settings::default()
            };
            for (a, b) in [
                ([0.25f32, 0.5, 0.15], [0.95, 0.55, 0.15]),
                ([0.35, 0.55, 0.9], [0.8, 0.15, 0.15]),
                ([0.9, 0.85, 0.2], [0.2, 0.3, 0.8]),
            ] {
                let n = 200;
                let mut row = Vec::with_capacity(n * 3);
                for i in 0..n {
                    let k = i as f32 / (n - 1) as f32;
                    for c in 0..3 {
                        row.push(
                            t.to_screen(t.to_linear(a[c]) * (1.0 - k) + t.to_linear(b[c]) * k),
                        );
                    }
                }
                purple.run(
                    &mut row,
                    n,
                    1,
                    Frame::whole(n, 1),
                    Encoding::Screen,
                    &s,
                    None,
                );
                let worst = row
                    .chunks_exact(3)
                    .zip(row.chunks_exact(3).skip(1))
                    .map(|(p, q)| (0..3).map(|c| (p[c] - q[c]).abs()).fold(0.0f32, f32::max))
                    .fold(0.0f32, f32::max);
                assert!(
                    worst < 0.03,
                    "ISO {iso}, {a:?} → {b:?}: a {worst} jump between neighbours"
                );
            }
        }
    }

    /// The host cuts the photo into bands padded by the overlap; every step, the neighbourhood
    /// ones (halation, softness, Denoise) included, must give the same pixels either way.
    fn bands_match(kit: &Kit, s: &Settings) {
        let (w, h, cut) = (48, 100, 50);
        let src: Vec<f32> = (0..w * h * 3)
            .map(|i| ((i * 7919) % 101) as f32 / 101.0)
            .collect();
        let mut whole = src.clone();
        let canvas = Frame {
            x: 0,
            y: 0,
            canvas_w: 4000.0,
            canvas_h: h as f32,
        };
        kit.run(
            &mut whole,
            w,
            h,
            canvas,
            Encoding::Screen,
            s,
            Some(Normalize {
                gain: 1.6,
                black: 0.004,
            }),
        );
        let top = cut - crate::OVERLAP as usize;
        let mut band = src[top * w * 3..].to_vec();
        kit.run(
            &mut band,
            w,
            h - top,
            Frame {
                y: top as i64,
                ..canvas
            },
            Encoding::Screen,
            s,
            Some(Normalize {
                gain: 1.6,
                black: 0.004,
            }),
        );
        let off = (cut - top) * w * 3;
        let worst = whole[cut * w * 3..]
            .iter()
            .zip(&band[off..])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(worst < 1e-4, "{worst}");
    }

    #[test]
    fn linear_encoding_round_trips_when_nothing_is_on() {
        let kit = Kit::new();
        let s = Settings {
            normalize: false,
            look: false,
            film: Character::NONE,
            ..Settings::default()
        };
        let mut b = vec![0.01, 0.18, 0.7];
        let orig = b.clone();
        kit.run(&mut b, 1, 1, Frame::whole(1, 1), Encoding::Linear, &s, None);
        for (a, o) in b.iter().zip(&orig) {
            assert!((a - o).abs() < 2e-3, "{a} vs {o}");
        }
    }

    #[test]
    fn base_only_keeps_the_base_and_drops_the_modifiers() {
        let s = Settings {
            strength: 0.6,
            cool: 25.0,
            hc: true,
            banding: true,
            denoise: Some(Default::default()),
            ..Settings::default()
        };
        let b = s.base_only();
        assert!(b.normalize && b.look && b.strength == 0.6);
        assert!(b.cool == 0.0 && !b.hc && !b.banding && b.denoise.is_none());
    }
}
