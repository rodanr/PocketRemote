# Installing PocketRemote

PocketRemote is not distributed publicly yet. These steps install a build you made yourself.

## Mac host

1. Build: `scripts/build-macos.sh` (needs Rust; output in `dist/`).
2. Open `dist/PocketRemote-<version>.dmg` and drag **PocketRemote** into **Applications**.
3. The app is **ad-hoc signed, not notarized** (no paid Apple account). The first time, macOS may block it:
   System Settings, Privacy & Security, scroll to the message about PocketRemote, **Open Anyway**.
4. Start it. A menu-bar icon appears and the Control Panel opens.
5. **Accessibility:** follow the panel's three steps (System Settings, Privacy & Security, Accessibility, switch PocketRemote on). Re-grant it after updating, because unsigned apps can lose the grant.
6. **Keychain:** macOS asks whether PocketRemote may use its own keychain items (`pocketremote`). Enter your Mac password and choose **Always Allow**. It only ever reads the three items it created.
7. **Network:** on a network it has not seen, PocketRemote stays paused until you press **Allow this network**.

**If the app icon looks generic at first** (in Finder, the menu bar or notifications): macOS registers a newly installed app's icon in the background and can take a short while. To refresh it right away, run `/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f /Applications/PocketRemote.app && killall Dock Finder`.

**Notification icon:** the "Phone connected" notification always shows the PocketRemote icon, including while PocketRemote is the app you are currently using. (A plain fallback with a generic icon is used only if you deny notification permission.)

Turn on **Open at login** in the Control Panel if you want it to start with your Mac.

## Android phone

Release APK (sideload): run `scripts/build-android.sh`, copy `dist/PocketRemote-<version>.apk` to the phone (or `adb install -r` it), and allow installs from your file manager. Or, while developing, open the app in **Expo Go**.

## Pair

1. In the Control Panel press **Pair phone** (QR appears with a countdown).
2. In the phone app scan the QR.
3. Check the 6-digit code matches on both screens, then press **Approve** on the computer.

If the phone ever says it is not paired with this computer (keys changed, phone removed), choose **Scan again** and pair again.
