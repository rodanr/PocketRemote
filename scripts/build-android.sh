#!/usr/bin/env bash
# Builds a sideloadable release APK (no Expo Go needed): dist/PocketRemote-<version>.apk
# Needs the Android SDK (ANDROID_HOME) and JDK 17+. The generated app/android folder is git-ignored.
# The APK is signed with Expo's default debug keystore: fine for installing on your own phone,
# not for the Play Store (a real upload key is a later, owner decision).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/app"
VERSION="$(node -p "require('./app.json').expo.version")"
: "${ANDROID_HOME:=$HOME/Library/Android/sdk}"
export ANDROID_HOME ANDROID_SDK_ROOT="$ANDROID_HOME"
npx expo prebuild --platform android --no-install --clean
(cd android && ./gradlew assembleRelease --console=plain)
mkdir -p "$ROOT/dist"
cp android/app/build/outputs/apk/release/app-release.apk "$ROOT/dist/PocketRemote-${VERSION}.apk"
echo "done: dist/PocketRemote-${VERSION}.apk"
