# Contributing

The project is private while it is prepared for release. These rules apply to humans and AI agents alike.

1. Read `PLAN.md` (spec) and `CLAUDE.md` (rules). Security MUSTs are binding.
2. Keep changes small and focused. Add or update tests with every behaviour change.
3. Run `make check` before committing; every check must pass.
4. Do not weaken limits, skip checks, remove tests, add command types, log secrets, or add paid dependencies.
5. Justify every new dependency (what, why, license, alternatives considered). Prefer fewer.
6. Protocol changes start in `desktop/src/protocol.rs`; regenerate `app/src/protocol.ts` and update `protocol/fixtures/`.
7. Security-sensitive files (see `.github/CODEOWNERS`) need the owner's review. Say in the commit or PR what changed in them and why.
8. UI changes follow `docs/design-brief.md` and are checked on the emulator (also at 1.3x font size) and, for the Mac panel, in light and dark mode.
9. Write commit messages that say what and why. Reference docs you updated.

Security issues: do not open a public issue; see `SECURITY.md`.
