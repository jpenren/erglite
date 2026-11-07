<script lang="ts">
  import Icon from '../lib/components/Icon.svelte';
  import MetricTile from '../lib/components/MetricTile.svelte';
  import WorkoutProfile from '../lib/components/WorkoutProfile.svelte';
  import ElevationProfile from '../lib/components/ElevationProfile.svelte';
  import { app, ask, finishRide, go } from '../lib/state.svelte';
  import { backend } from '../lib/backend';
  import { t } from '../lib/i18n.svelte';
  import { flatten, fmtDuration, fmtKm, gradeBand, gradeColor, plannedDistanceM, plannedTimeS, segmentPct, timeBlocks, zoneColor, zoneForPct, zoneName } from '../lib/workout';
  import { averageGrade } from '../lib/physics';
  import type { Key } from '../lib/i18n.svelte';
  const BANDS: Key[] = ['rt.steepDescent', 'rt.descent', 'rt.flatLegend', 'rt.climbLegend', 'rt.hard', 'rt.veryHard'];

  const s = $derived(app.session);
  const tick = $derived(s?.tick ?? null);
  const w = $derived(s?.workout ?? null);
  const isRoute = $derived(w?.kind === 'route' && !!w.route);
  const ftp = $derived(app.profile.ftp_w);
  const segs = $derived(w ? flatten(w) : []);
  const blocks = $derived(w ? timeBlocks(w, ftp) : []);
  const power = $derived(tick?.power_w ?? 0);
  const zone = $derived(zoneForPct((power / ftp) * 100));
  const next = $derived.by(() => {
    if (!tick || !w) return '';
    if (isRoute && w.route) {
      const pts = w.route.points, pos = tick.route_pos_m, cur = gradeBand(averageGrade(w.route, pos, pos + 50));
      for (let i = 1; i < pts.length; i++) {
        if (pts[i][0] <= pos + 50) continue;
        const g = ((pts[i][1] - pts[i - 1][1]) / (pts[i][0] - pts[i - 1][0])) * 100;
        if (gradeBand(g) !== cur) { const d = Math.max(pts[i - 1][0], pos) - pos; return `${g.toFixed(0)} % · ${d >= 1000 ? fmtKm(d) + ' km' : Math.round(d / 10) * 10 + ' m'}`; }
      }
      return t('ses.endOfRoute');
    }
    const n = segs[tick.segment_index + 1];
    if (!n) return '';
    const p = segmentPct(n.control, ftp);
    return `${n.label ?? ''} · ${p ? Math.round(p.from) : '–'} % · ${fmtDuration(tick.segment_remaining_s ?? 0)}`;
  });
  const progressText = $derived(isRoute && w ? `${fmtKm(tick?.distance_m ?? 0, 1)} / ${fmtKm(plannedDistanceM(w))} km` : `${fmtDuration(tick?.elapsed_s ?? 0)} / ${fmtDuration(w ? plannedTimeS(w) : 0)}`);
  const remainingEst = $derived.by(() => {
    if (!isRoute || !tick || !w) return '';
    const done = tick.distance_m, left = plannedDistanceM(w) - done;
    const speed = Math.max(tick.speed_kmh ?? 20, 8) / 3.6;
    return fmtDuration(left / speed);
  });
  const guideLo = $derived(ftp * 0.75), guideHi = $derived(ftp * 0.85);
  const gx = (v: number) => `${Math.min(v / (ftp * 1.2), 1) * 100}%`;
  const insideGuide = $derived(power >= guideLo && power <= guideHi);
  const sparkline = $derived.by(() => {
    const tr = s?.trace ?? [];
    if (tr.length < 2) return '';
    const hi = Math.max(...tr, ftp), lo = Math.min(...tr);
    return tr.map((v, i) => `${(i / 179) * 300},${46 - ((v - lo) / Math.max(hi - lo, 1)) * 42}`).join(' ');
  });
  const paused = $derived(tick?.status === 'paused');
  async function toggle() { if (paused) await backend.resumeSession(); else await backend.pauseSession(); }
  async function end() { if (await ask(t('ses.confirmFinish'))) await finishRide(); }
  async function difficulty(delta: number) { if (!s) return; s.difficulty = Math.round(Math.min(Math.max(s.difficulty + delta, 0), 1.5) * 100) / 100; await backend.setDifficulty(s.difficulty); }
  let speed = $state(1);
  function setSpeed(n: number) { speed = n; backend.setDemoSpeed?.(n); }
  $effect(() => { if (!app.session) go('home'); });
</script>

{#if s && w}
  <div class="ses">
    <header>
      <div><h1>{w.name}</h1><p class="muted">{isRoute ? t('ses.mode.sim') : t('ses.mode.erg')} · {t('ses.block')} {(tick?.segment_index ?? 0) + 1} {t('ses.of')} {tick?.segment_count ?? segs.length}</p></div>
      <div class="num prog">{progressText}</div>
      <div class="chips">
        <span class="chip"><span class="dot" class:ok={tick?.connection === 'connected'}></span> {app.trainer.caps?.name ?? 'Trainer'}</span>
        <span class="chip"><Icon name="heart" size={14} /> {tick?.hr_bpm ?? '–'} {t('unit.bpm')}</span>
      </div>
    </header>

    {#if tick && tick.connection !== 'connected'}<div class="banner" role="alert">{t('ses.lost')}</div>{/if}

    <section class="tiles" class:route={isRoute}>
      {#if isRoute}
        <MetricTile label={t('common.grade')} value={(tick?.grade_pct ?? 0).toFixed(0)} unit="%" size="xl" accent={gradeColor(tick?.grade_pct ?? 0)}>
          <b style:color={gradeColor(tick?.grade_pct ?? 0)}>{t(BANDS[gradeBand(tick?.grade_pct ?? 0)])}</b>
        </MetricTile>
        <MetricTile label={t('common.power')} value={power} unit="W" size="lg">
          <div class="track"><i style:left={gx(guideLo)} style:width="calc({gx(guideHi)} - {gx(guideLo)})"></i><b style:left={gx(power)}></b></div>
          <small class="muted">{t('ses.guide')} {Math.round(guideLo)}–{Math.round(guideHi)} W{#if insideGuide} · {t('ses.inside')}{/if}</small>
        </MetricTile>
        <MetricTile label={t('common.speed')} value={(tick?.speed_kmh ?? 0).toFixed(1)} unit="km/h" size="lg" />
        <MetricTile label={t('common.cadence')} value={tick?.cadence_rpm ?? 0} unit="rpm" size="lg" />
      {:else}
        <MetricTile label={t('common.power')} value={power} unit="W" size="xl" accent={zoneColor(zone)}>
          {t('ses.target')} <b>{tick?.target_power_w ?? '–'} W</b> · {Math.round((power / ftp) * 100)} % FTP · <b style:color={zoneColor(zone)}>Z{zone} {zoneName(zone)}</b>
        </MetricTile>
        <MetricTile label={t('common.cadence')} value={tick?.cadence_rpm ?? 0} unit="rpm" size="lg">{segs[tick?.segment_index ?? 0]?.cadence_rpm ? `${t('ses.target')} ${segs[tick?.segment_index ?? 0].cadence_rpm}` : ''}</MetricTile>
        <MetricTile label={t('common.heartRate')} value={tick?.hr_bpm ?? 0} unit="bpm" size="lg">
          <svg class="spark" viewBox="0 0 300 46" preserveAspectRatio="none" aria-hidden="true"><polyline points={sparkline} /></svg>
        </MetricTile>
      {/if}
    </section>

    <section class="chart card">
      {#if isRoute && w.route}
        <div class="fillbox"><ElevationProfile points={w.route.points} cursorM={tick?.route_pos_m ?? 0} fill /></div>
      {:else}
        <div class="fillbox"><WorkoutProfile {blocks} cursorS={tick?.status === 'countdown' ? 0 : (tick?.elapsed_s ?? 0)} fill /></div>
      {/if}
    </section>

    <div class="info">
      <div><span class="muted">{t('ses.next')}:</span> <b>{next || '–'}</b></div>
      <div class="muted">
        {#if isRoute}{t('common.time')} <b class="num">{fmtDuration(tick?.elapsed_s ?? 0)}</b> · {t('ses.remaining')} ≈ <b class="num">{remainingEst}</b> · {t('rt.difficulty')} {Math.round((s.difficulty) * 100)} %
        {:else}{t('ses.remaining')} <b class="num">{fmtDuration(tick?.segment_remaining_s ?? 0)}</b>{/if}
      </div>
    </div>

    <footer>
      {#if isRoute}
        <button class="btn btn-lg" onclick={() => difficulty(-0.1)}><Icon name="minus" size={16} /> {t('rt.difficulty')}</button>
      {:else}
        <button class="btn btn-lg" onclick={() => backend.adjustIntensity(-5)}>{t('ses.lessIntensity')}</button>
      {/if}
      <button class="btn btn-primary btn-lg main" onclick={toggle}><Icon name={paused ? 'play' : 'pause'} /> {paused ? t('ses.resume') : t('ses.pause')}</button>
      {#if isRoute}
        <button class="btn btn-lg" onclick={() => difficulty(0.1)}><Icon name="plus" size={16} /> {t('rt.difficulty')}</button>
      {:else}
        <button class="btn btn-lg" onclick={() => backend.adjustIntensity(5)}>{t('ses.moreIntensity')}</button>
        <button class="btn btn-lg" onclick={() => backend.skipSegment()}><Icon name="skip" size={16} /> {t('ses.skip')}</button>
      {/if}
      <button class="btn btn-lg btn-danger" onclick={end}><Icon name="stop" size={16} /> {t('ses.finish')}</button>
      {#if backend.kind === 'mock'}
        <div class="demo" role="group" aria-label={t('ses.demo')}><small>{t('ses.demo')}</small>{#each [1, 5, 20] as n (n)}<button class:on={speed === n} onclick={() => setSpeed(n)}>×{n}</button>{/each}</div>
      {/if}
    </footer>

    {#if tick?.status === 'countdown'}
      <div class="overlay"><div><div class="kicker">{t('ses.getReady')}</div><div class="num count">{tick.countdown_s}</div></div></div>
    {:else if paused}
      <div class="overlay"><div class="pause card"><div class="kicker">{t('ses.paused')}</div><button class="btn btn-primary btn-lg" onclick={toggle}><Icon name="play" /> {t('ses.resume')}</button></div></div>
    {/if}
  </div>
{/if}

<style>
  .ses { position: relative; height: 100%; display: flex; flex-direction: column; gap: 18px; padding: 26px 44px 28px; }
  header { display: flex; align-items: center; justify-content: space-between; gap: 24px; } .prog { font-size: 52px; font-weight: 700; } .chips { display: flex; gap: 10px; }
  .banner { background: color-mix(in srgb, var(--danger) 22%, var(--surface)); border: 1px solid var(--danger); padding: 10px 18px; border-radius: 12px; font-weight: 600; }
  .tiles { display: grid; grid-template-columns: 1.5fr 1fr 1fr; gap: 18px; } .tiles.route { grid-template-columns: 1.5fr 1fr 1fr 1fr; }
  .chart { padding: 14px 18px 8px; flex: 1; min-height: 170px; display: flex; flex-direction: column; } .fillbox { flex: 1; min-height: 0; }
  .info { display: flex; justify-content: space-between; font-size: 18px; } .info b { color: var(--text); }
  footer { display: flex; gap: 12px; justify-content: center; align-items: center; flex-wrap: wrap; position: relative; } footer .main { min-width: 190px; }
  .track { position: relative; height: 10px; background: var(--border-subtle); border-radius: 5px; margin: 14px 0 8px; } .track i { position: absolute; top: 0; bottom: 0; background: var(--ok); border-radius: 5px; opacity: .85; } .track b { position: absolute; top: -4px; width: 4px; height: 18px; background: var(--text); border-radius: 2px; transform: translateX(-2px); }
  .spark { width: 100%; height: 46px; } .spark polyline { fill: none; stroke: var(--accent); stroke-width: 2; vector-effect: non-scaling-stroke; }
  .overlay { position: absolute; inset: 0; background: color-mix(in srgb, var(--bg) 72%, transparent); backdrop-filter: blur(8px); display: grid; place-items: center; text-align: center; z-index: 5; }
  .count { font-size: 220px; font-weight: 700; color: var(--accent); animation: pop 1s ease-out infinite; } @keyframes pop { from { transform: scale(1.12); opacity: .4; } to { transform: scale(1); opacity: 1; } }
  .pause { padding: 32px 44px; display: flex; flex-direction: column; gap: 18px; align-items: center; }
  .demo { position: absolute; right: 0; bottom: 0; display: flex; gap: 6px; align-items: center; color: var(--text-muted); } .demo button { all: unset; cursor: pointer; padding: 4px 10px; border-radius: 8px; border: 1px solid var(--border); font-size: 13px; } .demo button.on { background: var(--surface-selected); color: var(--text); }
</style>
