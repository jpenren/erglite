<script lang="ts">
  import { zoneColor, zoneForPct, type Block } from '../workout';
  let { blocks, cursorS = null, selected = -1, onselect, height = 160, fill = false, maxPct = 140, showFtp = true }: {
    blocks: Block[]; cursorS?: number | null; selected?: number; onselect?: (ref: number) => void; height?: number; fill?: boolean; maxPct?: number; showFtp?: boolean;
  } = $props();
  const W = 1000;
  const total = $derived(blocks.reduce((s, b) => s + b.durationS, 0) || 1);
  const y = (p: number) => height - (Math.min(Math.max(p, 0), maxPct) / maxPct) * (height - 10);
  const shapes = $derived.by(() => {
    let x = 0;
    return blocks.map((b, i) => {
      const w = (b.durationS / total) * W, x0 = x;
      x += w;
      const mid = (b.from + b.to) / 2;
      return { i, x0, x1: x0 + w, ref: b.ref ?? i, color: zoneColor(zoneForPct(mid)), d: `${x0},${height} ${x0},${y(b.from)} ${x0 + w},${y(b.to)} ${x0 + w},${height}` };
    });
  });
  const cursorX = $derived(cursorS == null ? null : Math.min(cursorS / total, 1) * W);
</script>

<div class="wrap" style:height={fill ? '100%' : `${height}px`}>
  <svg viewBox="0 0 {W} {height}" preserveAspectRatio="none" role="img" aria-label="Workout intensity profile">
    {#if showFtp}<line x1="0" x2={W} y1={y(100)} y2={y(100)} class="ftp" vector-effect="non-scaling-stroke" />{/if}
    {#each shapes as s (s.i)}
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_tabindex -->
      <polygon points={s.d} fill={s.color} class="blk" class:sel={selected === s.ref} class:past={cursorX != null && s.x1 <= cursorX} class:clickable={!!onselect}
        role={onselect ? 'button' : undefined} tabindex={onselect ? 0 : undefined} aria-label={onselect ? `Block ${s.ref + 1}` : undefined}
        onclick={() => onselect?.(s.ref)} onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && onselect?.(s.ref)} vector-effect="non-scaling-stroke" />
    {/each}
    {#if cursorX != null}<line x1={cursorX} x2={cursorX} y1="0" y2={height} class="cursor" vector-effect="non-scaling-stroke" />{/if}
  </svg>
  {#if showFtp}<span class="ftp-label" style:top="calc({(y(100) / height) * 100}% - 10px)">FTP</span>{/if}
</div>

<style>
  .wrap { position: relative; width: 100%; }
  svg { width: 100%; height: 100%; display: block; overflow: visible; }
  .blk { stroke: var(--bg); stroke-width: 2; stroke-linejoin: round; transition: opacity .2s, filter .15s; }
  .blk.past { opacity: 0.38; }
  .blk.clickable { cursor: pointer; }
  .blk.clickable:hover { filter: brightness(1.15); }
  .blk.sel { stroke: var(--text); stroke-width: 2.5; filter: brightness(1.12); }
  .ftp { stroke: var(--text-muted); stroke-width: 1; stroke-dasharray: 5 5; opacity: 0.55; }
  .cursor { stroke: var(--text); stroke-width: 3; }
  .ftp-label { position: absolute; right: 4px; font-size: 11px; letter-spacing: 0.08em; color: var(--text-muted); background: var(--bg); padding: 0 4px; border-radius: 4px; }
</style>
