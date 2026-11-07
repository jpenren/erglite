<script lang="ts">
  // Minimal 24x24 stroke icons (MIT-licensed shapes drawn for this project).
  type Name = 'home' | 'devices' | 'library' | 'route' | 'history' | 'settings' | 'play' | 'pause' | 'skip' | 'plus' | 'minus' | 'trash' | 'copy' | 'edit' | 'bolt' | 'heart' | 'download' | 'upload' | 'check' | 'back' | 'up' | 'down' | 'mountain' | 'clock' | 'stop';
  let { name, size = 20 }: { name: Name; size?: number } = $props();
  const paths: Record<Name, string> = {
    home: 'M3 11l9-8 9 8M5 10v10h5v-6h4v6h5V10',
    devices: 'M7 7l10 10-5 5V2l5 5L7 17',
    library: 'M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z',
    route: 'M3 19l6-9 4 5 3-4 5 8H3z',
    mountain: 'M3 19l6-9 4 5 3-4 5 8H3z',
    history: 'M3 12a9 9 0 1 0 3-6.7M3 4v5h5M12 8v4l3 2',
    clock: 'M12 7v5l3 2M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0z',
    settings: 'M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM19 12a7 7 0 0 0-.1-1.2l2-1.6-2-3.4-2.4 1a7 7 0 0 0-2-1.2L14 3h-4l-.5 2.6a7 7 0 0 0-2 1.2l-2.4-1-2 3.4 2 1.6A7 7 0 0 0 5 12c0 .4 0 .8.1 1.2l-2 1.6 2 3.4 2.4-1a7 7 0 0 0 2 1.2L10 21h4l.5-2.6a7 7 0 0 0 2-1.2l2.4 1 2-3.4-2-1.6c.1-.4.1-.8.1-1.2z',
    play: 'M7 4l13 8-13 8z', pause: 'M8 4v16M16 4v16', stop: 'M6 6h12v12H6z', skip: 'M5 4l10 8-10 8zM19 4v16',
    plus: 'M12 5v14M5 12h14', minus: 'M5 12h14', trash: 'M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3',
    copy: 'M9 9h11v11H9zM5 15V4h11', edit: 'M4 20h4L19 9l-4-4L4 16zM14 6l4 4',
    bolt: 'M13 2L4 14h7l-1 8 9-12h-7z', heart: 'M12 21s-8-5-8-11a4.5 4.5 0 0 1 8-2.5A4.5 4.5 0 0 1 20 10c0 6-8 11-8 11z',
    download: 'M12 3v12M7 10l5 5 5-5M4 20h16', upload: 'M12 15V3M7 8l5-5 5 5M4 20h16', check: 'M4 12l5 5L20 6',
    back: 'M15 5l-7 7 7 7', up: 'M6 15l6-6 6 6', down: 'M6 9l6 6 6-6',
  };
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.9" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  <path d={paths[name]} />
</svg>
