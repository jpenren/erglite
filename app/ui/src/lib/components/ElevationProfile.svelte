<script lang="ts">
  import { gradeColor } from '../workout';
  import { elevationAt } from '../physics';
  import { t } from '../i18n.svelte';
  type Pt = [number, number];
  let { points, cursorM = null, editable = false, selected = -1, onselect, onchange, height: fixedHeight = 280, fill = false, interactiveHint = '' }: {
    points: Pt[]; cursorM?: number | null; editable?: boolean; selected?: number; onselect?: (i: number) => void;
    onchange?: (pts: Pt[], selected?: number) => void; height?: number; fill?: boolean; interactiveHint?: string;
  } = $props();
  let hh = $state(0);
  const height = $derived(fill && hh > 0 ? hh : fixedHeight);

  let w = $state(900);
  let svg: SVGSVGElement | undefined = $state();
  const padL = 52, padR = 18, padT = 18, padB = 30;
  const len = $derived(points.length ? points[points.length - 1][0] : 1);
  const elevs = $derived(points.map((p) => p[1]));
  const yMin = $derived(Math.floor((Math.min(...elevs) - 20) / 50) * 50);
  const yMax = $derived(Math.max(Math.ceil((Math.max(...elevs) + 30) / 50) * 50, yMin + 100));
  const yStep = $derived(yMax - yMin > 300 ? 100 : 50);
  const x = (d: number) => padL + (d / len) * (w - padL - padR);
  const y = (e: number) => padT + (1 - (e - yMin) / (yMax - yMin)) * (height - padT - padB);
  const ex = (px: number) => ((px - padL) / (w - padL - padR)) * len;
  const ey = (py: number) => yMin + (1 - (py - padT) / (height - padT - padB)) * (yMax - yMin);
  const segs = $derived(points.slice(1).map((p, i) => {
    const a = points[i], g = ((p[1] - a[1]) / (p[0] - a[0])) * 100;
    return { color: gradeColor(g), d: `${x(a[0])},${y(yMin)} ${x(a[0])},${y(a[1])} ${x(p[0])},${y(p[1])} ${x(p[0])},${y(yMin)}` };
  }));
  const xStep = $derived([500, 1000, 2000, 5000, 10000].find((s) => len / s <= 12) ?? 20000);
  const xTicks = $derived(Array.from({ length: Math.floor(len / xStep) + 1 }, (_, i) => i * xStep));
  const yTicks = $derived(Array.from({ length: Math.floor((yMax - yMin) / yStep) + 1 }, (_, i) => yMin + i * yStep));
  const line = $derived(points.map((p) => `${x(p[0])},${y(p[1])}`).join(' '));
  const cur = $derived(cursorM == null ? null : { x: x(Math.min(cursorM, len)), y: y(elevationAt({ resolution_m: 100, points }, cursorM)) });

  let dragging = -1;
  function pos(e: PointerEvent): [number, number] {
    const r = svg!.getBoundingClientRect();
    return [e.clientX - r.left, e.clientY - r.top];
  }
  function down(i: number, e: PointerEvent) {
    if (!editable) return;
    dragging = i; onselect?.(i);
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (dragging < 0 || !svg) return;
    const [px, py] = pos(e);
    const pts = points.map((p) => [...p] as Pt);
    const last = pts.length - 1;
    if (dragging > 0 && dragging < last) pts[dragging][0] = Math.round(Math.min(Math.max(ex(px), pts[dragging - 1][0] + 50), pts[dragging + 1][0] - 50) / 10) * 10;
    pts[dragging][1] = Math.round(Math.min(Math.max(ey(py), 0), 4000));
    onchange?.(pts, dragging);
  }
  const up = () => { dragging = -1; };
  function dbl(e: MouseEvent) {
    if (!editable || !svg) return;
    const r = svg.getBoundingClientRect();
    const d = Math.round(ex(e.clientX - r.left) / 10) * 10;
    if (d <= 0 || d >= len || points.some((p) => Math.abs(p[0] - d) < 50)) return;
    const pts = [...points.map((p) => [...p] as Pt), [d, Math.round(ey(e.clientY - r.top))] as Pt].sort((a, b) => a[0] - b[0]);
    onchange?.(pts, pts.findIndex((p) => p[0] === d));
  }
  function key(i: number, e: KeyboardEvent) {
    if (!editable) return;
    const pts = points.map((p) => [...p] as Pt);
    const step = e.shiftKey ? 1 : 5;
    if (e.key === 'ArrowUp') pts[i][1] += step;
    else if (e.key === 'ArrowDown') pts[i][1] = Math.max(0, pts[i][1] - step);
    else if (e.key === 'ArrowLeft' && i > 0 && i < pts.length - 1) pts[i][0] = Math.max(pts[i - 1][0] + 50, pts[i][0] - 50);
    else if (e.key === 'ArrowRight' && i > 0 && i < pts.length - 1) pts[i][0] = Math.min(pts[i + 1][0] - 50, pts[i][0] + 50);
    else if ((e.key === 'Delete' || e.key === 'Backspace') && i > 0 && i < pts.length - 1) { pts.splice(i, 1); onchange?.(pts, Math.max(i - 1, 0)); e.preventDefault(); return; }
    else return;
    e.preventDefault(); onchange?.(pts, i);
  }
</script>

<div class="wrap" bind:clientWidth={w} bind:clientHeight={hh} style:height={fill ? '100%' : `${fixedHeight}px`}>
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <svg bind:this={svg} width={w} {height} role="img" aria-label="Elevation profile" ondblclick={dbl} onpointermove={move} onpointerup={up} onpointercancel={up}>
    <g class="grid">
      {#each yTicks as tk (tk)}<line x1={padL} x2={w - padR} y1={y(tk)} y2={y(tk)} /><text x={padL - 10} y={y(tk) + 4} text-anchor="end">{tk} m</text>{/each}
      {#each xTicks as tk (tk)}<text x={x(tk)} y={height - 8} text-anchor="middle">{tk / 1000 === Math.floor(tk / 1000) ? tk / 1000 : (tk / 1000).toFixed(1)}{tk === xTicks[xTicks.length - 1] ? ' km' : ''}</text>{/each}
    </g>
    {#each segs as s, i (i)}<polygon points={s.d} fill={s.color} fill-opacity="0.9" />{/each}
    {#if cur}<rect x={padL} y={padT} width={Math.max(cur.x - padL, 0)} height={height - padT - padB} class="past" />{/if}
    <polyline points={line} class="outline" />
    {#if cur}
      <line x1={cur.x} x2={cur.x} y1={padT} y2={height - padB} class="cursor" />
      <circle cx={cur.x} cy={cur.y} r="8" class="now" />
      <g transform="translate({Math.min(cur.x + 10, w - 132)},{padT + 2})"><rect width="122" height="26" rx="7" class="tag" /><text x="12" y="18" class="tag-t">{t('ses.now')} · km {((cursorM ?? 0) / 1000).toFixed(1)}</text></g>
    {/if}
    {#if editable}
      {#each points as p, i (i)}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <circle cx={x(p[0])} cy={y(p[1])} r={selected === i ? 10 : 7} class="node" class:sel={selected === i} role="slider" tabindex="0" aria-label="{t('rt.node')} {i + 1}"
          aria-valuenow={p[1]} aria-valuetext="{(p[0] / 1000).toFixed(2)} km, {p[1]} m" onpointerdown={(e) => down(i, e)} onfocus={() => onselect?.(i)} onkeydown={(e) => key(i, e)} />
      {/each}
    {/if}
  </svg>
  {#if editable && interactiveHint}<div class="hint">{interactiveHint}</div>{/if}
</div>

<style>
  .wrap { position: relative; width: 100%; }
  svg { display: block; touch-action: none; }
  .grid line { stroke: var(--border-subtle); stroke-dasharray: 3 5; }
  .grid text { fill: var(--text-muted); font-size: 12px; }
  .outline { fill: none; stroke: var(--text); stroke-width: 2.4; stroke-linejoin: round; stroke-linecap: round; }
  .past { fill: var(--bg); opacity: 0.5; }
  .cursor { stroke: var(--text); stroke-width: 3; }
  .now { fill: var(--text); stroke: var(--bg); stroke-width: 3; }
  .tag { fill: var(--text); } .tag-t { fill: var(--bg); font-size: 13px; font-weight: 700; }
  .node { fill: var(--bg); stroke: var(--text); stroke-width: 2.5; cursor: grab; transition: r .12s; }
  .node:hover { r: 9; } .node.sel { fill: var(--accent); stroke: var(--text); }
  .node:active { cursor: grabbing; }
  .hint { position: absolute; left: 64px; top: 22px; font-size: 13px; color: var(--text-muted); pointer-events: none; }
</style>
