# AGENTS.md

Instructions for AI coding agents (Claude Code, Codex, Cursor, Copilot, and others) working in this repository.
`CLAUDE.md` holds the binding rules in full; this file is the fast map. If they ever disagree, `CLAUDE.md` and `PLAN.md` win.

## Read first

1. `PLAN.md`: the normative spec. Everything marked **MUST** in section 3 is binding.
2. `CLAUDE.md`: non-negotiable rules and when to stop and ask.
3. `docs/ARCHITECTURE.md`: how the pieces fit. `docs/DEVELOPMENT.md`: how to run and test.

## Non-negotiables (short form)

- Security first. Never weaken a limit, skip a check, remove a test, or add a command type to make a task easier.
- Noise only (`snow` on the host, `app/src/noise/` on the phone). Never change the cryptography without asking.
- Never log or print keys, pairing tokens, decrypted payloads or typed text.
- Never add commands for shell, files, URLs, clipboard or arbitrary key combos.
- Nothing paid: no paid services, accounts or dependencies.
- One paired phone. Android only for the phone app in 1.0. Expo Go must keep working in development (no `expo prebuild` dependency).
- Protocol changes start in `desktop/src/protocol.rs`; regenerate `app/src/protocol.ts`; never hand-edit it.
- Security-sensitive files need the human's review: `desktop/src/{noise,pairing,keystore,net,server}.rs`, `app/src/noise/**`, `app/src/session.ts`, `protocol/`, `.github/`.
- New dependencies: justify each one, check current docs, prefer fewer.

## Repository map

```
desktop/src/            Rust host library (protocol, noise, pairing, keystore, net, server, executor, input/)
desktop/src/main.rs     terminal host (dev tool)
desktop/src/bin/host/   GUI host: tray + Control Panel (cargo feature "gui")
desktop/examples/       interop_server, input_selftest, cursor_pos
app/App.tsx             phone app entry; app/src/ui/ screens and keys; app/src/noise/ client crypto
app/src/{session,connection,useRemote}.ts   protocol session, reconnect state machine, React hook
protocol/               shared fixtures and test vectors
brand/, scripts/        icons and packaging
```

## Commands that must pass after every change

```sh
# desktop
cd desktop && cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings \
  && cargo clippy --features gui --all-targets --locked -- -D warnings && cargo test --locked && cargo deny check
# phone app
cd app && npx tsc --noEmit && npx eslint . && npx jest && npm run test:interop
```

`make check` runs all of it. After touching `protocol.rs`: regenerate and confirm `git diff --exit-code app/src/protocol.ts`.

## Working conventions

- Work directly on `main` in this project (owner's choice); commit small and often with clear messages. Do not open PRs unless asked.
- UI: follow `docs/design-brief.md`. Amber is only for the lead action and live state; one key radius; icons-only main page; keys must have a visible edge (about 3:1). Wording says "computer", never "Mac", in the phone app.
- Owner rule for features: only simple, reliable ones. No redundant controls. Skip anything risky, confusing or hacky.
- Verify visually: boot the Android emulator and use an isolated dry-run host (recipe in `docs/DEVELOPMENT.md`). Do not disturb the owner's real session or send real input while testing. The Mac Control Panel can photograph itself (`POCKETREMOTE_SHOTS`, see `docs/DEVELOPMENT.md`); use that to check it in light and dark.
- Read tool output before taking screenshots (compile errors look like a broken UI).

## Useful environment switches (development only)

`POCKETREMOTE_FILE_KEYS=1` keys in a 0600 file instead of the Keychain · `POCKETREMOTE_DRY_RUN=1` record input, do not perform it · `POCKETREMOTE_DEBUG=1` log which secrets are touched (names only).

## When to stop and ask

At every **ASK** marker in `PLAN.md`, before tagging `v1.0.0`, before deleting or renaming anything under `protocol/`, and whenever a failing check could only be fixed by changing a MUST.
