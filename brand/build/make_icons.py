#!/usr/bin/env python3
"""Generate PocketRemote app and tray icon SVGs from the master symbol geometry.
Run from brand/: python3 build/make_icons.py   (then render with the logo-design render_png.py)
Colours: amber #ffb454 on ground #07070a. The mark is one amber shape; the pocket shows the ground."""
import re, os
AMBER, GROUND = "#ffb454", "#07070a"
OUT = "kit/dist"
os.makedirs(OUT, exist_ok=True)

def geometry(path):
    s = open(path).read()
    g = re.search(r"<g transform=\"translate\(([-\d.]+) 0\)\">\s*(.*?)\s*</g>", s, re.S)
    return float(g.group(1)), g.group(2)

tx_main, body_main = geometry("kit/symbol.svg")          # P with the light
tx_small, body_small = geometry("kit/symbol-small.svg")  # heavier cut with the light
tx_glyph, body_glyph = geometry("kit/symbol-glyph.svg")  # heavier cut, empty pocket

def fill(body, colour):
    return body.replace('fill="#000"', f'fill="{colour}"')

def svg(vb, inner, extra=""):
    return f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{vb}"{extra}>\n{inner}\n</svg>\n'

def place(body, tx, colour, cx, cy, scale):
    """Mark centred on (cx, cy): the symbol's own centre is (128, 128) before translate(tx)."""
    return (f'<g transform="translate({cx} {cy}) scale({scale}) translate({-128 - tx} -128)">'
            f'<g transform="translate({tx} 0)">{fill(body, colour)}</g></g>')

def write(name, text):
    open(f"{OUT}/{name}", "w").write(text)

# ---- Android adaptive icon: 108 dp canvas, safe zone = 66 dp circle. Mark diagonal ~ 62 dp.
s = 62 / 216.0
write("ic_launcher_background.svg", svg("0 0 108 108", f'<rect width="108" height="108" fill="{GROUND}"/>'))
write("ic_launcher_foreground.svg", svg("0 0 108 108", place(body_main, tx_main, AMBER, 54, 54, s)))
write("ic_launcher_monochrome.svg", svg("0 0 108 108", place(body_main, tx_main, "#000", 54, 54, s)))
# Review composite with the round mask Android may apply
write("android-adaptive-preview.svg", svg("0 0 108 108",
  f'<clipPath id="m"><circle cx="54" cy="54" r="54"/></clipPath><g clip-path="url(#m)">'
  f'<rect width="108" height="108" fill="{GROUND}"/>{place(body_main, tx_main, AMBER, 54, 54, s)}</g>'))
# Play Store: 512 square, full bleed, mark ~ 52% of the height
write("play-store-512.svg", svg("0 0 512 512",
  f'<rect width="512" height="512" fill="{GROUND}"/>{place(body_main, tx_main, AMBER, 256, 256, 0.56*512/176/ (512/512))}'))

# ---- macOS app icon: 1024 canvas, 824 tile with 185 corner radius (Big Sur grid), mark ~ 54% of tile.
write("macos-app-icon-1024.svg", svg("0 0 1024 1024",
  f'<rect x="100" y="100" width="824" height="824" rx="185" fill="{GROUND}"/>'
  + place(body_main, tx_main, AMBER, 512, 512, 520/176.0)))

# ---- Menu bar glyphs: 18 pt image, mark 16 pt tall, centred. viewBox keeps the mark at 16/18.
side = 188 * 18 / 16.0
cx, cy = 130 - 0.0, 128.0   # glyph geometry centre after its own translate
vb = f"{cx-side/2:.2f} {cy-side/2:.2f} {side:.2f} {side:.2f}"
def tray(name, body, tx, colour, opacity=1.0):
    g = f'<g transform="translate({tx} 0)" opacity="{opacity}">{fill(body, colour)}</g>'
    write(name, svg(vb, g))
tray("tray-waiting-template.svg", body_glyph, tx_glyph, "#000")            # empty pocket
tray("tray-paused-template.svg", body_glyph, tx_glyph, "#000", 0.4)        # dimmed
tray("tray-controlled.svg", body_small, tx_small, AMBER)                    # lit, amber, not a template
print("wrote", sorted(os.listdir(OUT)))
