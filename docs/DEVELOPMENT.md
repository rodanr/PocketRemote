# Development

## Prerequisites

Rust (stable, edition 2024), Node 22+, `cargo-deny`, Android Studio's SDK and a virtual device (`Pixel_9_Pro` was used), Expo Go on the phone. Optional: nightly Rust and `cargo-fuzz` for the fuzz targets.

## Run it

```sh
# Mac host with the Control Panel and tray (feature "gui")
cd desktop && cargo run --features gui --bin pocketremote-host
# Terminal host (dev tool). Flags: --file-keys  --print-uri  --dry-run
cd desktop && cargo run -- --file-keys --dry-run
# Phone app in Expo Go
cd app && npm install && npx expo start
```

Environment switches (development only): `POCKETREMOTE_FILE_KEYS=1` (keys in a 0600 file, avoids Keychain prompts), `POCKETREMOTE_DRY_RUN=1` (record input, do not perform it), `POCKETREMOTE_DEBUG=1` (log which secrets are touched; names only).

macOS needs the **Accessibility** permission for whichever app launches the host (your terminal while developing). Unsigned debug builds look like a new program to the Keychain after every rebuild, so it may ask again; use `POCKETREMOTE_FILE_KEYS=1` while iterating.

## Checks (all must pass)

```sh
make check          # everything below
cd desktop && cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings \
  && cargo clippy --features gui --all-targets --locked -- -D warnings && cargo test --locked && cargo deny check
cd app && npx tsc --noEmit && npx eslint . && npx jest && npm run test:interop
```

After changing `desktop/src/protocol.rs`: regenerate with `UPDATE_PROTOCOL_TS=1 cargo test --locked protocol`, update `protocol/fixtures/`, then `git diff --exit-code app/src/protocol.ts`.

## Visual verification without touching your real session

An agent or developer can look at the phone UI on the emulator against an isolated host:

```sh
export ANDROID_HOME=$HOME/Library/Android/sdk PATH=$PATH:$ANDROID_HOME/emulator:$ANDROID_HOME/platform-tools
emulator -avd Pixel_9_Pro -no-window -no-audio -gpu swiftshader_indirect &        # headless
HOME=/tmp/pr-visual-home ./desktop/target/debug/pocketremote --file-keys --print-uri --dry-run < fifo &
adb shell am start -a android.intent.action.VIEW -d "exp://<your-LAN-ip>:8081"   # Expo Go
adb exec-out screencap -p > shot.png ; adb shell settings put system font_scale 1.3   # also test large text
```

The terminal host in `--dry-run` never moves the pointer. Use the dev-only paste-link field on the phone's pairing screen with the `DEV-URI` it prints.

## Packaging

```sh
scripts/build-macos.sh      # dist/PocketRemote.app (menu-bar app, ad-hoc signed) and dist/PocketRemote-<version>.dmg
```

Android release APK: `cd app && npx expo prebuild --platform android && cd android && ./gradlew assembleRelease` (the generated `android/` folder is git-ignored; Expo Go development is unaffected). Signing and distribution notes are in [INSTALL.md](INSTALL.md).

## Regenerating assets

`python3 brand/build/make_icons.py` (app icons), `python3 brand/build/make_tray_rgba.py` (tray icons, raw RGBA embedded by the host).
