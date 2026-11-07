// SPDX-License-Identifier: MIT OR Apache-2.0
//! Workout execution engine.
//!
//! The engine is a deterministic state machine driven by explicit time deltas. It never talks to
//! a trainer: it returns [`TrainerCommand`]s that the caller forwards to a backend.
//!
//! * **Time segments** advance with the clock and drive ERG targets.
//! * **Distance segments** advance with the speed reported by the trainer and drive grade
//!   simulation, with a smoothed, rate-limited grade.

use crate::workout::{Control, End, GradeControl, Route, Segment, Workout, WorkoutError};

/// Execution state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Created, not started.
    Idle,
    /// Countdown before the first target.
    Countdown,
    /// Running.
    Running,
    /// Paused.
    Paused,
    /// All segments completed or stopped.
    Finished,
}

/// Command the engine asks the trainer backend to execute.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrainerCommand {
    /// Start or resume the trainer.
    Start,
    /// Pause the trainer.
    Pause,
    /// Stop the trainer.
    Stop,
    /// ERG: hold this power.
    SetTargetPower {
        /// Target power in watts.
        watts: u16,
    },
    /// Simulation: apply this grade.
    SetGrade {
        /// Grade in percent.
        grade_pct: f32,
    },
}

/// Event for the UI and the recorder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineEvent {
    /// Countdown began.
    CountdownStarted,
    /// First target is active.
    Started,
    /// Paused.
    Paused,
    /// Resumed.
    Resumed,
    /// A new segment became active.
    SegmentChanged {
        /// Index in the flattened segment list.
        index: usize,
    },
    /// The workout ended.
    Finished,
}

/// Telemetry input from the trainer.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Telemetry {
    /// Power in watts.
    pub power_w: Option<u16>,
    /// Cadence in rpm.
    pub cadence_rpm: Option<f32>,
    /// Heart rate in bpm.
    pub hr_bpm: Option<u8>,
    /// Speed in km/h.
    pub speed_kmh: Option<f32>,
}

/// Output of one engine call.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Step {
    /// Commands for the trainer, in order.
    pub commands: Vec<TrainerCommand>,
    /// Events that happened.
    pub events: Vec<EngineEvent>,
}

/// Engine settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EngineConfig {
    /// Rider FTP in watts.
    pub ftp_w: u16,
    /// Countdown length in milliseconds.
    pub countdown_ms: u32,
    /// Lowest power the engine will ever command.
    pub min_power_w: u16,
    /// Highest power the engine will ever command.
    pub max_power_w: u16,
    /// Power commanded while paused.
    pub pause_power_w: u16,
    /// Lowest grade commanded (percent).
    pub grade_min_pct: f32,
    /// Highest grade commanded (percent).
    pub grade_max_pct: f32,
    /// Look-ahead window used to average the grade (meters).
    pub grade_window_m: f64,
    /// Maximum grade change per second (percentage points).
    pub grade_rate_pct_per_s: f32,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            ftp_w: 200,
            countdown_ms: 3000,
            min_power_w: 30,
            max_power_w: 1000,
            pause_power_w: 50,
            grade_min_pct: -10.0,
            grade_max_pct: 20.0,
            grade_window_m: 30.0,
            grade_rate_pct_per_s: 1.0,
        }
    }
}

/// A read-only view of the engine for the UI.
#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    /// Current status.
    pub status: Status,
    /// Active segment index.
    pub segment_index: usize,
    /// Number of segments.
    pub segment_count: usize,
    /// Running time in seconds.
    pub elapsed_s: f64,
    /// Covered distance in meters.
    pub distance_m: f64,
    /// Position along the route in meters.
    pub route_pos_m: f64,
    /// Progress inside the active segment (0..1).
    pub segment_progress: f32,
    /// Power target currently commanded.
    pub target_power_w: Option<u16>,
    /// Grade currently commanded.
    pub grade_pct: Option<f32>,
    /// Seconds left in the active time segment.
    pub segment_remaining_s: Option<f64>,
    /// Whole seconds left in the countdown (0 outside the countdown).
    pub countdown_s: u32,
}

/// The workout engine.
#[derive(Debug, Clone)]
pub struct Engine {
    cfg: EngineConfig,
    segments: Vec<Segment>,
    route: Option<Route>,
    status: Status,
    idx: usize,
    seg_elapsed_ms: u64,
    seg_distance_m: f64,
    route_pos_m: f64,
    total_ms: u64,
    total_distance_m: f64,
    countdown_left_ms: u32,
    intensity_pct: f32,
    difficulty: f32,
    last_power: Option<u16>,
    last_grade_cmd: Option<f32>,
    grade: f32,
    grade_ready: bool,
}

impl Engine {
    /// Build an engine for a validated workout.
    pub fn new(workout: &Workout, cfg: EngineConfig) -> Result<Self, WorkoutError> {
        workout.validate()?;
        Ok(Self {
            cfg,
            segments: workout.flatten(),
            route: workout.route.clone(),
            status: Status::Idle,
            idx: 0,
            seg_elapsed_ms: 0,
            seg_distance_m: 0.0,
            route_pos_m: 0.0,
            total_ms: 0,
            total_distance_m: 0.0,
            countdown_left_ms: 0,
            intensity_pct: 0.0,
            difficulty: 1.0,
            last_power: None,
            last_grade_cmd: None,
            grade: 0.0,
            grade_ready: false,
        })
    }

    /// Current status.
    pub fn status(&self) -> Status {
        self.status
    }

    /// Flattened segments (what the UI should draw).
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }

    /// Begin the countdown.
    pub fn start(&mut self) -> Step {
        let mut out = Step::default();
        if self.status == Status::Idle {
            self.status = Status::Countdown;
            self.countdown_left_ms = self.cfg.countdown_ms;
            out.events.push(EngineEvent::CountdownStarted);
            if self.cfg.countdown_ms == 0 {
                self.begin_running(&mut out);
            }
        }
        out
    }

    /// Pause the session.
    pub fn pause(&mut self) -> Step {
        let mut out = Step::default();
        if self.status == Status::Running {
            self.status = Status::Paused;
            out.events.push(EngineEvent::Paused);
            out.commands.push(TrainerCommand::SetTargetPower {
                watts: self.cfg.pause_power_w,
            });
            self.last_power = Some(self.cfg.pause_power_w);
        }
        out
    }

    /// Resume after a pause (a short countdown runs first).
    pub fn resume(&mut self) -> Step {
        let mut out = Step::default();
        if self.status == Status::Paused {
            self.status = Status::Countdown;
            self.countdown_left_ms = self.cfg.countdown_ms;
            self.last_power = None;
            self.last_grade_cmd = None;
            out.events.push(EngineEvent::Resumed);
            if self.cfg.countdown_ms == 0 {
                self.status = Status::Running;
                self.emit_control(&mut out, 0);
            }
        }
        out
    }

    /// Stop the session.
    pub fn stop(&mut self) -> Step {
        let mut out = Step::default();
        if self.status != Status::Finished {
            self.status = Status::Finished;
            out.commands.push(TrainerCommand::Stop);
            out.events.push(EngineEvent::Finished);
        }
        out
    }

    /// Jump to the next segment.
    pub fn skip_segment(&mut self) -> Step {
        let mut out = Step::default();
        if matches!(self.status, Status::Running | Status::Paused) {
            self.advance_segment(&mut out);
            if self.status == Status::Running {
                self.emit_control(&mut out, 0);
            }
        }
        out
    }

    /// Shift every power target by `delta_pct` percent (clamped to -50..=50 in total).
    pub fn adjust_intensity(&mut self, delta_pct: f32) -> Step {
        self.intensity_pct = (self.intensity_pct + delta_pct).clamp(-50.0, 50.0);
        let mut out = Step::default();
        if self.status == Status::Running {
            self.emit_control(&mut out, 0);
        }
        out
    }

    /// Set the grade scale (`1.0` = 100 %).
    pub fn set_difficulty(&mut self, scale: f32) {
        self.difficulty = scale.clamp(0.0, 1.5);
    }

    /// Current intensity offset in percent.
    pub fn intensity_pct(&self) -> f32 {
        self.intensity_pct
    }

    /// Current difficulty scale.
    pub fn difficulty(&self) -> f32 {
        self.difficulty
    }

    /// Advance the engine by `dt_ms` using the latest telemetry.
    pub fn step(&mut self, dt_ms: u32, telemetry: Option<&Telemetry>) -> Step {
        let mut out = Step::default();
        match self.status {
            Status::Countdown => {
                self.countdown_left_ms = self.countdown_left_ms.saturating_sub(dt_ms);
                if self.countdown_left_ms == 0 {
                    if self.total_ms == 0 {
                        self.begin_running(&mut out);
                    } else {
                        self.status = Status::Running;
                        self.emit_control(&mut out, 0);
                    }
                }
            }
            Status::Running => {
                self.total_ms += u64::from(dt_ms);
                let dist = telemetry
                    .and_then(|t| t.speed_kmh)
                    .map_or(0.0, |v| f64::from(v) / 3.6 * f64::from(dt_ms) / 1000.0);
                self.total_distance_m += dist;
                self.seg_elapsed_ms += u64::from(dt_ms);
                let Some(seg) = self.segments.get(self.idx) else {
                    return self.finish(out);
                };
                let done = match seg.end {
                    End::TimeS(t) => self.seg_elapsed_ms >= u64::from(t) * 1000,
                    End::DistanceM(d) => {
                        self.seg_distance_m += dist;
                        self.route_pos_m += dist;
                        self.seg_distance_m >= d
                    }
                };
                if done {
                    self.advance_segment(&mut out);
                }
                if self.status == Status::Running {
                    self.emit_control(&mut out, dt_ms);
                }
            }
            Status::Idle | Status::Paused | Status::Finished => {}
        }
        out
    }

    /// Read-only view of the current state.
    pub fn snapshot(&self) -> Snapshot {
        let remaining = self.segments.get(self.idx).and_then(|s| match s.end {
            End::TimeS(t) => Some((f64::from(t) - self.seg_elapsed_ms as f64 / 1000.0).max(0.0)),
            End::DistanceM(_) => None,
        });
        Snapshot {
            status: self.status,
            segment_index: self.idx.min(self.segments.len().saturating_sub(1)),
            segment_count: self.segments.len(),
            elapsed_s: self.total_ms as f64 / 1000.0,
            distance_m: self.total_distance_m,
            route_pos_m: self.route_pos_m,
            segment_progress: self
                .segments
                .get(self.idx)
                .map_or(1.0, |s| self.progress(s)),
            target_power_w: self.last_power,
            grade_pct: self
                .last_grade_cmd
                .or(self.grade_ready.then_some(self.grade)),
            segment_remaining_s: remaining,
            countdown_s: if self.status == Status::Countdown {
                self.countdown_left_ms.div_ceil(1000)
            } else {
                0
            },
        }
    }

    fn begin_running(&mut self, out: &mut Step) {
        self.status = Status::Running;
        out.events.push(EngineEvent::Started);
        out.events
            .push(EngineEvent::SegmentChanged { index: self.idx });
        out.commands.push(TrainerCommand::Start);
        self.emit_control(out, 0);
    }

    fn finish(&mut self, mut out: Step) -> Step {
        if self.status != Status::Finished {
            self.status = Status::Finished;
            out.commands.push(TrainerCommand::Stop);
            out.events.push(EngineEvent::Finished);
        }
        out
    }

    fn advance_segment(&mut self, out: &mut Step) {
        self.idx += 1;
        self.seg_elapsed_ms = 0;
        self.seg_distance_m = 0.0;
        self.last_power = None;
        self.last_grade_cmd = None;
        self.grade_ready = false;
        if self.idx >= self.segments.len() {
            let finished = self.finish(Step::default());
            out.commands.extend(finished.commands);
            out.events.extend(finished.events);
        } else {
            out.events
                .push(EngineEvent::SegmentChanged { index: self.idx });
        }
    }

    fn progress(&self, seg: &Segment) -> f32 {
        let p = match seg.end {
            End::TimeS(t) => self.seg_elapsed_ms as f64 / (f64::from(t) * 1000.0),
            End::DistanceM(d) => self.seg_distance_m / d,
        };
        p.clamp(0.0, 1.0) as f32
    }

    fn emit_control(&mut self, out: &mut Step, dt_ms: u32) {
        let Some(seg) = self.segments.get(self.idx).cloned() else {
            return;
        };
        let progress = self.progress(&seg);
        match &seg.control {
            Control::Power(target) => {
                if let Some(w) = target.resolve(f32::from(self.cfg.ftp_w), progress) {
                    let adjusted = w * (1.0 + self.intensity_pct / 100.0);
                    let watts = adjusted.round().clamp(
                        f32::from(self.cfg.min_power_w),
                        f32::from(self.cfg.max_power_w),
                    ) as u16;
                    if self.last_power != Some(watts) {
                        self.last_power = Some(watts);
                        out.commands.push(TrainerCommand::SetTargetPower { watts });
                    }
                }
            }
            Control::Grade(control) => {
                let raw = match (control, &self.route) {
                    (GradeControl::FromRoute, Some(route)) => route
                        .average_grade(self.route_pos_m, self.route_pos_m + self.cfg.grade_window_m)
                        as f32,
                    (GradeControl::Fixed { pct }, _) => *pct,
                    (GradeControl::Ramp { from_pct, to_pct }, _) => {
                        from_pct + (to_pct - from_pct) * progress
                    }
                    (GradeControl::FromRoute, None) => 0.0,
                };
                let target =
                    (raw * self.difficulty).clamp(self.cfg.grade_min_pct, self.cfg.grade_max_pct);
                if self.grade_ready {
                    let max_delta = self.cfg.grade_rate_pct_per_s * dt_ms as f32 / 1000.0;
                    self.grade += (target - self.grade).clamp(-max_delta, max_delta);
                } else {
                    self.grade = target;
                    self.grade_ready = true;
                }
                if self
                    .last_grade_cmd
                    .is_none_or(|g| (g - self.grade).abs() >= 0.1)
                {
                    self.last_grade_cmd = Some(self.grade);
                    out.commands.push(TrainerCommand::SetGrade {
                        grade_pct: self.grade,
                    });
                }
            }
            Control::Free => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::workout::{PowerTarget, SegmentNode, WorkoutKind};

    fn seg(end: End, control: Control) -> SegmentNode {
        SegmentNode::Single(Segment {
            label: None,
            end,
            control,
            cadence_rpm: None,
            message: None,
        })
    }

    fn workout(segments: Vec<SegmentNode>, route: Option<Route>) -> Workout {
        Workout {
            schema_version: 1,
            name: "t".into(),
            description: None,
            kind: WorkoutKind::Time,
            route,
            segments,
        }
    }

    fn cfg() -> EngineConfig {
        EngineConfig {
            ftp_w: 200,
            ..Default::default()
        }
    }

    fn run(engine: &mut Engine, seconds: u32, tel: Option<Telemetry>) -> Vec<TrainerCommand> {
        let mut cmds = Vec::new();
        for _ in 0..seconds {
            cmds.extend(engine.step(1000, tel.as_ref()).commands);
        }
        cmds
    }

    #[test]
    fn erg_workout_runs_to_completion_with_expected_targets() {
        let w = workout(
            vec![
                seg(
                    End::TimeS(60),
                    Control::Power(PowerTarget::steady_pct(50.0)),
                ),
                seg(
                    End::TimeS(120),
                    Control::Power(PowerTarget::steady_pct(100.0)),
                ),
            ],
            None,
        );
        let mut e = Engine::new(&w, cfg()).unwrap();
        e.start();
        let cmds = run(&mut e, 3 + 180 + 1, None);
        assert_eq!(cmds.first(), Some(&TrainerCommand::Start));
        let targets: Vec<u16> = cmds
            .iter()
            .filter_map(|c| {
                if let TrainerCommand::SetTargetPower { watts } = c {
                    Some(*watts)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(targets, vec![100, 200]);
        assert_eq!(cmds.last(), Some(&TrainerCommand::Stop));
        assert_eq!(e.status(), Status::Finished);
    }

    #[test]
    fn ramp_sends_increasing_targets() {
        let w = workout(
            vec![seg(
                End::TimeS(10),
                Control::Power(PowerTarget {
                    from_watts: Some(100.0),
                    to_watts: Some(200.0),
                    ..Default::default()
                }),
            )],
            None,
        );
        let mut e = Engine::new(
            &w,
            EngineConfig {
                countdown_ms: 0,
                ..cfg()
            },
        )
        .unwrap();
        e.start();
        let targets: Vec<u16> = run(&mut e, 10, None)
            .iter()
            .filter_map(|c| {
                if let TrainerCommand::SetTargetPower { watts } = c {
                    Some(*watts)
                } else {
                    None
                }
            })
            .collect();
        assert!(
            targets.len() >= 9 && targets.windows(2).all(|p| p[0] < p[1]),
            "{targets:?}"
        );
    }

    #[test]
    fn pause_freezes_progress_and_resume_counts_down() {
        let w = workout(
            vec![seg(
                End::TimeS(100),
                Control::Power(PowerTarget::steady_pct(100.0)),
            )],
            None,
        );
        let mut e = Engine::new(
            &w,
            EngineConfig {
                countdown_ms: 0,
                ..cfg()
            },
        )
        .unwrap();
        e.start();
        run(&mut e, 10, None);
        let pause = e.pause();
        assert_eq!(
            pause.commands,
            vec![TrainerCommand::SetTargetPower { watts: 50 }]
        );
        run(&mut e, 30, None);
        assert_eq!(e.snapshot().elapsed_s, 10.0);
        e.resume();
        assert_eq!(e.status(), Status::Running);
    }

    #[test]
    fn intensity_adjustment_changes_the_target() {
        let w = workout(
            vec![seg(
                End::TimeS(100),
                Control::Power(PowerTarget::steady_pct(100.0)),
            )],
            None,
        );
        let mut e = Engine::new(
            &w,
            EngineConfig {
                countdown_ms: 0,
                ..cfg()
            },
        )
        .unwrap();
        e.start();
        let step = e.adjust_intensity(10.0);
        assert_eq!(
            step.commands,
            vec![TrainerCommand::SetTargetPower { watts: 220 }]
        );
    }

    #[test]
    fn skip_segment_moves_on() {
        let w = workout(
            vec![
                seg(
                    End::TimeS(100),
                    Control::Power(PowerTarget::steady_pct(50.0)),
                ),
                seg(
                    End::TimeS(100),
                    Control::Power(PowerTarget::steady_pct(90.0)),
                ),
            ],
            None,
        );
        let mut e = Engine::new(
            &w,
            EngineConfig {
                countdown_ms: 0,
                ..cfg()
            },
        )
        .unwrap();
        e.start();
        let step = e.skip_segment();
        assert_eq!(
            step.commands,
            vec![TrainerCommand::SetTargetPower { watts: 180 }]
        );
        assert_eq!(e.snapshot().segment_index, 1);
    }

    fn route_workout(points: Vec<(f64, f64)>, length: f64) -> Workout {
        let mut w = workout(
            vec![seg(
                End::DistanceM(length),
                Control::Grade(GradeControl::FromRoute),
            )],
            Some(Route {
                resolution_m: 100.0,
                points,
            }),
        );
        w.kind = WorkoutKind::Route;
        w
    }

    #[test]
    fn route_workout_finishes_after_the_distance_is_covered() {
        let w = route_workout(
            vec![(0.0, 100.0), (5000.0, 150.0), (10_000.0, 150.0)],
            10_000.0,
        );
        let mut e = Engine::new(
            &w,
            EngineConfig {
                countdown_ms: 0,
                ..cfg()
            },
        )
        .unwrap();
        let tel = Telemetry {
            speed_kmh: Some(36.0),
            ..Default::default()
        };
        let mut seconds = 0;
        let mut grades = Vec::new();
        for c in e.start().commands {
            if let TrainerCommand::SetGrade { grade_pct } = c {
                grades.push(grade_pct);
            }
        }
        while e.status() != Status::Finished && seconds < 2000 {
            for c in e.step(1000, Some(&tel)).commands {
                if let TrainerCommand::SetGrade { grade_pct } = c {
                    grades.push(grade_pct);
                }
            }
            seconds += 1;
        }
        assert!((1000..=1002).contains(&seconds), "took {seconds}s");
        assert!((grades[0] - 1.0).abs() < 0.2, "first grade {}", grades[0]);
        assert!(grades.iter().any(|g| g.abs() < 0.2));
        assert!((e.snapshot().distance_m - 10_000.0).abs() < 20.0);
    }

    #[test]
    fn grade_changes_are_rate_limited() {
        let w = route_workout(
            vec![(0.0, 0.0), (500.0, 0.0), (600.0, 10.0), (3000.0, 10.0)],
            3000.0,
        );
        let mut e = Engine::new(
            &w,
            EngineConfig {
                countdown_ms: 0,
                ..cfg()
            },
        )
        .unwrap();
        e.start();
        let tel = Telemetry {
            speed_kmh: Some(36.0),
            ..Default::default()
        };
        let mut last: Option<f32> = None;
        let mut max_jump = 0.0_f32;
        let mut peak = 0.0_f32;
        for _ in 0..120 {
            for c in e.step(1000, Some(&tel)).commands {
                if let TrainerCommand::SetGrade { grade_pct } = c {
                    if let Some(l) = last {
                        max_jump = max_jump.max((grade_pct - l).abs());
                    }
                    last = Some(grade_pct);
                    peak = peak.max(grade_pct);
                }
            }
        }
        assert!(max_jump <= 1.05, "jump {max_jump}");
        assert!(peak > 5.0, "peak {peak}");
    }

    #[test]
    fn difficulty_scales_and_clamps_the_grade() {
        let mut w = route_workout(vec![(0.0, 0.0), (1000.0, 100.0)], 1000.0);
        w.kind = WorkoutKind::Route;
        let mut e = Engine::new(
            &w,
            EngineConfig {
                countdown_ms: 0,
                ..cfg()
            },
        )
        .unwrap();
        e.set_difficulty(0.5);
        let step = e.start();
        let grade = step.commands.iter().find_map(|c| {
            if let TrainerCommand::SetGrade { grade_pct } = c {
                Some(*grade_pct)
            } else {
                None
            }
        });
        assert_eq!(grade, Some(5.0));
    }

    #[test]
    fn route_progress_stalls_without_speed() {
        let w = route_workout(vec![(0.0, 0.0), (1000.0, 0.0)], 1000.0);
        let mut e = Engine::new(
            &w,
            EngineConfig {
                countdown_ms: 0,
                ..cfg()
            },
        )
        .unwrap();
        e.start();
        run(&mut e, 60, None);
        assert_eq!(e.snapshot().distance_m, 0.0);
        assert_eq!(e.status(), Status::Running);
    }
}
