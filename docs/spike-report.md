# Phase 0 spike report

Machine: MacBook, Apple M4 Pro (arm64), macOS. Rust 1.100, Node 26. Spike code lives on branch `spike/phase0` (throwaway).
Crates tried: iced 0.14.0 (tokio feature), tray-icon 0.26.1, muda 0.21.2, enigo 0.6.1, snow 0.10.0, tokio-tungstenite 0.30.0, @noble/{curves,ciphers,hashes} 2.4.0.

## Results

| Item | Result |
|---|---|
| `iced::daemon` with zero windows + tray on macOS | Works. Tray built on the main thread from the first `update` after the event loop started. |
| Window close and reopen while daemon stays alive | Works (open, close, reopen from closed state, all as scripted). |
| Tray menu clicks | NOT tested (needs a human click). Event polling code exists but was not exercised. |
| `enigo` cursor move on macOS | Works, verified by reading the cursor position back: moved exactly (+100, +50) and returned. |
| `enigo` scroll, volume keys | Calls returned success. NOT visually confirmed. |
| macOS Accessibility | Required. Without it `Enigo::new` returns `NoPermission`. The grant belongs to the app that launches the binary (here: the terminal). |
| `snow` KK responder over `tokio-tungstenite` vs a Node client on @noble | Works. Hand-written JS KK initiator (about 60 lines) interoperates with `snow` first time. 300 encrypted echoes, no failures. |
| Loopback round trip (encrypt, send, decrypt, echo) | p50 0.094 ms, p95 0.157 ms, max 0.300 ms. Localhost only, says nothing about Wi-Fi. |
| Idle RAM (release build) | 77 MB tray only, no window. About 100 MB with a window open (wgpu renderer). |

## What needs replacing or watching
- Idle RAM is higher than hoped because iced starts its GPU renderer. Consider the `tiny-skia` software renderer feature to cut it. Measure again in Phase 2.
- The Dock icon behaviour of the daemon was not checked.
- Volume/media keys and scroll need a visual or a different check before we trust them.

## Not tested (no machine or needs the phone)
- Windows, Linux X11, Wayland: untested, no machine available.
- Real Wi-Fi latency to the Nord 5 (target under 30 ms p95).
- Whether Android needs a local-network permission for a LAN WebSocket, and which `Origin` React Native sends.
- Tray menu clicks and the global pause hotkey.
- The full XXpsk3 pairing pattern (only KK was spiked).

## Recommendation
Continue to Phase 1 with the planned stack. Nothing found so far argues for Tauri or a different crypto scheme. The JS Noise implementation looked manageable for KK; XXpsk3 is still to do and must pass the public test vectors.
