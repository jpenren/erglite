// SPDX-License-Identifier: MIT OR Apache-2.0
// Thin IPC layer over the Tauri shell. Command and event names are the contract documented in
// docs/specification.md section 13.4; the Rust side lives in app/src-tauri.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { Backend, TickEvent } from '../types';

export function createTauriBackend(): Backend {
  const call = <T>(cmd: string, args?: Record<string, unknown>) => invoke<T>(cmd, args);
  return {
    kind: 'tauri',
    getProfile: () => call('get_profile'),
    setProfile: (profile) => call('set_profile', { profile }),
    listWorkouts: () => call('list_workouts'),
    saveWorkout: (id, workout, source = 'user') => call('save_workout', { id, workout, source }),
    deleteWorkout: (id) => call('delete_workout', { id }),
    importWorkout: (fileName, text, simplifyEveryM?: number) => call('import_workout', { fileName, text, simplify_every_m: simplifyEveryM }),
    scanDevices: () => call('scan_devices'),
    connectTrainer: (id) => call('connect_trainer', { id }),
    disconnectTrainer: () => call('disconnect_trainer'),
    startSession: (workoutId, workout, options) => call('start_session', { workoutId, workout, options }),
    pauseSession: () => call('pause_session'),
    resumeSession: () => call('resume_session'),
    stopSession: () => call('stop_session'),
    skipSegment: () => call('skip_segment'),
    adjustIntensity: (deltaPct) => call('adjust_intensity', { deltaPct }),
    setDifficulty: (scale) => call('set_difficulty', { scale }),
    listSessions: () => call('list_sessions'),
    sessionSamples: (id) => call('session_samples', { id }),
    exportSession: (id, format) => call('export_session', { id, format }),
    onTick(cb) {
      let stop: (() => void) | null = null;
      let cancelled = false;
      listen<TickEvent>('session:tick', (e) => cb(e.payload)).then((un) => (cancelled ? un() : (stop = un)));
      return () => { cancelled = true; stop?.(); };
    },
  };
}
