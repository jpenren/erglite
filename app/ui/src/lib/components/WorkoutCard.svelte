<script lang="ts">
  import Icon from './Icon.svelte';
  import WorkoutProfile from './WorkoutProfile.svelte';
  import { app } from '../state.svelte';
  import { t } from '../i18n.svelte';
  import { estimateTss, fmtDuration, fmtKm, gradeColor, plannedDistanceM, plannedTimeS, timeBlocks } from '../workout';
  import { routeStats } from '../physics';
  import type { WorkoutRecord } from '../types';
  let { rec, onride, onedit, onduplicate, ondelete }: { rec: WorkoutRecord; onride: () => void; onedit: () => void; onduplicate: () => void; ondelete: () => void } = $props();
  const w = $derived(rec.workout);
  const isRoute = $derived(w.kind === 'route' && !!w.route);
  const tss = $derived(isRoute ? null : estimateTss(w, app.profile.ftp_w));
  const stats = $derived(isRoute && w.route ? routeStats(w.route) : null);
  const mini = $derived.by(() => {
    if (!w.route) return [];
    const pts = w.route.points, len = pts[pts.length - 1][0];
    const lo = Math.min(...pts.map((p) => p[1])), hi = Math.max(...pts.map((p) => p[1]));
    const y = (e: number) => 66 - ((e - lo) / Math.max(hi - lo, 20)) * 54;
    return pts.slice(1).map((p, i) => {
      const a = pts[i], g = ((p[1] - a[1]) / (p[0] - a[0])) * 100;
      const [x0, x1] = [(a[0] / len) * 300, (p[0] / len) * 300];
      return { c: gradeColor(g), d: `${x0},72 ${x0},${y(a[1])} ${x1},${y(p[1])} ${x1},72` };
    });
  });
</script>

<article class="card wc">
  <div class="top">
    <span class="chip">{isRoute ? t('lib.route') : t('lib.time')}</span>
    <span class="menu">
      <button class="btn btn-ghost btn-sm" title={t('common.duplicate')} aria-label={t('common.duplicate')} onclick={onduplicate}><Icon name="copy" size={16} /></button>
      <button class="btn btn-ghost btn-sm" title={t('common.delete')} aria-label={t('common.delete')} onclick={ondelete}><Icon name="trash" size={16} /></button>
    </span>
  </div>
  <h3>{w.name}</h3>
  <p class="muted desc">{w.description ?? ''}</p>
  <div class="chart">
    {#if isRoute}
      <svg viewBox="0 0 300 72" preserveAspectRatio="none" aria-hidden="true">{#each mini as s, i (i)}<polygon points={s.d} fill={s.c} />{/each}</svg>
    {:else}
      <WorkoutProfile blocks={timeBlocks(w, app.profile.ftp_w)} height={72} showFtp={false} />
    {/if}
  </div>
  <div class="stats">
    {#if isRoute && stats}
      <span><b class="num">{fmtKm(plannedDistanceM(w))}</b> {t('unit.km')}</span>
      <span><b class="num">+{Math.round(stats.gainM)}</b> {t('unit.m')}</span>
    {:else}
      <span><b class="num">{fmtDuration(plannedTimeS(w))}</b></span>
      {#if tss != null}<span><b class="num">{Math.round(tss)}</b> TSS</span>{/if}
    {/if}
  </div>
  <div class="actions">
    <button class="btn btn-primary" onclick={onride}><Icon name="play" size={16} /> {t('common.ride')}</button>
    <button class="btn" onclick={onedit}><Icon name="edit" size={16} /> {t('common.edit')}</button>
  </div>
</article>

<style>
  .wc { padding: 18px 20px 20px; display: flex; flex-direction: column; gap: 10px; transition: transform .15s, border-color .15s; }
  .wc:hover { transform: translateY(-2px); border-color: var(--border); }
  .top { display: flex; justify-content: space-between; align-items: center; }
  .menu { display: flex; gap: 2px; opacity: 0.6; } .wc:hover .menu, .menu:focus-within { opacity: 1; }
  h3 { font-size: 20px; font-weight: 650; margin-top: 2px; }
  .desc { font-size: 14px; min-height: 20px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .chart { background: var(--bg); border-radius: 12px; padding: 8px 8px 0; overflow: hidden; }
  .chart svg { width: 100%; height: 72px; display: block; }
  .stats { display: flex; gap: 18px; color: var(--text-muted); font-size: 15px; }
  .stats b { color: var(--text); font-size: 24px; margin-right: 2px; }
  .actions { display: flex; gap: 10px; margin-top: 4px; } .actions .btn-primary { flex: 1; }
</style>
