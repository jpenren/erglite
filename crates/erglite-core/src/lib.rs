// SPDX-License-Identifier: MIT OR Apache-2.0
//! ErgLite core: workout model, execution engine, physics estimator and metrics.
//!
//! This crate is pure: no I/O, no async, no wall-clock access. The engine is driven by explicit
//! time deltas, which makes every behavior deterministic and testable.
#![forbid(unsafe_code)]

pub mod clock;
pub mod engine;
pub mod metrics;
pub mod physics;
pub mod profile;
pub mod workout;

pub use engine::{
    Engine, EngineConfig, EngineEvent, Snapshot, Status, Step, Telemetry, TrainerCommand,
};
pub use workout::{Control, End, GradeControl, PowerTarget, Route, Segment, SegmentNode, Workout};
