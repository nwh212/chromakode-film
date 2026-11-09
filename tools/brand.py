"""Writes ChromaKode Film's logo and icon (brand/*.svg).

Sleek and quiet:

- The wordmark is a light, wide-set geometric capital: thin even strokes, soft rounded corners
  on the round letters, generous spacing, one tone.
- The "O" of KODE is the only colour: a slim slide window in three crisp bands, Kodachrome's
  dye layers in the order they lie in the film (yellow, magenta, cyan, top to bottom).
- "FILM" is a small, muted tag at the end, under a row of tiny sprocket marks.
- The icon is that slide window on a plain tile, with a slim K inside.

Everything is plain shapes (no text, fonts or images), so the host's locked-down SVG renderer
draws it exactly. Run `python tools/brand.py` from the project folder.
"""

from pathlib import Path

OUT = Path(__file__).resolve().parent.parent / "brand"

THEMES = {
    "dark": {"ink": "#f1ece2", "muted": "#8f897d", "tile": "#16140f", "tile_edge": "#2c2820"},
    "light": {"ink": "#1b1915", "muted": "#7a7468", "tile": "#f4efe4", "tile_edge": "#dcd2bd"},
}
# Kodachrome's three dye layers, in emulsion order from the top: yellow, magenta, cyan (crisp bands)
DYES = [("0", "#f5c400"), ("0.333", "#f5c400"), ("0.333", "#e0247f"), ("0.667", "#e0247f"), ("0.667", "#1d9fe0"), ("1", "#1d9fe0")]
# ChromaGold 200: the Gold box's yellow, orange and red
GOLD_DYES = [("0", "#f5b800"), ("0.333", "#f5b800"), ("0.333", "#ee7d1a"), ("0.667", "#ee7d1a"), ("0.667", "#d8361f"), ("1", "#d8361f")]


# ChromaStill 800T: halation red, tungsten amber, night teal
STILL_DYES = [("0", "#e23a2c"), ("0.333", "#e23a2c"), ("0.333", "#f2a03d"), ("0.667", "#f2a03d"), ("0.667", "#1f8f9a"), ("1", "#1f8f9a")]


# ChromaLome Purple: what the film turns colours into: purple, teal, pink
PURPLE_DYES = [("0", "#8a4fd0"), ("0.333", "#8a4fd0"), ("0.333", "#1fa58f"), ("0.667", "#1fa58f"), ("0.667", "#e0478f"), ("1", "#e0478f")]


def dye_id(dyes):
    return {id(DYES): "dye", id(GOLD_DYES): "golddye", id(STILL_DYES): "stilldye"}.get(id(dyes), "purpledye")


def gradient(x1, y1, x2, y2, dyes=DYES):
    stops = "".join(f'<stop offset="{o}" stop-color="{c}"/>' for o, c in dyes)
    return f'<linearGradient id="{dye_id(dyes)}" gradientUnits="userSpaceOnUse" x1="{x1}" y1="{y1}" x2="{x2}" y2="{y2}">{stops}</linearGradient>'


# --------------------------------------------------------------------------------------------
# Capitals on a 150-high grid, stroke 18 (lines sit 9 inside the box), corner radius 34.
# Each glyph: (advance width, [path data in glyph coordinates]).
# --------------------------------------------------------------------------------------------
STROKE = 18
GLYPHS = {
    "C": (100, ["M 97 9 H 43 A 34 34 0 0 0 9 43 V 107 A 34 34 0 0 0 43 141 H 97"]),
    "H": (100, ["M 9 0 V 150", "M 91 0 V 150", "M 9 75 H 91"]),
    "R": (100, ["M 9 150 V 9 H 58 A 33 33 0 0 1 58 75 H 9", "M 50 75 L 94 150"]),
    "O": (100, ["M 43 9 H 57 A 34 34 0 0 1 91 43 V 107 A 34 34 0 0 1 57 141 H 43 A 34 34 0 0 1 9 107 V 43 A 34 34 0 0 1 43 9 Z"]),
    "M": (120, ["M 9 150 V 0", "M 111 150 V 0", "M 13 5 L 60 80 L 107 5"]),
    "A": (100, ["M 9 150 V 43 A 34 34 0 0 1 43 9 H 57 A 34 34 0 0 1 91 43 V 150", "M 9 90 H 91"]),
    "K": (100, ["M 9 0 V 150", "M 95 3 L 20 75 L 95 147"]),
    "D": (100, ["M 9 9 H 57 A 34 34 0 0 1 91 43 V 107 A 34 34 0 0 1 57 141 H 9 Z"]),
    "E": (100, ["M 96 9 H 9 V 141 H 96", "M 9 75 H 82"]),
    "G": (100, ["M 97 9 H 43 A 34 34 0 0 0 9 43 V 107 A 34 34 0 0 0 43 141 H 91 V 80 H 56"]),
    "L": (96, ["M 9 0 V 141 H 96"]),
    "S": (100, ["M 97 9 H 43 A 33 33 0 0 0 43 75 H 57 A 33 33 0 0 1 57 141 H 3"]),
    "T": (100, ["M 0 9 H 100", "M 50 9 V 150"]),
    "I": (18, ["M 9 0 V 150"]),
}
TRACK = 44

# FILM: 50 high, stroke 8
SMALL = {
    "F": (32, ["M 32 4 H 4 V 50", "M 4 26 H 27"]),
    "I": (8, ["M 4 0 V 50"]),
    "L": (32, ["M 4 0 V 46 H 32"]),
    "M": (44, ["M 4 50 V 0", "M 40 50 V 0", "M 5 3 L 22 30 L 39 3"]),
    "2": (32, ["M 4 15 A 12 11 0 0 1 28 15 L 5 46 H 32"]),
    "0": (32, ["M 16 4 A 12 12 0 0 1 28 16 V 34 A 12 12 0 0 1 4 34 V 16 A 12 12 0 0 1 16 4 Z"]),
    "P": (32, ["M 4 50 V 4 H 19 A 11 11 0 0 1 19 26 H 4"]),
    "U": (32, ["M 4 0 V 36 A 12 12 0 0 0 28 36 V 0"]),
    "R": (32, ["M 4 50 V 4 H 19 A 11 11 0 0 1 19 26 H 4", "M 16 26 L 30 50"]),
    "E": (30, ["M 30 4 H 4 V 46 H 30", "M 4 25 H 24"]),
    "8": (32, ["M 16 4 A 10.5 10.5 0 1 1 16 25 A 10.5 10.5 0 1 1 16 4 Z", "M 16 25 A 11.5 11.5 0 1 1 16 48 A 11.5 11.5 0 1 1 16 25 Z"]),
    "T": (32, ["M 0 4 H 32", "M 16 4 V 50"]),
}


def setline(text, table, x0, y0, track, special=None):
    out, x = [], x0
    for i, ch in enumerate(text):
        w, ds = table[ch]
        attrs = special(i) if special else ""
        out.append(f'<g transform="translate({x} {y0})"{attrs}>' + "".join(f'<path d="{d}"/>' for d in ds) + "</g>")
        x += w + track
    return "".join(out), x - track


# ChromaStill's glowing letter: red, with a halation of wider, fainter red strokes around it
HALO_RED = "#e8261b"
HALO = [(30, 0.10), (20, 0.16), (10, 0.28)]  # extra stroke width, opacity


def glow(paths, x, y, width, core):
    """`paths` (glyph coordinates) at (x, y) as a red light with its halation: plain strokes only."""
    layers = "".join(
        f'<g transform="translate({x} {y})" fill="none" stroke="{HALO_RED}" stroke-opacity="{o}" stroke-width="{width + w}" '
        f'stroke-linecap="round" stroke-linejoin="round">' + "".join(f'<path d="{d}"/>' for d in paths) + "</g>"
        for w, o in HALO)
    return layers + (f'<g transform="translate({x} {y})" fill="none" stroke="{core}" stroke-width="{width}" stroke-linecap="butt" '
                     f'stroke-linejoin="miter" stroke-miterlimit="2">' + "".join(f'<path d="{d}"/>' for d in paths) + "</g>")


def logo(theme, word_text="CHROMAKODE", tag="FILM", dyes=DYES, title="ChromaKode Film", o_index=7, glow_index=None):
    t = THEMES[theme]
    y0, x0 = 30, 16
    # the O of KODE / GOLD (the 8th letter) is drawn in the dye gradient; ChromaStill's S glows red instead
    def special(i):
        if glow_index is not None:
            return ' stroke="none"' if i == glow_index else ""
        return f' stroke="url(#{dye_id(dyes)})"' if i == o_index else ""
    word, end = setline(word_text, GLYPHS, x0, y0, TRACK, special=special)
    glowing = ""
    if glow_index is not None:
        gx = x0 + sum(GLYPHS[ch][0] + TRACK for ch in word_text[:glow_index])
        glowing = "\n" + glow(GLYPHS[word_text[glow_index]][1], gx, y0, STROKE, HALO_RED)
    fx = end + 64
    film, fend = setline(tag, SMALL, fx, y0 + 100, 20)
    # a row of sprocket marks over FILM
    n = 6
    span = fend - fx
    step = (span - 10) / (n - 1)
    holes = "".join(f'<rect x="{fx + i * step:.1f}" y="{y0 + 62}" width="10" height="14" rx="3"/>' for i in range(n))
    W, H = fend + 16, 210
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {W} {H}" width="{W * 2}" height="{H * 2}">
<title>{title}</title>
<defs>{gradient(0, 0, 0, 150, dyes)}</defs>
<g fill="none" stroke="{t['ink']}" stroke-width="{STROKE}" stroke-linecap="butt" stroke-linejoin="miter" stroke-miterlimit="2">{word}</g>
<g fill="none" stroke="{t['muted']}" stroke-width="8" stroke-linecap="butt" stroke-linejoin="miter" stroke-miterlimit="2">{film}</g>
<g fill="{t['muted']}">{holes}</g>{glowing}
</svg>
"""


K_LETTER = '<path d="M 106 92 V 164"/><path d="M 154 94 L 113 128 L 154 162"/>'
L_LETTER = '<path d="M 108 92 V 158 H 154"/>'
S_LETTER = '<path d="M 154 98 H 124 A 15 15 0 0 0 124 128 H 132 A 15 15 0 0 1 132 158 H 102"/>'
G_LETTER = '<path d="M 156 98 H 124 A 22 22 0 0 0 102 120 V 136 A 22 22 0 0 0 124 158 H 152 V 130 H 130"/>'


def icon(theme, dyes=DYES, letter=K_LETTER, title="ChromaKode Film", glowing=False):
    t = THEMES[theme]
    if glowing:
        # ChromaStill: a plain slide window, the S inside it a red light with its halation
        import re
        paths = re.findall(r'd="([^"]+)"', letter)
        return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="512" height="512">
<title>{title}</title>
<rect x="10" y="10" width="236" height="236" rx="44" fill="{t['tile']}" stroke="{t['tile_edge']}" stroke-width="3"/>
<rect x="62" y="62" width="132" height="132" rx="24" fill="none" stroke="{t['muted']}" stroke-width="15"/>
{glow(paths, 0, 0, 13, HALO_RED)}
</svg>
"""
    return f"""<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 256 256" width="512" height="512">
<title>{title}</title>
<defs>{gradient(0, 62, 0, 194, dyes)}</defs>
<rect x="10" y="10" width="236" height="236" rx="44" fill="{t['tile']}" stroke="{t['tile_edge']}" stroke-width="3"/>
<rect x="62" y="62" width="132" height="132" rx="24" fill="none" stroke="url(#{dye_id(dyes)})" stroke-width="15"/>
<g fill="none" stroke="{t['ink']}" stroke-width="13" stroke-linecap="butt" stroke-linejoin="miter" stroke-miterlimit="2">
{letter}
</g>
</svg>
"""


def main():
    OUT.mkdir(exist_ok=True)
    files = {
        "logo-dark.svg": logo("dark"),
        "logo-light.svg": logo("light"),
        "icon.svg": icon("dark"),
        "icon-dark.svg": icon("dark"),
        "icon-light.svg": icon("light"),
        "gold-logo-dark.svg": logo("dark", "CHROMAGOLD", "200", GOLD_DYES, "ChromaGold 200"),
        "gold-logo-light.svg": logo("light", "CHROMAGOLD", "200", GOLD_DYES, "ChromaGold 200"),
        "gold-icon.svg": icon("dark", GOLD_DYES, G_LETTER, "ChromaGold 200"),
        "gold-icon-dark.svg": icon("dark", GOLD_DYES, G_LETTER, "ChromaGold 200"),
        "gold-icon-light.svg": icon("light", GOLD_DYES, G_LETTER, "ChromaGold 200"),
        "still-logo-dark.svg": logo("dark", "CHROMASTILL", "800T", STILL_DYES, "ChromaStill 800T", glow_index=6),
        "still-logo-light.svg": logo("light", "CHROMASTILL", "800T", STILL_DYES, "ChromaStill 800T", glow_index=6),
        "still-icon.svg": icon("dark", STILL_DYES, S_LETTER, "ChromaStill 800T", glowing=True),
        "still-icon-dark.svg": icon("dark", STILL_DYES, S_LETTER, "ChromaStill 800T", glowing=True),
        "still-icon-light.svg": icon("light", STILL_DYES, S_LETTER, "ChromaStill 800T", glowing=True),
        "purple-logo-dark.svg": logo("dark", "CHROMALOME", "PURPLE", PURPLE_DYES, "ChromaLome Purple"),
        "purple-logo-light.svg": logo("light", "CHROMALOME", "PURPLE", PURPLE_DYES, "ChromaLome Purple"),
        "purple-icon.svg": icon("dark", PURPLE_DYES, L_LETTER, "ChromaLome Purple"),
        "purple-icon-dark.svg": icon("dark", PURPLE_DYES, L_LETTER, "ChromaLome Purple"),
        "purple-icon-light.svg": icon("light", PURPLE_DYES, L_LETTER, "ChromaLome Purple"),
    }
    for name, svg in files.items():
        (OUT / name).write_text(svg, encoding="utf-8", newline="\n")
        assert len(svg.encode()) < 24 * 1024, name
        print(f"brand/{name}: {len(svg.encode())} bytes")


if __name__ == "__main__":
    main()
