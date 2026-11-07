# Contributing

Thanks for helping. A few rules keep the project healthy.

- **English only** for code, identifiers, comments, docs, commit messages and UI source strings (`en.json`/`i18n`). Spanish is a translation.
- **Sign off your commits** (`git commit -s`) to accept the [Developer Certificate of Origin](https://developercertificate.org/). There is no CLA.
- **Conventional Commits** (`feat:`, `fix:`, `docs:`, `test:`...). One work package or topic per pull request.
- **Never invent hardware behavior.** If something depends on a real trainer, mark it `// HW-VERIFY:` and add it to `docs/hardware-testing.md`.
- Pure crates (`erglite-core`, `erglite-ftms`, `erglite-formats`) have no I/O and no async; they are deterministic and driven by explicit time deltas.
- Run before pushing: `cargo fmt --all`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and in `app/ui` `npm run check && npm test`.
- UI changes need screenshots and must follow `design/` (tokens, component map, accessibility rules).
- Trainer reports are very welcome: use the *Trainer compatibility report* issue template with the output of `erglite-cli doctor` (planned).
