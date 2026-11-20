# Product

<!-- impeccable:product-schema 1 -->

## Platform

android

(Primary designed surface is the Android phone app. A secondary desktop surface, the macOS Control Panel and menu bar tray, is a native Rust app built with `iced` and is designed with equal ambition; it is not a web or mobile-web product.)

## Users

Primary user: the owner, using a OnePlus Nord 5 (Android) as a remote for their MacBook (Apple M4 Pro). Real situations, all confirmed by the user: on the couch with the Mac driving a TV (YouTube, Netflix, VLC, Jellyfin), often in a dim room; giving presentations while walking around; controlling the Mac from another room (bed, kitchen) for media and quick typing. The user also intends to release it as open source, so strangers will install it, pair it and judge it cold. Those later users are the hardest test: first run and pairing must be self-explanatory.

Jobs: pause/seek/volume without getting up; move a cursor and click; type a short string; flip slides or tabs; pair once and have it reconnect on its own.

## Product Purpose

A free, open-source phone remote for a desktop computer on the same Wi-Fi. A session gives full control of the desktop, so trust and visibility are part of the product, not an add-on: the user must always know whether the phone is connected, whether the host is paused, and who is paired. Success is a remote the user reaches for without thinking and a pairing flow a stranger completes in under a minute without help.

## Positioning

Free, no account, no cloud, nothing leaves the local network. Security is the top priority and visible in the experience (host-side approval with a matching code, one paired phone, a clear "controlled" state on the Mac). Cross-platform host (macOS first; Windows and Linux planned).

## Operating Context

- Phone in one hand, usually one thumb, often in a dim room, sometimes without looking at the screen (eyes on the TV or the slides).
- The Mac is across the room: the Control Panel is visited rarely (pairing, revoke, pause), and the menu bar tray is the everyday surface. Its states (paused, waiting, controlled) must be readable at a glance.
- Pairing is a two-device moment: scan a QR shown on the Mac, compare a 6-digit code on both screens, press Approve on the Mac.
- Same-LAN only. Reconnect happens whenever the app comes to the foreground.

## Capabilities and Constraints

- Phone UI (from PLAN.md section 7): header with status dot, host name, profile picker and settings; keyboard bar (text field + Send, live-typing toggle); media deck (Play/Pause, back 10 s, forward 10 s, Fullscreen, Captions, Mute, Vol -, Vol +); D-pad with OK and Back; browser row (Back, Forward, Reload, Prev tab, Next tab, Close tab); trackpad filling the rest of the screen, full screen in landscape.
- Gestures: one-finger drag moves, tap clicks, double tap double-clicks, two-finger tap right-clicks, long press 350 ms then drag holds the button, two-finger drag scrolls (natural-scroll toggle).
- Targets at least 48 dp, accessibility labels on every control, haptics on clicks, screen kept awake while the remote is open. Settings: pointer speed, acceleration, natural scroll.
- Status dot: green session up, amber connecting, red failed or paused, with the reason in words.
- Host surfaces (PLAN.md 5.4): Control Panel (server switch, address and port, interface picker, network trust list, Pair phone with QR and approve dialog and code, paired phone with Revoke, connected status with Disconnect, launch at login, pause hotkey, macOS Accessibility status). Tray: status line, Open Control Panel, Pause/Resume, Disconnect, Pair phone, Quit. Tray icon has distinct states: paused, waiting, controlled.
- The Control Panel must not take focus while a session is active (keystrokes go to the focused window).
- Android only for 1.0; no web client, no iOS. Expo Go must stay runnable (no `expo prebuild`). Nothing paid, so no paid fonts or services.
- The phone name is not yet carried by the protocol, so the approve dialog currently cannot name the phone.

## Brand Commitments

Name: PocketRemote. No logo, palette, typeface or voice has been chosen. The user's only stated design requirement: the UI and UX must be beautiful.

## Evidence on Hand

PLAN.md sections 5.4, 6 and 7; a working protocol, pairing and server (no UI yet). No screenshots, no brand assets, no user research. Do not fabricate testimonials, store ratings or usage claims.

## Product Principles

1. Trust is visible: connection, pause and pairing state are always legible, on the phone and on the Mac.
2. Operable without looking: large, predictable targets, generous haptics, no precision required in the dark.
3. Pairing is the first impression: a stranger should succeed alone, and it should feel secure and considered.
4. Calm by default: the remote is background to the media; nothing animates or glows without meaning.
5. The phone and the Mac feel like one product.

## Accessibility & Inclusion

Accessibility labels on every control; targets at least 48 dp; status never conveyed by color alone (the dot always comes with words); respect reduced motion and system font scaling. No further product-specific standard established.
