// SPDX-License-Identifier: MIT OR Apache-2.0
//! Training metrics: power zones, normalized power, intensity factor, TSS and session summary.

use serde::{Deserialize, Serialize};

/// One recorded sample (1 Hz during a session).
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct SamplePoint {
    /// Milliseconds since the session started.
    pub t_ms: u64,
    /// Power in watts.
    pub power_w: Option<u16>,
    /// Cadence in rpm.
    pub cadence_rpm: Option<f32>,
    /// Heart rate in bpm.
    pub hr_bpm: Option<u8>,
    /// Speed in km/h.
    pub speed_kmh: Option<f32>,
    /// Covered distance in meters.
    pub distance_m: Option<f64>,
    /// Virtual altitude in meters.
    pub altitude_m: Option<f64>,
    /// Grade in percent.
    pub grade_pct: Option<f32>,
    /// Target power in watts.
    pub target_power_w: Option<u16>,
}

/// Coggan power zone (1 to 7) for a power value.
pub fn zone_for(power_w: f64, ftp_w: f64) -> u8 {
    let pct = if ftp_w > 0.0 {
        power_w / ftp_w * 100.0
    } else {
        0.0
    };
    match pct {
        p if p < 55.5 => 1,
        p if p < 75.5 => 2,
        p if p < 90.5 => 3,
        p if p < 105.5 => 4,
        p if p < 120.5 => 5,
        p if p < 150.5 => 6,
        _ => 7,
    }
}

/// Normalized power from 1 Hz power samples (30 s rolling average, 4th-power mean).
pub fn normalized_power(power_1hz: &[f64]) -> Option<f64> {
    const WINDOW: usize = 30;
    if power_1hz.len() < WINDOW {
        return None;
    }
    let mut sum: f64 = power_1hz[..WINDOW].iter().sum();
    let mut acc = (sum / WINDOW as f64).powi(4);
    let mut n = 1.0;
    for i in WINDOW..power_1hz.len() {
        sum += power_1hz[i] - power_1hz[i - WINDOW];
        acc += (sum / WINDOW as f64).powi(4);
        n += 1.0;
    }
    Some((acc / n).powf(0.25))
}

/// Intensity factor: normalized power divided by FTP.
pub fn intensity_factor(np_w: f64, ftp_w: f64) -> f64 {
    if ftp_w > 0.0 { np_w / ftp_w } else { 0.0 }
}

/// Training stress score.
pub fn tss(duration_s: f64, np_w: f64, ftp_w: f64) -> f64 {
    if ftp_w <= 0.0 {
        return 0.0;
    }
    let if_ = np_w / ftp_w;
    duration_s * np_w * if_ / (ftp_w * 3600.0) * 100.0
}

/// Aggregated session results.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    /// Duration in seconds.
    pub duration_s: f64,
    /// Average power in watts.
    pub avg_power_w: f64,
    /// Maximum power in watts.
    pub max_power_w: u16,
    /// Normalized power in watts.
    pub normalized_power_w: Option<f64>,
    /// Intensity factor.
    pub intensity_factor: Option<f64>,
    /// Training stress score.
    pub tss: Option<f64>,
    /// Seconds spent in each of the 7 zones.
    pub zone_seconds: [u32; 7],
    /// Covered distance in meters.
    pub distance_m: f64,
    /// Positive elevation gain in meters.
    pub elevation_gain_m: f64,
    /// Average heart rate in bpm.
    pub avg_hr_bpm: Option<f64>,
}

/// Summarize a session recorded at roughly 1 Hz.
pub fn summarize(samples: &[SamplePoint], ftp_w: f64) -> Summary {
    let mut s = Summary::default();
    let (Some(first), Some(last)) = (samples.first(), samples.last()) else {
        return s;
    };
    s.duration_s = (last.t_ms - first.t_ms) as f64 / 1000.0;
    let powers: Vec<f64> = samples
        .iter()
        .map(|p| f64::from(p.power_w.unwrap_or(0)))
        .collect();
    s.avg_power_w = powers.iter().sum::<f64>() / powers.len() as f64;
    s.max_power_w = samples.iter().filter_map(|p| p.power_w).max().unwrap_or(0);
    for p in &powers {
        s.zone_seconds[usize::from(zone_for(*p, ftp_w) - 1)] += 1;
    }
    s.normalized_power_w = normalized_power(&powers);
    s.intensity_factor = s.normalized_power_w.map(|np| intensity_factor(np, ftp_w));
    s.tss = s.normalized_power_w.map(|np| tss(s.duration_s, np, ftp_w));
    s.distance_m = samples
        .iter()
        .filter_map(|p| p.distance_m)
        .fold(0.0, f64::max);
    let alts: Vec<f64> = samples.iter().filter_map(|p| p.altitude_m).collect();
    s.elevation_gain_m = alts.windows(2).map(|w| (w[1] - w[0]).max(0.0)).sum();
    let hrs: Vec<f64> = samples
        .iter()
        .filter_map(|p| p.hr_bpm.map(f64::from))
        .collect();
    if !hrs.is_empty() {
        s.avg_hr_bpm = Some(hrs.iter().sum::<f64>() / hrs.len() as f64);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zones_follow_coggan_limits() {
        assert_eq!(zone_for(100.0, 250.0), 1);
        assert_eq!(zone_for(150.0, 250.0), 2);
        assert_eq!(zone_for(220.0, 250.0), 3);
        assert_eq!(zone_for(250.0, 250.0), 4);
        assert_eq!(zone_for(290.0, 250.0), 5);
        assert_eq!(zone_for(350.0, 250.0), 6);
        assert_eq!(zone_for(500.0, 250.0), 7);
    }

    #[test]
    fn steady_ride_has_np_equal_to_power_and_tss_100_at_ftp_for_an_hour() {
        let p = vec![250.0; 3600];
        let np = normalized_power(&p).unwrap_or(0.0);
        assert!((np - 250.0).abs() < 1e-6);
        assert!((tss(3600.0, np, 250.0) - 100.0).abs() < 1e-6);
        assert!(normalized_power(&[100.0; 10]).is_none());
    }

    #[test]
    fn variable_power_raises_np_above_average() {
        let p: Vec<f64> = (0..600)
            .map(|i| if (i / 60) % 2 == 0 { 100.0 } else { 300.0 })
            .collect();
        assert!(normalized_power(&p).unwrap_or(0.0) > 200.0);
    }

    #[test]
    fn summary_counts_zone_time() {
        let samples: Vec<SamplePoint> = (0..120)
            .map(|i| SamplePoint {
                t_ms: i * 1000,
                power_w: Some(if i < 60 { 100 } else { 250 }),
                ..Default::default()
            })
            .collect();
        let s = summarize(&samples, 250.0);
        assert_eq!(s.zone_seconds[0], 60);
        assert_eq!(s.zone_seconds[3], 60);
        assert_eq!(s.max_power_w, 250);
    }
}
