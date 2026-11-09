<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="brand/logo-dark.svg">
    <img alt="ChromaKode Film" src="brand/logo-light.svg" width="620">
  </picture>
</p>

<p align="center">
  <b>Film stocks for your photos, each with its own kit of modifiers.</b><br>
  Pixel filters for <a href="https://github.com/emmertex/chromaforge">Chromaforge</a>, from the maker of ChromaLab.
</p>

---

ChromaKode Film is a set of filters that share one engine. Each has its own panel.

## Download

Each link downloads that film's plug-in, always the newest release:

| Film | Download | What it is |
| --- | --- | --- |
| **ChromaKode 64** | [⬇ chromakode.wasm](https://github.com/nwh212/chromakode-film/releases/latest/download/chromakode.wasm) | Kodachrome 64 slide film |
| **ChromaGold 200** | [⬇ chromagold.wasm](https://github.com/nwh212/chromakode-film/releases/latest/download/chromagold.wasm) | Kodak Gold 200 colour negative film, printed |
| **ChromaStill 800T** | [⬇ chromastill.wasm](https://github.com/nwh212/chromakode-film/releases/latest/download/chromastill.wasm) | CineStill 800T (Kodak VISION3 500T, rem-jet removed), printed |
| **ChromaLome Purple** | [⬇ chromalome.wasm](https://github.com/nwh212/chromakode-film/releases/latest/download/chromalome.wasm) | LomoChrome Purple: greens purple, blues teal, yellows pink |

Then in Chromaforge: **File ▸ Plug-ins ▸ Plug-in Manager… ▸ Install from File…**, or drop the
file on the window. Install as many films as you like; each shows up as its own filter.

# ChromaKode 64

Add **ChromaKode 64** to a photo in **Develop ▸ Plug-in Filters** and the photo has the
Kodachrome 64 look straight away: purer primaries, rich reds, olive greens, deep blue skies,
dense shadows and whites that stop short of paper white. Then tick whichever extras you want. They always run in the
kit's fixed order, whatever order you tick them in:

```
Normalize → Coolness → Denoise → Kodachrome 64 → Color Shift → High contrast → Vintage → Lifted Fade → Warm Band
└───── before the look ─────┘     (the film)    └──────────────── after the look ─────────────────┘
```

Leave **Stage** on *Display* (the default): the look then goes on top of your finished edit, like
a film stock. At *Source* it runs before your edit (before white balance on a raw), and your
develop settings change the look afterwards.

## The panel

| Control | What it does |
| --- | --- |
| **Strength** | 0–100 %, blends the Kodachrome 64 look with the photo as it was just before it (default 100). 0 turns the look off and leaves the extras. |
| **Normalize** | Off by default. The Lightroom kit's Normalize Exposure: a fixed lift of about half a stop before the film. Nothing to measure. |
| **Base only** | Turns every extra off without forgetting how you set them (the kit's `.RESET MODS`). Tick it to compare against the bare film. The Film sliders stay, as they're part of the film. |

**Film**: the film's own character, on from the start. The recipe sheet's numbers leave it out.

- **Grain** (default 50): soft, clumpy, mostly black-and-white grain with a little colour, fixed
  to the photo so it doesn't crawl between previews.
- **Halation** (default 50): the warm glow that bright areas throw into the darker areas around
  them, as light bounces back through slide film. The bright areas themselves stay clean.
- **Softness** (default 30): film is never pin-sharp. It takes the digital edge off.

Their sizes follow the photo (they're set for a 2400 px long side), so a preview and a big export
look the same. When Normalize brightens a photo, the brightest parts also roll off softly into
white instead of clipping, as film's shoulder does.

**Before the look**

- **Coolness** (0–100, default 25): a cooler white balance, overall brightness kept. 25 is the
  sheet's Cool 25, and higher goes further the same way.
- **Denoise**: *Off*, *Light*, *Medium*, *Strong* or *Max*. Cleans noise before the film's
  contrast and saturation exaggerate it (see below).

**After the look**

- **Color Shift**: Kodachrome's hue nudges: yellows and greens warmer, aquas bluer, blues
  toward cyan.
- **High contrast**: the kit's HC, an S-curve with richer colour.
- **Vintage**: the sheet's Found, a slide found in a box: 85 % saturation, lifted warm curves, darker corners.
- **Lifted Fade**: the kit's gentle matte: black lifted a touch, white eased down a touch.
- **Warm Band**: the Lightroom kit's Banding step: brighter, warm midtones, blue shadows pulled
  down, a little more saturation, plus ±1.2/255 of fixed fine noise so smooth skies don't show
  steps. It always runs last.

# ChromaGold 200

Kodak Gold 200 as a print from the negative. The film is a **physical model of the C-41
negative and its print**, built from Kodak's data sheet rather than fitted (`src/print.rs`):

1. **The layers' spectral sensitivities.** Each layer sees a broad band, overlapping its
   neighbours (the green layer also has a bump in the violet). Camera colours become the three
   layers' exposures through a mix worked out from the sheet's sensitivity curves against the sRGB
   primaries.
2. **The negative's characteristic curves**, read off the sheet: low contrast, a long straight
   line, and a toe that differs slightly per layer. The base and the orange mask are the curves'
   offsets.
3. **Printing.** The printer is balanced so a mid grey prints neutral, which drops out the orange
   mask. A lab prints every frame back to normal brightness, so an off exposure comes back to the
   right brightness but not to the right curve.
4. **The paper**: a steep S-curve between paper white and paper black, and dyes with unwanted
   absorptions, so strong colours print darker (subtractive colour). The scan sets paper white
   to white.

(Kodachrome, ChromaKode 64, was K-14 reversal film: no couplers or mask in the emulsion, dyes
added one layer at a time in development, viewed directly as a contrasty slide. None of that
applies to a C-41 negative.)

What comes out without being painted in: greys neutral from the midtones up, warm brown shadows
from the layers' different toes (a touch magenta at the very bottom), dense blacks, airy
midtones, soft highlights that hold, and strong reds printing deep. On top, only what real Gold
200 is known for beyond its sheet: yellows and oranges a little brighter, greens toward olive,
reds a touch toward carmine, with muted colours taking these too (as in Lightroom's colour
mixer). Its grain is soft, irregular
dye clouds, coarser than ChromaKode 64's and a little stronger in the shadows, and its halation
lighter (negative film has an anti-halation layer). There's no Coolness: Gold is a daylight film.

```
Normalize → Denoise → Gold 200 negative (Exposure) → print → Sun Cast → Lifted Fade → Banding Fix
```

| Control | What it does |
| --- | --- |
| **Strength**, **Base only** | As in ChromaKode 64. |
| **Normalize** | Off by default. A fixed lift of a third of a stop before the film, as in ChromaKode 64. Nothing to measure. |
| **Grain** (55), **Halation** (30), **Softness** (30) | The Film section, in Gold's own style. |
| **Denoise** | As in ChromaKode 64. |
| **Exposure** (−2 … +3) | How the negative was exposed, in stops. It moves the scene along the film's own curves and the lab prints it back to normal brightness, so under-exposure puts the shadows on the toe (flat, yellow-green, grainier) and over-exposure puts the highlights on the shoulder (paler, purplish-red), as real Gold does. 0 is a normal exposure. |
| **Sun Cast** (0–100) | The soft yellow cast Gold takes in harsh midday sun, strongest in the bright areas. |
| **Lifted Fade** | The gentle matte, as in ChromaKode 64. |
| **Banding Fix** | ±1.2/255 of fixed fine noise against banding (no warm grade; that's ChromaKode 64's Warm Band). |

**Where its numbers come from.** Kodak's data sheet,
[E-7022](https://business.kodakmoments.com/sites/default/files/files/resources/E7022_Gold_200.pdf):
the three characteristic curves run parallel (neutral greys), soft contrast with latitude from
two stops under to three over, broadly overlapping layer sensitivities, Print Grain Index 44. The
film is modelled from the sheet; the small touches on top (yellows and oranges a little brighter,
greens toward olive, reds a touch toward carmine) are what Gold 200 is known for.

# ChromaStill 800T

CineStill 800T is Kodak VISION3 500T (5219/7219) cinema film with its rem-jet backing removed so it
can run in C-41. The film is the same physical negative-and-print model as ChromaGold's, with
5219's own data (`src/print.rs`, from Kodak's H-1-5219): its layers' spectral sensitivities, its
very soft characteristic curves (γ ≈ 0.45, made to be printed or scanned with a lot of contrast),
rated at 800 rather than 500.

**The red halation.** The rem-jet is what normally stops light that has gone through the emulsion
from bouncing off the back of the base. Without it, that light comes back into the bottom layer,
the red-sensitive one, so bright lights and edges get a saturated red ring. ChromaStill does the
same: the glow is fed by how much red is in the light (a red neon glows hardest, a teal one
hardly at all), lands in the red layer only, and reaches about as far as on real 800T negatives
(peaking 0.2–0.4 mm out, gone by about 0.5 mm). It needs more
context around each band than the other films, so this filter asks Chromaforge for 48 pixels.

| Control | What it does |
| --- | --- |
| **Strength**, **Base only** | As in the other films. |
| **Normalize** | Off by default. A fixed lift of a third of a stop before the film. |
| **Grain** (60), **Halation** (70), **Softness** (25) | The Film section, in 800T's style: coarser grain, strong red halation. |
| **Hue** (0–100, default 30) | How far the picture leans green-blue, as 800T usually does (a tungsten film under mixed and daylight light, scanned). 0 is neutral, as if shot under the 3200 K light it's balanced for; higher leans further. |
| **Denoise** | As in the other films. |
| **Exposure** (−2 … +3) | How the negative was exposed, as in ChromaGold: along the film's own curves, printed back to normal. |
| **Lifted Fade** | A real matte: black lifted to 22/255, white eased to 249/255 (the sheet's curve; the gentle Lightroom-kit one disappears in 800T's dense blacks). There's no Banding Fix: 800T's grain already breaks up banding. |

# ChromaLome Purple

LomoChrome Purple is a C-41 colour negative whose layers see swapped light: greens print
purple, blues green-teal, yellows pink, while reds stay red. ChromaLome is the same
negative-and-print model as the other negative films, with that swap built into what the film's
layers see (`src/print.rs`, `PURPLE`).

**Where its numbers come from.** Lomography's own film guide has a ColorChecker shot on
LomoChrome Purple at ISO 100 (and the same chart on their ordinary Color Negative 100). The swap
and the scan are fitted to all 24 patches of that chart (8.2/255 rms): the red layer sees red
plus about half the green (so foliage goes purple), the green layer mostly blue (skies go teal),
the blue layer green and blue, and the scan crushes red in the shadows (Purple's blue-teal
darks). Lomography publishes no characteristic curves, so the negative and paper are Gold 200's.

**Smooth colour everywhere.** There are no colour bands (which move hues band by band and can
make a soft edge between two colours do something odd): the swap is a mix in linear light, and
the scan's black point bends in through a smooth knee. A test runs soft edges between very
different colours (green into orange, blue into red, yellow into blue) through it and checks
there's no jump anywhere along them.

| Control | What it does |
| --- | --- |
| **Strength**, **Normalize**, **Base only** | As in the other films. Strength is handy here: it blends the swap with the photo. |
| **Grain** (45), **Halation** (25), **Softness** (25) | The Film section: Purple's fine, cine-style grain. |
| **Denoise** | As in the other films. |
| **ISO** (100, 200, 400; loads at 400) | The speed you rated the film at. 400 is where it looks right: clean, slightly lavender whites, deep violet foliage, teal, slate-teal darks. 100 is two stops over: Lomography's chart look, faded and pastel with cream-green highlights. 200 sits between. |
| **Warmth** (0–100, default 30) | Leans the scan toward cream-yellow highlights, as many lab scans of Purple are. |
| **Lifted Fade** | A real matte, as in ChromaStill. |

## Denoise

The look adds contrast and saturation, and High contrast adds more. Both make noise more
visible: grain in flat areas and blotches of colour in the shadows. Denoise cleans that up
*before* the look, so the film works on a clean picture. It's for high-ISO photos, phone
pictures and grainy scans. Clean photos don't need it.

It works on two things at once, keeping edges sharp: **grain** (speckles of brightness) and
**colour speckles** (red/green/blue blotches), and it puts back a little of the finest texture
so faces and fabric don't turn waxy. The levels are the recipe sheet's table:

| Level | Sheet setting |
| --- | --- |
| Light | Amount 25 |
| Medium | Amount 50 (the sheet's default) |
| Strong | Amount 100 |
| Max | Amount 100 + Extra strong (keeps less texture) |

## Measuring

Neither filter measures any more: both Normalizes are fixed lifts, like the Lightroom kit's, so
neither has a Measure button. (The measuring code stays for the tools.) It used the measurement Chromaforge
takes when you press **Measure**: the log-average brightness of the photo inside its crop (at
most 250,000 points). It also records the photo's black level, which the filter doesn't use:
Measure reads the photo before your develop edits and the filter runs after them, so a black point
taken from it could crush an edited photo's shadows. It's stored with the photo, so zooming, cropping later and exporting all
use the same correction. Measure again after a big crop change.

With several photos selected you can **Measure… ▸ N Selected as a Roll**: the whole roll gets
one measurement and so one correction, as a lab would print a roll of slides. *Each on Its Own*
measures every photo separately.

The measurement is of the photo as it comes in, before your develop settings, and the filter
runs after them (the Display stage). So your own Exposure slider still works on top: Normalize
sets where the photo starts, you move it from there.

## Install

1. Download a film from the table under **Download** at the top (or take it from `dist/`, or
   build it, below).
2. In Chromaforge: **File ▸ Plug-ins ▸ Plug-in Manager… ▸ Install from File…**, or drop the file
   on the window. Pixel filters ask for no permissions.

Its icon and header picture show in hosts with the plug-in pictures feature. Others show the
name and an icon drawn from the id. The folding sections need a host that supports them; without
it the settings show as one list.

## Notes on the recipe sheet

- **The look is fitted to the Lightroom kit, step by step.** Every number in ChromaKode 64 (the
  Kodachrome 64 step's dye matrix, colour bands, curves and saturation; Normalize's lift; Cool 25;
  Color Shift; HC; Lifted Fade; Warm Band) starts from the sheet's and is fitted to the kit in
  Lightroom: Lightroom's own rendering of a photo with no look, and after each step in turn
  (Kodachrome 64, + Normalize, + Color Shift, + Cool 25, + HC, + Lifted Fade, + Banding), so every
  checkbox does what that preset does. HC, Lifted Fade and Warm Band were checked on a second
  photo. As in Lightroom's colour mixer, muted colours take the colour changes too; greys don't.
  Hue shifts are held close to the sheet's and neighbouring bands' close to each other, so
  colours change smoothly. ChromaKode then matches Lightroom as closely as Chromaforge's rendering
  of the raw matches Lightroom's; making those match is a camera-profile job for Chromaforge.

- **No borders**, in any of the films: a filter changes the picture's colours and never draws a frame around it.
- **[k] Auto Geometry isn't in the filter.** A pixel filter changes colours and can't move pixels
  or crop, so perspective correction belongs to Chromaforge's own **Upright** (Develop ▸
  Transform), which does exactly what the sheet describes (level horizon, upright verticals,
  crop the empty corners).
- **Denoise's filter sizes** follow the photo (the table is for a 2048 px long side, scaled ×0.35
  to ×2.5). A filter only sees a band of the photo plus 24 pixels around it, shared with Softness,
  so a radius is held to 9 px, and Halation's glow to 8 px per pass. That only bites at the
  largest sizes: Strong and Max on big exports get a slightly smaller colour filter than the table
  asks for.
- **Colour bands** use eight bands (red 0°, orange 30°, yellow 60°, green 120°, aqua 180°, blue 240°,
  purple 270°, magenta 300°), each colour blended smoothly between the two nearest. Greys never
  move, and the stronger a colour, the more of its band's change it gets.
- **Curves** are smooth monotone curves through the sheet's points, so they never overshoot
  between points.
- Steps that work in linear light (Normalize, Coolness, Halation) do. The rest work on 0–1 screen values,
  as the sheet gives them. Both Chromaforge stages work: at the Source stage or on an HDR render
  the samples arrive linear and go back linear.

## Build

```sh
rustup target add wasm32-unknown-unknown
sh tools/build.sh      # both filters into dist/
```

One filter at a time: `cargo build --release --target wasm32-unknown-unknown` is ChromaKode 64,
`--features gold` makes it ChromaGold 200, `--features still` ChromaStill 800T and
`--features purple` ChromaLome Purple (all come out
as `target/wasm32-unknown-unknown/release/chromakode.wasm`). Tests and the sample renderer take
the same features.

`cargo test` runs the kit's tests natively. `cargo run --release --example render -- photo.jpg
'{"hc": true, "coolness": 25, "denoise": "Medium"}'` renders a photo as the host would and writes
`photo.k64.png`. The parameter names are the manifest's (`src/manifest.json`).

## Working on it

- `src/kit.rs`: the steps in order, ChromaKode 64's numbers, and each film's `Profile`
  (ChromaGold's and ChromaStill's numbers are there too).
- `src/print.rs`: the negative-and-print model, and each negative film's data-sheet numbers
  (Gold 200, VISION3 500T). A new colour negative film is a new `PrintParams` plus a profile.
- **Adding a film:** a `PrintParams` from its data sheet (spectral sensitivities, characteristic
  curves), a `Stock` and `Profile` in `kit.rs`, a manifest, a build feature, a logo in
  `tools/brand.py` and a line in `tools/build.sh`.
- `src/film.rs`: the film's character: highlight shoulder, halation, softness, grain.
- `src/denoise.rs`: the Denoise modifier (guided filters).
- `src/color.rs`: curves, colour bands, sRGB, position-fixed noise.
- `src/lib.rs`: the ABI exports (`pc_filter`, `pc_analyze`, `pc_merge`); the `gold` feature picks
  the film, its manifest (`src/manifest.json` or `src/manifest_gold.json`) and its pictures.
- `brand/`: the logo and icon, written by `python tools/brand.py` (plain shapes only, so the
  host's locked-down SVG renderer draws them exactly).
- The ids, `io.github.nwh212.chromakode`, `io.github.nwh212.chromagold`,
  `io.github.nwh212.chromastill` and `io.github.nwh212.chromalome`, never change. The
  version comes from `Cargo.toml` only.

## References

The films are modelled from the manufacturers' own technical publications:

- **ChromaKode 64**: Kodak, *Kodachrome 25, 64 and 200 Professional Film*, technical data E-55
  (Kodachrome 64, process K-14).
- **ChromaGold 200**: Kodak, *Kodak Gold 200 Film*, technical data
  [E-7022](https://business.kodakmoments.com/sites/default/files/files/resources/E7022_Gold_200.pdf)
  (process C-41).
- **ChromaStill 800T**: Kodak, *Kodak VISION3 500T Color Negative Film 5219 / 7219*, technical
  data H-1-5219 (the stock CineStill 800T is made from, process ECN-2; CineStill runs it in C-41
  with the rem-jet removed).
- **ChromaLome Purple**: Lomography, *LomoChrome Purple* film guide (its ColorChecker shot on the
  film, process C-41).

## Licence

MIT.
