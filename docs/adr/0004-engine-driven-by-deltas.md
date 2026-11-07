# ADR 0004: The engine is driven by time deltas, not a `Clock`

**Status:** accepted. The specification suggested injecting a `Clock`. `Engine::step(dt_ms, telemetry)` takes the elapsed
time explicitly, which is simpler and just as deterministic. `Clock`/`ManualClock` remain available in `erglite-core::clock`.
