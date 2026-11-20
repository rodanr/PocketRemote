# Design brief: PocketRemote ("Aisle Light")

Status: direction chosen by the owner and reviewed by an independent Opus design review (2026-11-20). Not yet built. Product truth is in `/PRODUCT.md`; the chosen sketch is `docs/design/aisle-light-sketch.html` (a static first-screen sketch, not a spec of final sizes).

Quality bar from the owner: nice, modern, clean, sleek, on both the phone app and the Mac app. In practice: few things on screen, one 8 dp rhythm, restrained purposeful motion, shared palette and radii on both devices, one flourish only (the trackpad aisle lights).

## The idea
A remote for a dim room and one thumb. Near-black ground, one warm amber used sparingly like cinema aisle lights. Signature move: a runway of small amber lights edging the trackpad.

## Tokens (starting point)
- Ground `#07070a`; raised surface about `#1c1914` (raised from `#14120f` so shapes are visible); trackpad `#0e0d0b`.
- Outline at least 3:1 against the ground (about `#6b5a3e`).
- Text `#e8e2d6`; muted text `#a08a6a` (6.1:1 on ground, estimated); amber `#ffb454` (about 11:1 on ground; dark text on amber about 10.5:1).
- Status: connected = green dot; connecting = pulsing hollow ring; paused = hollow red ring plus pause glyph. Always with words, never colour alone.
- Roboto with tabular figures on both apps (SF cannot be bundled). Material 3 type roles in sp. Phone is dark only; no Dynamic Color (deliberate).

## Phone screen (top to bottom)
1. Header: status dot + words, Mac name, profile picker, settings; all 48 dp targets.
2. Keyboard bar: pill field + Send (tonal until the field has text); live-typing toggle shown as an amber "Live" outline.
3. Media grid, 48 dp+ targets, 8 dp gaps: `[Play (double width, amber lead)][-10][+10] / [Mute][Vol -][Vol +] / [CC][Full]`. Vol -/+ repeat while held with a tick per step. A three-finger tap on the trackpad toggles Play/Pause with a firm haptic.
4. D-pad: 160 dp circle, 64 dp OK, up/down/left/right, Back (Esc). Glyphs in warm text, not amber.
5. Browser row, 48 dp: Back, Forward, Reload, Prev tab, Next tab. Close tab is separate, outlined, and needs a 300 ms hold.
6. Trackpad fills the rest, aisle lights inside a 24 dp side margin so Android's Back gesture cannot fire mid-drag. Landscape: full screen, system bars hidden; a Back gesture first reveals the bars.
- Keyboard open: collapse media deck and D-pad instead of squashing the trackpad. Edge-to-edge with real insets. System Back closes sheets/scanner, otherwise leaves the app; it never sends Esc.
- Connection loss is a persistent banner, not a toast. TalkBack: word labels ("Previous tab"), click and right-click actions on the trackpad. Test at 1.3x and 2x font scale.

## Motion and haptics (never idle animation)
- Aisle dots near the finger's height brighten, the rest about 35%; drag lock (350 ms) turns the runway solid with the firmest haptic. Reduced motion: instant brightness changes.
- Connect: dots fill bottom to top once (about 500 ms). Paused: lights drop to about 10% and the word "Paused" shows. Lost: one heavy haptic.
- Haptic grammar: tick for keys, click for taps, double tick for double click, tick per volume step.

## Pairing and first run
- Scanner frame uses the same amber dot frame as the Mac QR. Code shown as `123 456`, identical on both screens.
- Phone first run: one dim runway leading to "Scan the code on your Mac".
- Approve dialog on the Mac shows "your phone" plus its IP and request time (the protocol does not carry a phone name yet).

## Mac Control Panel and tray
- Same world, native feel, follows the Mac's light/dark setting. First run hero is the Accessibility permission, then the QR, then the full 5.4 list (server switch, address/port, interface picker, network trust, launch at login, Accessibility status; the pause hotkey was dropped by the owner).
- QR: dark modules on a near-white tile with a proper quiet zone; countdown ring and amber dots go outside the tile.
- Tray states at 16-18 pt: waiting = outline, paused = pause glyph, controlled = filled and amber (owner-approved; like macOS recording indicators). Disconnect and Revoke set apart; Revoke confirms.
- Opening the panel during a session must not steal focus; show a "Phone connected" notification. Only the Approve step may take focus.

## Cut
Status-dot glow, idle animation of the lights, amber D-pad glyphs, always-amber Send, Dynamic Color, ring-around-glyph tray state.

## Owner decisions recorded
- Media deck stays in the spec's order (top), with the three-finger tap shortcut.
- Controlled tray icon is amber.

## Addendum (2026-11-20): owner feedback from the first real-phone test
Three swipeable pages (dots above the trackpad show which one):
- **Main:** the media deck and the D-pad, icons only. No words on the main keys (words made it noisy).
- **Browser:** six equal, big, text-only keys (Back, Forward, Reload, Close tab, Previous tab, Next tab), one text size (18 sp) so none looks smaller. Close tab is outlined in red but is a plain tap like the others (a hold gesture was confusing and read as "not working").
- **Browser & slides (page 2, swipe):** sections BROWSER (Back, Forward, Reload, Prev tab, Next tab, Close tab) and SLIDES (Previous / Next slide); words only, one text size, centred in the same height as page 1 so the trackpad never changes size. Two small dots (own 14 dp row, no labels) show there are two pages. Previous/Next track live on the Remote page beside volume. Left/Right click keys were removed as redundant.
- **Typing bar:** one row of the field plus three round 48 dp buttons that never change: Backspace, Enter (becomes the amber Send once text is typed), Live (amber when on). While typing the field is a fixed four-line height (scrolls inside, so it never resizes), a filled keyboard-arrow circle inside it hides the keyboard (the app's own way out, no Back key needed), Backspace and Enter always act on the computer in every state; the third slot is Live, or the amber Send once text is typed, the controls collapse and the whole screen lifts above the keyboard so the trackpad stays usable. Wording says "computer", never "Mac".
- Settings lists the trackpad gestures.
- Compact layout keeps the trackpad large: 48 dp keys, 52 dp header, page dots sit in the gap above the trackpad and cost no height.
- Rule from the owner for adding anything: only simple, reliable actions. Deliberately NOT added: three-finger swipe shortcuts (OnePlus system gestures, accidental triggers), Mission Control (needs a new protocol command and depends on a macOS shortcut that may be off; revisit after a 30 s on-device check), a mouse Hold toggle (can desync), slide First/Last (inconsistent across apps), a Stop key (not available in enigo).

## Addendum 2026-11-20 (later): settings page, connection dialog, dots
- **Settings** is a dedicated full-screen page (back arrow; Android Back returns to the remote), not a bottom sheet.
- **No connection:** a dialog card floats over the controls, below the header, so nothing is pushed down and Settings stays reachable. Paused shows "Paused on your computer", others show the reason with Reconnect (or Scan again when pairing is needed).
- **Page dots** are 4 dp and sit inside the existing gap above the trackpad, so they cost no height.

## Addendum 2026-11-20 (visibility pass, reviewed with Opus)
- **Why:** keys (#1c1914) on the ground (#07070a) were about 1.15:1, so they vanished. Dark fills alone cannot reach 3:1, so the edge carries it.
- **Phone palette:** ground #141210, key #2e2a24 with a 1 dp #6b6152 edge (about 3:1), panel/trackpad #1f1c18, text #ede7dc, muted #b0a08a, danger #ff7a6a. One key radius (16); only the D-pad and OK stay round. Sentence-case labels. Settings: 48 dp keys, fixed value slot, full-width "Forget this computer" danger key.
- **Mac panel:** follows the system light/dark setting (iced `system::theme`). Dark: ground #1c1b19, card #262421, button #46423c with #756d61 edge. Light: ground #f4f1ec, white cards, white buttons with #8f8577 edge, amber text #9a5a00. Buttons hug their label, 30 px high, radius 8; cards radius 10; actions right-aligned, lead last; Remove is a quiet red text button. Window 400x560. The QR plate is always white.

## Typeface (2026-11-20)
Manrope (SIL Open Font License, free), regular 400 / medium 500 / semibold 600, on the phone (`expo-font`, via `app/src/ui/Text.tsx` which maps weights to families because Android ignores `fontWeight` for custom fonts) and on the Mac panel (bundled in `desktop/assets/fonts/` with its license). The Mac app and tray use the logo kit icons (`brand/kit/dist/macos/AppIcon.icns`, `brand/kit/dist/tray/`).
