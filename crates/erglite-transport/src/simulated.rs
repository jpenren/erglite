// SPDX-License-Identifier: MIT OR Apache-2.0
//! A deterministic simulated trainer for tests, demos and CI.

use crate::{ConnectionState, TrainerBackend, TrainerCapabilities, TransportError};
use async_trait::async_trait;
use erglite_core::physics::{RiderParams, speed_for_power};
use erglite_core::{Telemetry, TrainerCommand};

/// Simulated rider and trainer settings.
#[derive(Debug, Clone, PartialEq)]
pub struct SimConfig {
    /// Power the simulated rider produces in simulation (grade) mode.
    pub rider_power_w: f64,
    /// Rider and bike physics.
    pub rider: RiderParams,
    /// Cadence reported.
    pub cadence_rpm: f32,
    /// Optional link drop: `(start_ms, duration_ms)` of simulated time.
    pub drop_window_ms: Option<(u64, u64)>,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            rider_power_w: 190.0,
            rider: RiderParams::default(),
            cadence_rpm: 90.0,
            drop_window_ms: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Mode {
    Idle,
    Erg(f64),
    Sim(f64),
}

/// A trainer that follows the same physics the estimator uses.
#[derive(Debug, Clone)]
pub struct SimulatedTrainer {
    cfg: SimConfig,
    mode: Mode,
    connected: bool,
    running: bool,
    clock_ms: u64,
    power: f64,
    speed_mps: f64,
    hr: f64,
    latest: Option<Telemetry>,
    commands: Vec<TrainerCommand>,
}

impl SimulatedTrainer {
    /// Create a trainer with the given settings.
    pub fn new(cfg: SimConfig) -> Self {
        Self {
            cfg,
            mode: Mode::Idle,
            connected: false,
            running: false,
            clock_ms: 0,
            power: 0.0,
            speed_mps: 0.0,
            hr: 62.0,
            latest: None,
            commands: Vec::new(),
        }
    }

    /// Every command received so far (for assertions).
    pub fn received(&self) -> &[TrainerCommand] {
        &self.commands
    }

    fn link_up(&self) -> bool {
        match self.cfg.drop_window_ms {
            Some((start, len)) => !(self.clock_ms >= start && self.clock_ms < start + len),
            None => true,
        }
    }
}

#[async_trait]
impl TrainerBackend for SimulatedTrainer {
    async fn connect(&mut self) -> Result<TrainerCapabilities, TransportError> {
        self.connected = true;
        Ok(TrainerCapabilities {
            name: "Simulated trainer".into(),
            supports_erg: true,
            supports_simulation: true,
            supports_resistance: false,
            min_power_w: Some(0),
            max_power_w: Some(2000),
            reports_cadence: true,
            reports_power: true,
            reports_heart_rate: true,
        })
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.connected = false;
        self.latest = None;
        Ok(())
    }

    async fn apply(&mut self, command: TrainerCommand) -> Result<(), TransportError> {
        if !self.connected || !self.link_up() {
            return Err(TransportError::NotConnected);
        }
        self.commands.push(command);
        match command {
            TrainerCommand::Start => self.running = true,
            TrainerCommand::Pause | TrainerCommand::Stop => {
                self.running = false;
                self.mode = Mode::Idle;
            }
            TrainerCommand::SetTargetPower { watts } => self.mode = Mode::Erg(f64::from(watts)),
            TrainerCommand::SetGrade { grade_pct } => self.mode = Mode::Sim(f64::from(grade_pct)),
        }
        Ok(())
    }

    fn latest(&self) -> Option<Telemetry> {
        if self.connected && self.link_up() {
            self.latest
        } else {
            None
        }
    }

    fn connection(&self) -> ConnectionState {
        if self.connected && self.link_up() {
            ConnectionState::Connected
        } else {
            ConnectionState::Disconnected
        }
    }

    fn advance(&mut self, dt_ms: u32) {
        let dt = f64::from(dt_ms) / 1000.0;
        self.clock_ms += u64::from(dt_ms);
        let t = self.clock_ms as f64 / 1000.0;
        let wobble = (t * 0.37).sin() * 3.0;
        let (target_power, grade) = match self.mode {
            Mode::Erg(w) => (w, 0.0),
            Mode::Sim(g) => (self.cfg.rider_power_w + wobble, g),
            Mode::Idle => (
                if self.running {
                    self.cfg.rider_power_w
                } else {
                    0.0
                },
                0.0,
            ),
        };
        self.power += (target_power - self.power) * (1.0 - (-dt / 2.0).exp());
        let target_speed = speed_for_power(self.power, grade, &self.cfg.rider);
        self.speed_mps += (target_speed - self.speed_mps) * (1.0 - (-dt / 3.0).exp());
        let hr_target = 70.0 + self.power * 0.38;
        self.hr += (hr_target - self.hr) * (1.0 - (-dt / 25.0).exp());
        self.latest = Some(Telemetry {
            power_w: Some(self.power.round().max(0.0) as u16),
            cadence_rpm: Some(if self.power > 5.0 {
                self.cfg.cadence_rpm
            } else {
                0.0
            }),
            hr_bpm: Some(self.hr.round().clamp(40.0, 200.0) as u8),
            speed_kmh: Some((self.speed_mps * 3.6) as f32),
        });
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn erg_power_converges_to_the_target() {
        let mut t = SimulatedTrainer::new(SimConfig::default());
        t.connect().await.unwrap();
        t.apply(TrainerCommand::Start).await.unwrap();
        t.apply(TrainerCommand::SetTargetPower { watts: 250 })
            .await
            .unwrap();
        for _ in 0..30 {
            t.advance(1000);
        }
        assert_eq!(t.latest().unwrap().power_w, Some(250));
    }

    #[tokio::test]
    async fn climbing_slows_the_simulated_rider() {
        let mut t = SimulatedTrainer::new(SimConfig::default());
        t.connect().await.unwrap();
        t.apply(TrainerCommand::Start).await.unwrap();
        t.apply(TrainerCommand::SetGrade { grade_pct: 0.0 })
            .await
            .unwrap();
        for _ in 0..40 {
            t.advance(1000);
        }
        let flat = t.latest().unwrap().speed_kmh.unwrap();
        t.apply(TrainerCommand::SetGrade { grade_pct: 8.0 })
            .await
            .unwrap();
        for _ in 0..40 {
            t.advance(1000);
        }
        assert!(t.latest().unwrap().speed_kmh.unwrap() < flat * 0.6);
    }

    #[tokio::test]
    async fn link_drop_hides_telemetry_and_rejects_commands() {
        let mut t = SimulatedTrainer::new(SimConfig {
            drop_window_ms: Some((5000, 10_000)),
            ..Default::default()
        });
        t.connect().await.unwrap();
        for _ in 0..6 {
            t.advance(1000);
        }
        assert_eq!(t.connection(), ConnectionState::Disconnected);
        assert!(t.latest().is_none());
        assert!(t.apply(TrainerCommand::Start).await.is_err());
        for _ in 0..10 {
            t.advance(1000);
        }
        assert_eq!(t.connection(), ConnectionState::Connected);
    }
}
