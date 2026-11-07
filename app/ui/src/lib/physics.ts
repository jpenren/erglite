// SPDX-License-Identifier: MIT OR Apache-2.0
// Port of erglite-core::physics, used for live route estimates in the editor.
// Keep in sync with the Rust implementation (see physics.test.ts for shared reference values).
import type { Profile, Route } from './types';

const G = 9.80665;
export const MAX_SPEED_MPS = 30;

export interface RiderParams { massKg: number; crr: number; cdaM2: number; rho: number; eff: number }

export function riderFromProfile(p: Profile): RiderParams {
  return { massKg: p.weight_kg + p.bike_kg, crr: p.crr, cdaM2: p.cda_m2, rho: 1.225, eff: 0.975 };
}

function powerNeeded(v: number, gradePct: number, p: RiderParams): number {
  const theta = Math.atan(gradePct / 100);
  const force = p.massKg * G * (Math.sin(theta) + p.crr * Math.cos(theta));
  return v * force + 0.5 * p.rho * p.cdaM2 * v ** 3;
}

/** Steady-state speed (m/s) for a pedaling power and grade. */
export function speedForPower(powerW: number, gradePct: number, p: RiderParams): number {
  const available = powerW * p.eff;
  if (available <= 0) return 0;
  if (powerNeeded(MAX_SPEED_MPS, gradePct, p) <= available) return MAX_SPEED_MPS;
  let lo = 0, hi = MAX_SPEED_MPS;
  for (let i = 0; i < 60; i++) {
    const mid = (lo + hi) / 2;
    if (powerNeeded(mid, gradePct, p) > available) hi = mid; else lo = mid;
  }
  return (lo + hi) / 2;
}

export function routeLength(r: Route): number { return r.points.length ? r.points[r.points.length - 1][0] : 0; }

export function elevationAt(r: Route, d: number): number {
  const pts = r.points;
  if (!pts.length) return 0;
  if (d <= pts[0][0]) return pts[0][1];
  if (d >= pts[pts.length - 1][0]) return pts[pts.length - 1][1];
  let i = 1;
  while (pts[i][0] <= d) i++;
  const [a, b] = [pts[i - 1], pts[i]];
  return a[1] + ((b[1] - a[1]) * (d - a[0])) / (b[0] - a[0]);
}

export function averageGrade(r: Route, from: number, to: number): number {
  const end = Math.min(to, routeLength(r));
  const start = Math.max(Math.min(from, end - 1), 0);
  if (end - start < 1e-6) return 0;
  return ((elevationAt(r, end) - elevationAt(r, start)) / (end - start)) * 100;
}

export function estimateRoute(r: Route, powerW: number, p: RiderParams): { timeS: number; avgKmh: number } {
  const total = routeLength(r);
  const step = Math.max(r.resolution_m, 10);
  let d = 0, t = 0;
  while (d < total) {
    const len = Math.min(step, total - d);
    const v = Math.max(speedForPower(powerW, averageGrade(r, d, d + len), p), 0.5);
    t += len / v;
    d += len;
  }
  return { timeS: t, avgKmh: t > 0 ? (total / t) * 3.6 : 0 };
}

export interface RouteStats { lengthM: number; gainM: number; maxClimb: number; maxDescent: number }

export function routeStats(r: Route): RouteStats {
  let gain = 0, up = 0, down = 0;
  for (let i = 1; i < r.points.length; i++) {
    const dz = r.points[i][1] - r.points[i - 1][1];
    const g = (dz / (r.points[i][0] - r.points[i - 1][0])) * 100;
    gain += Math.max(dz, 0); up = Math.max(up, g); down = Math.min(down, g);
  }
  return { lengthM: routeLength(r), gainM: gain, maxClimb: up, maxDescent: down };
}
