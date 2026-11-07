// SPDX-License-Identifier: MIT OR Apache-2.0
// In-browser backend used for `npm run dev` and the single-file demo. It mimics the Rust engine
// (see erglite-core/src/engine.rs) closely enough to develop and review the UI without a trainer.
// The real engine, trainer I/O and SQLite storage live in the Rust crates.
import type { Backend, Capabilities, Device, Profile, Route, Sample, Segment, SessionRecord, SessionStatus, Summary, TickEvent, Workout, WorkoutRecord } from '../types';
import { averageGrade, elevationAt, riderFromProfile, speedForPower } from '../physics';
import { distOf, flatten, normalizedPower, powerSeries, routeWorkout, routeTemplate, timeOf, zoneForPct, segmentPct } from '../workout';

const DEFAULT_PROFILE: Profile = { ftp_w: 250, weight_kg: 74, bike_kg: 9, max_hr: 186, units: 'metric', language: 'en', crr: 0.004, cda_m2: 0.35 };

const ROLLING: Route = { resolution_m: 100, points: [[0,100],[1000,100],[2000,115],[3000,140],[4000,170],[5000,210],[6000,240],[7000,210],[8000,170],[9000,220],[10000,150]] };

const steady = (label: string, s: number, pct: number): Segment => ({ label, end: { time_s: s }, control: { power: { pct_ftp: pct } } });
const ramp = (label: string, s: number, a: number, b: number): Segment => ({ label, end: { time_s: s }, control: { power: { from_pct_ftp: a, to_pct_ftp: b } } });

function seedWorkouts(): Workout[] {
  const t = (name: string, description: string, segments: Workout['segments']): Workout => ({ schema_version: 1, name, description, kind: 'time', segments });
  return [
    t('Sweet Spot 3×8', 'Three 8 minute efforts at 95 % FTP.', [ramp('Warm-up', 840, 45, 70), { repeat: 3, segments: [steady('Work', 480, 95), steady('Rest', 240, 55)] }, ramp('Cool-down', 720, 60, 45)]),
    t('VO2max 4×4', 'Four hard 4 minute efforts at 115 % FTP.', [ramp('Warm-up', 600, 45, 75), { repeat: 4, segments: [steady('Work', 240, 115), steady('Rest', 240, 50)] }, ramp('Cool-down', 480, 60, 40)]),
    t('Endurance 60', 'Aerobic base ride with a gentle ramp.', [ramp('Warm-up', 600, 45, 65), steady('Endurance', 2400, 68), ramp('Cool-down', 600, 65, 40)]),
    t('Tempo 2×20', 'Two long tempo blocks at 85 % FTP.', [ramp('Warm-up', 720, 45, 70), { repeat: 2, segments: [steady('Tempo', 1200, 85), steady('Rest', 300, 50)] }, ramp('Cool-down', 480, 60, 40)]),
    routeWorkout('10 km rolling', { ...ROLLING }),
    { ...routeWorkout('Mountain climb 8 km', routeTemplate('climb', 8000, 500, 300)), description: 'A steady climb to the summit.' },
    routeWorkout('Flat 20 km', routeTemplate('flat', 20000, 1000, 50)),
  ];
}

const clamp = (v: number, a: number, b: number) => Math.min(b, Math.max(a, v));
const wait = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Deterministic pseudo-noise so seeded history looks real. */
const noise = (i: number, amp: number) => Math.sin(i * 12.9898) * Math.cos(i * 4.1414) * amp;

function summarize(samples: Sample[], ftp: number): Summary {
  const powers = samples.map((s) => s.power_w ?? 0);
  const zones = [0, 0, 0, 0, 0, 0, 0];
  for (const p of powers) zones[zoneForPct((p / ftp) * 100) - 1]++;
  const np = normalizedPower(powers);
  const duration = samples.length ? (samples[samples.length - 1].t_ms - samples[0].t_ms) / 1000 : 0;
  const alts = samples.map((s) => s.altitude_m).filter((a): a is number => a != null);
  let gain = 0;
  for (let i = 1; i < alts.length; i++) gain += Math.max(alts[i] - alts[i - 1], 0);
  const hrs = samples.map((s) => s.hr_bpm).filter((h): h is number => h != null);
  return {
    duration_s: duration, avg_power_w: powers.length ? powers.reduce((a, b) => a + b, 0) / powers.length : 0,
    max_power_w: Math.max(0, ...powers), normalized_power_w: np,
    intensity_factor: np == null ? null : np / ftp, tss: np == null ? null : (duration * np * (np / ftp)) / (ftp * 3600) * 100,
    zone_seconds: zones, distance_m: Math.max(0, ...samples.map((s) => s.distance_m ?? 0)), elevation_gain_m: gain,
    avg_hr_bpm: hrs.length ? hrs.reduce((a, b) => a + b, 0) / hrs.length : null,
  };
}

class MockSession {
  segs: Segment[]; route?: Route; status: SessionStatus = 'countdown'; countdown = 3;
  idx = 0; segEl = 0; segDist = 0; routePos = 0; total = 0; dist = 0;
  intensity = 0; difficulty: number; target: number | null = null; grade: number | null = null;
  power = 0; speed = 0; hr = 64; samples: Sample[] = []; notices: string[] = [];
  constructor(public workout: Workout, public profile: Profile, difficulty: number) {
    this.segs = flatten(workout); this.route = workout.route; this.difficulty = difficulty;
  }
  private progress(s: Segment): number {
    const t = timeOf(s.end), d = distOf(s.end);
    return clamp(t != null ? this.segEl / t : this.segDist / (d ?? 1), 0, 1);
  }
  private control(dt: number) {
    const seg = this.segs[this.idx];
    if (!seg) return;
    const c = seg.control;
    this.target = null;
    if (typeof c !== 'string' && 'power' in c) {
      const r = segmentPct(c, this.profile.ftp_w);
      if (r) this.target = Math.round(((this.profile.ftp_w * (r.from + (r.to - r.from) * this.progress(seg))) / 100) * (1 + this.intensity / 100));
      this.grade = null;
    } else if (typeof c !== 'string' && 'grade' in c) {
      const raw = c.grade === 'from_route' && this.route ? averageGrade(this.route, this.routePos, this.routePos + 30)
        : typeof c.grade === 'object' && 'fixed' in c.grade ? c.grade.fixed.pct : 0;
      const goal = clamp(raw * this.difficulty, -10, 20);
      this.grade = this.grade == null ? goal : this.grade + clamp(goal - this.grade, -dt, dt);
    }
  }
  step(): TickEvent {
    this.notices = [];
    const rider = riderFromProfile(this.profile);
    if (this.status === 'countdown') {
      this.countdown -= 1;
      if (this.countdown <= 0) { this.status = 'running'; this.notices.push('started'); this.control(1); }
    } else if (this.status === 'running') {
      const v = this.speed / 3.6;
      this.total += 1; this.segEl += 1; this.dist += v;
      const seg = this.segs[this.idx];
      let done = false;
      if (seg) {
        const t = timeOf(seg.end), d = distOf(seg.end);
        if (t != null) done = this.segEl >= t; else if (d != null) { this.segDist += v; this.routePos += v; done = this.segDist >= d; }
      }
      if (done) {
        this.idx += 1; this.segEl = 0; this.segDist = 0; this.grade = null;
        if (this.idx >= this.segs.length) { this.status = 'finished'; this.notices.push('finished'); }
        else this.notices.push('segment');
      }
      if (this.status === 'running') this.control(1);
    }
    const running = this.status === 'running';
    const riderPower = this.profile.ftp_w * 0.74 + noise(this.total, 6);
    const goalPower = !running ? 0 : this.target != null ? this.target + noise(this.total, 2) : riderPower;
    this.power += (goalPower - this.power) * 0.45;
    const g = this.grade ?? 0;
    this.speed += (speedForPower(Math.max(this.power, 0), g, rider) * 3.6 - this.speed) * 0.25;
    this.hr += ((running ? 72 + this.power * 0.36 : 62) - this.hr) * 0.06;
    const cadence = running ? 88 + noise(this.total, 3) : 0;
    if (running) {
      this.samples.push({
        t_ms: this.total * 1000, power_w: Math.round(this.power), cadence_rpm: Math.round(cadence), hr_bpm: Math.round(this.hr),
        speed_kmh: this.speed, distance_m: this.dist, altitude_m: this.route ? elevationAt(this.route, this.routePos) : null,
        grade_pct: this.grade, target_power_w: this.target,
      });
    }
    const seg = this.segs[Math.min(this.idx, this.segs.length - 1)];
    const tEnd = seg ? timeOf(seg.end) : null;
    return {
      status: this.status, segment_index: Math.min(this.idx, this.segs.length - 1), segment_count: this.segs.length,
      elapsed_s: this.total, distance_m: this.dist, route_pos_m: this.routePos, segment_progress: seg ? this.progress(seg) : 1,
      target_power_w: this.target, grade_pct: this.grade, segment_remaining_s: tEnd != null ? Math.max(tEnd - this.segEl, 0) : null,
      power_w: Math.round(this.power), cadence_rpm: Math.round(cadence), hr_bpm: Math.round(this.hr), speed_kmh: this.speed,
      connection: 'connected', countdown_s: Math.max(this.countdown, 0), notices: this.notices,
    };
  }
}

export class MockBackend implements Backend {
  readonly kind = 'mock' as const;
  private profile = { ...DEFAULT_PROFILE };
  private workouts = new Map<string, WorkoutRecord>();
  private sessions: SessionRecord[] = [];
  private samples = new Map<string, Sample[]>();
  private listeners = new Set<(t: TickEvent) => void>();
  private session: MockSession | null = null;
  private sessionId = '';
  private sessionWorkoutId: string | null = null;
  private timer: ReturnType<typeof setInterval> | null = null;
  private speedFactor = 1;
  private connected = false;
  private seq = 0;

  constructor() {
    seedWorkouts().forEach((w) => this.put(null, w, 'builtin'));
    const list = [...this.workouts.values()];
    [[0, 3, 1.0], [1, 1, 0.97], [4, 6, 0.92]].forEach(([i, daysAgo, k]) => {
      const rec = list[i]; if (!rec) return;
      const ftp = this.profile.ftp_w;
      const series = rec.workout.kind === 'time' ? powerSeries(rec.workout, ftp) : Array.from({ length: 1500 }, () => ftp * 0.78);
      const samples: Sample[] = series.map((p, n) => ({
        t_ms: n * 1000, power_w: Math.max(0, Math.round(p * k + noise(n, 5))), cadence_rpm: 90, hr_bpm: Math.round(95 + p * 0.3), speed_kmh: 30 + noise(n, 3),
        distance_m: n * 8.3, altitude_m: null, grade_pct: null, target_power_w: Math.round(p),
      }));
      const id = `seed-${i}`;
      this.samples.set(id, samples);
      this.sessions.push({ id, started_at: new Date(Date.now() - daysAgo * 86_400_000).toISOString(), status: 'completed', workout_name: rec.workout.name, summary: summarize(samples, ftp) });
    });
  }

  private put(id: string | null, w: Workout, source: string): string {
    const key = id ?? `w${++this.seq}`;
    this.workouts.set(key, { id: key, workout: structuredClone(w), source, updated_at: new Date().toISOString() });
    return key;
  }
  async getProfile() { return { ...this.profile }; }
  async setProfile(p: Profile) { this.profile = { ...p }; }
  async listWorkouts() { return [...this.workouts.values()].map((r) => structuredClone(r)); }
  async saveWorkout(id: string | null, w: Workout, source = 'user') { return this.put(id, w, source); }
  async deleteWorkout(id: string) { this.workouts.delete(id); }
  async importWorkout(fileName: string, text: string, simplifyEveryM?: number): Promise<WorkoutRecord> {
    const doc = new DOMParser().parseFromString(text, 'application/xml');
    if (doc.querySelector('parsererror')) throw new Error('Invalid XML');
    let w: Workout;
    if (fileName.toLowerCase().endsWith('.gpx')) {
      const trkpts = doc.evaluate('//*[local-name()="trkpt"]', doc, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
      let pts: Array<{ lat: number; lon: number; ele: number }> = [];
      for (let i = 0; i < trkpts.snapshotLength; i++) {
        const p = trkpts.snapshotItem(i);
        pts.push({ lat: +(p.getAttribute('lat') ?? 0), lon: +(p.getAttribute('lon') ?? 0), ele: +(p.querySelector('ele')?.textContent ?? 0) });
      }
      // Simplify points if requested
      if (simplifyEveryM && pts.length > 2) {
        const hav = (a: typeof pts[0], b: typeof pts[0]) => { const r = 6371000, f = Math.PI / 180, s = Math.sin(((b.lat - a.lat) * f) / 2) ** 2 + Math.cos(a.lat * f) * Math.cos(b.lat * f) * Math.sin(((b.lon - a.lon) * f) / 2) ** 2; return 2 * r * Math.asin(Math.sqrt(s)); };
        const dist = [0]; for (let i = 1; i < pts.length; i++) dist.push(dist[i - 1] + hav(pts[i - 1], pts[i]));
        const simplified: typeof pts = [pts[0]];
        const simplifiedDist = [0];
        let nextTarget = simplifyEveryM;
        for (let i = 1; i < pts.length - 1; i++) {
          if (dist[i] >= nextTarget) {
            simplified.push(pts[i]);
            simplifiedDist.push(dist[i]);
            nextTarget += simplifyEveryM;
          }
        }
        simplified.push(pts[pts.length - 1]);
        simplifiedDist.push(dist[dist.length - 1]);
        pts = simplified;
      }
      if (pts.length < 2) throw new Error('GPX needs at least two track points');
      if (pts.length < 2) throw new Error('GPX needs at least two track points');
      const hav = (a: typeof pts[0], b: typeof pts[0]) => { const r = 6371000, f = Math.PI / 180, s = Math.sin(((b.lat - a.lat) * f) / 2) ** 2 + Math.cos(a.lat * f) * Math.cos(b.lat * f) * Math.sin(((b.lon - a.lon) * f) / 2) ** 2; return 2 * r * Math.asin(Math.sqrt(s)); };
      const dist = [0]; for (let i = 1; i < pts.length; i++) dist.push(dist[i - 1] + hav(pts[i - 1], pts[i]));
      const sm = pts.map((_, i) => { const a = Math.max(0, i - 4), b = Math.min(pts.length, i + 5); return pts.slice(a, b).reduce((s, p) => s + p.ele, 0) / (b - a); });
      const route: Route = { resolution_m: 100, points: [] };
      let j = 0; for (let d = 0; d <= dist[dist.length - 1]; d += 100) { while (j < dist.length - 2 && dist[j + 1] < d) j++; const t = clamp((d - dist[j]) / Math.max(dist[j + 1] - dist[j], 1e-9), 0, 1); route.points.push([d, Math.round((sm[j] + (sm[j + 1] - sm[j]) * t) * 10) / 10]); }
      w = routeWorkout(fileName.replace(/\.gpx$/i, ''), route);
    } else {
      const segs: Workout['segments'] = [];
      const n = (e: Element, k: string) => parseFloat(e.getAttribute(k) ?? 'NaN');
      for (const e of [...doc.querySelectorAll('workout > *')]) {
        const tag = e.tagName;
        if (['Warmup', 'Cooldown', 'Ramp'].includes(tag)) segs.push(ramp(tag === 'Warmup' ? 'Warm-up' : tag === 'Cooldown' ? 'Cool-down' : 'Ramp', n(e, 'Duration'), n(e, 'PowerLow') * 100, n(e, 'PowerHigh') * 100));
        else if (tag === 'SteadyState') segs.push(steady('Steady', n(e, 'Duration'), n(e, 'Power') * 100));
        else if (tag === 'IntervalsT') segs.push({ repeat: n(e, 'Repeat'), segments: [steady('Work', n(e, 'OnDuration'), n(e, 'OnPower') * 100), steady('Rest', n(e, 'OffDuration'), n(e, 'OffPower') * 100)] });
        else if (tag === 'FreeRide') segs.push({ label: 'Free ride', end: { time_s: n(e, 'Duration') }, control: 'free' });
      }
      if (!segs.length) throw new Error('No supported workout elements found');
      w = { schema_version: 1, name: doc.querySelector('name')?.textContent ?? fileName, description: doc.querySelector('description')?.textContent ?? undefined, kind: 'time', segments: segs };
    }
    const id = this.put(null, w, fileName.toLowerCase().endsWith('.gpx') ? 'gpx' : 'zwo');
    return this.workouts.get(id)!;
  }
  async scanDevices(): Promise<Device[]> {
    await wait(900);
    return [
      { id: 'sim-trainer', name: 'Virtual Trainer', role: 'trainer', rssi: -52 },
      { id: 'sim-hr', name: 'Heart rate strap', role: 'heart_rate', rssi: -61 },
      { id: 'sim-pm', name: 'Power meter pedals', role: 'power', rssi: -78 },
    ];
  }
  async connectTrainer(): Promise<Capabilities> {
    await wait(700); this.connected = true;
    return { name: 'Virtual Trainer', supports_erg: true, supports_simulation: true, supports_resistance: false, min_power_w: 0, max_power_w: 2000, reports_cadence: true, reports_power: true, reports_heart_rate: true };
  }
  async disconnectTrainer() { this.connected = false; }
  setDemoSpeed(f: number) { this.speedFactor = f; if (this.timer) this.schedule(); }
  private schedule() { if (this.timer) clearInterval(this.timer); this.timer = setInterval(() => this.tick(), 1000 / this.speedFactor); }
  private tick() { if (!this.session) return; const e = this.session.step(); this.listeners.forEach((l) => l(e)); if (e.status === 'finished') this.halt(); }
  private halt() { if (this.timer) clearInterval(this.timer); this.timer = null; }
  async startSession(workoutId: string | null, workout: Workout, options: { difficulty: number }) {
    this.session = new MockSession(workout, this.profile, options.difficulty);
    this.sessionId = `s${Date.now()}`; this.sessionWorkoutId = workoutId;
    this.schedule(); this.tick();
    return this.sessionId;
  }
  async pauseSession() { if (this.session?.status === 'running') this.session.status = 'paused'; }
  async resumeSession() { if (this.session?.status === 'paused') { this.session.status = 'countdown'; this.session.countdown = 3; } }
  async skipSegment() { const s = this.session; if (s && s.idx < s.segs.length - 1) { s.idx += 1; s.segEl = 0; s.segDist = 0; s.grade = null; } else if (s) { s.status = 'finished'; } }
  async adjustIntensity(d: number) { if (this.session) this.session.intensity = clamp(this.session.intensity + d, -50, 50); }
  async setDifficulty(k: number) { if (this.session) this.session.difficulty = clamp(k, 0, 1.5); }
  async stopSession(): Promise<Summary> {
    this.halt();
    const s = this.session; if (!s) throw new Error('No session');
    const summary = summarize(s.samples, this.profile.ftp_w);
    this.samples.set(this.sessionId, s.samples);
    this.sessions.unshift({ id: this.sessionId, started_at: new Date().toISOString(), status: 'completed', workout_name: s.workout.name, summary });
    this.session = null;
    return summary;
  }
  async listSessions() { return structuredClone(this.sessions); }
  async sessionSamples(id: string) { return this.samples.get(id) ?? []; }
  async exportSession(id: string) {
    const rows = (this.samples.get(id) ?? []).map((s) => `<Trackpoint><Time>${new Date(Date.now() + s.t_ms).toISOString()}</Time><DistanceMeters>${(s.distance_m ?? 0).toFixed(1)}</DistanceMeters><Extensions><ns3:TPX><ns3:Watts>${s.power_w ?? 0}</ns3:Watts></ns3:TPX></Extensions></Trackpoint>`).join('\n');
    return `<?xml version="1.0" encoding="UTF-8"?>\n<TrainingCenterDatabase xmlns="http://www.garmin.com/xmlschemas/TrainingCenterDatabase/v2" xmlns:ns3="http://www.garmin.com/xmlschemas/ActivityExtension/v2"><Activities><Activity Sport="Biking"><Id>${new Date().toISOString()}</Id><Lap StartTime="${new Date().toISOString()}"><Calories>0</Calories><Intensity>Active</Intensity><TriggerMethod>Manual</TriggerMethod><Track>\n${rows}\n</Track></Lap></Activity></Activities></TrainingCenterDatabase>\n`;
  }
  onTick(cb: (t: TickEvent) => void) { this.listeners.add(cb); return () => { this.listeners.delete(cb); }; }
  get isConnected() { return this.connected; }
  get currentWorkoutId() { return this.sessionWorkoutId; }
}
