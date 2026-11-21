# Release checklist

Applies to every release; `v1.0.0` is the first. The owner decides when to tag (`CLAUDE.md`: ask before tagging `v1.0.0`).

## Pre-release gates (status as of 2026-11-20)

| Gate | Command | Status |
|---|---|---|
| Desktop checks | `make desktop` (fmt, clippy with and without `gui`, tests, deny) | passing |
| App checks | `make app` (tsc, eslint, jest, interop) | passing |
| Protocol in sync | `git diff --exit-code app/src/protocol.ts` after regenerating | passing (CI job) |
| Expo health | `cd app && npx expo-doctor` | 20/20 |
| Dependency audit | `cd app && npm audit --omit=dev --audit-level=high` | passing; 3 moderate findings in Expo build tooling (`minimatch`), not shipped in the app |
| CI matrix | Linux, macOS, Windows, deny, fuzz build, interop | green on `main` |
| Real-device test | pair, trackpad, media, typing, reconnect on a phone | done by the owner (Expo Go, Nord 5) |
| Standalone build | `scripts/build-android.sh`, install and launch | done on the emulator |

## Open owner decisions before a public release

- **Trademark** check on "PocketRemote".
- Owner **review of the security-sensitive files** (`noise.rs`, `pairing.rs`, `keystore.rs`, `net.rs`, `server.rs`, `app/src/noise/**`, `app/src/session.ts`).
- Whether to apply to SignPath (free OSS signing for Windows) when a Windows build ships.
- A Play Store upload key (the APK is signed with Expo's debug key and is for sideloading).

## Steps

1. Pick the version. Keep `desktop/Cargo.toml` and `app/app.json` (and `app/package.json`) equal; run `cd desktop && cargo check --locked` after editing so `Cargo.lock` follows.
2. Move the `[Unreleased]` notes in `CHANGELOG.md` under the new version and date.
3. Build the artifacts: `scripts/build-macos.sh` and `scripts/build-android.sh` (outputs in `dist/`).
4. Checksums: `cd dist && shasum -a 256 PocketRemote-*.dmg PocketRemote-*.apk > SHA256SUMS`.
5. Check `wc -c README.md` is non-zero and `git show --stat HEAD` does not touch files you did not mean to. Commit ("release vX.Y.Z"), push, wait for CI to be green.
6. Tag and publish: `git tag -a vX.Y.Z -m "PocketRemote X.Y.Z" && git push origin vX.Y.Z`, then
   `gh release create vX.Y.Z dist/PocketRemote-*.dmg dist/PocketRemote-*.apk dist/SHA256SUMS --notes-file <notes>`.
7. Say in the release notes that the Mac app is ad-hoc signed and not notarized (see `INSTALL.md`).

## Release notes for 1.0.0 (released 2026-11-20 as a draft, then published by the owner)

**PocketRemote 1.0.0** is a free phone remote for your computer over your own Wi-Fi. No account, no cloud.

- Trackpad, typing bar, media keys, D-pad, browser and slide keys.
- Mac menu-bar app with a Control Panel: pair with a QR and a matching code, approve on the computer, pause, disconnect, remove the phone, open at login.
- End-to-end encrypted sessions (Noise), exactly one paired phone, a new network starts paused.
- Android phone app (sideload the APK) and a macOS app (`.dmg`, ad-hoc signed: approve it once under Privacy & Security, Open Anyway).

Known limits: macOS host only (Windows and Linux planned), Android only, the Mac app is not notarized.
