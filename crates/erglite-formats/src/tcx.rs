// SPDX-License-Identifier: MIT OR Apache-2.0
//! TCX session export (accepted by Strava and Garmin Connect).

use erglite_core::clock::iso8601_utc;
use erglite_core::metrics::SamplePoint;
use std::fmt::Write;

/// Export a recorded session as a TCX document.
///
/// `start_unix_s` is the session start time. Samples are expected at roughly 1 Hz.
pub fn export_tcx(start_unix_s: i64, samples: &[SamplePoint]) -> String {
    let start = iso8601_utc(start_unix_s);
    let total_s = samples.last().map_or(0.0, |s| s.t_ms as f64 / 1000.0);
    let dist = samples
        .iter()
        .filter_map(|s| s.distance_m)
        .fold(0.0, f64::max);
    let mut x = String::with_capacity(samples.len() * 260 + 800);
    let _ = write!(
        x,
        concat!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
            "<TrainingCenterDatabase xmlns=\"http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2\" ",
            "xmlns:ns3=\"http://www.garmin.com/xmlschemas/ActivityExtension/v2\">\n",
            "<Activities><Activity Sport=\"Biking\"><Id>{start}</Id>\n",
            "<Lap StartTime=\"{start}\"><TotalTimeSeconds>{total_s:.0}</TotalTimeSeconds>",
            "<DistanceMeters>{dist:.1}</DistanceMeters><Calories>0</Calories>",
            "<Intensity>Active</Intensity><TriggerMethod>Manual</TriggerMethod><Track>\n"
        ),
        start = start,
        total_s = total_s,
        dist = dist
    );
    for s in samples {
        let _ = write!(
            x,
            "<Trackpoint><Time>{}</Time>",
            iso8601_utc(start_unix_s + (s.t_ms / 1000) as i64)
        );
        if let Some(a) = s.altitude_m {
            let _ = write!(x, "<AltitudeMeters>{a:.1}</AltitudeMeters>");
        }
        if let Some(d) = s.distance_m {
            let _ = write!(x, "<DistanceMeters>{d:.1}</DistanceMeters>");
        }
        if let Some(h) = s.hr_bpm {
            let _ = write!(x, "<HeartRateBpm><Value>{h}</Value></HeartRateBpm>");
        }
        if let Some(c) = s.cadence_rpm {
            let _ = write!(
                x,
                "<Cadence>{}</Cadence>",
                c.round().clamp(0.0, 254.0) as u8
            );
        }
        if let Some(p) = s.power_w {
            let _ = write!(
                x,
                "<Extensions><ns3:TPX><ns3:Watts>{p}</ns3:Watts></ns3:TPX></Extensions>"
            );
        }
        x.push_str("</Trackpoint>\n");
    }
    x.push_str("</Track></Lap></Activity></Activities></TrainingCenterDatabase>\n");
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_one_trackpoint_per_sample() {
        let samples: Vec<SamplePoint> = (0..5)
            .map(|i| SamplePoint {
                t_ms: i * 1000,
                power_w: Some(200 + i as u16),
                hr_bpm: Some(140),
                cadence_rpm: Some(90.4),
                distance_m: Some(i as f64 * 8.0),
                altitude_m: Some(100.0),
                ..Default::default()
            })
            .collect();
        let xml = export_tcx(1_793_530_756, &samples);
        assert_eq!(xml.matches("<Trackpoint>").count(), 5);
        assert!(xml.contains("<ns3:Watts>204</ns3:Watts>"));
        assert!(xml.contains("<Id>2026-11-01T10:59:16Z</Id>"));
        assert!(xml.contains("<Cadence>90</Cadence>"));
        assert!(xml.contains("<TotalTimeSeconds>4</TotalTimeSeconds>"));
    }
}
