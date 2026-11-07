# ADR 0003: A separate `erglite-runtime` crate

**Status:** accepted. The specification placed orchestration in the Tauri shell. The session runner (engine + trainer +
recorder, including reconnection handling) was moved to `erglite-runtime` so it can be tested end to end against the
simulated trainer and reused by the CLI. The shell only forwards commands and events.
