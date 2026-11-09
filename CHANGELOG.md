# Changelog

## 0.1.0 (2026-11-08)

First release: four filters sharing one engine.

**ChromaLome Purple** (`chromalome.wasm`)

- LomoChrome Purple: a C-41 film model whose layers see swapped light, fitted to a ColorChecker
  shot on the real film (Lomography's guide).
- Smooth colour throughout (no colour bands, a smooth knee in the scan), so soft edges between
  colours stay smooth.
- ISO 100 / 200 / 400 (the speed you rated it at; loads at 400, the well-exposed look, with 100 the faded two-stops-over look), Warmth, Denoise, Lifted Fade.
- ChromaLome Purple icon and header logo.

**ChromaStill 800T** (`chromastill.wasm`)

- CineStill 800T (Kodak VISION3 500T, rem-jet removed) as a physical model of the negative and
  its print, from Kodak's 5219 data sheet.
- The red halation of a film with no rem-jet: fed by red light, landing in the red layer only, as
  far-reaching as on real 800T negatives.
- Hue (800T's green-blue lean, gentle by default), Exposure, Denoise, and a real matte Lifted Fade
  (no Banding Fix: the grain already breaks up banding).
- ChromaStill 800T icon and header logo.

**ChromaGold 200** (`chromagold.wasm`)

- Kodak Gold 200 as a physical model of the C-41 negative and its print, from Kodak's data sheet:
  the layers' spectral sensitivities, the negative's characteristic curves, printer balance and
  paper; plus the touches real Gold 200 is known for.
- Gold's own grain (soft dye clouds, coarser) and halation (lighter).
- Exposure (−2 … +3): moves the scene along the film's curves and prints it back to normal, so
  under- and over-exposure look as they do on real Gold. Sun Cast: Gold's yellow cast in harsh sun.
  Normalize (a fixed lift), Denoise, Lifted Fade, Banding Fix (no Coolness: it's a daylight film).
- ChromaGold 200 icon and header logo.

**ChromaKode 64** (`chromakode.wasm`)

- The Kodachrome 64 look (dye matrix, colour bands, tone curves, saturation), with Strength,
  tuned to match reference renders.
- Normalize is off by default; when on, a fixed lift of about half a stop before the film.
  Nothing to measure.
- The film's character, on by default: Grain, Halation and Softness sliders, and a highlight
  shoulder so brightening never clips.
- Extras, always run in a fixed order: Coolness (0–100, default 25), Denoise (Off / Light / Medium / Strong / Max),
  Color Shift, High contrast, Vintage, Lifted Fade, Warm Band (a warm grade plus fine noise against banding).
- Base only: compare against the bare film without losing your setup.
- Short labels that fit the panel's name column.
- ChromaKode Film icon and header logo for dark and light themes.
