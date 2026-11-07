import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { viteSingleFile } from 'vite-plugin-singlefile';

// `vite build --mode demo` produces one self-contained HTML file that runs on the in-browser mock
// backend. The normal build is consumed by the Tauri shell.
export default defineConfig(({ mode }) => ({
  plugins: [svelte(), ...(mode === 'demo' ? [viteSingleFile()] : [])],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'es2022', assetsInlineLimit: mode === 'demo' ? 100_000_000 : 4096 },
  test: { environment: 'node', include: ['src/**/*.test.ts'] },
}));
