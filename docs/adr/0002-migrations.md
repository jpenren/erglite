# ADR 0002: Own migration runner instead of `rusqlite_migration`

**Status:** accepted. The specification named `rusqlite_migration`. `erglite-store` uses a 20-line runner over
`PRAGMA user_version` and embedded SQL files instead, which avoids coupling the crate to a specific `rusqlite` release.
Migrations stay forward-only SQL files in `migrations/`. Revisit if migrations become complex.
