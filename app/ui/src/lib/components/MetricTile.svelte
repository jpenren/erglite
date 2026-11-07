<script lang="ts">
  import type { Snippet } from 'svelte';
  let { label, value, unit = '', accent = '', size = 'lg', children }: {
    label: string; value: string | number; unit?: string; accent?: string; size?: 'xl' | 'lg' | 'md'; children?: Snippet;
  } = $props();
</script>

<div class="tile {size}" style:--accent-line={accent || 'transparent'}>
  <div class="kicker">{label}</div>
  <div class="value num">{value}{#if unit}<span class="unit">{unit}</span>{/if}</div>
  <div class="extra">{@render children?.()}</div>
</div>

<style>
  .tile { position: relative; background: linear-gradient(180deg, var(--surface-raised), var(--surface)); border: 1px solid var(--border-subtle); border-radius: var(--radius-tile); padding: 18px 24px 20px; box-shadow: var(--shadow); overflow: hidden; display: flex; flex-direction: column; gap: 4px; min-width: 0; }
  .tile::after { content: ''; position: absolute; left: 0; right: 0; bottom: 0; height: 5px; background: var(--accent-line); }
  .value { font-weight: 700; display: flex; align-items: baseline; gap: 8px; white-space: nowrap; }
  .xl .value { font-size: clamp(96px, 11vw, 150px); }
  .lg .value { font-size: clamp(64px, 7vw, 100px); }
  .md .value { font-size: clamp(48px, 5vw, 72px); }
  .unit { font-size: 0.32em; color: var(--text-muted); font-weight: 600; }
  .extra { font-size: 16px; color: var(--text-secondary); min-height: 22px; }
</style>
