# Architecture

```
UI (Svelte)  <->  Tauri IPC (commands and events)  <->  erglite-runtime  <->  TrainerBackend
                                                         |   |                 (simulated | BLE FTMS)
                                                         |   +-- erglite-core (engine, physics, metrics)
                                                         +------ erglite-store (SQLite)
```

- **Engine** (`erglite-core::Engine`) is a deterministic state machine. It is driven by `step(dt_ms, telemetry)`
  and returns `TrainerCommand`s and events. It never reads a clock and never talks to hardware.
- **Time segments** advance with the clock and command ERG targets. **Distance segments** advance with the
  speed reported by the trainer and command a smoothed, rate-limited grade taken from the route.
- **Runner** (`erglite-runtime::SessionRunner`) ties one engine, one backend and the recorder together:
  it pauses on connection loss, resumes on recovery, records 1 Hz samples and flushes them to SQLite every 5 s.
- **Backends** implement `TrainerBackend` and are polled; real backends ignore `advance`.
- **UI** talks to a `Backend` interface (`app/ui/src/lib/types.ts`) implemented by the Tauri IPC layer and by an
  in-browser mock used for development and the demo build.

## Command and event contract

Commands: `get_profile`, `set_profile`, `list_workouts`, `save_workout`, `delete_workout`, `import_workout`,
`scan_devices`, `connect_trainer`, `disconnect_trainer`, `start_session`, `pause_session`, `resume_session`,
`skip_segment`, `adjust_intensity`, `set_difficulty`, `stop_session`, `list_sessions`, `session_samples`,
`export_session`. Event: `session:tick` (one per second, payload mirrors `TickEvent`).
