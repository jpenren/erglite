<script lang="ts">
  import Icon from '../lib/components/Icon.svelte';
  import WorkoutCard from '../lib/components/WorkoutCard.svelte';
  import { app, ask, askGpx, go, refresh, startRide, toast } from '../lib/state.svelte';
  import { backend } from '../lib/backend';
  import { t } from '../lib/i18n.svelte';
  let { kind }: { kind: 'time' | 'route' } = $props();
  const items = $derived(app.workouts.filter((r) => (kind === 'route' ? r.workout.kind === 'route' : r.workout.kind !== 'route')));
  let fileInput: HTMLInputElement | undefined = $state();
  async function importFile(e: Event) {
    const f = (e.currentTarget as HTMLInputElement).files?.[0];
    if (!f) return;
    const text = await f.text();
    try {
      let simplifyEveryM: number | undefined;
      if (f.name.toLowerCase().endsWith('.gpx')) {
        const parser = new DOMParser();
        const doc = parser.parseFromString(text, 'application/xml');
        // Handle GPX namespace
        const trkpts = doc.evaluate('//*[local-name()="trkpt"]', doc, null, XPathResult.ORDERED_NODE_SNAPSHOT_TYPE, null);
        const pts = trkpts.snapshotLength;
        if (pts > 500) {
          const choice = await askGpx(`${t('lib.gpxManyPoints').replace('{pts}', String(pts))}\n${t('lib.gpxSimplifyPrompt')}`);
          if (choice === 'simplify') simplifyEveryM = 500;
          // 'original' means simplifyEveryM stays undefined
          if (choice === 'cancel') return;
        }
      }
      const rec = await backend.importWorkout(f.name, text, simplifyEveryM);
      await refresh();
      toast(`${rec.workout.name} ✓`);
    } catch (err) { toast(String(err)); }
    (e.currentTarget as HTMLInputElement).value = '';
  }
  async function duplicate(id: string) { const r = app.workouts.find((x) => x.id === id); if (r) { await backend.saveWorkout(null, { ...r.workout, name: `${r.workout.name} (copy)` }); await refresh(); } }
  async function remove(id: string) { if (await ask(t('lib.confirmDelete'))) { await backend.deleteWorkout(id); await refresh(); } }
</script>

<div class="page">
  <header class="page-head">
    <div><h1>{kind === 'route' ? t('nav.routes') : t('lib.title')}</h1><p>{t('lib.sub')}</p></div>
    <div class="row">
      <input bind:this={fileInput} type="file" accept={kind === 'route' ? '.gpx' : '.zwo,.gpx'} hidden onchange={importFile} />
      <button class="btn" onclick={() => fileInput?.click()}><Icon name="upload" size={18} /> {t('lib.import')}</button>
      <button class="btn btn-primary" onclick={() => go(kind === 'route' ? 'route/new' : 'editor/new')}><Icon name="plus" size={18} /> {kind === 'route' ? t('lib.newRoute') : t('lib.newWorkout')}</button>
    </div>
  </header>
  {#if items.length}
    <div class="cards">
      {#each items as rec (rec.id)}
        <WorkoutCard {rec} onride={() => startRide(rec.id, rec.workout)} onedit={() => go(rec.workout.kind === 'route' ? `route/${rec.id}` : `editor/${rec.id}`)} onduplicate={() => duplicate(rec.id)} ondelete={() => remove(rec.id)} />
      {/each}
    </div>
  {:else}<p class="muted">{t('lib.empty')}</p>{/if}
</div>

<style>.cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 18px; }</style>