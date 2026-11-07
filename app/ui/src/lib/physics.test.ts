import { describe, expect, it } from 'vitest';
import { estimateRoute, routeStats, speedForPower, type RiderParams } from './physics';
import type { Route } from './types';

// Reference values come from `erglite-cli estimate fixtures/workouts/rolling-10km.json --power 200`.
const ROLLING: Route = { resolution_m: 100, points: [[0,100],[1000,100],[2000,115],[3000,140],[4000,170],[5000,210],[6000,240],[7000,210],[8000,170],[9000,220],[10000,150]] };
const DEFAULT: RiderParams = { massKg: 84, crr: 0.004, cdaM2: 0.35, rho: 1.225, eff: 0.975 };

describe('physics port', () => {
  it('matches the Rust estimator for the rolling 10 km route', () => {
    const e = estimateRoute(ROLLING, 200, DEFAULT);
    expect(Math.abs(e.timeS - 1427)).toBeLessThan(15); // 23:47 in Rust
  });
  it('flat speed at 200 W is plausible', () => {
    const v = speedForPower(200, 0, { ...DEFAULT, massKg: 80, cdaM2: 0.32 }) * 3.6;
    expect(v).toBeGreaterThan(32); expect(v).toBeLessThan(37);
  });
  it('route stats', () => {
    const s = routeStats(ROLLING);
    expect(s.gainM).toBe(190); expect(s.maxClimb).toBeCloseTo(5); expect(s.maxDescent).toBeCloseTo(-7);
  });
});
