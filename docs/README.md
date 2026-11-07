# Documentation

The full development specification (goals, protocols, architecture, workout engine, UI proposal,
compatibility study, work packages and risks) is maintained as a document. Export it as Markdown to
`docs/specification.md` when you publish the repository.

The sections that matter most when implementing:

- 3: connectivity (FTMS, ANT+ FE-C) and 6.4: distance workouts and grade simulation
- 7: SQLite schema (implemented in `migrations/0001_init.sql`)
- 12: compatibility with other trainers
- 13: technical requirements, IPC contract, work packages
