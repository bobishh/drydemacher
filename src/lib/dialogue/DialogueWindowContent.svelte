<script lang="ts">
  import type { ComponentProps } from 'svelte';
  import PromptPanel from '../PromptPanel.svelte';

  type PromptPanelProps = ComponentProps<typeof PromptPanel>;

  let {
    rememberLayout,
    onRememberLayoutChange,
    activeThreadId,
    promptProps,
    activeVersionId = $bindable(null),
  }: {
    rememberLayout: boolean;
    onRememberLayoutChange?: (remember: boolean) => void;
    activeThreadId: string | null;
    promptProps: PromptPanelProps;
    activeVersionId?: string | null;
  } = $props();
</script>

<div class="dialogue-content">
  <div class="dialogue-toolbar">
    <label class="dialogue-toolbar__remember">
      <input
        type="checkbox"
        checked={rememberLayout}
        onchange={(event) => onRememberLayoutChange?.((event.currentTarget as HTMLInputElement).checked)}
      />
      <span>Remember layout</span>
    </label>
  </div>
  {#key activeThreadId ?? 'new-thread'}
    <PromptPanel {...promptProps} bind:activeVersionId />
  {/key}
</div>

<style>
  .dialogue-content { flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden; }

  .dialogue-toolbar {
    flex: 0 0 auto;
    padding: 6px 10px;
    border-bottom: 1px solid var(--bg-300);
    background: color-mix(in srgb, var(--bg-200) 90%, transparent);
  }

  .dialogue-toolbar__remember {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    font-size: 0.65rem;
    color: var(--text-dim);
    font-family: var(--font-mono);
  }
</style>
