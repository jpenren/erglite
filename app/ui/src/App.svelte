<script lang="ts">
  import { onMount } from 'svelte';
  import SideNav from './lib/components/SideNav.svelte';
  import { answer, answerGpx, app, dialog, gpxDialog, init, router } from './lib/state.svelte';
  import { t } from './lib/i18n.svelte';
  import Home from './screens/Home.svelte';
  import Devices from './screens/Devices.svelte';
  import Library from './screens/Library.svelte';
  import WorkoutEditor from './screens/WorkoutEditor.svelte';
  import RouteEditor from './screens/RouteEditor.svelte';
  import Session from './screens/Session.svelte';
  import Summary from './screens/Summary.svelte';
  import History from './screens/History.svelte';
  import Settings from './screens/Settings.svelte';
  onMount(() => { void init(); });
  const fullscreen = $derived(router.name === 'session');
</script>

{#if app.ready}
  <div class="shell">
    {#if !fullscreen}<SideNav />{/if}
    <main>
      {#key router.name + router.id}
        <div class="screen">
          {#if router.name === 'devices'}<Devices />
          {:else if router.name === 'library'}<Library kind="time" />
          {:else if router.name === 'routes'}<Library kind="route" />
          {:else if router.name === 'editor'}<WorkoutEditor id={router.id} />
          {:else if router.name === 'route'}<RouteEditor id={router.id} />
          {:else if router.name === 'session'}<Session />
          {:else if router.name === 'summary'}<Summary id={router.id} />
          {:else if router.name === 'history'}<History />
          {:else if router.name === 'settings'}<Settings />
          {:else}<Home />{/if}
        </div>
      {/key}
    </main>
  </div>
  {#if dialog.open}
    <div class="backdrop" role="presentation" onclick={() => answer(false)}>
      <div class="modal card" role="alertdialog" aria-modal="true" aria-label={dialog.message} tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === 'Escape' && answer(false)}>
        <p>{dialog.message}</p>
        <div class="acts"><button class="btn" onclick={() => answer(false)}>{t('common.cancel')}</button><button class="btn btn-primary" onclick={() => answer(true)}>{t('common.ok')}</button></div>
      </div>
    </div>
  {/if}
  {#if app.toast}<div class="toast" role="status">{app.toast}</div>{/if}
  {#if gpxDialog.open}
    <div class="backdrop" role="presentation" onclick={() => answerGpx('cancel')}>      <div class="modal card modal-wide" role="alertdialog" aria-modal="true" aria-label={gpxDialog.message} tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.key === 'Escape' && answerGpx('cancel')}>        <p>{gpxDialog.message}</p>        <div class="acts">          <button class="btn" onclick={() => answerGpx('cancel')}>{t('common.cancel')}</button>          <button class="btn" onclick={() => answerGpx('original')}>{t('lib.gpxImportOriginal')}</button>          <button class="btn btn-primary" onclick={() => answerGpx('simplify')}>{t('lib.gpxSimplify')}</button>        </div>      </div>    </div>  {/if}
{/if}

<style>
  .shell { display: flex; height: 100%; }
  main { flex: 1; min-width: 0; height: 100%; }
  .screen { height: 100%; animation: enter .28s ease-out; }
  @keyframes enter { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: none; } }
  .backdrop { position: fixed; inset: 0; background: color-mix(in srgb, var(--bg) 70%, transparent); backdrop-filter: blur(6px); display: grid; place-items: center; z-index: 60; }
  .modal { padding: 26px 30px; display: flex; flex-direction: column; gap: 22px; max-width: 420px; font-size: 18px; }
  .modal-wide { max-width: 560px; }
  .acts { display: flex; justify-content: flex-end; gap: 10px; }
  .toast { position: fixed; bottom: 28px; left: 50%; transform: translateX(-50%); background: var(--text); color: var(--bg); padding: 12px 22px; border-radius: 999px; font-weight: 600; box-shadow: var(--shadow); z-index: 50; }
</style>
