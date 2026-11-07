// SPDX-License-Identifier: MIT OR Apache-2.0
//! Session runner: one tick connects the workout engine, the trainer backend and the recorder.
//!
//! The runner is driven by explicit ticks (`tick(dt_ms)`), so it works in real time (the app calls
//! it once per second) and in accelerated virtual time (tests and the CLI simulator).
#![forbid(unsafe_code)]

use erglite_core::engine::{EngineConfig, EngineEvent, Snapshot, Status};
use erglite_core::metrics::{SamplePoint, Summary, summarize};
use erglite_core::profile::Profile;
use erglite_core::workout::{Route, Workout, WorkoutError};
use erglite_core::{Engine, Telemetry, TrainerCommand};
use erglite_store::{Store, StoreError};
use erglite_transport::{ConnectionState, TrainerBackend};
use thiserror::Error;

const FLUSH_EVERY_SAMPLES: usize = 5;

/// Runner errors.
#[derive(Debug, Error)]
pub enum RunnerError {
    /// The workout is invalid.
    #[error(transparent)]
    Workout(#[from] WorkoutError),
    /// Storage failure.
    #[error(transparent)]
    Store(#[from] StoreError),
}

/// Event emitted by the runner.
#[derive(Debug, Clone, PartialEq)]
pub enum RunnerEvent {
    /// Event from the workout engine.
    Engine(EngineEvent),
    /// The trainer link was lost; the session was paused automatically.
    ConnectionLost,
    /// The link came back; the session resumes after a countdown.
    ConnectionRestored,
    /// A command could not be delivered.
    CommandFailed(String),
}

/// Everything the UI needs after one tick.
#[derive(Debug, Clone, PartialEq)]
pub struct TickOutput {
    /// Engine state.
    pub snapshot: Snapshot,
    /// Latest trainer telemetry.
    pub telemetry: Option<Telemetry>,
    /// Link state.
    pub connection: ConnectionState,
    /// Events produced during the tick.
    pub events: Vec<RunnerEvent>,
    /// A new 1 Hz sample, when one was recorded.
    pub sample: Option<SamplePoint>,
}

/// Runs a workout against a trainer backend.
pub struct SessionRunner<B: TrainerBackend> {
    backend: B,
    engine: Engine,
    route: Option<Route>,
    ftp_w: u16,
    store: Option<(Store, String)>,
    samples: Vec<SamplePoint>,
    unsaved_from: usize,
    since_sample_ms: u32,
    auto_paused: bool,
}

impl<B: TrainerBackend> SessionRunner<B> {
    /// Prepare a session. When a store is given, a `recording` session row is created.
    pub fn new(
        backend: B,
        workout: &Workout,
        profile: &Profile,
        store: Option<Store>,
        workout_id: Option<&str>,
    ) -> Result<Self, RunnerError> {
        let engine = Engine::new(
            workout,
            EngineConfig {
                ftp_w: profile.ftp_w,
                ..Default::default()
            },
        )?;
        let store = match store {
            Some(s) => {
                let id = s.start_session(workout_id, workout, profile.ftp_w, None)?;
                Some((s, id))
            }
            None => None,
        };
        Ok(Self {
            backend,
            engine,
            route: workout.route.clone(),
            ftp_w: profile.ftp_w,
            store,
            samples: Vec::new(),
            unsaved_from: 0,
            since_sample_ms: 0,
            auto_paused: false,
        })
    }

    /// Mutable access to the backend (to connect it first).
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// Give the backend back (after `finish`).
    pub fn into_backend(self) -> B {
        self.backend
    }

    /// Read-only engine access.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Session id in the store, if any.
    pub fn session_id(&self) -> Option<&str> {
        self.store.as_ref().map(|(_, id)| id.as_str())
    }

    /// Recorded samples so far.
    pub fn samples(&self) -> &[SamplePoint] {
        &self.samples
    }

    async fn deliver(&mut self, commands: Vec<TrainerCommand>, events: &mut Vec<RunnerEvent>) {
        for c in commands {
            if let Err(e) = self.backend.apply(c).await {
                events.push(RunnerEvent::CommandFailed(e.to_string()));
            }
        }
    }

    async fn forward(&mut self, step: erglite_core::Step) -> Vec<RunnerEvent> {
        let mut events: Vec<RunnerEvent> =
            step.events.into_iter().map(RunnerEvent::Engine).collect();
        self.deliver(step.commands, &mut events).await;
        events
    }

    /// Start the countdown.
    pub async fn start(&mut self) -> Vec<RunnerEvent> {
        let step = self.engine.start();
        self.forward(step).await
    }

    /// Pause by user request.
    pub async fn pause(&mut self) -> Vec<RunnerEvent> {
        let step = self.engine.pause();
        self.forward(step).await
    }

    /// Resume by user request.
    pub async fn resume(&mut self) -> Vec<RunnerEvent> {
        self.auto_paused = false;
        let step = self.engine.resume();
        self.forward(step).await
    }

    /// Skip to the next segment.
    pub async fn skip_segment(&mut self) -> Vec<RunnerEvent> {
        let step = self.engine.skip_segment();
        self.forward(step).await
    }

    /// Shift power targets by a percentage.
    pub async fn adjust_intensity(&mut self, delta_pct: f32) -> Vec<RunnerEvent> {
        let step = self.engine.adjust_intensity(delta_pct);
        self.forward(step).await
    }

    /// Set the grade difficulty scale.
    pub fn set_difficulty(&mut self, scale: f32) {
        self.engine.set_difficulty(scale);
    }

    /// Advance the session by `dt_ms`.
    pub async fn tick(&mut self, dt_ms: u32) -> TickOutput {
        self.backend.advance(dt_ms);
        let connection = self.backend.connection();
        let telemetry = self.backend.latest();
        let mut events = Vec::new();
        if connection != ConnectionState::Connected {
            if self.engine.status() == Status::Running && !self.auto_paused {
                self.auto_paused = true;
                let step = self.engine.pause();
                events.push(RunnerEvent::ConnectionLost);
                events.extend(self.forward(step).await);
            }
        } else if self.auto_paused && self.engine.status() == Status::Paused {
            self.auto_paused = false;
            events.push(RunnerEvent::ConnectionRestored);
            let step = self.engine.resume();
            events.extend(self.forward(step).await);
        }
        let step = self.engine.step(dt_ms, telemetry.as_ref());
        events.extend(self.forward(step).await);

        let snapshot = self.engine.snapshot();
        let mut sample = None;
        if snapshot.status == Status::Running {
            self.since_sample_ms += dt_ms;
            if self.since_sample_ms >= 1000 {
                self.since_sample_ms -= 1000;
                let altitude = self
                    .route
                    .as_ref()
                    .map(|r| r.elevation_at(snapshot.route_pos_m));
                let s = SamplePoint {
                    t_ms: (snapshot.elapsed_s * 1000.0).round() as u64,
                    power_w: telemetry.and_then(|t| t.power_w),
                    cadence_rpm: telemetry.and_then(|t| t.cadence_rpm),
                    hr_bpm: telemetry.and_then(|t| t.hr_bpm),
                    speed_kmh: telemetry.and_then(|t| t.speed_kmh),
                    distance_m: Some(snapshot.distance_m),
                    altitude_m: altitude,
                    grade_pct: snapshot.grade_pct,
                    target_power_w: snapshot.target_power_w,
                };
                self.samples.push(s);
                sample = Some(s);
                if self.samples.len() - self.unsaved_from >= FLUSH_EVERY_SAMPLES {
                    self.flush();
                }
            }
        }
        TickOutput {
            snapshot,
            telemetry,
            connection,
            events,
            sample,
        }
    }

    fn flush(&mut self) {
        if let Some((store, id)) = &self.store {
            if store
                .append_samples(id, &self.samples[self.unsaved_from..])
                .is_ok()
            {
                self.unsaved_from = self.samples.len();
            }
        }
    }

    /// Stop the session, store the summary and return it.
    pub async fn finish(&mut self, completed: bool) -> Result<Summary, RunnerError> {
        let step = self.engine.stop();
        let _ = self.forward(step).await;
        self.flush();
        let summary = summarize(&self.samples, f64::from(self.ftp_w));
        if let Some((store, id)) = &self.store {
            store.finish_session(id, completed, &summary)?;
        }
        Ok(summary)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use erglite_transport::{SimConfig, SimulatedTrainer};

    fn load(json: &str) -> Workout {
        serde_json::from_str(json).unwrap()
    }

    const SWEET_SPOT: &str = include_str!("../../../fixtures/workouts/sweet-spot-3x8.json");
    const ROLLING: &str = include_str!("../../../fixtures/workouts/rolling-10km.json");

    fn profile() -> Profile {
        Profile {
            ftp_w: 250,
            ..Profile::default()
        }
    }

    #[tokio::test]
    async fn sweet_spot_survives_a_disconnection_and_is_stored() {
        let workout = load(SWEET_SPOT);
        let sim = SimulatedTrainer::new(SimConfig {
            drop_window_ms: Some((1_200_000, 20_000)),
            ..Default::default()
        });
        let store = Store::open_in_memory().unwrap();
        let mut runner = SessionRunner::new(sim, &workout, &profile(), Some(store), None).unwrap();
        runner.backend_mut().connect().await.unwrap();
        let mut events = runner.start().await;
        let mut max_target = 0;
        let mut ticks = 0;
        while runner.engine().status() != Status::Finished && ticks < 6000 {
            let out = runner.tick(1000).await;
            max_target = max_target.max(out.snapshot.target_power_w.unwrap_or(0));
            events.extend(out.events);
            ticks += 1;
        }
        assert_eq!(
            runner.engine().status(),
            Status::Finished,
            "took {ticks} ticks"
        );
        assert!(events.contains(&RunnerEvent::ConnectionLost));
        assert!(events.contains(&RunnerEvent::ConnectionRestored));
        assert_eq!(max_target, 238, "95 % of 250 W");
        let n = runner.samples().len();
        assert!((3700..=3725).contains(&n), "{n} samples");
        let summary = runner.finish(true).await.unwrap();
        assert!(summary.normalized_power_w.unwrap() > 120.0);
        assert!(summary.tss.unwrap() > 20.0);
        assert!(summary.zone_seconds.iter().sum::<u32>() as usize == n);
    }

    #[tokio::test]
    async fn rolling_route_is_ridden_by_distance() {
        let workout = load(ROLLING);
        let mut runner = SessionRunner::new(
            SimulatedTrainer::new(SimConfig::default()),
            &workout,
            &profile(),
            None,
            None,
        )
        .unwrap();
        runner.backend_mut().connect().await.unwrap();
        runner.start().await;
        let mut ticks = 0;
        while runner.engine().status() != Status::Finished && ticks < 4000 {
            runner.tick(1000).await;
            ticks += 1;
        }
        let snap = runner.engine().snapshot();
        assert_eq!(snap.status, Status::Finished);
        assert!(
            (snap.distance_m - 10_000.0).abs() < 120.0,
            "{}",
            snap.distance_m
        );
        let summary = runner.finish(true).await.unwrap();
        assert!(
            summary.elevation_gain_m > 100.0,
            "{}",
            summary.elevation_gain_m
        );
        assert!((600..2400).contains(&ticks), "{ticks}");
        let grades: Vec<f32> = runner
            .samples()
            .iter()
            .filter_map(|s| s.grade_pct)
            .collect();
        assert!(grades.iter().any(|g| *g > 3.0) && grades.iter().any(|g| *g < -3.0));
    }
}
