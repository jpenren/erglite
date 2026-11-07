// SPDX-License-Identifier: MIT OR Apache-2.0
import type { Backend } from '../types';
import { createTauriBackend } from './tauri';
import { MockBackend } from './mock';

/** Inside the Tauri shell use the Rust engine; in a plain browser use the mock. */
export const backend: Backend =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window ? createTauriBackend() : new MockBackend();
