// SPDX-License-Identifier: MIT OR Apache-2.0
import type { Control, End, Profile, RepeatBlock, Route, Segment, SegmentNode, Workout } from './types';

export const isRepeat = (n: SegmentNode): n is RepeatBlock => 'repeat' in n;
export const timeOf = (e: End): number | null => ('time_s' in e ? e.time_s : null);
export const distOf = (e: End): number | null => ('distance_m' in e ? e.distance_m : null);

export function flatten(w: Workout): Segment[] {
  const out: Segment[] = [];
  for (const n of w.segments) {
    if (isRepeat(n)) for (let i = 0; i < n.repeat; i++) out.push(...n.segments);
    else out.push(n);
  }
  return out;
}

export const plannedTimeS = (w: Workout) => flatten(w).reduce((s, g) => s + (timeOf(g.end) ?? 0), 0);
export const plannedDistanceM = (w: Workout) => flatten(w).reduce((s, g) => s + (distOf(g.end) ?? 0), 0);

/** Power range of a segment as percent of FTP. */
export function segmentPct(c: Control, ftp: number): { from: number; to: number } | null {
  if (typeof c === 'string' || !('power' in c)) return null;
  const p = c.power;
  const w2p = (w: number) => (w / ftp) * 100;
  if (p.pct_ftp != null) return { from: p.pct_ftp, to: p.pct_ftp };
  if (p.watts != null) return { from: w2p(p.watts), to: w2p(p.watts) };
  if (p.from_pct_ftp != null && p.to_pct_ftp != null) return { from: p.from_pct_ftp, to: p.to_pct_ftp };
  if (p.from_watts != null && p.to_watts != null) return { from: w2p(p.from_watts), to: w2p(p.to_watts) };
  return null;
}

export interface Block { durationS: number; from: number; to: number; label?: string; ref?: number }

/** Time blocks for the profile chart (percent of FTP). Free segments are drawn at 50 %. */
export function timeBlocks(w: Workout, ftp: number): Block[] {
  return flatten(w)
    .filter((s) => timeOf(s.end) != null)
    .map((s) => {
      const p = segmentPct(s.control, ftp) ?? { from: 50, to: 50 };
      return { durationS: timeOf(s.end) ?? 0, from: p.from, to: p.to, label: s.label };
    });
}

/** Per-second target power (W) of a time workout, used for TSS estimates. */
export function powerSeries(w: Workout, ftp: number): number[] {
  const out: number[] = [];
  for (const b of timeBlocks(w, ftp)) {
    for (let i = 0; i < b.durationS; i++) {
      const pct = b.from + ((b.to - b.from) * i) / Math.max(b.durationS - 1, 1);
      out.push((ftp * pct) / 100);
    }
  }
  return out;
}

export function normalizedPower(p: number[]): number | null {
  const W = 30;
  if (p.length < W) return null;
  let sum = 0;
  for (let i = 0; i < W; i++) sum += p[i];
  let acc = (sum / W) ** 4, n = 1;
  for (let i = W; i < p.length; i++) { sum += p[i] - p[i - W]; acc += (sum / W) ** 4; n++; }
  return (acc / n) ** 0.25;
}

export function estimateTss(w: Workout, ftp: number): number | null {
  const series = powerSeries(w, ftp);
  const np = normalizedPower(series);
  return np == null ? null : (series.length * np * (np / ftp)) / (ftp * 3600) * 100;
}

export const zoneForPct = (pct: number): number =>
  pct < 55.5 ? 1 : pct < 75.5 ? 2 : pct < 90.5 ? 3 : pct < 105.5 ? 4 : pct < 120.5 ? 5 : pct < 150.5 ? 6 : 7;
export const zoneName = (z: number) => ['Recovery', 'Endurance', 'Tempo', 'Threshold', 'VO2max', 'Anaerobic', 'Neuromuscular'][z - 1];
export const zoneColor = (z: number) => `var(--zone-${z})`;

export const gradeBand = (g: number): number => (g < -5 ? 0 : g < -2 ? 1 : g <= 2 ? 2 : g <= 4 ? 3 : g <= 6 ? 4 : 5);
export const GRADE_VARS = ['--grade-descent-steep', '--grade-descent', '--grade-flat', '--grade-climb', '--grade-climb-hard', '--grade-climb-very-hard'];
export const gradeColor = (g: number) => `var(${GRADE_VARS[gradeBand(g)]})`;

export function fmtDuration(s: number): string {
  const t = Math.max(0, Math.round(s));
  const h = Math.floor(t / 3600), m = Math.floor((t % 3600) / 60), sec = t % 60;
  return h > 0 ? `${h}:${String(m).padStart(2, '0')}:${String(sec).padStart(2, '0')}` : `${m}:${String(sec).padStart(2, '0')}`;
}
export const fmtKm = (m: number, digits = 1) => (m / 1000).toFixed(digits);

export function newTimeWorkout(name = 'New workout'): Workout {
  return {
    schema_version: 1, name, kind: 'time',
    segments: [
      { label: 'Warm-up', end: { time_s: 600 }, control: { power: { from_pct_ftp: 45, to_pct_ftp: 65 } } },
      { label: 'Steady', end: { time_s: 1200 }, control: { power: { pct_ftp: 75 } } },
      { label: 'Cool-down', end: { time_s: 300 }, control: { power: { from_pct_ftp: 60, to_pct_ftp: 40 } } },
    ],
  };
}

export type RouteTemplate = 'flat' | 'rolling' | 'climb' | 'valley' | 'stairs';

/** Build an elevation profile from a template. Distances in meters, one point per `step`. */
export function routeTemplate(kind: RouteTemplate, lengthM: number, step = 500, base = 100): Route {
  const pts: [number, number][] = [];
  for (let d = 0; d <= lengthM + 1; d += step) {
    const x = Math.min(d / lengthM, 1);
    const e = {
      flat: 0,
      rolling: 28 * Math.sin(x * Math.PI * 4) + 18 * Math.sin(x * Math.PI * 9),
      climb: 220 * x ** 1.1,
      valley: 90 * Math.sin(x * Math.PI) * -1 + 90 * x,
      stairs: Math.floor(x * 6) * 35 + (x * 6 - Math.floor(x * 6)) * 12,
    }[kind];
    pts.push([Math.min(d, lengthM), Math.round(base + e)]);
  }
  return { resolution_m: 100, points: pts };
}

export function routeWorkout(name: string, route: Route): Workout {
  const length = route.points[route.points.length - 1][0];
  return {
    schema_version: 1, name, kind: 'route', route,
    segments: [{ label: 'Circuit', end: { distance_m: length }, control: { grade: 'from_route' } }],
  };
}

export const ftpOf = (p: Profile) => p.ftp_w;
