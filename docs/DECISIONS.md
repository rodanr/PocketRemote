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
