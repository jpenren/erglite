// SPDX-License-Identifier: MIT OR Apache-2.0
// Application state (Svelte 5 runes). All data access goes through the Backend interface.
import { backend } from './backend';
import { i18n } from './i18n.svelte';
import type { Capabilities, ConnectionState, Profile, SessionRecord, Summary, TickEvent, Workout, WorkoutRecord } from './types';

export const app = $state({
  ready: false,
  profile: { ftp_w: 250, weight_kg: 74, bike_kg: 9, max_hr: 186, units: 'metric', language: 'en', crr: 0.004, cda_m2: 0.35 } as Profile,
  workouts: [] as WorkoutRecord[],
  sessions: [] as SessionRecord[],
  trainer: { state: 'disconnected' as ConnectionState, caps: null as Capabilities | null },
  theme: 'dark' as 'dark' | 'light',
  session: null as null | {
    workout: Workout; workoutId: string | null; tick: TickEvent | null; trace: number[];
    difficulty: number; id: string | null;
  },
  lastSummary: null as null | { id: string; summary: Summary },
  toast: '',
});

export const router = $state({ name: 'home', id: '' });

function parseHash() {
  const [name, id = ''] = location.hash.replace(/^#\/?/, '').split('/');
  router.name = name || 'home';
  router.id = decodeURIComponent(id);
}
export const go = (path: string) => { location.hash = `#/${path}`; };

let toastTimer: ReturnType<typeof setTimeout> | undefined;
export function toast(message: string) {
  app.toast = message;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (app.toast = ''), 2800);
}

export async function refresh() {
  [app.workouts, app.sessions] = await Promise.all([backend.listWorkouts(), backend.listSessions()]);
}

export function applyTheme(theme: 'dark' | 'light') {
  app.theme = theme;
  document.documentElement.dataset.theme = theme;
  try { localStorage.setItem('erglite.theme', theme); } catch { /* storage unavailable */ }
}

export async function init() {
  parseHash();
  window.addEventListener('hashchange', parseHash);
  app.profile = await backend.getProfile();
  i18n.lang = app.profile.language;
  let saved: string | null = null;
  try { saved = localStorage.getItem('erglite.theme'); } catch { /* storage unavailable */ }
  applyTheme(saved === 'light' ? 'light' : 'dark');
  await refresh();
  backend.onTick(onTick);
  app.ready = true;
}

export async function saveProfile(p: Profile) {
  app.profile = p;
  i18n.lang = p.language;
  await backend.setProfile(p);
}

export async function connectTrainer(id: string) {
  app.trainer.state = 'connecting';
  try {
    app.trainer.caps = await backend.connectTrainer(id);
    app.trainer.state = 'connected';
  } catch (e) {
    app.trainer.state = 'disconnected';
    throw e;
  }
}

export async function disconnectTrainer() {
  await backend.disconnectTrainer();
  app.trainer.state = 'disconnected';
  app.trainer.caps = null;
}

export async function startRide(workoutId: string | null, workout: Workout, difficulty = 1) {
  if (app.trainer.state !== 'connected') {
    if (backend.kind === 'mock') await connectTrainer('sim-trainer');
    else { go('devices'); return; }
  }
  app.session = { workout, workoutId, tick: null, trace: [], difficulty, id: null };
  go('session');
  app.session.id = await backend.startSession(workoutId, workout, { difficulty });
}

function onTick(t: TickEvent) {
  const s = app.session;
  if (!s) return;
  s.tick = t;
  app.trainer.state = t.connection;
  if (t.status === 'running' && t.power_w != null) {
    s.trace.push(t.power_w);
    if (s.trace.length > 180) s.trace.shift();
  }
  if (t.status === 'finished') void finishRide();
}

let finishing = false;
export async function finishRide() {
  const s = app.session;
  if (!s || finishing) return;
  finishing = true;
  const id = s.id ?? '';
  let summary: Summary;
  try { summary = await backend.stopSession(); } finally { finishing = false; }
  app.session = null;
  app.lastSummary = { id, summary };
  await refresh();
  go(`summary/${id}`);
}

export const freeRideWorkout = (pct = 65): Workout => ({
  schema_version: 1, name: 'Free ride', kind: 'time',
  segments: [{ label: 'Free ride', end: { time_s: 3600 }, control: { power: { pct_ftp: pct } } }],
});

/** In-app confirmation dialog (native `confirm()` is unavailable in sandboxed frames). */
export const dialog = $state({ open: false, message: '', resolve: null as null | ((v: boolean) => void) });
export function ask(message: string): Promise<boolean> {
  return new Promise((resolve) => { dialog.message = message; dialog.open = true; dialog.resolve = resolve; });
}
export function answer(v: boolean) { dialog.open = false; dialog.resolve?.(v); dialog.resolve = null; }

/** Three-option dialog for GPX import: 'cancel' | 'original' | 'simplify' */
export const gpxDialog = $state({ open: false, message: '', resolve: null as null | ((v: 'cancel' | 'original' | 'simplify') => void) });
export function askGpx(message: string): Promise<'cancel' | 'original' | 'simplify'> {
  return new Promise((resolve) => { gpxDialog.message = message; gpxDialog.open = true; gpxDialog.resolve = resolve; });
}
export function answerGpx(v: 'cancel' | 'original' | 'simplify') { gpxDialog.open = false; gpxDialog.resolve?.(v); gpxDialog.resolve = null; }
