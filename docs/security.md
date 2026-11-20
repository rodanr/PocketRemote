# Security

This document is the threat model for PocketRemote. The binding requirements are in `PLAN.md` section 3.

## Threat model

A session gives full control of the user's desktop session. A stolen session is treated as a fully compromised account.

| Adversary | Capability | Defence |
|---|---|---|
| Passive LAN sniffer | Reads all traffic | Noise-encrypted transport |
| Active LAN attacker (ARP spoofing, rogue AP) | Relays, modifies, replays, injects | Noise KK mutual authentication with pinned static keys; AEAD with nonces |
| Stranger on public Wi-Fi | Connects to the open port | Server paused on untrusted networks; pairing needs a fresh token and approval on the host |
| Malicious web page in any browser | Opens a WebSocket to the host (CSWSH) | Origin check; the host serves no web pages; Noise keys never in a browser |
| Someone who saw the QR (screen share, stream, photo) | Tries to pair | QR shown only on demand, single use, 120 s expiry, host-side Approve with a matching code |
| Lost or stolen phone | Uses the stored key | Revoke in the Control Panel or tray; pairing a new phone revokes the old one |
| Later key theft | Decrypts recorded old sessions | Forward secrecy from ephemeral keys (Noise) |
| Compromised dependency or release pipeline | Ships malicious code | Lockfiles, `cargo-deny`, audits, pinned Actions, signed checksums, build provenance |

Out of scope: malware already running on the computer or the phone. The owner of the paired phone is fully trusted.

## Notifications

The Mac app shows a fixed-text notification when a phone connects (`Phone connected`), through Apple's UserNotifications framework when running from the installed app, with a plain fallback otherwise. It never includes keys, tokens, typed text or anything from the session. It needs no network access or special entitlements, and the user can revoke it in System Settings, Notifications.

## Hard rules

- Noise only (`snow` on the host, a tested implementation on the phone). No home-made handshake.
- Never log keys, pairing tokens, decrypted payloads or typed text. Log message types and sizes only.
- No command that runs a shell, touches files, opens URLs, reads the clipboard or sends arbitrary key combinations.
- Exactly one paired phone and one active session.

## Per-OS notes

### macOS
- Input injection needs the Accessibility permission. It is granted to the app that launches the process, so during development it is the terminal. Unsigned builds may lose the grant after an update.
- Input is not delivered while the session is locked or into secure text fields.
