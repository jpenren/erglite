<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../lib/components/Icon.svelte';
  import { app, connectTrainer, disconnectTrainer, toast } from '../lib/state.svelte';
  import { backend } from '../lib/backend';
  import { t } from '../lib/i18n.svelte';
  import type { Device } from '../lib/types';
  let devices = $state<Device[]>([]);
  let scanning = $state(false);
  async function scan() { scanning = true; try { devices = await backend.scanDevices(); } finally { scanning = false; } }
  onMount(() => { void scan(); });
  const trainers = $derived(devices.filter((d) => d.role === 'trainer'));
  const sensors = $derived(devices.filter((d) => d.role !== 'trainer'));
  const bars = (rssi: number | null) => (rssi == null ? 0 : rssi > -60 ? 4 : rssi > -70 ? 3 : rssi > -80 ? 2 : 1);
  async function connect(d: Device) { try { await connectTrainer(d.id); toast(`${d.name} ✓`); } catch (e) { toast(String(e)); } }
</script>

<div class="page">
  <header class="page-head"><div><h1>{t('dev.title')}</h1>{#if backend.kind === 'mock'}<p>{t('dev.simNote')}</p>{/if}</div>
    <button class="btn" onclick={scan} disabled={scanning}>{scanning ? t('dev.scanning') : t('dev.scan')}</button></header>

  <section class="card hero">
    <div class="kicker">{t('dev.trainer')}</div>
    {#if app.trainer.caps}
      <div class="row between">
        <div><h2><span class="dot ok"></span> {app.trainer.caps.name}</h2><p class="muted">Bluetooth · FTMS · {t('dev.signalGood')}</p></div>
        <button class="btn" onclick={disconnectTrainer}>{t('common.disconnect')}</button>
      </div>
      <div class="chips">
        {#if app.trainer.caps.supports_erg}<span class="chip"><Icon name="bolt" size={14} /> {t('dev.cap.erg')}</span>{/if}
        {#if app.trainer.caps.supports_simulation}<span class="chip"><Icon name="mountain" size={14} /> {t('dev.cap.sim')}</span>{/if}
        {#if app.trainer.caps.reports_cadence}<span class="chip">{t('dev.cap.cadence')}</span>{/if}
        {#if app.trainer.caps.reports_power}<span class="chip">{t('dev.cap.power')}</span>{/if}
        {#if app.trainer.caps.reports_heart_rate}<span class="chip">{t('dev.cap.hr')}</span>{/if}
      </div>
    {:else}<p class="muted">{t('dev.none')}</p>{/if}
  </section>

  <div class="two grid">
    <section class="card pad"><div class="kicker">{t('dev.found')} · {t('dev.trainer')}</div>
      {#each trainers as d (d.id)}
        <div class="item"><div><b>{d.name}</b><div class="muted sm">{t('dev.signal')} {'▮'.repeat(bars(d.rssi))}{'▯'.repeat(4 - bars(d.rssi))}</div></div>
          <button class="btn btn-primary btn-sm" disabled={app.trainer.state === 'connecting' || app.trainer.caps?.name === d.name} onclick={() => connect(d)}>{t('common.connect')}</button></div>
      {:else}<p class="muted">{scanning ? t('dev.scanning') : t('dev.none')}</p>{/each}
    </section>
    <section class="card pad"><div class="kicker">{t('dev.sensors')}</div>
      {#each sensors as d (d.id)}
        <div class="item"><div><b>{d.name}</b><div class="muted sm">{d.role === 'heart_rate' ? t('dev.cap.hr') : t('dev.cap.power')} · {t('dev.signal')} {'▮'.repeat(bars(d.rssi))}{'▯'.repeat(4 - bars(d.rssi))}</div></div>
          <button class="btn btn-sm">{t('common.connect')}</button></div>
      {:else}<p class="muted">{scanning ? t('dev.scanning') : t('dev.none')}</p>{/each}
    </section>
  </div>
  <p class="muted note">{t('dev.busy')}</p>
</div>

<style>
  .hero { padding: 24px 28px; margin-bottom: 20px; display: flex; flex-direction: column; gap: 12px; } .between { justify-content: space-between; }
  h2 { display: flex; align-items: center; gap: 10px; } .chips { display: flex; flex-wrap: wrap; gap: 8px; }
  .two { grid-template-columns: 1fr 1fr; } .pad { padding: 20px 24px; } .item { display: flex; justify-content: space-between; align-items: center; padding: 14px 0; border-top: 1px solid var(--border-subtle); margin-top: 8px; } .sm { font-size: 13px; letter-spacing: 0.02em; }
  .note { margin-top: 20px; max-width: 70ch; }
</style>
