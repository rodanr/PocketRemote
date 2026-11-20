# Decisions log

Short records of choices that are not obvious from the code. Newest last. Format: decision, why, consequence.

1. **Rust host, TypeScript phone app (Expo).** The owner prefers Rust and TypeScript; the host needs a small, safe, fast native process, and the phone app must run in Expo Go during development. Consequence: no `expo prebuild` dependency; config plugins are fine.
2. **Noise protocol, no home-made crypto.** `snow` on the host and a tested TypeScript implementation on the phone, checked with public test vectors. Pairing uses `XXpsk3`, sessions use `KK` with pinned keys.
3. **macOS first.** The owner's only test machine. Everything OS-specific sits behind the `InputBackend` trait; CI builds and tests on Linux, macOS and Windows.
4. **Android only for 1.0.** No web client and no iOS-specific code.
5. **Work directly on `main`.** Owner's choice for speed on this private repo; no feature branches or PRs unless asked.
6. **"Aisle Light" look.** Warm charcoal ground, a single amber accent used only for the lead action and live state, one key radius, keys with a visible 3:1 edge. Review with Opus found keys were invisible at 1.15:1 and drove the visibility pass (see `design-brief.md`).
7. **Phone pages.** A Remote page (icons only) plus a swipe page for Browser and slides, with 4 dp dots that cost no height. A tab strip, a header button and a bottom sheet were tried and rejected as noisy or space-hungry.
8. **No redundant controls.** Left/Right click keys were removed because tap and two-finger tap already do it. Backspace and Enter stay because they act on the computer's own text boxes.
9. **Typing bar.** One row: field plus Backspace, Enter, Live. The third slot is Send once text is typed, otherwise the Live switch. The field is fixed-height while typing so it never resizes; a filled keyboard control inside it hides the keyboard (the Back key is never required).
10. **Settings is a full page**, and the no-connection state is a floating dialog, so nothing is pushed down.
11. **Stale pairing detection.** Two failed handshakes after reaching the host mean the pairing is stale, so the phone offers Scan again at once. Pressing Reconnect never hides that option.
12. **Host app is behind a `gui` cargo feature** (`iced` with the software renderer, `tray-icon`, `muda`). The core crate and its CI stay free of GUI and system-library dependencies.
13. **Secrets cached in memory in the GUI host** (read from the Keychain once), so a phone that retries every few seconds cannot trigger endless Keychain prompts.
14. **Licenses allowed in `deny.toml`:** MIT, Apache-2.0, BSD-2/3, ISC, Unicode-3.0, Zlib, CC0, BSL-1.0, Apache-2.0 WITH LLVM-exception; MPL-2.0 only for `option-ext`. The project's own license is still undecided.
15. **Manrope** (SIL OFL) on the phone and the Mac panel, bundled, nothing paid.
16. **Nothing paid.** The Mac app is ad-hoc signed and not notarized; users approve it once. SignPath (free OSS signing for Windows) is an open question for the owner.
17. **No pause hotkey** (owner, 2026-11-20). Pause is already one click in the tray and a switch in the Control Panel; skipping `global-hotkey` also avoids a dependency and OS key clashes. Recorded in `PLAN.md` section 3.10.
18. **Dependabot skips Expo-pinned majors** (owner, 2026-11-20). `react`/`react-native` stay on the Expo SDK's pinned versions (a mismatched renderer breaks the app); majors of `typescript`, `eslint` and `@types/node` wait for Expo tooling and the Node version in use. PRs #1-#4 were closed for this reason. Minor and patch updates still arrive.
19. **License: Apache-2.0** (owner, 2026-11-20). Permissive with a patent grant and a trademark clause; matches the MIT/Apache/BSD dependencies. The name and logo are not licensed for reuse (see `NOTICE`).
20. **Session goes live after the first valid encrypted message** (2026-11-20, from the security review). The Noise KK first message can be replayed, so it must not replace an active session. The phone pings right after the handshake.
21. **Network identity includes the gateway MAC** (2026-11-20, from the security review). `192.168.1.0/24 via 192.168.1.1` also matches many other networks. Where the system exposes the router's hardware address (macOS, Linux) it is part of the identity; otherwise we fall back to subnet plus gateway IP. Networks trusted under the old format are asked about once more.
22. **In the Mac app, input runs on the main thread** (2026-11-21, after a crash report from the owner). On macOS the keyboard-layout lookup behind letter keys (Cmd+W, Cmd+R, brackets) may only run on the main thread; on the old input thread it killed the whole app with SIGTRAP, which showed as the app closing and the phone disconnecting. The app now queues commands and wakes the main thread to run them (`Executor::manual`, `Pump`, `WakingSink` in `engine.rs`). Reproduced with a harmless key-release probe: the worker thread died with exit code 133, the main thread survived. The terminal host still uses a thread; it is a plain command-line process and does not enforce this.
