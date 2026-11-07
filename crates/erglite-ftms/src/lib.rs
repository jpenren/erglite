// SPDX-License-Identifier: MIT OR Apache-2.0
//! Pure codec for the Bluetooth **Fitness Machine Service (FTMS)**.
//!
//! No I/O and no async: bytes in, typed values out (and the reverse). Everything that depends
//! on real trainer behavior is marked `HW-VERIFY` and must be confirmed on a real device.
#![forbid(unsafe_code)]

use thiserror::Error;

/// 16-bit Bluetooth SIG UUIDs used by FTMS.
pub mod uuid16 {
    /// Fitness Machine Service.
    pub const SERVICE: u16 = 0x1826;
    /// Fitness Machine Feature (read).
    pub const FEATURE: u16 = 0x2ACC;
    /// Indoor Bike Data (notify).
    pub const INDOOR_BIKE_DATA: u16 = 0x2AD2;
    /// Training Status (notify).
    pub const TRAINING_STATUS: u16 = 0x2AD3;
    /// Supported Resistance Level Range (read).
    pub const RESISTANCE_RANGE: u16 = 0x2AD6;
    /// Supported Power Range (read).
    pub const POWER_RANGE: u16 = 0x2AD8;
    /// Fitness Machine Control Point (write + indicate).
    pub const CONTROL_POINT: u16 = 0x2AD9;
    /// Fitness Machine Status (notify).
    pub const MACHINE_STATUS: u16 = 0x2ADA;
}

/// Errors produced while decoding a characteristic value.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    /// The value ended before all announced fields were read.
    #[error("value truncated at offset {offset}")]
    Truncated {
        /// Offset where reading failed.
        offset: usize,
    },
    /// The value does not follow the expected layout.
    #[error("unexpected value: {0}")]
    Unexpected(&'static str),
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, i: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], ParseError> {
        let slice = self
            .b
            .get(self.i..self.i + n)
            .ok_or(ParseError::Truncated { offset: self.i })?;
        self.i += n;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, ParseError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, ParseError> {
        let s = self.take(2)?;
        Ok(u16::from_le_bytes([s[0], s[1]]))
    }
    fn i16(&mut self) -> Result<i16, ParseError> {
        let s = self.take(2)?;
        Ok(i16::from_le_bytes([s[0], s[1]]))
    }
    fn u24(&mut self) -> Result<u32, ParseError> {
        let s = self.take(3)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], 0]))
    }
}

/// Decoded Indoor Bike Data notification. Absent fields are `None`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct IndoorBikeData {
    /// Instantaneous speed in km/h.
    pub speed_kmh: Option<f32>,
    /// Average speed in km/h.
    pub average_speed_kmh: Option<f32>,
    /// Instantaneous cadence in rpm.
    pub cadence_rpm: Option<f32>,
    /// Average cadence in rpm.
    pub average_cadence_rpm: Option<f32>,
    /// Total distance in meters.
    pub total_distance_m: Option<u32>,
    /// Resistance level (unit-less, trainer specific).
    pub resistance_level: Option<i16>,
    /// Instantaneous power in watts.
    pub power_w: Option<i16>,
    /// Average power in watts.
    pub average_power_w: Option<i16>,
    /// Total expended energy in kcal.
    pub total_energy_kcal: Option<u16>,
    /// Heart rate in bpm.
    pub heart_rate_bpm: Option<u8>,
    /// Elapsed time in seconds.
    pub elapsed_s: Option<u16>,
    /// Remaining time in seconds.
    pub remaining_s: Option<u16>,
}

/// Parse an Indoor Bike Data (0x2AD2) notification.
///
/// Field order follows the FTMS specification. Bit 0 of the flags is *More Data*: the
/// instantaneous speed is present when it is **0**.
pub fn parse_indoor_bike_data(bytes: &[u8]) -> Result<IndoorBikeData, ParseError> {
    let mut r = Reader::new(bytes);
    let flags = r.u16()?;
    let has = |bit: u16| flags & (1 << bit) != 0;
    let mut d = IndoorBikeData::default();
    if !has(0) {
        d.speed_kmh = Some(f32::from(r.u16()?) * 0.01);
    }
    if has(1) {
        d.average_speed_kmh = Some(f32::from(r.u16()?) * 0.01);
    }
    if has(2) {
        d.cadence_rpm = Some(f32::from(r.u16()?) * 0.5);
    }
    if has(3) {
        d.average_cadence_rpm = Some(f32::from(r.u16()?) * 0.5);
    }
    if has(4) {
        d.total_distance_m = Some(r.u24()?);
    }
    if has(5) {
        d.resistance_level = Some(r.i16()?);
    }
    if has(6) {
        d.power_w = Some(r.i16()?);
    }
    if has(7) {
        d.average_power_w = Some(r.i16()?);
    }
    if has(8) {
        d.total_energy_kcal = Some(r.u16()?);
        r.take(3)?; // energy per hour (u16) and per minute (u8)
    }
    if has(9) {
        d.heart_rate_bpm = Some(r.u8()?);
    }
    if has(10) {
        r.u8()?; // metabolic equivalent
    }
    if has(11) {
        d.elapsed_s = Some(r.u16()?);
    }
    if has(12) {
        d.remaining_s = Some(r.u16()?);
    }
    Ok(d)
}

/// Indoor bike simulation parameters (opcode `0x11`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimulationParams {
    /// Wind speed in m/s (positive = headwind).
    pub wind_speed_mps: f32,
    /// Grade in percent.
    pub grade_pct: f32,
    /// Coefficient of rolling resistance.
    pub crr: f32,
    /// Wind resistance coefficient in kg/m (`0.5 * rho * CdA`).
    pub cw_kg_m: f32,
}

impl Default for SimulationParams {
    fn default() -> Self {
        Self {
            wind_speed_mps: 0.0,
            grade_pct: 0.0,
            crr: 0.004,
            cw_kg_m: 0.51,
        }
    }
}

/// A Fitness Machine Control Point command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// `0x00` Request Control.
    RequestControl,
    /// `0x01` Reset.
    Reset,
    /// `0x04` Set Target Resistance Level (raw value, resolution 0.1).
    SetTargetResistance(u8),
    /// `0x05` Set Target Power in watts.
    SetTargetPower(i16),
    /// `0x07` Start or Resume.
    Start,
    /// `0x08` Stop (parameter `0x01`).
    Stop,
    /// `0x08` Pause (parameter `0x02`).
    Pause,
    /// `0x11` Set Indoor Bike Simulation Parameters.
    SetSimulation(SimulationParams),
}

impl Command {
    /// The Control Point opcode of this command.
    pub fn opcode(&self) -> u8 {
        match self {
            Command::RequestControl => 0x00,
            Command::Reset => 0x01,
            Command::SetTargetResistance(_) => 0x04,
            Command::SetTargetPower(_) => 0x05,
            Command::Start => 0x07,
            Command::Stop | Command::Pause => 0x08,
            Command::SetSimulation(_) => 0x11,
        }
    }

    /// Encode the command as the bytes written to the Control Point.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = vec![self.opcode()];
        match self {
            Command::SetTargetResistance(v) => out.push(*v),
            Command::SetTargetPower(w) => out.extend_from_slice(&w.to_le_bytes()),
            Command::Stop => out.push(0x01),
            Command::Pause => out.push(0x02),
            Command::SetSimulation(p) => {
                let wind = (p.wind_speed_mps * 1000.0).round().clamp(-32768.0, 32767.0) as i16;
                let grade = (p.grade_pct * 100.0).round().clamp(-32768.0, 32767.0) as i16;
                out.extend_from_slice(&wind.to_le_bytes());
                out.extend_from_slice(&grade.to_le_bytes());
                out.push((p.crr / 0.0001).round().clamp(0.0, 255.0) as u8);
                out.push((p.cw_kg_m / 0.01).round().clamp(0.0, 255.0) as u8);
            }
            Command::RequestControl | Command::Reset | Command::Start => {}
        }
        out
    }
}

/// Result code of a Control Point response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultCode {
    /// `0x01` success.
    Success,
    /// `0x02` opcode not supported.
    NotSupported,
    /// `0x03` invalid parameter.
    InvalidParameter,
    /// `0x04` operation failed.
    OperationFailed,
    /// `0x05` control not permitted.
    ControlNotPermitted,
    /// Any other value.
    Other(u8),
}

/// Decoded Control Point indication (`0x80 <request opcode> <result>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlResponse {
    /// Opcode of the request this response answers.
    pub request_opcode: u8,
    /// Result of the request.
    pub result: ResultCode,
}

/// Parse a Control Point indication.
pub fn parse_control_response(bytes: &[u8]) -> Result<ControlResponse, ParseError> {
    let mut r = Reader::new(bytes);
    if r.u8()? != 0x80 {
        return Err(ParseError::Unexpected("response code must start with 0x80"));
    }
    let request_opcode = r.u8()?;
    let result = match r.u8()? {
        0x01 => ResultCode::Success,
        0x02 => ResultCode::NotSupported,
        0x03 => ResultCode::InvalidParameter,
        0x04 => ResultCode::OperationFailed,
        0x05 => ResultCode::ControlNotPermitted,
        other => ResultCode::Other(other),
    };
    Ok(ControlResponse {
        request_opcode,
        result,
    })
}

/// Fitness Machine Feature characteristic (machine features + target setting features).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MachineFeatures {
    /// Raw fitness machine feature bits.
    pub machine: u32,
    /// Raw target setting feature bits.
    pub target_settings: u32,
}

impl MachineFeatures {
    /// Cadence is reported.
    pub fn reports_cadence(&self) -> bool {
        self.machine & (1 << 1) != 0
    }
    /// Power measurement is reported.
    pub fn reports_power(&self) -> bool {
        self.machine & (1 << 14) != 0
    }
    /// Heart rate measurement is reported.
    pub fn reports_heart_rate(&self) -> bool {
        self.machine & (1 << 10) != 0
    }
    /// Target resistance level can be set.
    pub fn supports_resistance_target(&self) -> bool {
        self.target_settings & (1 << 2) != 0
    }
    /// Target power can be set (ERG).
    pub fn supports_power_target(&self) -> bool {
        self.target_settings & (1 << 3) != 0
    }
    /// Indoor bike simulation parameters can be set (grade simulation).
    pub fn supports_simulation(&self) -> bool {
        self.target_settings & (1 << 13) != 0
    }
}

/// Parse the Fitness Machine Feature characteristic (8 bytes).
pub fn parse_machine_features(bytes: &[u8]) -> Result<MachineFeatures, ParseError> {
    let mut r = Reader::new(bytes);
    let lo = u32::from(r.u16()?) | (u32::from(r.u16()?) << 16);
    let hi = u32::from(r.u16()?) | (u32::from(r.u16()?) << 16);
    Ok(MachineFeatures {
        machine: lo,
        target_settings: hi,
    })
}

/// Supported Power Range characteristic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerRange {
    /// Minimum target power in watts.
    pub min_w: i16,
    /// Maximum target power in watts.
    pub max_w: i16,
    /// Minimum increment in watts.
    pub step_w: u16,
}

/// Parse the Supported Power Range characteristic.
pub fn parse_power_range(bytes: &[u8]) -> Result<PowerRange, ParseError> {
    let mut r = Reader::new(bytes);
    Ok(PowerRange {
        min_w: r.i16()?,
        max_w: r.i16()?,
        step_w: r.u16()?,
    })
}

/// Fitness Machine Status notification (subset).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MachineStatus {
    /// `0x01` reset.
    Reset,
    /// `0x02` stopped or paused by the user.
    StoppedByUser,
    /// `0x04` started or resumed by the user.
    StartedByUser,
    /// `0x08` target power changed (watts).
    TargetPowerChanged(i16),
    /// `0x12` indoor bike simulation parameters changed.
    SimulationChanged,
    /// Any other status opcode.
    Other(u8),
}

/// Parse a Fitness Machine Status notification.
pub fn parse_machine_status(bytes: &[u8]) -> Result<MachineStatus, ParseError> {
    let mut r = Reader::new(bytes);
    Ok(match r.u8()? {
        0x01 => MachineStatus::Reset,
        0x02 => MachineStatus::StoppedByUser,
        0x04 => MachineStatus::StartedByUser,
        0x08 => MachineStatus::TargetPowerChanged(r.i16()?),
        0x12 => MachineStatus::SimulationChanged,
        other => MachineStatus::Other(other),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn encodes_target_power() {
        assert_eq!(
            Command::SetTargetPower(200).encode(),
            vec![0x05, 0xC8, 0x00]
        );
    }

    #[test]
    fn encodes_simulation_with_five_percent_grade() {
        let p = SimulationParams {
            grade_pct: 5.0,
            ..Default::default()
        };
        assert_eq!(
            Command::SetSimulation(p).encode(),
            vec![0x11, 0, 0, 0xF4, 0x01, 40, 51]
        );
    }

    #[test]
    fn encodes_negative_grade() {
        let p = SimulationParams {
            grade_pct: -3.5,
            ..Default::default()
        };
        let bytes = Command::SetSimulation(p).encode();
        assert_eq!(i16::from_le_bytes([bytes[3], bytes[4]]), -350);
    }

    #[test]
    fn encodes_stop_pause_start() {
        assert_eq!(Command::Stop.encode(), vec![0x08, 0x01]);
        assert_eq!(Command::Pause.encode(), vec![0x08, 0x02]);
        assert_eq!(Command::Start.encode(), vec![0x07]);
        assert_eq!(Command::RequestControl.encode(), vec![0x00]);
    }

    #[test]
    fn parses_speed_cadence_power_frame() {
        let d = parse_indoor_bike_data(&[0x44, 0x00, 0xB8, 0x0B, 0xB4, 0x00, 0xC8, 0x00]).unwrap();
        assert_eq!(d.speed_kmh, Some(30.0));
        assert_eq!(d.cadence_rpm, Some(90.0));
        assert_eq!(d.power_w, Some(200));
        assert_eq!(d.heart_rate_bpm, None);
    }

    #[test]
    fn parses_distance_hr_and_elapsed() {
        let flags: u16 = 1 | (1 << 4) | (1 << 9) | (1 << 11);
        let mut f = flags.to_le_bytes().to_vec();
        f.extend_from_slice(&[0x10, 0x27, 0x00, 142, 0x2C, 0x01]);
        let d = parse_indoor_bike_data(&f).unwrap();
        assert_eq!(d.speed_kmh, None);
        assert_eq!(d.total_distance_m, Some(10_000));
        assert_eq!(d.heart_rate_bpm, Some(142));
        assert_eq!(d.elapsed_s, Some(300));
    }

    #[test]
    fn truncated_frame_is_an_error() {
        assert!(matches!(
            parse_indoor_bike_data(&[0x44, 0x00, 0xB8]),
            Err(ParseError::Truncated { .. })
        ));
    }

    #[test]
    fn parses_control_response() {
        let r = parse_control_response(&[0x80, 0x00, 0x01]).unwrap();
        assert_eq!(
            r,
            ControlResponse {
                request_opcode: 0,
                result: ResultCode::Success
            }
        );
        assert!(parse_control_response(&[0x00, 0x00, 0x01]).is_err());
    }

    #[test]
    fn parses_features() {
        let bytes = [0x02, 0x44, 0x00, 0x00, 0x08, 0x20, 0x00, 0x00];
        let f = parse_machine_features(&bytes).unwrap();
        assert!(f.reports_cadence() && f.reports_power());
        assert!(f.supports_power_target() && f.supports_simulation());
        assert!(!f.supports_resistance_target());
    }

    #[test]
    fn parses_power_range_and_status() {
        let r = parse_power_range(&[0x00, 0x00, 0xB0, 0x04, 0x01, 0x00]).unwrap();
        assert_eq!((r.min_w, r.max_w, r.step_w), (0, 1200, 1));
        assert_eq!(
            parse_machine_status(&[0x08, 0xC8, 0x00]).unwrap(),
            MachineStatus::TargetPowerChanged(200)
        );
    }

    proptest! {
        #[test]
        fn parsers_never_panic(bytes in proptest::collection::vec(any::<u8>(), 0..40)) {
            let _ = parse_indoor_bike_data(&bytes);
            let _ = parse_control_response(&bytes);
            let _ = parse_machine_features(&bytes);
            let _ = parse_power_range(&bytes);
            let _ = parse_machine_status(&bytes);
        }
    }
}
