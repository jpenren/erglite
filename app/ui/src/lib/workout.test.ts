import { describe, expect, it } from 'vitest';
import { estimateTss, flatten, normalizedPower, plannedTimeS, timeBlocks, zoneForPct } from './workout';
import type { Workout } from './types';

const sweetSpot: Workout = {
  schema_version: 1, name: 'Sweet Spot 3x8', kind: 'time',
  segments: [
    { end: { time_s: 840 }, control: { power: { from_pct_ftp: 45, to_pct_ftp: 70 } } },
    { repeat: 3, segments: [
      { end: { time_s: 480 }, control: { power: { pct_ftp: 95 } } },
      { end: { time_s: 240 }, control: { power: { pct_ftp: 55 } } },
    ] },
    { end: { time_s: 720 }, control: { power: { from_pct_ftp: 60, to_pct_ftp: 45 } } },
  ],
};

describe('workout helpers', () => {
  it('flattens repeats and sums the planned time', () => {
    expect(flatten(sweetSpot)).toHaveLength(8);
    expect(plannedTimeS(sweetSpot)).toBe(840 + 3 * 720 + 720);
  });
  it('builds chart blocks in percent of FTP', () => {
    const b = timeBlocks(sweetSpot, 250);
    expect(b[0]).toMatchObject({ durationS: 840, from: 45, to: 70 });
    expect(b[1]).toMatchObject({ from: 95, to: 95 });
  });
  it('matches the Rust TSS for a steady hour at FTP', () => {
    const w: Workout = { schema_version: 1, name: 'x', kind: 'time', segments: [{ end: { time_s: 3600 }, control: { power: { pct_ftp: 100 } } }] };
    expect(estimateTss(w, 250)).toBeCloseTo(100, 6);
    expect(normalizedPower([100, 100])).toBeNull();
  });
  it('uses the Coggan zone limits', () => {
    expect([40, 65, 85, 100, 110, 140, 200].map(zoneForPct)).toEqual([1, 2, 3, 4, 5, 6, 7]);
  });
});
