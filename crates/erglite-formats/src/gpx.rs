// SPDX-License-Identifier: MIT OR Apache-2.0
//! GPX route import with altitude smoothing and resampling.

use crate::FormatError;
use erglite_core::workout::Route;
use quick_xml::Reader;
use quick_xml::events::Event;

fn haversine_m(a: (f64, f64), b: (f64, f64)) -> f64 {
    let r = 6_371_000.0;
    let (la1, la2) = (a.0.to_radians(), b.0.to_radians());
    let dlat = la2 - la1;
    let dlon = (b.1 - a.1).to_radians();
    let h = (dlat / 2.0).sin().powi(2) + la1.cos() * la2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * r * h.sqrt().asin()
}

/// Import the first track of a GPX file as a [`Route`].
///
/// GPS altitude is noisy, so elevations are smoothed with a moving average of
/// `smooth_window` points (odd numbers work best) before resampling every `resolution_m`.
pub fn import_gpx_route(
    xml: &str,
    resolution_m: f64,
    smooth_window: usize,
    simplify_every_m: Option<f64>,
) -> Result<Route, FormatError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut pts: Vec<(f64, f64, f64)> = Vec::new(); // lat, lon, ele
    let (mut cur, mut in_ele) = (None::<(f64, f64)>, false);
    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == "trkpt" => {
                let get = |k: &str| {
                    e.attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == k)
                        .and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok()?.parse::<f64>().ok())
                };
                cur = get("lat").zip(get("lon"));
            }
            Event::Start(e) if e.name().as_ref() == "ele" => in_ele = true,
            Event::Text(t) if in_ele => {
                if let Some((lat, lon)) = cur {
                    let text = t.as_ref();
                    if let Ok(ele) = text.trim().parse::<f64>() {
                        pts.push((lat, lon, ele));
                    }
                }
            }
            Event::End(e) if e.name().as_ref() == "ele" => in_ele = false,
            Event::End(e) if e.name().as_ref() == "trkpt" => cur = None,
            Event::Eof => break,
            _ => {}
        }
    }
    if pts.len() < 2 {
        return Err(FormatError::Invalid(
            "GPX needs at least two track points with elevation".into(),
        ));
    }
    let mut dist = vec![0.0];
    for w in pts.windows(2) {
        dist.push(dist[dist.len() - 1] + haversine_m((w[0].0, w[0].1), (w[1].0, w[1].1)));
    }
    // Optionally simplify points before smoothing
    let (pts, dist) = if let Some(every_m) = simplify_every_m {
        simplify_points(pts, every_m)?
    } else {
        (pts, dist)
    };

    let total = dist[dist.len() - 1];
    if total < resolution_m {
        return Err(FormatError::Invalid(
            "route is shorter than one resolution step".into(),
        ));
    }

    let half = smooth_window / 2;
    let smooth: Vec<f64> = (0..pts.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(half), (i + half + 1).min(pts.len()));
            pts[a..b].iter().map(|p| p.2).sum::<f64>() / (b - a) as f64
        })
        .collect();

    let mut points = Vec::new();
    let (mut d, mut j) = (0.0, 0usize);
    while d <= total {
        while j + 1 < dist.len() - 1 && dist[j + 1] < d {
            j += 1;
        }
        let span = (dist[j + 1] - dist[j]).max(1e-9);
        let t = ((d - dist[j]) / span).clamp(0.0, 1.0);
        points.push((d, smooth[j] + (smooth[j + 1] - smooth[j]) * t));
        d += resolution_m;
    }
    if points.last().is_none_or(|p| total - p.0 > 1.0) {
        points.push((total, smooth[smooth.len() - 1]));
    }
    Ok(Route {
        resolution_m,
        points,
    })
}

/// Simplify points by keeping one point every `every_m` meters.
/// Returns simplified points and their cumulative distances.
fn simplify_points(
    pts: Vec<(f64, f64, f64)>,
    every_m: f64,
) -> Result<(Vec<(f64, f64, f64)>, Vec<f64>), FormatError> {
    if pts.len() < 2 {
        return Err(FormatError::Invalid(
            "GPX needs at least two track points".into(),
        ));
    }
    let mut dist = vec![0.0_f64];
    for w in pts.windows(2) {
        let a: (f64, f64) = (w[0].0, w[0].1);
        let b: (f64, f64) = (w[1].0, w[1].1);
        dist.push(dist[dist.len() - 1] + haversine_m(a, b));
    }
    let total = dist[dist.len() - 1];
    
    // Keep first point, then every `every_m` meters, then last point
    let mut simplified = Vec::new();
    let mut simplified_dist = Vec::new();
    
    simplified.push(pts[0]);
    simplified_dist.push(0.0);
    
    let mut next_target = every_m;
    for i in 1..pts.len() - 1 {
        if dist[i] >= next_target {
            simplified.push(pts[i]);
            simplified_dist.push(dist[i]);
            next_target += every_m;
        }
    }
    
    // Always keep last point
    simplified.push(pts[pts.len() - 1]);
    simplified_dist.push(total);
    
    Ok((simplified, simplified_dist))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn gpx(n: usize) -> String {
        // Straight line north, about 11.1 m per 0.0001 degrees, climbing 1 m per point with noise.
        let pts: String = (0..n)
            .map(|i| {
                let noise = if i % 2 == 0 { 3.0 } else { -3.0 };
                format!(
                    r#"<trkpt lat="{:.5}" lon="10.0"><ele>{}</ele></trkpt>"#,
                    45.0 + i as f64 * 0.0001,
                    100.0 + i as f64 + noise
                )
            })
            .collect();
        format!("<gpx><trk><trkseg>{pts}</trkseg></trk></gpx>")
    }

    #[test]
    fn imports_resamples_and_smooths() {
        let r = import_gpx_route(&gpx(201), 100.0, 9, None).unwrap();
        let total = r.total_distance_m();
        assert!((total - 2224.0).abs() < 20.0, "{total}");
        assert!(r.points.windows(2).all(|w| w[1].0 > w[0].0));
        let (climb, descent) = r.grade_extremes();
        assert!(climb < 12.0 && descent > -4.0, "{climb} {descent}");
        assert!(
            (r.elevation_gain_m() - 200.0).abs() < 20.0,
            "{}",
            r.elevation_gain_m()
        );
    }

    #[test]
    fn rejects_empty_tracks() {
        assert!(import_gpx_route("<gpx></gpx>", 100.0, 5, None).is_err());
    }

    #[test]
    fn simplify_points_reduces_count() {
        // Create a 10 km straight line with points every 10 m (1000 points)
        let pts: Vec<(f64, f64, f64)> = (0..1000)
            .map(|i| (45.0 + i as f64 * 0.00009, 10.0, 100.0 + i as f64))
            .collect();
        
        let (simplified, _) = simplify_points(pts, 500.0).unwrap();
        
        // Should have ~20 points (10000m / 500m) + first + last
        assert!(simplified.len() >= 20 && simplified.len() <= 25, "got {}", simplified.len());
        assert_eq!(simplified[0], (45.0, 10.0, 100.0));
        assert_eq!(simplified[simplified.len() - 1], (45.0 + 999.0 * 0.00009, 10.0, 100.0 + 999.0));
    }

    #[test]
    fn import_gpx_with_simplification() {
        // Create a GPX with many points
        let gpx_content = gpx(2001); // ~20 km with 2001 points
        let r = import_gpx_route(&gpx_content, 100.0, 9, Some(500.0)).unwrap();
        
        // With 500m simplification, should have fewer points before resampling
        // The resampling will create points every 100m, so total points should be ~200
        let total_points = r.points.len();
        assert!(total_points > 150 && total_points < 250, "got {}", total_points);
    }
}
