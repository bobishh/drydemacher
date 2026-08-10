<script lang="ts">
  import type { DockIconId } from './dock';

  let { icon }: { icon: DockIconId } = $props();
</script>

<svg class="dock-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
  {#if icon === 'projects'}
    <path d="M3 7h7l2 2h9v11H3V7Z M6 12h12 M7 15h8" />
  {:else if icon === 'params'}
    <path d="M4 6h16 M4 12h16 M4 18h16 M8 4v4 M15 10v4 M11 16v4" />
  {:else if icon === 'dialogue'}
    <path d="M4 5h16v12H10l-6 4V5Z M8 10h8 M8 13h6" />
  {:else if icon === 'sketch'}
    <path d="M5 18A9 9 0 0 1 18 5 M5 18h14 M5 18v-4 M18 5v4 M8 15l7-7" />
    <circle cx="5" cy="18" r="1.25" /><circle cx="18" cy="5" r="1.25" />
  {:else if icon === 'code'}
    <path d="m9 6-5 6 5 6 M15 6l5 6-5 6 M14 3l-4 18" />
  {:else if icon === 'docs'}
    <path d="M3 5h7a3 3 0 0 1 2 1 3 3 0 0 1 2-1h7v15h-7a3 3 0 0 0-2 1 3 3 0 0 0-2-1H3V5Z M12 6v15" />
  {:else if icon === 'library'}
    <path d="M4 4h7v16H4V4Z M13 4h7v16h-7V4Z M7 8v8 M16 8v8" />
  {:else if icon === 'capture'}
    <path d="M4 8h4l2-3h4l2 3h4v10H4V8Z M8 12h8 M12 9v6" />
  {:else if icon === 'analysis'}
    <path d="M4 19 8 5l4 14 4-14 4 14 M6 13h12 M9 19h6" />
  {:else if icon === 'draw'}
    <path d="m5 18 1-5 9-9 5 5-9 9-6 1Z M14 6l4 4 M4 21h16" />
  {:else if icon === 'terminal'}
    <path d="M3 5h18v14H3V5Z M7 9l3 3-3 3 M12 15h5" />
  {:else}
    <path d="M12 3v3 M12 18v3 M3 12h3 M18 12h3 M5.6 5.6l2.1 2.1 M16.3 16.3l2.1 2.1 M18.4 5.6l-2.1 2.1 M7.7 16.3l-2.1 2.1 M8 12a4 4 0 1 0 8 0 4 4 0 0 0-8 0Z" />
  {/if}
</svg>

<style>
  .dock-icon {
    width: 22px;
    height: 22px;
    flex: 0 0 auto;
    fill: none;
    stroke: currentColor;
    stroke-width: 1.8;
    stroke-linecap: square;
    stroke-linejoin: miter;
  }
</style>
