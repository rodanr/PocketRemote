# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/). Releases follow semantic versioning.

## [Unreleased]

## [1.0.0] - 2026-11-20

First release: Android phone app and macOS host. Ad-hoc signed, not notarized (see docs/INSTALL.md).

### Added
- Release workflow (`.github/workflows/release.yml`): a version tag builds the Mac dmg and Android APK and drafts a GitHub release.
- Mac panel self-screenshot tour for review and README images.
- Apache-2.0 license (`LICENSE`, `NOTICE`).
- Mac menu-bar app and Control Panel (feature `gui`): tray with waiting / paused / controlled states, pairing QR with countdown, approve with matching code, network trust prompt, Accessibility step, Disconnect, Remove phone, Allow phone control switch, Open at login, connect notification, single-instance lock.
- `scripts/build-android.sh`: sideloadable release APK (package `com.pocketremote.app`), verified to install and launch standalone on the emulator.
- `scripts/build-macos.sh`: builds `PocketRemote.app` (menu-bar only, ad-hoc signed) and a `.dmg`.
- Phone: Remote page and a swipe page for Browser and slides, typing bar with Backspace / Enter / Live / Send, full-screen Settings page, floating connection dialog.
- Manrope typeface on the phone and the Mac panel.
- Light and dark Mac panel that follows the system setting.
- Documentation set: README, AGENTS.md, architecture, development, install, decisions.

### Changed
- Warmer charcoal palette; keys now have a visible edge (they were invisible at 1.15:1).
- Phone and panel wording says "computer" instead of "Mac".

### Fixed
- Security review follow-ups in `server.rs`: a replayed first handshake message can no longer end the real session or fake a "connected" alert (F1); a silent pairing attempt can no longer jam the pairing window (F2); Remove phone always disconnects first (F5). See `docs/security-review.md`.
- Pairing QR was drawn inverted (light on dark) in dark mode, which some scanners cannot read; it is now always black on white.
- Endless Keychain prompts (the host now reads each secret once).
- Feeling locked out after a stale pairing: two failed handshakes now offer "Scan again"; pressing Reconnect no longer hides it; the dialog no longer flashes during retries.
- Pairing refusals now say why.

### Removed
- Planned global pause hotkey (owner decision; tray and panel already pause instantly).
- Redundant Left/Right click keys.

## [0.1.0] - 2026-11-19
Phase 0 and 1: secure core (protocol, Noise, pairing, keystore, net, server), input layer, first phone UI, CI on three operating systems.
