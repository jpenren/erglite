// SPDX-License-Identifier: MIT OR Apache-2.0
//! Rider profile.

use crate::physics::RiderParams;
use serde::{Deserialize, Serialize};

/// The rider profile stored locally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// Functional threshold power in watts.
    pub ftp_w: u16,
    /// Rider weight in kg.
    pub weight_kg: f64,
    /// Bike weight in kg.
    pub bike_kg: f64,
    /// Maximum heart rate in bpm.
    pub max_hr: Option<u8>,
    /// `metric` or `imperial`.
    pub units: String,
    /// UI language code.
    pub language: String,
    /// Rolling resistance coefficient.
    pub crr: f64,
    /// Drag area in m².
    pub cda_m2: f64,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            ftp_w: 200,
            weight_kg: 75.0,
            bike_kg: 9.0,
            max_hr: None,
            units: "metric".into(),
            language: "en".into(),
            crr: 0.004,
            cda_m2: 0.35,
        }
    }
}

impl Profile {
    /// Physics parameters derived from this profile.
    pub fn rider_params(&self) -> RiderParams {
        RiderParams {
            mass_kg: self.weight_kg + self.bike_kg,
            crr: self.crr,
            cda_m2: self.cda_m2,
            ..RiderParams::default()
        }
    }
}
