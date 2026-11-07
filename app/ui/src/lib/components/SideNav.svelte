<script lang="ts">
  import Icon from './Icon.svelte';
  import { app, go, router } from '../state.svelte';
  import { t, type Key } from '../i18n.svelte';
  const items: { id: string; icon: 'home' | 'devices' | 'library' | 'route' | 'history' | 'settings'; label: Key }[] = [
    { id: 'home', icon: 'home', label: 'nav.home' },
    { id: 'devices', icon: 'devices', label: 'nav.devices' },
    { id: 'library', icon: 'library', label: 'nav.library' },
    { id: 'routes', icon: 'route', label: 'nav.routes' },
    { id: 'history', icon: 'history', label: 'nav.history' },
    { id: 'settings', icon: 'settings', label: 'nav.settings' },
  ];
  const active = (id: string) => router.name === id || (id === 'library' && router.name === 'editor') || (id === 'routes' && router.name === 'route');
</script>

<nav aria-label="Main">
  <div class="brand">
    <svg width="34" height="34" viewBox="0 0 34 34" aria-hidden="true">
      <defs><linearGradient id="bg" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="var(--accent-strong)" /><stop offset="1" stop-color="var(--accent)" /></linearGradient></defs>
      <rect width="34" height="34" rx="10" fill="url(#bg)" />
      <path d="M19 6L9 19h7l-1 9 10-13h-7z" fill="var(--on-accent)" />
    </svg>
    <span>ErgLite</span>
  </div>
  <ul>
    {#each items as it (it.id)}
      <li>
        <button class:active={active(it.id)} onclick={() => go(it.id)} aria-current={active(it.id) ? 'page' : undefined}>
          <Icon name={it.icon} /> {t(it.label)}
        </button>
      </li>
    {/each}
  </ul>
  <button class="status" onclick={() => go('devices')}>
    <span class="dot" class:ok={app.trainer.state === 'connected'} class:warn={app.trainer.state === 'connecting'}></span>
    <span class="txt">
      <b>{app.trainer.caps?.name ?? t('home.notConnected')}</b>
      <small>{app.trainer.state === 'connected' ? 'Bluetooth · FTMS' : t('home.connectCta')}</small>
    </span>
  </button>
</nav>

<style>
  nav { width: var(--nav-width); flex-shrink: 0; background: var(--surface-alt); border-right: 1px solid var(--border-subtle); padding: 22px 14px; display: flex; flex-direction: column; gap: 22px; }
  .brand { display: flex; align-items: center; gap: 12px; padding: 0 8px; font-weight: 700; font-size: 22px; letter-spacing: -0.01em; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; gap: 4px; }
  li button { all: unset; box-sizing: border-box; width: 100%; display: flex; align-items: center; gap: 12px; padding: 11px 14px; border-radius: 12px; cursor: pointer; color: var(--text-secondary); font-weight: 550; position: relative; transition: background .15s, color .15s; }
  li button:hover { background: var(--surface-selected); color: var(--text); }
  li button:focus-visible { outline: 2px solid var(--accent); }
  li button.active { background: var(--surface-selected); color: var(--text); }
  li button.active::before { content: ''; position: absolute; left: -14px; top: 10px; bottom: 10px; width: 4px; border-radius: 0 4px 4px 0; background: var(--accent); }
  .status { all: unset; box-sizing: border-box; margin-top: auto; display: flex; align-items: center; gap: 12px; padding: 12px 14px; border-radius: 14px; background: var(--surface); border: 1px solid var(--border-subtle); cursor: pointer; }
  .status:focus-visible { outline: 2px solid var(--accent); }
  .txt { display: flex; flex-direction: column; min-width: 0; }
  .txt b { font-size: 14px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .txt small { color: var(--text-muted); font-size: 12px; }
</style>
