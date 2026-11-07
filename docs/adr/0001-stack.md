# ADR 0001: Tauri 2 with a Rust core and a Svelte UI

**Status:** accepted. All trainer, engine, protocol and storage logic lives in Rust crates that are testable without a UI.
The UI is a web UI (Svelte 5, TypeScript) so the editors, charts and a large session screen are easy to build and style.
Alternatives considered: egui (one language, more technical look), Flutter (weaker desktop BLE), Web Bluetooth PWA (Chromium only, no ANT+).
