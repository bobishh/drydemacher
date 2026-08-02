<script lang="ts">
  let {
    src = null,
    alt,
    state = undefined,
    label = 'NO PREVIEW',
    class: className = '',
  }: {
    src?: string | null;
    alt: string;
    state?: 'ready' | 'empty' | 'loading' | 'error' | undefined;
    label?: string;
    class?: string;
  } = $props();

  const effectiveState = $derived(state ?? (src ? 'ready' : 'empty'));
</script>

<figure class={`preview-frame ${className}`} data-preview-state={effectiveState}>
  {#if effectiveState === 'ready' && src}
    <img src={src} {alt} />
  {:else}
    <figcaption>{effectiveState === 'loading' ? 'LOADING PREVIEW...' : effectiveState === 'error' ? 'PREVIEW UNAVAILABLE' : label}</figcaption>
  {/if}
</figure>

<style>
  .preview-frame {
    display: grid;
    width: 100%;
    aspect-ratio: 16 / 9;
    place-items: center;
    margin: 0;
    border-bottom: 1px solid var(--bg-300);
    background: #000;
    overflow: hidden;
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    opacity: 0.78;
  }

  figcaption {
    color: var(--bg-400);
    font-size: 0.58rem;
    letter-spacing: 0.1em;
  }
</style>
