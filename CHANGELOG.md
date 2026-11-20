# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/). The project is pre-release (0.x).

## [Unreleased]

### Added
- Mac menu-bar app and Control Panel (feature `gui`): tray with waiting / paused / controlled states, pairing QR with countdown, approve with matching code, network trust prompt, Accessibility step, Disconnect, Remove phone, Allow phone control switch, Open at login, connect notification, single-instance lock.
- `scripts/build-macos.sh`: builds `PocketRemote.app` (menu-bar only, ad-hoc signed) and a `.dmg`.
- Phone: Remote page and a swipe page for Browser and slides, typing bar with Backspace / Enter / Live / Send, full-screen Settings page, floating connection dialog.
- Manrope typeface on the phone and the Mac panel.
- Light and dark Mac panel that follows the system setting.
- Documentation set: README, AGENTS.md, architecture, development, install, decisions.

### Changed
- Warmer charcoal palette; keys now have a visible edge (they were invisible at 1.15:1).
- Phone and panel wording says "computer" instead of "Mac".

### Fixed
- Endless Keychain prompts (the host now reads each secret once).
- Feeling locked out after a stale pairing: two failed handshakes now offer "Scan again"; pressing Reconnect no longer hides it; the dialog no longer flashes during retries.
- Pairing refusals now say why.

### Removed
- Redundant Left/Right click keys.

## [0.1.0] - 2026-11
Phase 0 and 1: secure core (protocol, Noise, pairing, keystore, net, server), input layer, first phone UI, CI on three operating systems.
