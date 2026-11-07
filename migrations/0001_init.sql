-- SPDX-License-Identifier: MIT OR Apache-2.0
-- Initial schema. Forward-only; applied by erglite-store and tracked in PRAGMA user_version.
CREATE TABLE profile (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  ftp_w INTEGER NOT NULL CHECK (ftp_w BETWEEN 50 AND 600),
  weight_kg REAL NOT NULL, bike_kg REAL NOT NULL DEFAULT 9,
  max_hr INTEGER, units TEXT NOT NULL DEFAULT 'metric',
  language TEXT NOT NULL DEFAULT 'en',
  crr REAL NOT NULL DEFAULT 0.004, cda_m2 REAL NOT NULL DEFAULT 0.35,
  zones_json TEXT, updated_at TEXT NOT NULL
);
CREATE TABLE workout (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL, description TEXT,
  kind TEXT NOT NULL CHECK (kind IN ('time','route','mixed')),
  definition_json TEXT NOT NULL,
  source TEXT,
  created_at TEXT NOT NULL, updated_at TEXT NOT NULL
);
CREATE TABLE session (
  id TEXT PRIMARY KEY,
  workout_id TEXT REFERENCES workout(id) ON DELETE SET NULL,
  workout_snapshot_json TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN ('recording','completed','aborted')),
  started_at TEXT NOT NULL, ended_at TEXT,
  ftp_w INTEGER NOT NULL, device_id TEXT,
  summary_json TEXT
);
CREATE TABLE sample (
  session_id TEXT NOT NULL REFERENCES session(id) ON DELETE CASCADE,
  t_ms INTEGER NOT NULL,
  power_w INTEGER, cadence_rpm REAL, hr_bpm INTEGER, speed_kmh REAL,
  distance_m REAL, altitude_m REAL, grade_pct REAL, target_power_w INTEGER,
  PRIMARY KEY (session_id, t_ms)
) WITHOUT ROWID;
CREATE TABLE session_event (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  session_id TEXT NOT NULL REFERENCES session(id) ON DELETE CASCADE,
  t_ms INTEGER NOT NULL, kind TEXT NOT NULL, data_json TEXT
);
CREATE TABLE device (
  id TEXT PRIMARY KEY, name TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('trainer','heart_rate','power','cadence')),
  capabilities_json TEXT, quirks_profile TEXT, last_connected_at TEXT
);
CREATE TABLE setting (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE INDEX idx_session_started ON session(started_at DESC);
CREATE INDEX idx_event_session ON session_event(session_id, t_ms);
