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
) -> Result<Route, FormatError> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);
    let mut pts: Vec<(f64, f64, f64)> = Vec::new(); // lat, lon, ele
    let (mut cur, mut in_ele) = (None::<(f64, f64)>, false);
    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == b"trkpt" => {
                let get = |k: &[u8]| {
                    e.attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == k)
                        .and_then(|a| a.unescape_value().ok()?.parse::<f64>().ok())
                };
                cur = get(b"lat").zip(get(b"lon"));
            }
            Event::Start(e) if e.name().as_ref() == b"ele" => in_ele = true,
            Event::Text(t) if in_ele => {
                if let (Some((lat, lon)), Ok(text)) = (cur, t.decode()) {
                    if let Ok(ele) = text.trim().parse::<f64>() {
                        pts.push((lat, lon, ele));
                    }
                }
            }
            Event::End(e) if e.name().as_ref() == b"ele" => in_ele = false,
            Event::End(e) if e.name().as_ref() == b"trkpt" => cur = None,
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
    let half = smooth_window / 2;
    let smooth: Vec<f64> = (0..pts.len())
        .map(|i| {
            let (a, b) = (i.saturating_sub(half), (i + half + 1).min(pts.len()));
            pts[a..b].iter().map(|p| p.2).sum::<f64>() / (b - a) as f64
        })
        .collect();
    let total = dist[dist.len() - 1];
    if total < resolution_m {
        return Err(FormatError::Invalid(
            "route is shorter than one resolution step".into(),
        ));
    }
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
        let r = import_gpx_route(&gpx(201), 100.0, 9).unwrap();
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
        assert!(import_gpx_route("<gpx></gpx>", 100.0, 5).is_err());
    }
}
