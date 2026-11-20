# PocketRemote brand guide

## The mark
A geometric **P** with a D-shaped pocket in its bowl and a single light resting low inside it. The idea: a remote that lives in your pocket, and one light that tells you it is on. The pocket shows the background, so the mark is one colour.

Direction chosen from three concepts (Runway, Pocket P, Drag Trail) and refined after an independent design critique: round counter replaced by a pocket, walls balanced to about 30 units, junction filleted, light moved off-centre. The **Runway** (converging dots) and **Drag Trail** (growing dots) concepts are kept in `brand/concepts/` as secondary graphics for onboarding and loading motion; they are not part of the mark.

## Colour
| Role | Value |
|---|---|
| Amber (the mark, the one light) | `#ffb454` |
| Ground (icon tile, app background) | `#07070a` |
| One-colour versions | black `#000000`, white `#ffffff` |

Amber on ground is about 11:1 contrast. Use amber for the mark and for one primary action per screen; nothing else.

## Files (`brand/kit/`)
- `symbol.svg` master. `symbol-small.svg` heavier cut for 16-32 px. `symbol-glyph.svg` the cut without the light (menu bar).
- `wordmark.svg`, `lockup-horizontal.svg`: outlined Roboto Medium (SIL OFL), no live text.
- `dist/variants/`: black, white and amber versions (SVG + PNG). `dist/web/`: favicon set.
- `dist/android/`: adaptive icon layers (foreground, background, monochrome, 432 px) and Play Store 512. Installed in `app/assets/`.
- `dist/macos/AppIcon.icns` (+ iconset): macOS app icon on the Big Sur grid.
- `dist/tray/`: menu bar images at 1x/2x/3x.
- Rebuild with `build/make_wordmark.py` and `build/make_icons.py`.

## Menu bar states
| State | Image | Notes |
|---|---|---|
| Waiting | `tray-waiting-template` | Empty pocket. Template image: macOS tints it for light and dark bars. |
| Paused | `tray-paused-template` | Same glyph at 40% opacity. |
| Controlled | `tray-controlled` | Amber P with the light lit. **Not** a template image, on purpose: it is a security signal, like macOS recording indicators. |

## Clear space and sizes
- Clear space on every side: the width of the stem (one-eighth of the mark's height).
- Minimum size: symbol 16 px (use `symbol-small`), lockup 120 px wide. Below 18 px the light is dropped (menu bar).
- On a light ground use black; on photos use white. Never place the amber mark on a mid-tone.

## Misuse
Do not recolour outside the table above, add glow, shadow or gradient, stretch or rotate, outline the P, move the light to the centre, set the name in another font, or put the mark on busy backgrounds.

## Licences and clearance
- Wordmark outlines come from Roboto (SIL Open Font License 1.1), which allows use in logos. The OFL text is available from the Google Fonts repository (`ofl/roboto`).
- No trademark search has been done. "Pocket" is a common word in software names (for example Pocket Casts, Mozilla Pocket); check trademark databases for PocketRemote in your jurisdiction before a public launch.
