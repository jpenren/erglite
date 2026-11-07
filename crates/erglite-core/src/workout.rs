// SPDX-License-Identifier: MIT OR Apache-2.0
//! Workout and route data model (file format version 1).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Current workout file schema version.
pub const SCHEMA_VERSION: u32 = 1;

/// Validation errors for a workout definition.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkoutError {
    /// The workout has no segments.
    #[error("workout has no segments")]
    Empty,
    /// A segment has a zero or invalid end criterion.
    #[error("segment {0} has an invalid end criterion")]
    InvalidEnd(usize),
    /// A repeat block has zero repetitions or no children.
    #[error("repeat block {0} is empty")]
    InvalidRepeat(usize),
    /// A segment uses `from_route` but the workout has no route.
    #[error("segment {0} follows the route but the workout has no route")]
    MissingRoute(usize),
    /// The route points are not strictly increasing in distance.
    #[error("route distances must be strictly increasing")]
    InvalidRoute,
}

/// How a segment ends.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum End {
    /// After this many seconds.
    TimeS(u32),
    /// After this many meters.
    DistanceM(f64),
}

/// Power target of an ERG segment. Set either a steady value or a from/to ramp.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PowerTarget {
    /// Steady target as percent of FTP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pct_ftp: Option<f32>,
    /// Steady target in watts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub watts: Option<f32>,
    /// Ramp start as percent of FTP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_pct_ftp: Option<f32>,
    /// Ramp end as percent of FTP.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_pct_ftp: Option<f32>,
    /// Ramp start in watts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_watts: Option<f32>,
    /// Ramp end in watts.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_watts: Option<f32>,
}

impl PowerTarget {
    /// Steady target as percent of FTP.
    pub fn steady_pct(p: f32) -> Self {
        Self {
            pct_ftp: Some(p),
            ..Self::default()
        }
    }

    /// Linear ramp between two percentages of FTP.
    pub fn ramp_pct(from: f32, to: f32) -> Self {
        Self {
            from_pct_ftp: Some(from),
            to_pct_ftp: Some(to),
            ..Self::default()
        }
    }

    /// Resolve to watts for a given FTP and segment progress in `0.0..=1.0`.
    pub fn resolve(&self, ftp_w: f32, progress: f32) -> Option<f32> {
        let p = progress.clamp(0.0, 1.0);
        if let Some(v) = self.pct_ftp {
            return Some(ftp_w * v / 100.0);
        }
        if let Some(v) = self.watts {
            return Some(v);
        }
        if let (Some(a), Some(b)) = (self.from_pct_ftp, self.to_pct_ftp) {
            return Some(ftp_w * (a + (b - a) * p) / 100.0);
        }
        if let (Some(a), Some(b)) = (self.from_watts, self.to_watts) {
            return Some(a + (b - a) * p);
        }
        None
    }
}

/// Grade control of a simulation segment.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GradeControl {
    /// Take the grade from the workout route at the current position.
    FromRoute,
    /// A fixed grade in percent.
    Fixed {
        /// Grade in percent.
        pct: f32,
    },
    /// A linear grade ramp over the segment.
    Ramp {
        /// Start grade in percent.
        from_pct: f32,
        /// End grade in percent.
        to_pct: f32,
    },
}

/// What the trainer is told to do during a segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Control {
    /// ERG: hold a target power.
    Power(PowerTarget),
    /// Simulation: resistance follows a grade.
    Grade(GradeControl),
    /// No target: the trainer is left as is.
    Free,
}

/// One workout segment.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Segment {
    /// Optional display label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// End criterion.
    pub end: End,
    /// Control mode.
    pub control: Control,
    /// Suggested cadence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cadence_rpm: Option<u16>,
    /// Message shown on screen while the segment runs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// A repeated group of segments.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RepeatBlock {
    /// Number of repetitions.
    pub repeat: u32,
    /// Segments repeated in order.
    pub segments: Vec<Segment>,
}

/// A segment or a repeat block.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SegmentNode {
    /// A group repeated several times.
    Repeat(RepeatBlock),
    /// A single segment.
    Single(Segment),
}

/// Kind of workout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkoutKind {
    /// Time based (ERG).
    Time,
    /// Distance based (simulation).
    Route,
    /// Both kinds of segments.
    Mixed,
}

/// Elevation profile: `(distance_m, elevation_m)` points.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Route {
    /// Sampling resolution in meters.
    pub resolution_m: f64,
    /// Points sorted by distance.
    pub points: Vec<(f64, f64)>,
}

impl Route {
    /// Check that the profile is usable.
    pub fn validate(&self) -> Result<(), WorkoutError> {
        if self.points.len() < 2 || self.points.windows(2).any(|w| w[1].0 <= w[0].0) {
            return Err(WorkoutError::InvalidRoute);
        }
        Ok(())
    }

    /// Total length in meters.
    pub fn total_distance_m(&self) -> f64 {
        self.points.last().map_or(0.0, |p| p.0)
    }

    /// Linearly interpolated elevation at a distance (clamped to the route).
    pub fn elevation_at(&self, d: f64) -> f64 {
        let (first, last) = match (self.points.first(), self.points.last()) {
            (Some(a), Some(b)) => (*a, *b),
            _ => return 0.0,
        };
        if d <= first.0 {
            return first.1;
        }
        if d >= last.0 {
            return last.1;
        }
        let i = self.points.partition_point(|p| p.0 <= d);
        let (a, b) = (self.points[i - 1], self.points[i]);
        a.1 + (b.1 - a.1) * (d - a.0) / (b.0 - a.0)
    }

    /// Average grade (percent) between two distances.
    pub fn average_grade(&self, from_m: f64, to_m: f64) -> f64 {
        let end = to_m.min(self.total_distance_m());
        let start = from_m.min(end - 1.0).max(0.0);
        if end - start < 1e-6 {
            return 0.0;
        }
        (self.elevation_at(end) - self.elevation_at(start)) / (end - start) * 100.0
    }

    /// Positive elevation gain in meters.
    pub fn elevation_gain_m(&self) -> f64 {
        self.points
            .windows(2)
            .map(|w| (w[1].1 - w[0].1).max(0.0))
            .sum()
    }

    /// Steepest climb and descent grades in percent as `(max_climb, max_descent)`.
    pub fn grade_extremes(&self) -> (f64, f64) {
        let grades = self
            .points
            .windows(2)
            .map(|w| (w[1].1 - w[0].1) / (w[1].0 - w[0].0) * 100.0);
        grades.fold((0.0, 0.0), |(mx, mn), g| (f64::max(mx, g), f64::min(mn, g)))
    }
}

/// A complete workout.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workout {
    /// File format version.
    pub schema_version: u32,
    /// Display name.
    pub name: String,
    /// Optional description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Kind of workout.
    pub kind: WorkoutKind,
    /// Elevation profile for route segments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route: Option<Route>,
    /// Segments in order.
    pub segments: Vec<SegmentNode>,
}

impl Workout {
    /// A one-segment route workout that follows the given elevation profile.
    pub fn from_route(name: impl Into<String>, route: Route) -> Self {
        let length = route.total_distance_m();
        Self {
            schema_version: SCHEMA_VERSION,
            name: name.into(),
            description: None,
            kind: WorkoutKind::Route,
            route: Some(route),
            segments: vec![SegmentNode::Single(Segment {
                label: Some("Circuit".into()),
                end: End::DistanceM(length),
                control: Control::Grade(GradeControl::FromRoute),
                cadence_rpm: None,
                message: None,
            })],
        }
    }

    /// Expand repeat blocks into a flat list of segments.
    pub fn flatten(&self) -> Vec<Segment> {
        let mut out = Vec::new();
        for node in &self.segments {
            match node {
                SegmentNode::Single(s) => out.push(s.clone()),
                SegmentNode::Repeat(r) => {
                    for _ in 0..r.repeat {
                        out.extend(r.segments.iter().cloned());
                    }
                }
            }
        }
        out
    }

    /// Check structural validity.
    pub fn validate(&self) -> Result<(), WorkoutError> {
        if self.segments.is_empty() {
            return Err(WorkoutError::Empty);
        }
        if let Some(r) = &self.route {
            r.validate()?;
        }
        for (i, node) in self.segments.iter().enumerate() {
            if let SegmentNode::Repeat(r) = node {
                if r.repeat == 0 || r.segments.is_empty() {
                    return Err(WorkoutError::InvalidRepeat(i));
                }
            }
        }
        for (i, s) in self.flatten().iter().enumerate() {
            let valid = match s.end {
                End::TimeS(t) => t > 0,
                End::DistanceM(d) => d > 0.0 && d.is_finite(),
            };
            if !valid {
                return Err(WorkoutError::InvalidEnd(i));
            }
            if matches!(s.control, Control::Grade(GradeControl::FromRoute)) && self.route.is_none()
            {
                return Err(WorkoutError::MissingRoute(i));
            }
        }
        Ok(())
    }

    /// Total planned time in seconds (time segments only).
    pub fn planned_time_s(&self) -> u32 {
        self.flatten()
            .iter()
            .map(|s| if let End::TimeS(t) = s.end { t } else { 0 })
            .sum()
    }

    /// Total planned distance in meters (distance segments only).
    pub fn planned_distance_m(&self) -> f64 {
        self.flatten()
            .iter()
            .map(|s| {
                if let End::DistanceM(d) = s.end {
                    d
                } else {
                    0.0
                }
            })
            .sum()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const SWEET_SPOT: &str = r#"{
      "schema_version": 1, "name": "Sweet Spot 3x8", "kind": "time",
      "segments": [
        { "label": "Warm-up", "end": { "time_s": 840 },
          "control": { "power": { "from_pct_ftp": 45, "to_pct_ftp": 70 } } },
        { "repeat": 3, "segments": [
          { "end": { "time_s": 480 }, "control": { "power": { "pct_ftp": 95 } }, "cadence_rpm": 90 },
          { "end": { "time_s": 240 }, "control": { "power": { "pct_ftp": 55 } } } ] },
        { "label": "Cool-down", "end": { "time_s": 720 },
          "control": { "power": { "from_pct_ftp": 60, "to_pct_ftp": 45 } } }
      ] }"#;

    #[test]
    fn parses_the_documented_time_workout() {
        let w: Workout = serde_json::from_str(SWEET_SPOT).unwrap();
        w.validate().unwrap();
        assert_eq!(w.flatten().len(), 8);
        assert_eq!(w.planned_time_s(), 840 + 3 * (480 + 240) + 720);
        let back: Workout = serde_json::from_str(&serde_json::to_string(&w).unwrap()).unwrap();
        assert_eq!(back, w);
    }

    #[test]
    fn parses_the_documented_route_workout() {
        let json = r#"{ "schema_version": 1, "name": "10 km rolling", "kind": "route",
          "route": { "resolution_m": 100, "points": [[0,100],[1000,100],[2000,115]] },
          "segments": [ { "end": { "distance_m": 2000 }, "control": { "grade": "from_route" } } ] }"#;
        let w: Workout = serde_json::from_str(json).unwrap();
        w.validate().unwrap();
        assert_eq!(w.planned_distance_m(), 2000.0);
    }

    #[test]
    fn rejects_unknown_fields_and_missing_route() {
        assert!(
            serde_json::from_str::<Workout>(
                r#"{"schema_version":1,"name":"x","kind":"time","segments":[],"oops":1}"#
            )
            .is_ok()
        );
        let seg = Segment {
            label: None,
            end: End::DistanceM(100.0),
            control: Control::Grade(GradeControl::FromRoute),
            cadence_rpm: None,
            message: None,
        };
        let w = Workout {
            schema_version: 1,
            name: "x".into(),
            description: None,
            kind: WorkoutKind::Route,
            route: None,
            segments: vec![SegmentNode::Single(seg)],
        };
        assert_eq!(w.validate(), Err(WorkoutError::MissingRoute(0)));
    }

    #[test]
    fn route_geometry() {
        let r = Route {
            resolution_m: 100.0,
            points: vec![(0.0, 100.0), (1000.0, 150.0), (2000.0, 120.0)],
        };
        assert_eq!(r.elevation_at(500.0), 125.0);
        assert!((r.average_grade(0.0, 1000.0) - 5.0).abs() < 1e-9);
        assert_eq!(r.elevation_gain_m(), 50.0);
        let (up, down) = r.grade_extremes();
        assert!((up - 5.0).abs() < 1e-9 && (down + 3.0).abs() < 1e-9);
    }

    #[test]
    fn power_targets_resolve() {
        assert_eq!(
            PowerTarget::steady_pct(95.0).resolve(200.0, 0.0),
            Some(190.0)
        );
        assert_eq!(
            PowerTarget::ramp_pct(50.0, 100.0).resolve(200.0, 0.5),
            Some(150.0)
        );
    }
}
