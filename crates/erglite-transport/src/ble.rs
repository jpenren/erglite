// SPDX-License-Identifier: MIT OR Apache-2.0
//! Bluetooth LE backend for FTMS trainers, built on `btleplug`.
//!
//! HW-VERIFY: this backend compiles but has **not** been validated against a real ZPro yet.
//! Run `erglite-cli doctor` (work package WP4) and follow `docs/hardware-testing.md` first.

use crate::{ConnectionState, DeviceInfo, TrainerBackend, TrainerCapabilities, TransportError};
use async_trait::async_trait;
use btleplug::api::{
    Central, Characteristic, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::Manager;
pub use btleplug::platform::Peripheral;
use erglite_core::{Telemetry, TrainerCommand};
use erglite_ftms::{self as ftms, Command, SimulationParams, uuid16 as u16s};
use futures::StreamExt;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

const HEART_RATE_SERVICE: u16 = 0x180D;
const CYCLING_POWER_SERVICE: u16 = 0x1818;
const CSC_SERVICE: u16 = 0x1816;

fn uuid16(short: u16) -> Uuid {
    Uuid::from_u128(0x0000_0000_0000_1000_8000_0080_5f9b_34fb | (u128::from(short) << 96))
}

fn io<E: std::fmt::Display>(e: E) -> TransportError {
    TransportError::Io(e.to_string())
}

/// Scan for nearby fitness devices for `timeout`.
pub async fn scan(timeout: Duration) -> Result<Vec<(DeviceInfo, Peripheral)>, TransportError> {
    let manager = Manager::new().await.map_err(io)?;
    let adapter = manager
        .adapters()
        .await
        .map_err(io)?
        .into_iter()
        .next()
        .ok_or_else(|| TransportError::Io("no Bluetooth adapter found".into()))?;
    adapter
        .start_scan(ScanFilter::default())
        .await
        .map_err(io)?;
    tokio::time::sleep(timeout).await;
    let mut found = Vec::new();
    for p in adapter.peripherals().await.map_err(io)? {
        let Some(props) = p.properties().await.map_err(io)? else {
            continue;
        };
        let has = |s: u16| props.services.contains(&uuid16(s));
        let role = if has(u16s::SERVICE) {
            "trainer"
        } else if has(HEART_RATE_SERVICE) {
            "heart_rate"
        } else if has(CYCLING_POWER_SERVICE) {
            "power"
        } else if has(CSC_SERVICE) {
            "cadence"
        } else {
            continue;
        };
        found.push((
            DeviceInfo {
                id: p.id().to_string(),
                name: props.local_name.unwrap_or_else(|| "Unknown device".into()),
                role: role.into(),
                rssi: props.rssi,
            },
            p,
        ));
    }
    let _ = adapter.stop_scan().await;
    Ok(found)
}

#[derive(Default)]
struct Shared {
    latest: Option<Telemetry>,
    connected: bool,
}

/// FTMS trainer over Bluetooth LE.
pub struct BleFtmsBackend {
    peripheral: Peripheral,
    control_point: Option<Characteristic>,
    shared: Arc<Mutex<Shared>>,
    sim: SimulationParams,
    control_requested: bool,
}

impl BleFtmsBackend {
    /// Wrap a peripheral returned by [`scan`]. `sim` carries the rolling and wind coefficients.
    pub fn new(peripheral: Peripheral, sim: SimulationParams) -> Self {
        Self {
            peripheral,
            control_point: None,
            shared: Arc::default(),
            sim,
            control_requested: false,
        }
    }

    async fn write(&self, cmd: Command) -> Result<(), TransportError> {
        let cp = self
            .control_point
            .as_ref()
            .ok_or(TransportError::NotConnected)?;
        self.peripheral
            .write(cp, &cmd.encode(), WriteType::WithResponse)
            .await
            .map_err(io)
    }
}

#[async_trait]
impl TrainerBackend for BleFtmsBackend {
    async fn connect(&mut self) -> Result<TrainerCapabilities, TransportError> {
        self.peripheral.connect().await.map_err(io)?;
        self.peripheral.discover_services().await.map_err(io)?;
        let chars = self.peripheral.characteristics();
        let find = |short: u16| chars.iter().find(|c| c.uuid == uuid16(short)).cloned();
        let cp = find(u16s::CONTROL_POINT)
            .ok_or_else(|| TransportError::Io("no FTMS control point".into()))?;
        let features = match find(u16s::FEATURE) {
            Some(c) => ftms::parse_machine_features(&self.peripheral.read(&c).await.map_err(io)?)
                .map_err(io)?,
            None => ftms::MachineFeatures {
                machine: 0,
                target_settings: 0,
            },
        };
        let range = match find(u16s::POWER_RANGE) {
            Some(c) => ftms::parse_power_range(&self.peripheral.read(&c).await.map_err(io)?).ok(),
            None => None,
        };
        for short in [
            u16s::INDOOR_BIKE_DATA,
            u16s::CONTROL_POINT,
            u16s::MACHINE_STATUS,
        ] {
            if let Some(c) = find(short) {
                self.peripheral.subscribe(&c).await.map_err(io)?;
            }
        }
        let mut stream = self.peripheral.notifications().await.map_err(io)?;
        let shared = self.shared.clone();
        if let Ok(mut s) = shared.lock() {
            s.connected = true;
        }
        tokio::spawn(async move {
            while let Some(n) = stream.next().await {
                if n.uuid == uuid16(u16s::INDOOR_BIKE_DATA) {
                    if let Ok(d) = ftms::parse_indoor_bike_data(&n.value) {
                        if let Ok(mut s) = shared.lock() {
                            s.latest = Some(Telemetry {
                                power_w: d.power_w.map(|p| p.max(0) as u16),
                                cadence_rpm: d.cadence_rpm,
                                hr_bpm: d.heart_rate_bpm,
                                speed_kmh: d.speed_kmh,
                            });
                        }
                    }
                } else if n.uuid == uuid16(u16s::CONTROL_POINT) {
                    tracing::debug!(response = ?ftms::parse_control_response(&n.value), "control point");
                }
            }
            if let Ok(mut s) = shared.lock() {
                s.connected = false;
                s.latest = None;
            }
        });
        self.control_point = Some(cp);
        self.write(Command::RequestControl).await?; // HW-VERIFY: wait for the 0x80 response first.
        self.control_requested = true;
        let name = self
            .peripheral
            .properties()
            .await
            .map_err(io)?
            .and_then(|p| p.local_name)
            .unwrap_or_default();
        Ok(TrainerCapabilities {
            name,
            supports_erg: features.supports_power_target(),
            supports_simulation: features.supports_simulation(),
            supports_resistance: features.supports_resistance_target(),
            min_power_w: range.map(|r| r.min_w.max(0) as u16),
            max_power_w: range.map(|r| r.max_w.max(0) as u16),
            reports_cadence: features.reports_cadence(),
            reports_power: features.reports_power(),
            reports_heart_rate: features.reports_heart_rate(),
        })
    }

    async fn disconnect(&mut self) -> Result<(), TransportError> {
        self.peripheral.disconnect().await.map_err(io)
    }

    async fn apply(&mut self, command: TrainerCommand) -> Result<(), TransportError> {
        let cmd = match command {
            TrainerCommand::Start => Command::Start,
            TrainerCommand::Pause => Command::Pause,
            TrainerCommand::Stop => Command::Stop,
            TrainerCommand::SetTargetPower { watts } => Command::SetTargetPower(watts as i16),
            TrainerCommand::SetGrade { grade_pct } => Command::SetSimulation(SimulationParams {
                grade_pct,
                ..self.sim
            }),
        };
        self.write(cmd).await
    }

    fn latest(&self) -> Option<Telemetry> {
        self.shared.lock().ok().and_then(|s| s.latest)
    }

    fn connection(&self) -> ConnectionState {
        match self.shared.lock() {
            Ok(s) if s.connected => ConnectionState::Connected,
            _ => ConnectionState::Disconnected,
        }
    }
}
