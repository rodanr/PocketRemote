# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/). Releases follow semantic versioning.

## [Unreleased]

### Changed
- Mac: PocketRemote now keeps its keys in one Keychain item instead of three, so macOS asks for your password once instead of three times. Existing pairings carry over; the first launch after updating may still ask up to three times while the old items are moved.

### Added
- Mac: install with Homebrew, `brew install --cask rodanr/tap/pocketremote`. Install docs now cover checksums, updating and uninstalling.

## [1.0.4] - 2026-11-21

### Fixed
- Mac: the "Phone connected" notification now always shows the PocketRemote icon, including right after launch while PocketRemote is the active app (it used to show a generic icon until you switched to another app). The plain notification is now only a fallback if notification permission is denied or sending fails. No `unsafe` code; one dependency fewer.

## [1.0.3] - 2026-11-21

### Changed
- Mac: the "Phone connected" notification is now a native macOS notification, so it shows the PocketRemote icon (macOS asks once to allow notifications). Falls back to the old plain notification if permission is denied, if PocketRemote is the active app (macOS hides banners for the active app unless it handles them), or if anything fails. No `unsafe` code; one new dependency family (`objc2` bindings) already used by the tray.

## [1.0.2] - 2026-11-21

### Added
- Settings: a **Vibration** switch, a **Keep screen on** switch and a **Scroll speed** stepper.

### Changed
- Phone: 8 dp more space between the keys and the trackpad, so a thumb on the lowest keys is clear of the trackpad edge.
- Mac: the menu-bar icon is smaller (the "P" now measures about 15 pt, like the system icons), and the "Phone connected" notification is sent as PocketRemote so it uses the app's icon.

## [1.0.1] - 2026-11-21

Fixes a crash in 1.0.0: the Mac app quit by itself and the phone lost its connection when a browser key was pressed. **Upgrade from 1.0.0.**

### Fixed
- **The Mac app quit by itself and the phone lost its connection** whenever a key that needs the keyboard layout was pressed, for example any browser key (Close tab is Cmd+W, Reload is Cmd+R, back and forward use brackets). macOS only allows that lookup on an app's main thread, and our input ran on a separate thread, which killed the whole app (signal 5, SIGTRAP). In the Mac app, input now runs on the main thread (`Executor::manual` and `Pump`). The terminal tool is unchanged.

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
- Security review follow-ups in `server.rs`: a replayed first handshake message can no longer end the real session or fake a "connected" alert (F1); a silent pairing attempt can no longer jam the pairing window (F2); a stale pairing attempt cannot finish in a newer window (F3); a session cannot go live after its phone was revoked (F4); Remove phone always disconnects first (F5); the network identity now includes the router's hardware address on macOS and Linux (F6; networks allowed before need to be allowed once more). See `docs/security-review.md`.
- Pairing QR was drawn inverted (light on dark) in dark mode, which some scanners cannot read; it is now always black on white.
- Endless Keychain prompts (the host now reads each secret once).
- Feeling locked out after a stale pairing: two failed handshakes now offer "Scan again"; pressing Reconnect no longer hides it; the dialog no longer flashes during retries.
- Pairing refusals now say why.

### Removed
- Planned global pause hotkey (owner decision; tray and panel already pause instantly).
- Redundant Left/Right click keys.

## [0.1.0] - 2026-11-19
Phase 0 and 1: secure core (protocol, Noise, pairing, keystore, net, server), input layer, first phone UI, CI on three operating systems.
