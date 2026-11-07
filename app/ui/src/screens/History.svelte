<script lang="ts">
  import { app, go } from '../lib/state.svelte';
  import { i18n, t } from '../lib/i18n.svelte';
  import { fmtDuration, fmtKm } from '../lib/workout';
  const when = (iso: string) => new Date(iso).toLocaleString(i18n.lang, { weekday: 'short', day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
</script>

<div class="page">
  <header class="page-head"><div><h1>{t('hist.title')}</h1><p>{t('hist.sub')}</p></div></header>
  {#if app.sessions.length}
    <div class="card list">
      <div class="r head kicker"><span></span><span>{t('common.name')}</span><span>{t('common.duration')}</span><span>{t('common.power')}</span><span>NP</span><span>TSS</span><span>{t('common.distance')}</span></div>
      {#each app.sessions as s (s.id)}
        <button class="r" onclick={() => go(`summary/${s.id}`)}>
          <span class="muted">{when(s.started_at)}</span><b>{s.workout_name}</b><span>{fmtDuration(s.summary?.duration_s ?? 0)}</span>
          <span><b class="num">{Math.round(s.summary?.avg_power_w ?? 0)}</b> W</span><span>{s.summary?.normalized_power_w ? Math.round(s.summary.normalized_power_w) + ' W' : '–'}</span>
          <span>{s.summary?.tss ? Math.round(s.summary.tss) : '–'}</span><span>{fmtKm(s.summary?.distance_m ?? 0)} km</span>
        </button>
      {/each}
    </div>
  {:else}<p class="muted">{t('hist.empty')}</p>{/if}
</div>

<style>
  .list { padding: 8px; } .r { all: unset; box-sizing: border-box; width: 100%; display: grid; grid-template-columns: 170px 1fr 90px 100px 90px 70px 90px; gap: 12px; align-items: center; padding: 14px 16px; border-radius: 12px; cursor: pointer; }
  .r:not(.head):hover { background: var(--surface-selected); } .r:focus-visible { outline: 2px solid var(--accent); } .head { cursor: default; padding-bottom: 6px; } .num { font-size: 22px; }
</style>
