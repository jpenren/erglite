<script lang="ts">
  import Icon from '../lib/components/Icon.svelte';
  import WorkoutCard from '../lib/components/WorkoutCard.svelte';
  import { app, freeRideWorkout, go, startRide } from '../lib/state.svelte';
  import { backend } from '../lib/backend';
  import { i18n, t } from '../lib/i18n.svelte';
  import { fmtDuration } from '../lib/workout';
  let pct = $state(65);
  const watts = $derived(Math.round((app.profile.ftp_w * pct) / 100));
  const featured = $derived(app.workouts.slice(0, 3));
  const rides = $derived(app.sessions.slice(0, 3));
  const day = (iso: string) => new Date(iso).toLocaleDateString(i18n.lang, { weekday: 'short', day: 'numeric', month: 'short' });
  async function duplicate(id: string) { const r = app.workouts.find((x) => x.id === id); if (r) { await backend.saveWorkout(null, { ...r.workout, name: `${r.workout.name} (copy)` }); app.workouts = await backend.listWorkouts(); } }
</script>

<div class="page">
  <header class="hero">
    <div>
      <div class="kicker">ErgLite</div>
      <h1>{t('home.greeting')}</h1>
      <p class="muted">{t('home.sub')}</p>
    </div>
  </header>

  <section class="top grid">
    <div class="card quick">
      <div class="kicker">{t('home.quick')}</div>
      <h2>{t('home.freeRide')}</h2>
      <p class="muted">{t('home.freeRideSub')}</p>
      <div class="dial">
        <div class="num big">{watts}<span>W</span></div>
        <input type="range" min="40" max="110" step="1" bind:value={pct} aria-label={t('home.ftpHint')} />
        <div class="muted small">{pct} % FTP</div>
      </div>
      <button class="btn btn-primary btn-lg" onclick={() => startRide(null, freeRideWorkout(pct))}><Icon name="play" /> {t('home.start')}</button>
    </div>
    <div class="card trainer">
      <div class="kicker">{t('home.trainer')}</div>
      <div class="state">
        <span class="dot" class:ok={app.trainer.state === 'connected'}></span>
        <h2>{app.trainer.caps?.name ?? t('home.notConnected')}</h2>
      </div>
      {#if app.trainer.caps}
        <div class="chips">
          {#if app.trainer.caps.supports_erg}<span class="chip">{t('dev.cap.erg')}</span>{/if}
          {#if app.trainer.caps.supports_simulation}<span class="chip">{t('dev.cap.sim')}</span>{/if}
          {#if app.trainer.caps.reports_cadence}<span class="chip">{t('dev.cap.cadence')}</span>{/if}
        </div>
      {:else}<p class="muted">{t('home.connectCta')}</p>{/if}
      <button class="btn" onclick={() => go('devices')}><Icon name="devices" size={18} /> {t('nav.devices')}</button>
    </div>
  </section>

  <section>
    <div class="head"><h2>{t('home.recent')}</h2><button class="btn btn-ghost btn-sm" onclick={() => go('library')}>{t('nav.library')} →</button></div>
    <div class="cards">
      {#each featured as rec (rec.id)}
        <WorkoutCard {rec} onride={() => startRide(rec.id, rec.workout)} onedit={() => go(rec.workout.kind === 'route' ? `route/${rec.id}` : `editor/${rec.id}`)} onduplicate={() => duplicate(rec.id)} ondelete={async () => { await backend.deleteWorkout(rec.id); app.workouts = await backend.listWorkouts(); }} />
      {/each}
    </div>
  </section>

  <section>
    <div class="head"><h2>{t('home.history')}</h2><button class="btn btn-ghost btn-sm" onclick={() => go('history')}>{t('nav.history')} →</button></div>
    {#if rides.length}
      <div class="card list">
        {#each rides as s (s.id)}
          <button class="ride" onclick={() => go(`summary/${s.id}`)}>
            <span class="d muted">{day(s.started_at)}</span>
            <b>{s.workout_name}</b>
            <span class="muted">{fmtDuration(s.summary?.duration_s ?? 0)}</span>
            <span><b class="num">{Math.round(s.summary?.avg_power_w ?? 0)}</b> W</span>
            <span><b class="num">{Math.round(s.summary?.tss ?? 0)}</b> TSS</span>
          </button>
        {/each}
      </div>
    {:else}<p class="muted">{t('home.noRides')}</p>{/if}
  </section>
</div>

<style>
  .hero { margin-bottom: 26px; } h1 { font-size: 40px; margin: 6px 0; }
  .top { grid-template-columns: 1.4fr 1fr; margin-bottom: 36px; }
  .quick, .trainer { padding: 24px 28px; display: flex; flex-direction: column; gap: 10px; }
  .quick { background: radial-gradient(500px 220px at 100% 0, color-mix(in srgb, var(--accent) 14%, transparent), transparent 70%), linear-gradient(180deg, var(--surface-raised), var(--surface)); }
  .dial { display: grid; grid-template-columns: auto 1fr auto; align-items: center; gap: 20px; margin: 8px 0 12px; }
  .big { font-size: 64px; font-weight: 700; } .big span { font-size: 22px; color: var(--text-muted); margin-left: 6px; } .small { font-size: 14px; }
  input[type='range'] { padding: 0; min-height: 28px; accent-color: var(--accent); background: transparent; border: none; }
  .state { display: flex; align-items: center; gap: 12px; } .chips { display: flex; flex-wrap: wrap; gap: 8px; } .trainer .btn { align-self: flex-start; margin-top: auto; }
  section { margin-bottom: 36px; } .head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 18px; }
  .list { padding: 6px; } .ride { all: unset; box-sizing: border-box; display: grid; grid-template-columns: 110px 1fr 90px 110px 110px; gap: 12px; align-items: center; padding: 14px 18px; border-radius: 12px; cursor: pointer; width: 100%; }
  .ride:hover { background: var(--surface-selected); } .ride:focus-visible { outline: 2px solid var(--accent); } .ride .num { font-size: 22px; }
</style>
