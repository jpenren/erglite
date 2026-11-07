# ErgLite interface reference

Static reference for the ErgLite UI. The agent implementing work packages WP8 (time workouts) and WP9 (route mode) should follow these screens. Copy this folder to `design/` in the repository.

## Files

| File | Purpose |
|---|---|
| `tokens.css` | Colors, typography, radii and spacing. Port to CSS variables in `app/ui/src/styles/`. |
| `screens/01-session-time.html` | Session screen for ERG workouts (by time). |
| `screens/02-session-route.html` | Session screen for route workouts (by distance, grade simulation). |
| `screens/03-workout-editor.html` | Block editor for time workouts. |
| `screens/04-route-editor.html` | Elevation-versus-distance editor. |
| `screens/05-devices.html` | Trainer and sensor connection. |

Open any screen in a browser. They are fluid pages designed at 1280 x 720 and use sample data only. Hard-coded hex values in the HTML must become the tokens above.

## Rules

1. The mockups show layout, hierarchy and visual style. Behavior comes from the specification (sections 6, 11 and 13).
2. Do not reuse power-zone colors for grades or the other way around. Every zone and grade color needs a text label or legend.
3. Session screens: figures readable from 2 m, tabular numerals, no layout shift when numbers change.
4. UI strings go in `src/i18n/en.json` first, then `es.json`. No hard-coded text.
5. Show only what `TrainerCapabilities` allows. Hide unsupported controls.
6. Keyboard operable, visible focus, minimum 44 px targets, no information by color alone.

## Component map (Svelte)

| Component | Used in | Props and notes |
|---|---|---|
| `AppShell` + `SideNav` | 03, 04, 05 | Nav: Home, Devices, Library, Editor, Routes, History, Settings. The session screen is full screen without nav. |
| `MetricTile` | 01, 02 | label, value, unit, sub text, optional accent. Sizes: primary tile 130 to 150 px figure, secondary 84 to 110 px. |
| `StatusChip` | 01, 02, 05 | Trainer and sensor state, with a dot and text. |
| `WorkoutProfile` | 01, 03 | Blocks as bars colored by zone; props: segments, currentIndex, elapsed. Past blocks dimmed, cursor line. |
| `ElevationProfile` | 02, 04 | SVG area chart colored by grade; props: points, cursorDistance, selectedSegment, editable. Past part dimmed. |
| `ControlBar` | 01, 02 | Pause (primary), -/+ (intensity or difficulty), skip block (time only), finish (danger). |
| `BlockTable` + `BlockInspector` | 03 | Editable table and side form for the selected block. |
| `RouteToolbar` | 04 | Freehand, move nodes, constant grade, templates, import GPX, smooth. |
| `DeviceCard` + `SensorRow` | 05 | Capabilities as chips; connect and disconnect actions. |

## Screen notes and data bindings

- **01 Session (time)**: events `telemetry:tick`, `session:state`, `session:segment_changed`. Main figure is power with its target and zone label. "Next" line shows the following block and a countdown.
- **02 Session (route)**: same events plus covered distance and grade. Main figure is the current grade. Power has no target, only an optional guide band. Buttons adjust difficulty. Remaining time is an estimate (physics estimator).
- **03 Workout editor**: drag a block edge for duration, drag its top for intensity; the table edits the same data. Undo and redo. Totals (duration, estimated TSS) update live.
- **04 Route editor**: draggable nodes, freehand drawing resampled to 100 m, templates, GPX import with smoothing. Live totals: elevation gain, max climb, max descent, estimated time at a chosen power.
- **05 Devices**: commands `scan_devices`, `connect_trainer`, `disconnect_trainer`; event `device:discovered`. Show a clear message when the trainer is busy with another connection.

## States not drawn in the mockups (implement them)

Scanning, connecting, connection lost with automatic retry (session paused, banner over the profile), 3 second countdown, paused, empty library, import report with warnings, error toasts from `app:error`, session recovery prompt after a crash, light theme, and a mixed-workout transition notice (v1.2).

## Known simplifications

Sample data only, no interactions, fonts load from a CDN in these files for convenience (the app must bundle them), and the profile charts are drawn with static markup.
