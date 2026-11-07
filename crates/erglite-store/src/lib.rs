// SPDX-License-Identifier: MIT OR Apache-2.0
//! Local SQLite storage. All SQL lives in this crate.
//!
//! The schema version is tracked in `PRAGMA user_version`; migrations are forward-only SQL files
//! in the repository `migrations/` directory, embedded at compile time.
#![forbid(unsafe_code)]

use erglite_core::clock::{iso8601_utc, unix_now_s};
use erglite_core::metrics::{SamplePoint, Summary};
use erglite_core::profile::Profile;
use erglite_core::workout::Workout;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

const MIGRATIONS: &[&str] = &[include_str!("../../../migrations/0001_init.sql")];

/// Storage errors.
#[derive(Debug, Error)]
pub enum StoreError {
    /// SQLite failure.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    /// A stored JSON document could not be read or written.
    #[error("invalid stored JSON: {0}")]
    Json(#[from] serde_json::Error),
    /// The database was created by a newer version of the app.
    #[error("database schema version {found} is newer than supported {supported}")]
    TooNew {
        /// Version found in the file.
        found: usize,
        /// Highest version this build understands.
        supported: usize,
    },
}

/// Library row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkoutRow {
    /// Workout id.
    pub id: String,
    /// Display name.
    pub name: String,
    /// `time`, `route` or `mixed`.
    pub kind: String,
    /// Where it came from.
    pub source: Option<String>,
    /// Last update (ISO 8601).
    pub updated_at: String,
}

/// Session list row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionRow {
    /// Session id.
    pub id: String,
    /// Start time (ISO 8601).
    pub started_at: String,
    /// `recording`, `completed` or `aborted`.
    pub status: String,
    /// Workout name from the snapshot.
    pub workout_name: String,
    /// Summary when finished.
    pub summary: Option<Summary>,
}

/// Handle to the local database.
pub struct Store {
    conn: Connection,
}

impl Store {
    /// Open (and migrate) the database file, creating it if needed.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        Self::init(Connection::open(path)?)
    }

    /// Open a private in-memory database (tests).
    pub fn open_in_memory() -> Result<Self, StoreError> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self, StoreError> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA synchronous=NORMAL; PRAGMA busy_timeout=5000;",
        )?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        let version = usize::try_from(version).unwrap_or(0);
        if version > MIGRATIONS.len() {
            return Err(StoreError::TooNew {
                found: version,
                supported: MIGRATIONS.len(),
            });
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
            let tx = conn.unchecked_transaction()?;
            tx.execute_batch(sql)?;
            tx.execute_batch(&format!("PRAGMA user_version = {}", i + 1))?;
            tx.commit()?;
        }
        Ok(Self { conn })
    }

    /// Current schema version.
    pub fn schema_version(&self) -> Result<usize, StoreError> {
        let v: i64 = self
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))?;
        Ok(usize::try_from(v).unwrap_or(0))
    }

    /// The rider profile (defaults when never saved).
    pub fn profile(&self) -> Result<Profile, StoreError> {
        let row = self
            .conn
            .query_row(
                "SELECT ftp_w, weight_kg, bike_kg, max_hr, units, language, crr, cda_m2 FROM profile WHERE id = 1",
                [],
                |r| {
                    Ok(Profile {
                        ftp_w: r.get(0)?,
                        weight_kg: r.get(1)?,
                        bike_kg: r.get(2)?,
                        max_hr: r.get(3)?,
                        units: r.get(4)?,
                        language: r.get(5)?,
                        crr: r.get(6)?,
                        cda_m2: r.get(7)?,
                    })
                },
            )
            .optional()?;
        Ok(row.unwrap_or_default())
    }

    /// Save the rider profile.
    pub fn set_profile(&self, p: &Profile) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO profile (id, ftp_w, weight_kg, bike_kg, max_hr, units, language, crr, cda_m2, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(id) DO UPDATE SET ftp_w=?1, weight_kg=?2, bike_kg=?3, max_hr=?4, units=?5,
               language=?6, crr=?7, cda_m2=?8, updated_at=?9",
            params![p.ftp_w, p.weight_kg, p.bike_kg, p.max_hr, p.units, p.language, p.crr, p.cda_m2, now()],
        )?;
        Ok(())
    }

    /// Insert or update a workout. Returns its id.
    pub fn save_workout(
        &self,
        id: Option<&str>,
        w: &Workout,
        source: &str,
    ) -> Result<String, StoreError> {
        let id = id.map_or_else(|| uuid::Uuid::new_v4().to_string(), str::to_string);
        let kind = serde_json::to_value(w.kind)?
            .as_str()
            .unwrap_or("time")
            .to_string();
        let now = now();
        self.conn.execute(
            "INSERT INTO workout (id, name, description, kind, definition_json, source, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)
             ON CONFLICT(id) DO UPDATE SET name=?2, description=?3, kind=?4, definition_json=?5, updated_at=?7",
            params![id, w.name, w.description, kind, serde_json::to_string(w)?, source, now],
        )?;
        Ok(id)
    }

    /// Load a workout.
    pub fn workout(&self, id: &str) -> Result<Option<Workout>, StoreError> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT definition_json FROM workout WHERE id = ?1",
                [id],
                |r| r.get(0),
            )
            .optional()?;
        Ok(json.map(|j| serde_json::from_str(&j)).transpose()?)
    }

    /// List the library, most recently updated first.
    pub fn list_workouts(&self) -> Result<Vec<WorkoutRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT id, name, kind, source, updated_at FROM workout ORDER BY updated_at DESC, name",
        )?;
        let rows = st.query_map([], |r| {
            Ok(WorkoutRow {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get(2)?,
                source: r.get(3)?,
                updated_at: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Delete a workout (sessions keep their snapshot).
    pub fn delete_workout(&self, id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM workout WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Create a session in `recording` state. Returns its id.
    pub fn start_session(
        &self,
        workout_id: Option<&str>,
        snapshot: &Workout,
        ftp_w: u16,
        device: Option<&str>,
    ) -> Result<String, StoreError> {
        let id = uuid::Uuid::new_v4().to_string();
        self.conn.execute(
            "INSERT INTO session (id, workout_id, workout_snapshot_json, status, started_at, ftp_w, device_id)
             VALUES (?1, ?2, ?3, 'recording', ?4, ?5, ?6)",
            params![id, workout_id, serde_json::to_string(snapshot)?, now(), ftp_w, device],
        )?;
        Ok(id)
    }

    /// Append samples in a single transaction.
    pub fn append_samples(
        &self,
        session_id: &str,
        samples: &[SamplePoint],
    ) -> Result<(), StoreError> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut st = tx.prepare_cached(
                "INSERT OR REPLACE INTO sample (session_id, t_ms, power_w, cadence_rpm, hr_bpm, speed_kmh, distance_m, altitude_m, grade_pct, target_power_w)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            )?;
            for s in samples {
                st.execute(params![
                    session_id,
                    s.t_ms as i64,
                    s.power_w,
                    s.cadence_rpm,
                    s.hr_bpm,
                    s.speed_kmh,
                    s.distance_m,
                    s.altitude_m,
                    s.grade_pct,
                    s.target_power_w
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Record a session event.
    pub fn add_event(
        &self,
        session_id: &str,
        t_ms: u64,
        kind: &str,
        data: Option<&str>,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "INSERT INTO session_event (session_id, t_ms, kind, data_json) VALUES (?1, ?2, ?3, ?4)",
            params![session_id, t_ms as i64, kind, data],
        )?;
        Ok(())
    }

    /// Mark a session finished and store its summary.
    pub fn finish_session(
        &self,
        id: &str,
        completed: bool,
        summary: &Summary,
    ) -> Result<(), StoreError> {
        self.conn.execute(
            "UPDATE session SET status = ?2, ended_at = ?3, summary_json = ?4 WHERE id = ?1",
            params![
                id,
                if completed { "completed" } else { "aborted" },
                now(),
                serde_json::to_string(summary)?
            ],
        )?;
        Ok(())
    }

    /// Load all samples of a session in time order.
    pub fn samples(&self, session_id: &str) -> Result<Vec<SamplePoint>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT t_ms, power_w, cadence_rpm, hr_bpm, speed_kmh, distance_m, altitude_m, grade_pct, target_power_w
             FROM sample WHERE session_id = ?1 ORDER BY t_ms",
        )?;
        let rows = st.query_map([session_id], |r| {
            Ok(SamplePoint {
                t_ms: r.get::<_, i64>(0)? as u64,
                power_w: r.get(1)?,
                cadence_rpm: r.get(2)?,
                hr_bpm: r.get(3)?,
                speed_kmh: r.get(4)?,
                distance_m: r.get(5)?,
                altitude_m: r.get(6)?,
                grade_pct: r.get(7)?,
                target_power_w: r.get(8)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// List sessions, newest first.
    pub fn list_sessions(&self) -> Result<Vec<SessionRow>, StoreError> {
        let mut st = self.conn.prepare(
            "SELECT id, started_at, status, workout_snapshot_json, summary_json FROM session ORDER BY started_at DESC",
        )?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, started_at, status, snap, summary) = row?;
            let name = serde_json::from_str::<Workout>(&snap)
                .map(|w| w.name)
                .unwrap_or_default();
            let summary = summary.map(|s| serde_json::from_str(&s)).transpose()?;
            out.push(SessionRow {
                id,
                started_at,
                status,
                workout_name: name,
                summary,
            });
        }
        Ok(out)
    }

    /// Sessions left in `recording` state (for crash recovery).
    pub fn recoverable_sessions(&self) -> Result<Vec<SessionRow>, StoreError> {
        Ok(self
            .list_sessions()?
            .into_iter()
            .filter(|s| s.status == "recording")
            .collect())
    }

    /// Session start time as Unix seconds.
    pub fn session_started_unix(&self, id: &str) -> Result<i64, StoreError> {
        Ok(self.conn.query_row(
            "SELECT CAST(strftime('%s', started_at) AS INTEGER) FROM session WHERE id = ?1",
            [id],
            |r| r.get(0),
        )?)
    }

    /// Delete a session and its samples.
    pub fn delete_session(&self, id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM session WHERE id = ?1", [id])?;
        Ok(())
    }
}

fn now() -> String {
    iso8601_utc(unix_now_s())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use erglite_core::workout::{Control, End, PowerTarget, Segment, SegmentNode, WorkoutKind};

    fn workout(name: &str) -> Workout {
        Workout {
            schema_version: 1,
            name: name.into(),
            description: None,
            kind: WorkoutKind::Time,
            route: None,
            segments: vec![SegmentNode::Single(Segment {
                label: None,
                end: End::TimeS(60),
                control: Control::Power(PowerTarget::steady_pct(80.0)),
                cadence_rpm: None,
                message: None,
            })],
        }
    }

    #[test]
    fn migrates_and_reopens_idempotently() {
        let dir = std::env::temp_dir().join(format!("erglite-{}.db", uuid::Uuid::new_v4()));
        let s = Store::open(&dir).unwrap();
        assert_eq!(s.schema_version().unwrap(), 1);
        drop(s);
        assert_eq!(Store::open(&dir).unwrap().schema_version().unwrap(), 1);
        let _ = std::fs::remove_file(&dir);
    }

    #[test]
    fn profile_defaults_and_roundtrip() {
        let s = Store::open_in_memory().unwrap();
        assert_eq!(s.profile().unwrap(), Profile::default());
        let p = Profile {
            ftp_w: 262,
            weight_kg: 71.5,
            max_hr: Some(188),
            ..Profile::default()
        };
        s.set_profile(&p).unwrap();
        s.set_profile(&p).unwrap();
        assert_eq!(s.profile().unwrap(), p);
    }

    #[test]
    fn workout_crud() {
        let s = Store::open_in_memory().unwrap();
        let id = s.save_workout(None, &workout("A"), "user").unwrap();
        assert_eq!(s.workout(&id).unwrap().unwrap().name, "A");
        s.save_workout(Some(&id), &workout("B"), "user").unwrap();
        let list = s.list_workouts().unwrap();
        assert_eq!((list.len(), list[0].name.as_str()), (1, "B"));
        s.delete_workout(&id).unwrap();
        assert!(s.workout(&id).unwrap().is_none());
    }

    #[test]
    fn session_lifecycle_and_recovery() {
        let s = Store::open_in_memory().unwrap();
        let w = workout("Ride");
        let wid = s.save_workout(None, &w, "user").unwrap();
        let sid = s.start_session(Some(&wid), &w, 250, Some("zpro")).unwrap();
        let samples: Vec<SamplePoint> = (0..3600)
            .map(|i| SamplePoint {
                t_ms: i * 1000,
                power_w: Some(200),
                ..Default::default()
            })
            .collect();
        s.append_samples(&sid, &samples).unwrap();
        assert_eq!(s.recoverable_sessions().unwrap().len(), 1);
        s.add_event(&sid, 1000, "paused", None).unwrap();
        s.finish_session(
            &sid,
            true,
            &Summary {
                duration_s: 3599.0,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(s.recoverable_sessions().unwrap().is_empty());
        let loaded = s.samples(&sid).unwrap();
        assert_eq!((loaded.len(), loaded[10].power_w), (3600, Some(200)));
        assert!(s.session_started_unix(&sid).unwrap() > 1_700_000_000);
        let row = &s.list_sessions().unwrap()[0];
        assert_eq!(
            (row.workout_name.as_str(), row.status.as_str()),
            ("Ride", "completed")
        );
        s.delete_workout(&wid).unwrap();
        assert_eq!(
            s.list_sessions().unwrap().len(),
            1,
            "sessions keep their snapshot"
        );
        s.delete_session(&sid).unwrap();
        assert!(s.samples(&sid).unwrap().is_empty());
    }
}
