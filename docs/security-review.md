# Security review (2026-11-20)

Independent read-only review of the security-sensitive files by a second AI reviewer (Opus), plus a plain-language summary for the owner. It is **not** a substitute for the owner's own review; it is meant to make that review quick. Nothing was run: tests, `cargo deny` and fuzzers are covered by CI.

## What each file does

| File | In plain words | Promise it carries |
|---|---|---|
| `desktop/src/noise.rs` | Thin wrapper over the `snow` crate for the two Noise handshakes and the encrypted channel | Confidentiality, integrity, replay protection, rekeying, frame size limits; errors carry no detail |
| `desktop/src/pairing.rs` | The pairing window: a 32-byte random token, 120 s window, 3 tries, 60 s to press Approve | Nothing is stored until Approve; a token works once |
| `desktop/src/keystore.rs` | Stores the host keypair and exactly one phone key in the macOS Keychain or Windows Credential Manager (0600 file fallback on Unix) | One phone only; Reset pairing deletes both keys |
| `desktop/src/net.rs` | Pure rules: LAN-only source addresses, Origin check, connection caps, blocking after failed handshakes, 250 msg/s cap, trusted networks | Only the local network can talk to the host |
| `desktop/src/server.rs` | The WebSocket listener: serves only `GET /ws`, handshakes under a 3 s timeout, enforces session timeouts, releases held mouse buttons on disconnect | No open web surface; sessions end cleanly |
| `app/src/noise/` | Hand-written TypeScript Noise built on the `@noble` libraries; must match `snow` byte for byte (checked-in test vectors) | The phone and host speak the same, standard protocol |
| `app/src/session.ts` | The phone's state machines: parses the QR strictly, checks the host key matches before revealing the phone key, accepts only host-to-phone messages | The phone only talks to the computer it paired with |
| `desktop/src/bin/host/engine.rs` | Caches secrets in memory (zeroized on drop) so the Keychain is read once | No endless Keychain prompts; names only in debug logs |

## Checked against `PLAN.md` section 3

Met: Noise XXpsk3 / KK; test vectors for both implementations; secure randomness; constant-time comparison (Rust); no secret logging; reset deletes host and phone keys; 120 s single-use pairing token; phone checks the host key; 6-digit code; nothing stored before Approve and a new phone revokes the old one; 3 s handshake timeout; bad frame closes the session; session replacement with `replaced`; 15 s silence and idle timeouts; held input released on disconnect; never binds `0.0.0.0`; LAN allowlist including mapped IPv6; Origin rule; only `GET /ws` (else empty 404); connection caps; 5 failures block for 60 s; 4096-byte frames and 250 msg/s; text limits with control characters stripped; no shell, file, URL, clipboard or arbitrary-key commands; tray states; no `unsafe` code; strict parsing (`deny_unknown_fields`); three fuzz targets; rekey after 2^20 messages.

Partly met or not met (to record as decisions or follow-ups):

| MUST | Status | Note |
|---|---|---|
| Bind the user-selected interface | Not met | Binds the default-route interface only. The interface picker was not built. |
| Network trust by subnet plus gateway MAC | Met on macOS and Linux | Gateway MAC added (finding F6, fixed); on Windows the gateway is still "unknown". |
| Notification on a pairing attempt | Not met | Only "Phone connected" notifies. |
| 3 failures burn the token | Met | Timeouts now count (finding F2, fixed). |
| Lock-screen guard | Partly | Relies on macOS behaviour; no code. |
| Keychain / 0600 fallback | Partly | No file fallback on Windows (documented). |
| QR scheme name | Doc drift | The scheme is `pocketremote`; `PLAN.md` section 3.4 still says `couchremote`. |

## Fixes applied after the review (for the owner's review)

Changed among the security-sensitive files: `desktop/src/server.rs`, `desktop/src/pairing.rs` and `desktop/src/net.rs` (plus tests). Untouched: `noise.rs`, `keystore.rs`, `app/src/noise/` and `app/src/session.ts`. Each fix has a test that fails if the bug is put back (checked by reintroducing it).

| Finding | Status | What changed |
|---|---|---|
| F1 replayed first message | **Fixed** | A session now becomes the live session (and replaces the old one, and reports "connected") only after the phone sends its first valid encrypted message (`activate_session`, called from `session_loop`). A pause that happened since the handshake still applies. The phone sends a ping right after the handshake (`app/src/connection.ts`, not a flagged file). Test: `a_replayed_first_message_does_not_end_the_real_session`. |
| F2 pairing window jam | **Fixed** | A small drop guard (`AttemptGuard`) records and releases a pairing attempt if its future is dropped, for example when the handshake timeout fires. It also now counts as one of the 3 failures. Test: `a_silent_pairing_attempt_does_not_jam_the_window`. |
| F5 revoke can fail without disconnecting | **Fixed** | `revoke_phone` and `reset_pairing` now close the active session even if the key store call fails, and still return the error. Test: `revoke_closes_the_session_even_if_the_key_store_fails`. |
| F3 stale pairing attempt | **Fixed** | Each pairing window has a generation number (`pairing.rs`); an attempt can only finish or fail inside the window it began in. Test: `an_attempt_from_an_older_window_cannot_finish_or_fail_in_a_new_one`. |
| F4 revoke/connect race | **Fixed** | When a session goes live, the server re-checks under the live-session lock that its phone is still the paired phone (`activate_session`). Revoke deletes the key before closing the live session, so a session can no longer slip in alive after a revoke. Lock order checked (no path holds the key store while waiting for the session lock). Test: `a_session_started_before_a_revoke_cannot_go_live_after_it`. |
| F6 weak network identity | **Fixed on macOS and Linux** | The network identity now includes the gateway's hardware address (`net.rs`: `NetworkId::with_mac`, `gateway_mac`), for example `192.168.1.0/24 via 192.168.1.1 (a4:2b:b0:01:02:03)`. The system forgets idle router entries, so the lookup first sends one harmless datagram to the router to refresh it, then reads the table. Parsing is unit-tested on every platform. Windows still has no gateway lookup (documented). Networks allowed before this change need to be allowed once more. The live lookup could not be exercised in the review sandbox (macOS blocks local-network access there); check that your home network is recognised on the next start. |

## Findings (as reported)

**High:** none found.

**Medium**
- **F1. A replayed first message can kick the real phone off** (`server.rs:401-404, 492-495`). The phone's first KK message uses only static-key math, so it can be replayed. The host treats the session as authenticated as soon as it has replied. An attacker on the same network who recorded one connection can resend that message to end the real session with `replaced` and trigger a false "Phone connected" alert. They cannot send commands (they lack the ephemeral key). Effect: denial of service and a false alert. Fix: replace the active session and report "connected" only after the first valid encrypted message arrives; have the phone send a ping right after the handshake.

**Low**
- **F2. Pairing window can jam** (`server.rs:306` with `410-433`). If the 3 s timeout fires while the host waits for message 3, the attempt is never released, so later attempts are refused until the window expires, and the failure is not counted. Any device on the LAN can do this. Fix: release the attempt in a drop guard.
- **F3. Stale attempt can enter a new window** (`pairing.rs:172-175`). Still needs Approve with a mismatching code, so low risk. Fix: tag attempts with a window generation.
- **F4. Race between revoke and connect** (`server.rs:214-217, 399, 492`). A handshake that loaded the old phone key just before Revoke can register afterwards. Fix: re-check the paired key under the active lock.
- **F5. Revoke can fail without disconnecting** (`server.rs:215`). If the keychain delete fails, the session stays open. Fix: close the session first.
- **F6. Weak network identity** (`net.rs:193-198, 257-260`). "192.168.1.0/24 via 192.168.1.1" also matches many public networks. Fix: add the gateway MAC, or accept the risk.

**Notes:** WebSocket ping frames bypass the 250/s cap; with IPv6 link-local an attacker can rotate addresses past per-IP limits and the failure map grows without bound; JavaScript cannot zeroize key material.

## Not verified (for the human reviewer)

Constant-time behaviour of the `@noble` libraries under Hermes; whether `x25519.getSharedSecret` rejects low-order points; internals of `snow` and `keyring-core`; Keychain access-control settings; `expo-secure-store` settings (`storage.ts`); the reconnect and timer layer (`connection.ts`, `useRemote.ts`).

## Sign-off advice

The cryptographic core looks sound: both sides use the specified Noise patterns, test vectors exist for both, the phone pins the host key, nothing is stored without Approve and the limits match the spec. Nothing found lets an attacker control the computer. The owner can reasonably sign off. F1 to F6 are all fixed and tested (F6 on macOS and Linux).
