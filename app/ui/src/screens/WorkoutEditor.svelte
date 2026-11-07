<script lang="ts">
  import Icon from '../lib/components/Icon.svelte';
  import WorkoutProfile from '../lib/components/WorkoutProfile.svelte';
  import { app, go, refresh, startRide, toast } from '../lib/state.svelte';
  import { backend } from '../lib/backend';
  import { t } from '../lib/i18n.svelte';
  import { estimateTss, fmtDuration, isRepeat, newTimeWorkout, plannedTimeS, segmentPct, timeOf, type Block } from '../lib/workout';
  import type { Segment, Workout } from '../lib/types';
  let { id }: { id: string } = $props();

  type EBlock = { kind: 'steady' | 'ramp' | 'intervals'; label: string; durationS: number; from: number; to: number; repeat: number; workS: number; workPct: number; restS: number; restPct: number; cadence?: number; message?: string };
  const ftp = $derived(app.profile.ftp_w);
  const existing = app.workouts.find((r) => r.id === id);

  function fromWorkout(w: Workout): EBlock[] {
    const base = { repeat: 1, workS: 240, workPct: 100, restS: 120, restPct: 55 };
    return w.segments.map((n): EBlock => {
      if (isRepeat(n) && n.segments.length === 2) {
        const [a, b] = n.segments;
        const pa = segmentPct(a.control, app.profile.ftp_w), pb = segmentPct(b.control, app.profile.ftp_w);
        return { ...base, kind: 'intervals', label: 'Intervals', durationS: 0, from: 0, to: 0, repeat: n.repeat, workS: timeOf(a.end) ?? 240, workPct: Math.round(pa?.from ?? 100), restS: timeOf(b.end) ?? 120, restPct: Math.round(pb?.from ?? 55) };
      }
      const s = (isRepeat(n) ? n.segments[0] : n) as Segment;
      const p = segmentPct(s.control, app.profile.ftp_w) ?? { from: 60, to: 60 };
      return { ...base, kind: p.from === p.to ? 'steady' : 'ramp', label: s.label ?? 'Block', durationS: timeOf(s.end) ?? 300, from: Math.round(p.from), to: Math.round(p.to), cadence: s.cadence_rpm, message: s.message };
    });
  }
  let name = $state(existing?.workout.name ?? 'New workout');
  let blocks = $state<EBlock[]>(fromWorkout(existing?.workout ?? newTimeWorkout()));
  let selected = $state(0);
  const sel = $derived(blocks[selected]);

  const toSeg = (b: EBlock, label: string, s: number, from: number, to: number): Segment => ({
    label, end: { time_s: Math.max(1, Math.round(s)) }, cadence_rpm: b.cadence, message: b.message || undefined,
    control: { power: from === to ? { pct_ftp: from } : { from_pct_ftp: from, to_pct_ftp: to } },
  });
  const workout = $derived<Workout>({
    schema_version: 1, name: name.trim() || 'Workout', kind: 'time', description: existing?.workout.description,
    segments: blocks.map((b) => b.kind === 'intervals'
      ? { repeat: Math.max(1, b.repeat), segments: [toSeg(b, t('ed.work'), b.workS, b.workPct, b.workPct), toSeg(b, t('ed.rest'), b.restS, b.restPct, b.restPct)] }
      : toSeg(b, b.label, b.durationS, b.from, b.kind === 'steady' ? b.from : b.to)),
  });
  const chart = $derived.by<Block[]>(() => blocks.flatMap((b, i) => b.kind === 'intervals'
    ? Array.from({ length: Math.max(1, b.repeat) }, () => [{ durationS: b.workS, from: b.workPct, to: b.workPct, ref: i }, { durationS: b.restS, from: b.restPct, to: b.restPct, ref: i }]).flat()
    : [{ durationS: b.durationS, from: b.from, to: b.kind === 'steady' ? b.from : b.to, label: b.label, ref: i }]));
  const total = $derived(plannedTimeS(workout));
  const tss = $derived(estimateTss(workout, ftp));

  function add(kind: EBlock['kind'], label: string, partial: Partial<EBlock>) {
    blocks.push({ kind, label, durationS: 300, from: 70, to: 70, repeat: 3, workS: 240, workPct: 105, restS: 120, restPct: 55, ...partial });
    selected = blocks.length - 1;
  }
  function move(i: number, d: number) { const j = i + d; if (j < 0 || j >= blocks.length) return; [blocks[i], blocks[j]] = [blocks[j], blocks[i]]; selected = j; }
  function remove(i: number) { blocks.splice(i, 1); selected = Math.max(0, Math.min(selected, blocks.length - 1)); }
  async function save(ride: boolean) {
    const savedId = await backend.saveWorkout(existing ? id : null, workout);
    await refresh();
    if (ride) await startRide(savedId, workout); else { toast(t('set.saved')); go('library'); }
  }
  const mins = (s: number) => Math.round((s / 60) * 10) / 10;
  const summary = (b: EBlock) => b.kind === 'intervals' ? `${b.repeat} × ${mins(b.workS)} / ${mins(b.restS)} min · ${b.workPct}/${b.restPct} %` : `${mins(b.durationS)} min · ${b.kind === 'steady' ? b.from : `${b.from} → ${b.to}`} %`;
</script>

<div class="page">
  <header class="page-head">
    <div class="row"><button class="btn btn-ghost" aria-label={t('common.back')} onclick={() => go('library')}><Icon name="back" /></button>
      <div><div class="kicker">{t('ed.title')}</div><input class="title" bind:value={name} aria-label={t('ed.name')} /></div></div>
    <div class="row">
      <div class="totals"><span class="num">{fmtDuration(total)}</span><span class="muted">FTP {ftp} W{#if tss != null} · TSS ≈ {Math.round(tss)}{/if}</span></div>
      <button class="btn" onclick={() => save(false)}>{t('common.save')}</button>
      <button class="btn btn-primary" onclick={() => save(true)}><Icon name="play" size={16} /> {t('common.saveRide')}</button>
    </div>
  </header>

  <section class="card chart"><WorkoutProfile blocks={chart} height={220} selected={selected} onselect={(r) => (selected = r)} /></section>

  <div class="adds">
    <button class="btn" onclick={() => add('ramp', t('ed.addWarm').replace('+ ', ''), { durationS: 600, from: 45, to: 70 })}>{t('ed.addWarm')}</button>
    <button class="btn" onclick={() => add('steady', t('ed.steady'), { from: 75, to: 75 })}>{t('ed.addSteady')}</button>
    <button class="btn" onclick={() => add('ramp', t('ed.ramp'), { from: 60, to: 100 })}>{t('ed.addRamp')}</button>
    <button class="btn" onclick={() => add('intervals', t('ed.intervals'), {})}>{t('ed.addIntervals')}</button>
  </div>

  <div class="cols">
    <section class="card list">
      {#each blocks as b, i (i)}
        <div class="item" class:on={selected === i} role="button" tabindex="0" onclick={() => (selected = i)} onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && (selected = i)}>
          <span class="idx num">{i + 1}</span>
          <div class="main"><b>{b.kind === 'intervals' ? t('ed.intervals') : b.label}</b><span class="muted">{summary(b)}</span></div>
          <span class="acts">
            <button class="btn btn-ghost btn-sm" aria-label={t('ed.up')} onclick={(e) => { e.stopPropagation(); move(i, -1); }}><Icon name="up" size={16} /></button>
            <button class="btn btn-ghost btn-sm" aria-label={t('ed.down')} onclick={(e) => { e.stopPropagation(); move(i, 1); }}><Icon name="down" size={16} /></button>
            <button class="btn btn-ghost btn-sm" aria-label={t('ed.remove')} onclick={(e) => { e.stopPropagation(); remove(i); }}><Icon name="trash" size={16} /></button>
          </span>
        </div>
      {:else}<p class="muted pad">{t('ed.noBlocks')}</p>{/each}
    </section>

    <section class="card insp">
      {#if sel}
        <h2>{t('ed.block')} {selected + 1}</h2>
        {#if sel.kind === 'intervals'}
          <div class="f2">
            <label class="field">{t('ed.repeats')}<input type="number" min="1" max="50" bind:value={sel.repeat} /></label>
            <label class="field">{t('ed.work')} ({t('common.min')})<input type="number" min="0.25" step="0.25" value={mins(sel.workS)} oninput={(e) => (sel.workS = Math.round(+e.currentTarget.value * 60))} /></label>
            <label class="field">{t('ed.workTarget')}<input type="number" min="20" max="250" bind:value={sel.workPct} /></label>
            <label class="field">{t('ed.rest')} ({t('common.min')})<input type="number" min="0.25" step="0.25" value={mins(sel.restS)} oninput={(e) => (sel.restS = Math.round(+e.currentTarget.value * 60))} /></label>
            <label class="field">{t('ed.restTarget')}<input type="number" min="20" max="250" bind:value={sel.restPct} /></label>
          </div>
        {:else}
          <div class="f2">
            <label class="field" style="grid-column: span 2">{t('common.name')}<input bind:value={sel.label} /></label>
            <label class="field">{t('ed.durationMin')}<input type="number" min="0.25" step="0.25" value={mins(sel.durationS)} oninput={(e) => (sel.durationS = Math.round(+e.currentTarget.value * 60))} /></label>
            <label class="field">{sel.kind === 'ramp' ? t('ed.from') : t('ed.targetPct')}<input type="number" min="20" max="250" bind:value={sel.from} /></label>
            {#if sel.kind === 'ramp'}<label class="field">{t('ed.to')}<input type="number" min="20" max="250" bind:value={sel.to} /></label>{/if}
          </div>
        {/if}
        <div class="f2">
          <label class="field">{t('ed.cadence')} (rpm)<input type="number" min="40" max="140" bind:value={sel.cadence} /></label>
          <label class="field">{t('ed.message')}<input bind:value={sel.message} /></label>
        </div>
      {/if}
    </section>
  </div>
</div>

<style>
  .title { all: unset; font-size: 30px; font-weight: 700; border-bottom: 2px dashed transparent; min-width: 280px; } .title:hover, .title:focus { border-color: var(--border); }
  .totals { display: flex; flex-direction: column; align-items: flex-end; margin-right: 8px; } .totals .num { font-size: 36px; font-weight: 700; }
  .chart { padding: 18px 20px; margin-bottom: 16px; } .adds { display: flex; gap: 10px; flex-wrap: wrap; margin-bottom: 20px; }
  .cols { display: grid; grid-template-columns: 1.3fr 1fr; gap: 18px; align-items: start; } .list { padding: 8px; } .pad { padding: 20px; }
  .item { display: grid; grid-template-columns: 36px 1fr auto; align-items: center; gap: 12px; padding: 10px 12px; border-radius: 12px; cursor: pointer; } .item:hover { background: var(--surface-selected); } .item:focus-visible { outline: 2px solid var(--accent); }
  .item.on { background: var(--surface-selected); box-shadow: inset 3px 0 0 var(--accent); } .idx { font-size: 24px; color: var(--text-muted); } .main { display: flex; flex-direction: column; } .acts { display: flex; opacity: .6; } .item:hover .acts { opacity: 1; }
  .insp { padding: 22px 24px; display: flex; flex-direction: column; gap: 16px; } .f2 { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
</style>
