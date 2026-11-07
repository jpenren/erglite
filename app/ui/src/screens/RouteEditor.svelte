<script lang="ts">
  import Icon from '../lib/components/Icon.svelte';
  import ElevationProfile from '../lib/components/ElevationProfile.svelte';
  import { app, go, refresh, startRide, toast } from '../lib/state.svelte';
  import { backend } from '../lib/backend';
  import { t, type Key } from '../lib/i18n.svelte';
  import { estimateRoute, riderFromProfile, routeStats } from '../lib/physics';
  import { fmtDuration, fmtKm, routeTemplate, routeWorkout, type RouteTemplate } from '../lib/workout';
  let { id }: { id: string } = $props();
  const existing = app.workouts.find((r) => r.id === id);
  type Pt = [number, number];
  let name = $state(existing?.workout.name ?? 'New route');
  let points = $state<Pt[]>(existing?.workout.route?.points.map((p) => [...p] as Pt) ?? routeTemplate('rolling', 10000, 1000, 100).points);
  let selected = $state(1);
  let power = $state(Math.round(app.profile.ftp_w * 0.8));
  let difficulty = $state(100);
  const route = $derived({ resolution_m: 100, points });
  const stats = $derived(routeStats(route));
  const est = $derived(estimateRoute(route, power, riderFromProfile(app.profile)));
  const node = $derived(points[selected]);
  const templates: { id: RouteTemplate; label: Key }[] = [
    { id: 'flat', label: 'rt.flat' }, { id: 'rolling', label: 'rt.rolling' }, { id: 'climb', label: 'rt.climb' }, { id: 'valley', label: 'rt.valley' }, { id: 'stairs', label: 'rt.stairs' },
  ];
  function applyTemplate(k: RouteTemplate) { points = routeTemplate(k, stats.lengthM, Math.max(500, Math.round(stats.lengthM / 20 / 100) * 100), 100).points; selected = 1; }
  function setLength(km: number) {
    const len = Math.min(Math.max(km, 1), 200) * 1000, k = len / stats.lengthM;
    points = points.map(([d, e]) => [Math.round((d * k) / 10) * 10, e] as Pt);
  }
  function smooth() { points = points.map(([d, e], i) => [d, i === 0 || i === points.length - 1 ? e : Math.round((points[i - 1][1] + e + points[i + 1][1]) / 3)] as Pt); }
  function addNode() {
    let best = 0;
    for (let i = 1; i < points.length; i++) if (points[i][0] - points[i - 1][0] > points[best + 1 > points.length - 1 ? best : best + 1][0] - points[best][0]) best = i - 1;
    const [a, b] = [points[best], points[best + 1]];
    points = [...points.slice(0, best + 1), [Math.round((a[0] + b[0]) / 20) * 10, Math.round((a[1] + b[1]) / 2)] as Pt, ...points.slice(best + 1)];
    selected = best + 1;
  }
  function removeNode() { if (selected > 0 && selected < points.length - 1) { points = points.filter((_, i) => i !== selected); selected = Math.max(selected - 1, 0); } }
  function edit(field: 0 | 1, v: number) {
    const pts = points.map((p) => [...p] as Pt);
    if (field === 0 && selected > 0 && selected < pts.length - 1) pts[selected][0] = Math.min(Math.max(v, pts[selected - 1][0] + 50), pts[selected + 1][0] - 50);
    if (field === 1) pts[selected][1] = Math.max(0, v);
    points = pts;
  }
  async function save(ride: boolean) {
    const w = routeWorkout(name.trim() || 'Route', route);
    const savedId = await backend.saveWorkout(existing ? id : null, w);
    await refresh();
    if (ride) await startRide(savedId, w, difficulty / 100); else { toast(t('set.saved')); go('routes'); }
  }
  const legend: { c: string; l: Key; r: string }[] = [
    { c: '--grade-descent-steep', l: 'rt.steepDescent', r: '< −5 %' }, { c: '--grade-descent', l: 'rt.descent', r: '−5 … −2 %' }, { c: '--grade-flat', l: 'rt.flatLegend', r: '±2 %' },
    { c: '--grade-climb', l: 'rt.climbLegend', r: '2 … 4 %' }, { c: '--grade-climb-hard', l: 'rt.hard', r: '4 … 6 %' }, { c: '--grade-climb-very-hard', l: 'rt.veryHard', r: '> 6 %' },
  ];
  const gradeHere = $derived(selected > 0 ? ((points[selected][1] - points[selected - 1][1]) / (points[selected][0] - points[selected - 1][0])) * 100 : 0);
</script>

<div class="page">
  <header class="page-head">
    <div class="row"><button class="btn btn-ghost" aria-label={t('common.back')} onclick={() => go('routes')}><Icon name="back" /></button>
      <div><div class="kicker">{t('rt.title')}</div><input class="title" bind:value={name} aria-label={t('common.name')} /></div></div>
    <div class="row"><button class="btn" onclick={() => save(false)}>{t('common.save')}</button>
      <button class="btn btn-primary" onclick={() => save(true)}><Icon name="play" size={16} /> {t('common.saveRide')}</button></div>
  </header>

  <div class="stats">
    <div class="est"><span class="num">≈ {fmtDuration(est.timeS)}</span><span class="muted">{t('rt.atPower')} <input class="pw" type="number" min="60" max="600" bind:value={power} aria-label={t('common.power')} /> W · {est.avgKmh.toFixed(1)} km/h</span></div>
    <div class="kpis"><span>{t('rt.length')} <b>{fmtKm(stats.lengthM)} km</b></span><span>{t('rt.gain')} <b>+{Math.round(stats.gainM)} m</b></span><span>{t('rt.maxClimb')} <b>{stats.maxClimb.toFixed(1)} %</b></span><span>{t('rt.maxDescent')} <b>{stats.maxDescent.toFixed(1)} %</b></span></div>
  </div>

  <section class="card chart">
    <ElevationProfile {points} editable selected={selected} onselect={(i) => (selected = i)} onchange={(p, s) => { points = p as Pt[]; if (s != null) selected = s; }} height={300} interactiveHint={t('rt.hint')} />
    <div class="legend">{#each legend as g (g.c)}<span class="k"><i style:background="var({g.c})"></i>{t(g.l)} <small class="muted">{g.r}</small></span>{/each}</div>
  </section>

  <div class="tools">
    <span class="kicker">{t('rt.templates')}</span>
    {#each templates as tp (tp.id)}<button class="btn btn-sm" onclick={() => applyTemplate(tp.id)}>{t(tp.label)}</button>{/each}
    <span class="sp"></span>
    <button class="btn btn-sm" onclick={addNode}><Icon name="plus" size={14} /> {t('rt.addNode')}</button>
    <button class="btn btn-sm" onclick={smooth}>{t('rt.smooth')}</button>
  </div>

  <div class="cols">
    <section class="card pad">
      <h2>{t('rt.selected')} #{selected + 1}</h2>
      {#if node}
        <div class="f3">
          <label class="field">{t('rt.at')} (m)<input type="number" step="10" disabled={selected === 0 || selected === points.length - 1} value={node[0]} onchange={(e) => edit(0, +e.currentTarget.value)} /></label>
          <label class="field">{t('rt.elevation')} (m)<input type="number" value={node[1]} onchange={(e) => edit(1, +e.currentTarget.value)} /></label>
          <label class="field">{t('common.grade')} (%)<input disabled value={gradeHere.toFixed(1)} /></label>
        </div>
        <button class="btn btn-sm btn-danger" onclick={removeNode} disabled={selected === 0 || selected === points.length - 1}><Icon name="trash" size={14} /> {t('rt.removeNode')}</button>
      {/if}
    </section>
    <section class="card pad">
      <h2>{t('rt.feel')}</h2>
      <div class="f3">
        <label class="field">{t('rt.length')} (km)<input type="number" min="1" max="200" step="0.5" value={stats.lengthM / 1000} onchange={(e) => setLength(+e.currentTarget.value)} /></label>
        <label class="field">{t('rt.difficulty')} (%)<input type="number" min="0" max="150" step="5" bind:value={difficulty} /></label>
      </div>
    </section>
  </div>
</div>

<style>
  .title { all: unset; font-size: 30px; font-weight: 700; border-bottom: 2px dashed transparent; min-width: 260px; } .title:hover, .title:focus { border-color: var(--border); }
  .stats { display: flex; align-items: baseline; justify-content: space-between; gap: 24px; margin-bottom: 14px; flex-wrap: wrap; }
  .est { display: flex; align-items: baseline; gap: 14px; } .est .num { font-size: 44px; font-weight: 700; } .pw { width: 74px; min-height: 32px; display: inline-block; margin: 0 4px; text-align: center; }
  .kpis { display: flex; gap: 24px; color: var(--text-muted); flex-wrap: wrap; } .kpis b { color: var(--text); font-size: 18px; margin-left: 4px; }
  .chart { padding: 14px 18px 12px; margin-bottom: 14px; } .legend { display: flex; flex-wrap: wrap; gap: 6px 20px; padding: 4px 0 2px 52px; } .k { display: inline-flex; align-items: center; gap: 8px; font-size: 14px; color: var(--text-secondary); } .k i { width: 14px; height: 14px; border-radius: 4px; display: inline-block; }
  .tools { display: flex; align-items: center; gap: 8px; margin-bottom: 18px; flex-wrap: wrap; } .tools .kicker { margin-right: 6px; } .sp { flex: 1; }
  .cols { display: grid; grid-template-columns: 1.3fr 1fr; gap: 18px; } .pad { padding: 20px 24px; display: flex; flex-direction: column; gap: 14px; align-items: flex-start; } .f3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 14px; width: 100%; }
</style>
