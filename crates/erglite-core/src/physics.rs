// SPDX-License-Identifier: MIT OR Apache-2.0
//! Steady-state cycling physics used to estimate speed and route time.

use crate::workout::Route;

const G: f64 = 9.80665;

/// Rider, bike and environment parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiderParams {
    /// Rider plus bike mass in kg.
    pub mass_kg: f64,
    /// Rolling resistance coefficient.
    pub crr: f64,
    /// Drag area in m².
    pub cda_m2: f64,
    /// Air density in kg/m³.
    pub rho: f64,
    /// Drivetrain efficiency (0..1).
    pub drivetrain_eff: f64,
}

impl Default for RiderParams {
    fn default() -> Self {
        Self {
            mass_kg: 84.0,
            crr: 0.004,
            cda_m2: 0.35,
            rho: 1.225,
            drivetrain_eff: 0.975,
        }
    }
}

impl RiderParams {
    /// Wind resistance coefficient in kg/m as used by FTMS (`0.5 * rho * CdA`).
    pub fn cw_kg_m(&self) -> f64 {
        0.5 * self.rho * self.cda_m2
    }

    fn power_needed(&self, v: f64, grade_pct: f64) -> f64 {
        let theta = (grade_pct / 100.0).atan();
        let force = self.mass_kg * G * (theta.sin() + self.crr * theta.cos());
        v * force + 0.5 * self.rho * self.cda_m2 * v.powi(3)
    }
}

/// Maximum speed returned by the solver in m/s (about 108 km/h).
pub const MAX_SPEED_MPS: f64 = 30.0;

/// Steady-state speed in m/s for a given pedaling power and grade.
pub fn speed_for_power(power_w: f64, grade_pct: f64, p: &RiderParams) -> f64 {
    let available = power_w * p.drivetrain_eff;
    if available <= 0.0 {
        return 0.0;
    }
    if p.power_needed(MAX_SPEED_MPS, grade_pct) <= available {
        return MAX_SPEED_MPS;
    }
    let (mut lo, mut hi) = (0.0_f64, MAX_SPEED_MPS);
    for _ in 0..60 {
        let mid = 0.5 * (lo + hi);
        if p.power_needed(mid, grade_pct) > available {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    0.5 * (lo + hi)
}

/// Result of a route estimate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RouteEstimate {
    /// Estimated riding time in seconds.
    pub time_s: f64,
    /// Average speed in km/h.
    pub avg_speed_kmh: f64,
}

/// Estimate the time needed to ride a route at a constant power.
pub fn estimate_route(route: &Route, power_w: f64, p: &RiderParams) -> RouteEstimate {
    let total = route.total_distance_m();
    let step = route.resolution_m.max(10.0);
    let (mut d, mut t) = (0.0, 0.0);
    while d < total {
        let len = step.min(total - d);
        let grade = route.average_grade(d, d + len);
        let v = speed_for_power(power_w, grade, p).max(0.5);
        t += len / v;
        d += len;
    }
    RouteEstimate {
        time_s: t,
        avg_speed_kmh: if t > 0.0 { total / t * 3.6 } else { 0.0 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_speed_at_200w_is_plausible() {
        let v = speed_for_power(
            200.0,
            0.0,
            &RiderParams {
                mass_kg: 80.0,
                cda_m2: 0.32,
                ..Default::default()
            },
        );
        assert!((32.0..37.0).contains(&(v * 3.6)), "{}", v * 3.6);
    }

    #[test]
    fn climbing_is_slower_and_descending_faster() {
        let p = RiderParams::default();
        let (flat, up, down) = (
            speed_for_power(200.0, 0.0, &p),
            speed_for_power(200.0, 5.0, &p),
            speed_for_power(200.0, -5.0, &p),
        );
        assert!(up < flat && flat < down);
        assert!((13.0..19.0).contains(&(up * 3.6)));
    }

    #[test]
    fn zero_power_and_extreme_descent_are_bounded() {
        let p = RiderParams::default();
        assert_eq!(speed_for_power(0.0, 3.0, &p), 0.0);
        assert!(speed_for_power(300.0, -25.0, &p) <= MAX_SPEED_MPS);
    }

    #[test]
    fn route_estimate_grows_with_climbing() {
        let flat = Route {
            resolution_m: 100.0,
            points: vec![(0.0, 0.0), (10_000.0, 0.0)],
        };
        let hilly = Route {
            resolution_m: 100.0,
            points: vec![(0.0, 0.0), (5_000.0, 250.0), (10_000.0, 250.0)],
        };
        let p = RiderParams::default();
        assert!(estimate_route(&hilly, 200.0, &p).time_s > estimate_route(&flat, 200.0, &p).time_s);
    }
}
