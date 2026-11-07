<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { app, go } from '../lib/state.svelte';
  import { backend } from '../lib/backend';
  import { i18n, t } from '../lib/i18n.svelte';
  import { fmtDuration, fmtKm, zoneColor, zoneName } from '../lib/workout';
  import type { Sample, Summary } from '../lib/types';
  let { id }: { id: string } = $props();
  let samples = $state<Sample[]>([]);
  const rec = $derived(app.sessions.find((x) => x.id === id));
  const sum = $derived<Summary | null>(app.lastSummary?.id === id ? app.lastSummary.summary : (rec?.summary ?? null));
  onMount(async () => { samples = await backend.sessionSamples(id); });
  const W = 1000, H = 200;
  const chart = $derived.by(() => {
    if (samples.length < 2) return { p: '', tgt: '', area: '' };
    const smooth = samples.map((_, i) => { const a = Math.max(0, i - 4), b = Math.min(samples.length, i + 5); return samples.slice(a, b).reduce((s, x) => s + (x.power_w ?? 0), 0) / (b - a); });
    const hi = Math.max(...smooth, ...samples.map((s) => s.target_power_w ?? 0), 100) * 1.08;
    const X = (i: number) => (i / (samples.length - 1)) * W, Y = (v: number) => H - (v / hi) * (H - 6);
    const p = smooth.map((v, i) => `${X(i)},${Y(v)}`).join(' ');
    return { p, area: `0,${H} ${p} ${W},${H}`, tgt: samples.map((s, i) => `${X(i)},${Y(s.target_power_w ?? 0)}`).join(' ') };
  });
  const zoneTotal = $derived(sum ? sum.zone_seconds.reduce((a, b) => a + b, 0) || 1 : 1);
  async function exportTcx() {
    const xml = await backend.exportSession(id, 'tcx');
    const url = URL.createObjectURL(new Blob([xml], { type: 'application/vnd.garmin.tcx+xml' }));
    Object.assign(document.createElement('a'), { href: url, download: `erglite-${id}.tcx` }).click();
    URL.revokeObjectURL(url);
  }
  const when = $derived(rec ? new Date(rec.started_at).toLocaleString(i18n.lang, { dateStyle: 'full', timeStyle: 'short' }) : '');
</script>

<div class="page">
  <header class="page-head">
    <div class="row"><button class="btn btn-ghost" aria-label={t('common.back')} onclick={() => go('history')}><Icon name="back" /></button>
      <div><div class="kicker">{t('sum.title')}</div><h1>{rec?.workout_name ?? ''}</h1><p>{when}</p></div></div>
    <div class="row"><button class="btn" onclick={exportTcx}><Icon name="download" size={18} /> {t('sum.export')}</button><button class="btn btn-primary" onclick={() => go('home')}><Icon name="check" size={18} /> {t('sum.done')}</button></div>
  </header>

  {#if sum}
    <section class="kpis">
      {#each [
        [t('common.duration'), fmtDuration(sum.duration_s), ''], [t('sum.avgPower'), Math.round(sum.avg_power_w), 'W'], [t('sum.np'), sum.normalized_power_w ? Math.round(sum.normalized_power_w) : '–', 'W'],
        [t('sum.tss'), sum.tss ? Math.round(sum.tss) : '–', ''], [t('sum.if'), sum.intensity_factor ? sum.intensity_factor.toFixed(2) : '–', ''], [t('sum.maxPower'), sum.max_power_w, 'W'],
        ...(sum.distance_m > 0 ? [[t('common.distance'), fmtKm(sum.distance_m), 'km']] : []), ...(sum.elevation_gain_m > 1 ? [[t('common.elevation'), '+' + Math.round(sum.elevation_gain_m), 'm']] : []),
        ...(sum.avg_hr_bpm ? [[t('sum.avgHr'), Math.round(sum.avg_hr_bpm), 'bpm']] : []),
      ] as [label, value, unit] (label)}
        <div class="card k"><div class="kicker">{label}</div><div class="num v">{value}<small>{unit}</small></div></div>
      {/each}
    </section>

    <section class="card pad">
      <div class="kicker">{t('sum.powerChart')}</div>
      <svg viewBox="0 0 {W} {H}" preserveAspectRatio="none" role="img" aria-label="Power over time">
        <defs><linearGradient id="pg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="var(--accent)" stop-opacity=".45" /><stop offset="1" stop-color="var(--accent)" stop-opacity="0" /></linearGradient></defs>
        <polygon points={chart.area} fill="url(#pg)" /><polyline points={chart.tgt} class="tgt" vector-effect="non-scaling-stroke" /><polyline points={chart.p} class="pw" vector-effect="non-scaling-stroke" />
      </svg>
    </section>

    <section class="card pad">
      <div class="kicker">{t('sum.zones')}</div>
      <div class="bar">{#each sum.zone_seconds as sec, i (i)}{#if sec > 0}<span style:flex={sec} style:background={zoneColor(i + 1)} title="Z{i + 1} {fmtDuration(sec)}"></span>{/if}{/each}</div>
      <div class="zl">{#each sum.zone_seconds as sec, i (i)}<span class="k"><i style:background={zoneColor(i + 1)}></i>Z{i + 1} {zoneName(i + 1)} <b>{fmtDuration(sec)}</b> <small class="muted">{Math.round((sec / zoneTotal) * 100)} %</small></span>{/each}</div>
    </section>
  {/if}
</div>

<style>
  .kpis { display: grid; grid-template-columns: repeat(auto-fill, minmax(190px, 1fr)); gap: 14px; margin-bottom: 18px; } .k { padding: 16px 20px; } .v { font-size: 46px; font-weight: 700; margin-top: 6px; } .v small { font-size: 18px; color: var(--text-muted); margin-left: 6px; }
  .pad { padding: 18px 22px; margin-bottom: 18px; } svg { width: 100%; height: 200px; display: block; margin-top: 10px; } .pw { fill: none; stroke: var(--accent); stroke-width: 2.4; stroke-linejoin: round; } .tgt { fill: none; stroke: var(--text-muted); stroke-width: 1.5; stroke-dasharray: 5 5; }
  .bar { display: flex; height: 22px; border-radius: 11px; overflow: hidden; gap: 2px; margin: 12px 0; } .zl { display: flex; flex-wrap: wrap; gap: 8px 22px; } .zl .k { display: inline-flex; align-items: center; gap: 8px; padding: 0; font-size: 14px; color: var(--text-secondary); } .zl i { width: 12px; height: 12px; border-radius: 4px; display: inline-block; }
</style>
