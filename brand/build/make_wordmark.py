#!/usr/bin/env python3
"""Outline "PocketRemote" from Roboto (SIL OFL 1.1) into SVG paths. No live text in the output.

Usage: python3 -I make_wordmark.py <Roboto[wdth,wght].ttf> <out-dir>
Needs fontTools. Source of the font: github.com/google/fonts/tree/main/ofl/roboto
"""
import sys
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.pens.boundsPen import BoundsPen

font_path, out_dir = sys.argv[1], sys.argv[2]
WEIGHT = 500
TRACK = 6  # extra letter-spacing, in font units (2048 per em)
f = instantiateVariableFont(TTFont(font_path), {"wght": WEIGHT, "wdth": 100})
gs, cmap, hmtx = f.getGlyphSet(), f.getBestCmap(), f["hmtx"]
upem = f["head"].unitsPerEm
cap = f["OS/2"].sCapHeight

def outline(text, size, x0, baseline):
    """Return (path d, bbox) for text at `size` px per em, left edge x0, on `baseline`."""
    s = size / upem
    x, parts, bb = x0, [], [1e9, 1e9, -1e9, -1e9]
    for ch in text:
        name = cmap[ord(ch)]
        pen = SVGPathPen(gs)
        tp = TransformPen(pen, (s, 0, 0, -s, x, baseline))
        gs[name].draw(tp)
        parts.append(pen.getCommands())
        bp = BoundsPen(gs)
        gs[name].draw(TransformPen(bp, (s, 0, 0, -s, x, baseline)))
        if bp.bounds:
            b = bp.bounds
            bb = [min(bb[0], b[0]), min(bb[1], b[1]), max(bb[2], b[2]), max(bb[3], b[3])]
        x += (hmtx[name][0] + TRACK) * s
    return " ".join(parts), bb

def fmt(d):  # keep files small
    import re
    return re.sub(r"(\d+\.\d{2})\d+", r"\1", d)

# Wordmark alone: cap height 96 units.
size = 96 * upem / cap
d, bb = outline("PocketRemote", size, 0, 96)
w, h = bb[2] - bb[0], bb[3] - bb[1]
pad = 0
vb = f"{bb[0]-pad:.2f} {bb[1]-pad:.2f} {w+2*pad:.2f} {h+2*pad:.2f}"
open(f"{out_dir}/wordmark.svg", "w").write(
    f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb}" role="img" aria-label="PocketRemote">\n'
    f'  <title>PocketRemote</title>\n  <path fill="#000" d="{fmt(d)}"/>\n</svg>\n')

# Horizontal lockup on the symbol's 256 grid: P spans y 40..216 (centre 128); caps centred on it.
SYM = open(f"{out_dir}/symbol.svg").read()
import re
sym_g = re.search(r"(<g transform=.*?</g>)", SYM, re.S).group(1)
cap_px = 96
size2 = cap_px * upem / cap
baseline = 128 + cap_px / 2
x_text = 190.5 + 30  # symbol right edge + gap
d2, bb2 = outline("PocketRemote", size2, x_text, baseline)
x_min, y_min = 65.5, min(40, bb2[1])
x_max, y_max = bb2[2], max(216, bb2[3])
vb2 = f"{x_min:.2f} {y_min:.2f} {x_max-x_min:.2f} {y_max-y_min:.2f}"
open(f"{out_dir}/lockup-horizontal.svg", "w").write(
    f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb2}" role="img" aria-label="PocketRemote">\n'
    f'  <title>PocketRemote</title>\n  {sym_g}\n  <path fill="#000" d="{fmt(d2)}"/>\n</svg>\n')
print("wordmark", vb, "| lockup", vb2)
