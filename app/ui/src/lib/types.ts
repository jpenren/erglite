// SPDX-License-Identifier: MIT OR Apache-2.0
// Mirrors the JSON written by erglite-core (workout schema version 1) and the IPC payloads.

export type End = { time_s: number } | { distance_m: number };
export interface PowerTarget {
  pct_ftp?: number; watts?: number;
  from_pct_ftp?: number; to_pct_ftp?: number; from_watts?: number; to_watts?: number;
}
export type GradeControl = 'from_route' | { fixed: { pct: number } } | { ramp: { from_pct: number; to_pct: number } };
export type Control = { power: PowerTarget } | { grade: GradeControl } | 'free';
export interface Segment { label?: string; end: End; control: Control; cadence_rpm?: number; message?: string }
export interface RepeatBlock { repeat: number; segments: Segment[] }
export type SegmentNode = Segment | RepeatBlock;
export interface Route { resolution_m: number; points: [number, number][] }
export type WorkoutKind = 'time' | 'route' | 'mixed';
export interface Workout {
  schema_version: 1; name: string; description?: string; kind: WorkoutKind;
  route?: Route; segments: SegmentNode[];
}

export interface Profile {
  ftp_w: number; weight_kg: number; bike_kg: number; max_hr: number | null;
  units: 'metric' | 'imperial'; language: 'en' | 'es'; crr: number; cda_m2: number;
}
export interface Capabilities {
  name: string; supports_erg: boolean; supports_simulation: boolean; supports_resistance: boolean;
  min_power_w: number | null; max_power_w: number | null;
  reports_cadence: boolean; reports_power: boolean; reports_heart_rate: boolean;
}
export interface Device { id: string; name: string; role: 'trainer' | 'heart_rate' | 'power' | 'cadence'; rssi: number | null }
export type ConnectionState = 'disconnected' | 'connecting' | 'connected';
export type SessionStatus = 'idle' | 'countdown' | 'running' | 'paused' | 'finished';

export interface TickEvent {
  status: SessionStatus;
  segment_index: number; segment_count: number;
  elapsed_s: number; distance_m: number; route_pos_m: number; segment_progress: number;
  target_power_w: number | null; grade_pct: number | null; segment_remaining_s: number | null;
  power_w: number | null; cadence_rpm: number | null; hr_bpm: number | null; speed_kmh: number | null;
  connection: ConnectionState; countdown_s: number; notices: string[];
}
export interface Summary {
  duration_s: number; avg_power_w: number; max_power_w: number;
  normalized_power_w: number | null; intensity_factor: number | null; tss: number | null;
  zone_seconds: number[]; distance_m: number; elevation_gain_m: number; avg_hr_bpm: number | null;
}
export interface Sample {
  t_ms: number; power_w: number | null; cadence_rpm: number | null; hr_bpm: number | null;
  speed_kmh: number | null; distance_m: number | null; altitude_m: number | null;
  grade_pct: number | null; target_power_w: number | null;
}
export interface WorkoutRecord { id: string; workout: Workout; source: string; updated_at: string }
export interface SessionRecord { id: string; started_at: string; status: string; workout_name: string; summary: Summary | null }

/** Everything the UI needs from the engine. Implemented by the Tauri shell and by the browser mock. */
export interface Backend {
  readonly kind: 'tauri' | 'mock';
  getProfile(): Promise<Profile>;
  setProfile(p: Profile): Promise<void>;
  listWorkouts(): Promise<WorkoutRecord[]>;
  saveWorkout(id: string | null, w: Workout, source?: string): Promise<string>;
  deleteWorkout(id: string): Promise<void>;
  importWorkout(fileName: string, text: string, simplifyEveryM?: number): Promise<WorkoutRecord>;
  scanDevices(): Promise<Device[]>;
  connectTrainer(id: string): Promise<Capabilities>;
  disconnectTrainer(): Promise<void>;
  startSession(workoutId: string | null, workout: Workout, options: { difficulty: number }): Promise<string>;
  pauseSession(): Promise<void>;
  resumeSession(): Promise<void>;
  stopSession(): Promise<Summary>;
  skipSegment(): Promise<void>;
  adjustIntensity(deltaPct: number): Promise<void>;
  setDifficulty(scale: number): Promise<void>;
  listSessions(): Promise<SessionRecord[]>;
  sessionSamples(id: string): Promise<Sample[]>;
  exportSession(id: string, format: 'tcx'): Promise<string>;
  onTick(cb: (t: TickEvent) => void): () => void;
  /** Mock only: accelerate simulated time. */
  setDemoSpeed?(factor: number): void;
}
