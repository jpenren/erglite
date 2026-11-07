// SPDX-License-Identifier: MIT OR Apache-2.0
//! Trainer backends. The workout engine and the UI only know the [`TrainerBackend`] trait.
//!
//! Backends are polled: the runtime calls [`TrainerBackend::advance`] every tick (a no-op for
//! real hardware, the simulation step for [`SimulatedTrainer`]) and reads
//! [`TrainerBackend::latest`].
#![forbid(unsafe_code)]

#[cfg(feature = "ble")]
pub mod ble;
mod simulated;

pub use simulated::{SimConfig, SimulatedTrainer};

/// FTMS simulation coefficients from the rider profile (rolling resistance and `0.5 * rho * CdA`).
#[cfg(feature = "ble")]
pub fn ftms_simulation_defaults(crr: f32, cw_kg_m: f32) -> erglite_ftms::SimulationParams {
    erglite_ftms::SimulationParams {
        crr,
        cw_kg_m,
        ..Default::default()
    }
}

use async_trait::async_trait;
use erglite_core::{Telemetry, TrainerCommand};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Transport errors.
#[derive(Debug, Error)]
pub enum TransportError {
    /// The trainer is not connected.
    #[error("trainer not connected")]
    NotConnected,
    /// The trainer rejected a command.
    #[error("trainer rejected the command: {0}")]
    Rejected(String),
    /// Lower-level Bluetooth or USB failure.
    #[error("transport failure: {0}")]
    Io(String),
}

/// Connection state of a backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    /// Not connected.
    Disconnected,
    /// Connecting.
    Connecting,
    /// Connected and ready.
    Connected,
}

/// What a connected trainer can do. The UI shows only what is supported.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TrainerCapabilities {
    /// Device name.
    pub name: String,
    /// Target power (ERG) is supported.
    pub supports_erg: bool,
    /// Grade simulation is supported.
    pub supports_simulation: bool,
    /// Resistance level control is supported.
    pub supports_resistance: bool,
    /// Minimum target power.
    pub min_power_w: Option<u16>,
    /// Maximum target power.
    pub max_power_w: Option<u16>,
    /// Cadence is reported.
    pub reports_cadence: bool,
    /// Power is reported.
    pub reports_power: bool,
    /// Heart rate is reported.
    pub reports_heart_rate: bool,
}

/// A discovered Bluetooth device.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// Stable id on this machine.
    pub id: String,
    /// Advertised name.
    pub name: String,
    /// `trainer`, `heart_rate`, `power` or `cadence`.
    pub role: String,
    /// Signal strength in dBm when known.
    pub rssi: Option<i16>,
}

/// A trainer backend.
#[async_trait]
pub trait TrainerBackend: Send {
    /// Connect and negotiate capabilities.
    async fn connect(&mut self) -> Result<TrainerCapabilities, TransportError>;
    /// Disconnect.
    async fn disconnect(&mut self) -> Result<(), TransportError>;
    /// Execute an engine command.
    async fn apply(&mut self, command: TrainerCommand) -> Result<(), TransportError>;
    /// Latest telemetry, `None` when nothing has arrived or the link is down.
    fn latest(&self) -> Option<Telemetry>;
    /// Current connection state.
    fn connection(&self) -> ConnectionState;
    /// Advance simulated time. Real backends ignore it.
    fn advance(&mut self, _dt_ms: u32) {}
}

#[async_trait]
impl<T: TrainerBackend + ?Sized> TrainerBackend for Box<T> {
    async fn connect(&mut self) -> Result<TrainerCapabilities, TransportError> {
        (**self).connect().await
    }
    async fn disconnect(&mut self) -> Result<(), TransportError> {
        (**self).disconnect().await
    }
    async fn apply(&mut self, command: TrainerCommand) -> Result<(), TransportError> {
        (**self).apply(command).await
    }
    fn latest(&self) -> Option<Telemetry> {
        (**self).latest()
    }
    fn connection(&self) -> ConnectionState {
        (**self).connection()
    }
    fn advance(&mut self, dt_ms: u32) {
        (**self).advance(dt_ms);
    }
}
