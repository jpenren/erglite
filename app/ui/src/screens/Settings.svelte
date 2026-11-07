<script lang="ts">
  import { app, applyTheme, saveProfile, toast } from '../lib/state.svelte';
  import { t } from '../lib/i18n.svelte';
  import type { Profile } from '../lib/types';
  let p = $state<Profile>({ ...app.profile });
  let timer: ReturnType<typeof setTimeout> | undefined;
  function changed() {
    clearTimeout(timer);
    timer = setTimeout(async () => { await saveProfile({ ...p, ftp_w: Math.min(Math.max(p.ftp_w, 50), 600) }); toast(t('set.saved')); }, 500);
  }
</script>

<div class="page">
  <header class="page-head"><div><h1>{t('set.title')}</h1></div></header>
  <div class="two">
    <section class="card pad">
      <h2>{t('set.profile')}</h2>
      <div class="fields">
        <label class="field">{t('set.ftp')}<input type="number" min="50" max="600" bind:value={p.ftp_w} oninput={changed} /></label>
        <label class="field">{t('set.weight')}<input type="number" step="0.5" min="30" max="200" bind:value={p.weight_kg} oninput={changed} /></label>
        <label class="field">{t('set.bike')}<input type="number" step="0.5" min="5" max="30" bind:value={p.bike_kg} oninput={changed} /></label>
        <label class="field">{t('set.maxHr')}<input type="number" min="100" max="230" bind:value={p.max_hr} oninput={changed} /></label>
      </div>
      <h2>{t('set.physics')}</h2>
      <div class="fields">
        <label class="field">{t('set.crr')}<input type="number" step="0.0005" min="0.001" max="0.02" bind:value={p.crr} oninput={changed} /></label>
        <label class="field">{t('set.cda')}<input type="number" step="0.01" min="0.15" max="0.9" bind:value={p.cda_m2} oninput={changed} /></label>
      </div>
    </section>
    <section class="card pad">
      <h2>{t('set.language')}</h2>
      <div class="seg" role="group" aria-label={t('set.language')}>
        <button class:on={p.language === 'en'} onclick={() => { p.language = 'en'; changed(); }}>English</button>
        <button class:on={p.language === 'es'} onclick={() => { p.language = 'es'; changed(); }}>Español</button>
      </div>
      <h2>{t('set.theme')}</h2>
      <div class="seg" role="group" aria-label={t('set.theme')}>
        <button class:on={app.theme === 'dark'} onclick={() => applyTheme('dark')}>{t('set.dark')}</button>
        <button class:on={app.theme === 'light'} onclick={() => applyTheme('light')}>{t('set.light')}</button>
      </div>
      <h2>{t('set.data')}</h2>
      <p class="muted">{t('set.dataNote')}</p>
    </section>
  </div>
</div>

<style>
  .two { display: grid; grid-template-columns: 1.2fr 1fr; gap: 20px; align-items: start; } .pad { padding: 24px 28px; display: flex; flex-direction: column; gap: 16px; }
  .fields { display: grid; grid-template-columns: 1fr 1fr; gap: 14px; }
  .seg { display: inline-flex; background: var(--bg); border: 1px solid var(--border); border-radius: 12px; padding: 4px; align-self: flex-start; }
  .seg button { all: unset; cursor: pointer; padding: 9px 20px; border-radius: 9px; font-weight: 600; color: var(--text-muted); } .seg button:focus-visible { outline: 2px solid var(--accent); }
  .seg button.on { background: var(--surface-selected); color: var(--text); box-shadow: 0 1px 0 rgba(255,255,255,.05) inset; }
</style>
